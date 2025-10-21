use crate::Odometry;
use horus_core::error::HorusResult;
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

/// Encoder Node - Wheel/joint position feedback for odometry and control
///
/// Reads encoder data from wheels or joints and publishes position, velocity,
/// and odometry information for robot navigation and control feedback.
pub struct EncoderNode {
    publisher: Hub<Odometry>,

    // Configuration
    frame_id: String,
    child_frame_id: String,
    encoder_resolution: f64, // pulses per revolution
    wheel_radius: f64,       // wheel radius in meters
    gear_ratio: f64,         // gear ratio

    // State
    last_position: f64, // last encoder position
    last_time: u64,
    velocity: f64,
    total_distance: f64,

    // Simulation state
    sim_velocity: f64,
    sim_angular_velocity: f64,
}

impl EncoderNode {
    /// Create a new encoder node with default topic "odom"
    pub fn new() -> HorusResult<Self> {
        Self::new_with_topic("odom")
    }

    /// Create a new encoder node with custom topic
    pub fn new_with_topic(topic: &str) -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new(topic)?,
            frame_id: "odom".to_string(),
            child_frame_id: "base_link".to_string(),
            encoder_resolution: 1024.0, // 1024 pulses per revolution default
            wheel_radius: 0.1,          // 10cm wheel radius default
            gear_ratio: 1.0,            // Direct drive default
            last_position: 0.0,
            last_time: 0,
            velocity: 0.0,
            total_distance: 0.0,
            sim_velocity: 0.0,
            sim_angular_velocity: 0.0,
        })
    }

    /// Set encoder configuration parameters
    pub fn set_encoder_config(&mut self, resolution: f64, wheel_radius: f64, gear_ratio: f64) {
        self.encoder_resolution = resolution;
        self.wheel_radius = wheel_radius;
        self.gear_ratio = gear_ratio;
    }

    /// Set coordinate frame IDs
    pub fn set_frame_ids(&mut self, frame_id: &str, child_frame_id: &str) {
        self.frame_id = frame_id.to_string();
        self.child_frame_id = child_frame_id.to_string();
    }

    /// Get current velocity
    pub fn get_velocity(&self) -> f64 {
        self.velocity
    }

    /// Get total distance traveled
    pub fn get_total_distance(&self) -> f64 {
        self.total_distance
    }

    /// Reset encoder position and distance
    pub fn reset(&mut self) {
        self.last_position = 0.0;
        self.total_distance = 0.0;
        self.velocity = 0.0;
    }

    fn read_encoder_position(&self) -> f64 {
        // In a real implementation, this would read from actual encoder hardware
        // For simulation, generate synthetic encoder data
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as f64
            / 1000.0;

        // Simulate encoder position based on synthetic velocity
        current_time * self.sim_velocity
    }

    fn calculate_velocity(&mut self, current_position: f64, dt: f64) -> f64 {
        if dt > 0.0 {
            let position_delta = current_position - self.last_position;
            self.velocity = position_delta / dt;
            self.total_distance += position_delta.abs();
        } else {
            self.velocity = 0.0;
        }

        self.last_position = current_position;
        self.velocity
    }

    fn publish_odometry(&self, linear_velocity: f64, angular_velocity: f64) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        // Create odometry message (simplified - real implementation would calculate pose)
        let mut odom = Odometry::new();

        // Set frame information
        odom.frame_id = self
            .frame_id
            .clone()
            .into_bytes()
            .try_into()
            .unwrap_or([0; 32]);
        odom.child_frame_id = self
            .child_frame_id
            .clone()
            .into_bytes()
            .try_into()
            .unwrap_or([0; 32]);

        // Set velocities
        odom.twist.linear[0] = linear_velocity;
        odom.twist.angular[2] = angular_velocity;

        // Set timestamp
        odom.timestamp = current_time;

        let _ = self.publisher.send(odom, None);
    }

    /// Set simulation velocities (for testing without hardware)
    pub fn set_simulation_velocity(&mut self, linear: f64, angular: f64) {
        self.sim_velocity = linear;
        self.sim_angular_velocity = angular;
    }
}

impl Node for EncoderNode {
    fn name(&self) -> &'static str {
        "EncoderNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Calculate delta time
        let dt = if self.last_time > 0 {
            (current_time - self.last_time) as f64 / 1000.0
        } else {
            0.01 // 10ms default
        };
        self.last_time = current_time;

        // Read encoder position and calculate velocity
        let current_position = self.read_encoder_position();
        let linear_velocity = self.calculate_velocity(current_position, dt);

        // Publish odometry data
        self.publish_odometry(linear_velocity, self.sim_angular_velocity);
    }
}

// Default impl removed - use EncoderNode::new() instead which returns HorusResult
