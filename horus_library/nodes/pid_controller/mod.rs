use crate::{MotorCommand, PidConfig};
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

/// PID Controller Node - Generic PID control implementation
///
/// Implements a PID controller that can be used for various control applications.
/// Subscribes to setpoint and feedback values, publishes control output.
pub struct PidControllerNode {
    // Publishers and Subscribers
    output_publisher: Hub<MotorCommand>,
    setpoint_subscriber: Hub<f32>,
    feedback_subscriber: Hub<f32>,
    config_subscriber: Hub<PidConfig>,

    // PID Parameters
    kp: f32, // Proportional gain
    ki: f32, // Integral gain
    kd: f32, // Derivative gain

    // PID State
    setpoint: f32,
    feedback: f32,
    last_error: f32,
    integral: f32,
    last_time: u64,

    // Configuration
    output_min: f32,
    output_max: f32,
    integral_min: f32,
    integral_max: f32,
    deadband: f32,

    // State
    is_initialized: bool,
    motor_id: u8,
}

impl PidControllerNode {
    /// Create a new PID controller node with default topics
    pub fn new() -> Result<Self> {
        Self::new_with_topics("setpoint", "feedback", "pid_output", "pid_config")
    }

    /// Create a new PID controller node with custom topics
    pub fn new_with_topics(
        setpoint_topic: &str,
        feedback_topic: &str,
        output_topic: &str,
        config_topic: &str,
    ) -> Result<Self> {
        Ok(Self {
            output_publisher: Hub::new(output_topic)?,
            setpoint_subscriber: Hub::new(setpoint_topic)?,
            feedback_subscriber: Hub::new(feedback_topic)?,
            config_subscriber: Hub::new(config_topic)?,

            // Default PID gains
            kp: 1.0,
            ki: 0.1,
            kd: 0.05,

            // PID state
            setpoint: 0.0,
            feedback: 0.0,
            last_error: 0.0,
            integral: 0.0,
            last_time: 0,

            // Default limits
            output_min: -100.0,
            output_max: 100.0,
            integral_min: -50.0,
            integral_max: 50.0,
            deadband: 0.01,

            is_initialized: false,
            motor_id: 0,
        })
    }

    /// Set PID gains
    pub fn set_gains(&mut self, kp: f32, ki: f32, kd: f32) {
        self.kp = kp;
        self.ki = ki;
        self.kd = kd;
    }

    /// Set output limits
    pub fn set_output_limits(&mut self, min: f32, max: f32) {
        self.output_min = min;
        self.output_max = max;
    }

    /// Set integral limits (anti-windup)
    pub fn set_integral_limits(&mut self, min: f32, max: f32) {
        self.integral_min = min;
        self.integral_max = max;
    }

    /// Set deadband (minimum error threshold)
    pub fn set_deadband(&mut self, deadband: f32) {
        self.deadband = deadband.abs();
    }

    /// Set motor ID for output commands
    pub fn set_motor_id(&mut self, motor_id: u8) {
        self.motor_id = motor_id;
    }

    /// Reset PID controller state
    pub fn reset(&mut self) {
        self.last_error = 0.0;
        self.integral = 0.0;
        self.last_time = 0;
    }

    /// Get current PID state
    pub fn get_state(&self) -> (f32, f32, f32, f32) {
        (self.setpoint, self.feedback, self.last_error, self.integral)
    }

    fn calculate_pid_output(&mut self, dt: f32) -> f32 {
        let error = self.setpoint - self.feedback;

        // Apply deadband
        let effective_error = if error.abs() < self.deadband {
            0.0
        } else {
            error
        };

        // Proportional term
        let proportional = self.kp * effective_error;

        // Integral term with anti-windup
        if dt > 0.0 {
            self.integral += effective_error * dt;
            self.integral = self.integral.clamp(self.integral_min, self.integral_max);
        }
        let integral = self.ki * self.integral;

        // Derivative term
        let derivative = if dt > 0.0 {
            self.kd * (effective_error - self.last_error) / dt
        } else {
            0.0
        };

        self.last_error = effective_error;

        // Combine terms and apply output limits
        let output = proportional + integral + derivative;
        output.clamp(self.output_min, self.output_max)
    }

    fn publish_output(&self, output: f32) {
        let _current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let motor_cmd = MotorCommand::velocity(self.motor_id, output as f64);

        let _ = self.output_publisher.send(motor_cmd, &mut None);
    }
}

impl Node for PidControllerNode {
    fn name(&self) -> &'static str {
        "PidControllerNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Calculate delta time
        let dt = if self.last_time > 0 {
            (current_time - self.last_time) as f32 / 1000.0
        } else {
            0.01 // 10ms default
        };
        self.last_time = current_time;

        // Check for new setpoint
        if let Some(new_setpoint) = self.setpoint_subscriber.recv(&mut None) {
            self.setpoint = new_setpoint;
        }

        // Check for new feedback
        if let Some(new_feedback) = self.feedback_subscriber.recv(&mut None) {
            self.feedback = new_feedback;
        }

        // Check for new PID configuration
        if let Some(config) = self.config_subscriber.recv(&mut None) {
            self.kp = config.kp as f32;
            self.ki = config.ki as f32;
            self.kd = config.kd as f32;
        }

        // Calculate and publish PID output
        if self.is_initialized || (self.setpoint != 0.0 || self.feedback != 0.0) {
            let output = self.calculate_pid_output(dt);
            self.publish_output(output);
            self.is_initialized = true;
        }
    }
}

// Default impl removed - use PidControllerNode::new() instead which returns HorusResult
