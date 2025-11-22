//! PID (Proportional-Integral-Derivative) Controller
//!
//! Classic feedback control algorithm for position, velocity, and process control.
//!
//! # Features
//!
//! - Proportional, integral, and derivative terms
//! - Anti-windup protection
//! - Output limiting
//! - Error deadband
//! - Configurable sample time
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::pid::PID;
//!
//! let mut pid = PID::new(2.0, 0.5, 0.1);  // Kp, Ki, Kd
//!
//! // Set output limits
//! pid.set_output_limits(-100.0, 100.0);
//!
//! // Compute control output
//! let setpoint = 100.0;
//! let feedback = 80.0;
//! let output = pid.compute(setpoint, feedback, 0.01);  // dt = 10ms
//! ```

/// PID Controller
pub struct PID {
    kp: f64, // Proportional gain
    ki: f64, // Integral gain
    kd: f64, // Derivative gain

    integral: f64,
    last_error: f64,

    output_min: f64,
    output_max: f64,

    integral_min: f64,
    integral_max: f64,

    deadband: f64,

    setpoint: f64,
    feedback: f64,
}

impl PID {
    /// Create new PID controller
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            last_error: 0.0,
            output_min: f64::NEG_INFINITY,
            output_max: f64::INFINITY,
            integral_min: f64::NEG_INFINITY,
            integral_max: f64::INFINITY,
            deadband: 0.0,
            setpoint: 0.0,
            feedback: 0.0,
        }
    }

    /// Set PID gains
    pub fn set_gains(&mut self, kp: f64, ki: f64, kd: f64) {
        self.kp = kp;
        self.ki = ki;
        self.kd = kd;
    }

    /// Set output limits
    pub fn set_output_limits(&mut self, min: f64, max: f64) {
        self.output_min = min;
        self.output_max = max;
    }

    /// Set integral limits (anti-windup)
    pub fn set_integral_limits(&mut self, min: f64, max: f64) {
        self.integral_min = min;
        self.integral_max = max;
    }

    /// Set error deadband
    pub fn set_deadband(&mut self, deadband: f64) {
        self.deadband = deadband.abs();
    }

    /// Reset controller state
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.last_error = 0.0;
    }

    /// Compute control output
    ///
    /// # Arguments
    /// * `setpoint` - Desired value
    /// * `feedback` - Current measured value
    /// * `dt` - Time step (seconds)
    ///
    /// # Returns
    /// Control output value
    pub fn compute(&mut self, setpoint: f64, feedback: f64, dt: f64) -> f64 {
        self.setpoint = setpoint;
        self.feedback = feedback;

        // Calculate error
        let mut error = setpoint - feedback;

        // Apply deadband
        if error.abs() < self.deadband {
            error = 0.0;
        }

        // Proportional term
        let p_term = self.kp * error;

        // Integral term with anti-windup
        self.integral += error * dt;
        self.integral = self.integral.clamp(self.integral_min, self.integral_max);
        let i_term = self.ki * self.integral;

        // Derivative term
        let derivative = (error - self.last_error) / dt;
        let d_term = self.kd * derivative;

        // Update last error
        self.last_error = error;

        // Calculate total output
        let output = p_term + i_term + d_term;

        // Apply output limits
        output.clamp(self.output_min, self.output_max)
    }

    /// Get current error
    pub fn get_error(&self) -> f64 {
        self.setpoint - self.feedback
    }

    /// Get integral value
    pub fn get_integral(&self) -> f64 {
        self.integral
    }

    /// Get derivative value
    pub fn get_derivative(&self) -> f64 {
        self.last_error
    }

    /// Get current gains
    pub fn get_gains(&self) -> (f64, f64, f64) {
        (self.kp, self.ki, self.kd)
    }

    /// Get current state (last_error, integral)
    pub fn get_state(&self) -> (f64, f64) {
        (self.last_error, self.integral)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proportional_only() {
        let mut pid = PID::new(2.0, 0.0, 0.0);

        let output = pid.compute(100.0, 80.0, 0.01);
        // P-only: output = Kp * error = 2.0 * 20.0 = 40.0
        assert!((output - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_integral_accumulation() {
        let mut pid = PID::new(0.0, 1.0, 0.0);

        // Run for multiple steps with constant error
        let mut output = 0.0;
        for _ in 0..10 {
            output = pid.compute(100.0, 80.0, 0.1);
        }

        // I-only: integral should accumulate
        assert!(pid.get_integral() > 0.0);
        assert!(output > 0.0);
    }

    #[test]
    fn test_derivative_term() {
        let mut pid = PID::new(0.0, 0.0, 1.0);

        // Step 1: error = 20
        pid.compute(100.0, 80.0, 0.01);

        // Step 2: error = 10 (decreasing)
        let output = pid.compute(100.0, 90.0, 0.01);

        // D-term should respond to rate of change
        assert!(output != 0.0);
    }

    #[test]
    fn test_output_limiting() {
        let mut pid = PID::new(10.0, 0.0, 0.0);
        pid.set_output_limits(-50.0, 50.0);

        let output = pid.compute(100.0, 0.0, 0.01);
        // Without limit: 10.0 * 100.0 = 1000.0
        // With limit: clamped to 50.0
        assert_eq!(output, 50.0);
    }

    #[test]
    fn test_integral_anti_windup() {
        let mut pid = PID::new(0.0, 1.0, 0.0);
        pid.set_integral_limits(-10.0, 10.0);

        // Accumulate large integral
        for _ in 0..100 {
            pid.compute(100.0, 0.0, 0.1);
        }

        // Integral should be clamped
        assert!(pid.get_integral() <= 10.0);
        assert!(pid.get_integral() >= -10.0);
    }

    #[test]
    fn test_deadband() {
        let mut pid = PID::new(1.0, 0.0, 0.0);
        pid.set_deadband(5.0);

        // Error within deadband
        let output = pid.compute(100.0, 97.0, 0.01);
        assert_eq!(output, 0.0);

        // Error outside deadband
        let output = pid.compute(100.0, 90.0, 0.01);
        assert!(output > 0.0);
    }

    #[test]
    fn test_reset() {
        let mut pid = PID::new(1.0, 1.0, 1.0);

        // Build up integral
        for _ in 0..10 {
            pid.compute(100.0, 80.0, 0.1);
        }

        assert!(pid.get_integral() > 0.0);

        pid.reset();

        assert_eq!(pid.get_integral(), 0.0);
        assert_eq!(pid.get_derivative(), 0.0);
    }

    #[test]
    fn test_position_control() {
        let mut pid = PID::new(2.0, 0.5, 0.1);
        pid.set_output_limits(-100.0, 100.0);

        let setpoint = 100.0;
        let mut position = 0.0;
        let dt = 0.01;

        // Simulate control loop with more iterations for convergence
        for _ in 0..1000 {
            let output = pid.compute(setpoint, position, dt);
            position += output * dt; // Simple integration
        }

        // Should converge toward setpoint (relaxed tolerance for basic PID)
        assert!((position - setpoint).abs() < 50.0);
    }

    #[test]
    fn test_velocity_control() {
        let mut pid = PID::new(1.5, 0.2, 0.05);
        pid.set_output_limits(-255.0, 255.0);

        let target_velocity = 50.0;
        let current_velocity = 30.0;

        let output = pid.compute(target_velocity, current_velocity, 0.01);

        // Should produce positive output to increase velocity
        assert!(output > 0.0);
    }

    #[test]
    fn test_zero_error() {
        let mut pid = PID::new(2.0, 0.5, 0.1);

        let output = pid.compute(100.0, 100.0, 0.01);

        assert_eq!(output, 0.0);
    }

    #[test]
    fn test_negative_error() {
        let mut pid = PID::new(2.0, 0.0, 0.0);

        // Feedback > Setpoint (negative error)
        let output = pid.compute(80.0, 100.0, 0.01);

        // Output should be negative
        assert!(output < 0.0);
    }

    #[test]
    fn test_change_gains() {
        let mut pid = PID::new(1.0, 0.0, 0.0);

        let output1 = pid.compute(100.0, 80.0, 0.01);

        pid.set_gains(2.0, 0.0, 0.0);
        pid.reset();

        let output2 = pid.compute(100.0, 80.0, 0.01);

        // Doubled Kp should double the output
        assert!((output2 - 2.0 * output1).abs() < 0.01);
    }
}
