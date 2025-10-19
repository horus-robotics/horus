use crate::Imu;
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

/// IMU Node - Inertial Measurement Unit for orientation sensing
///
/// Reads accelerometer, gyroscope, and magnetometer data from IMU sensors
/// and publishes Imu messages with orientation and motion information.
pub struct ImuNode {
    publisher: Hub<Imu>,

    // Configuration
    frame_id: String,
    sample_rate: f32,

    // State
    is_initialized: bool,
    sample_count: u64,
    last_sample_time: u64,

    // Simulation state for synthetic data
    sim_angle: f32,
}

impl ImuNode {
    /// Create a new IMU node with default topic "imu"
    pub fn new() -> Self {
        Self::new_with_topic("imu")
    }

    /// Create a new IMU node with custom topic
    pub fn new_with_topic(topic: &str) -> Self {
        Self {
            publisher: Hub::new(topic).expect("Failed to create IMU hub"),
            frame_id: "imu_link".to_string(),
            sample_rate: 100.0, // 100 Hz default
            is_initialized: false,
            sample_count: 0,
            last_sample_time: 0,
            sim_angle: 0.0,
        }
    }

    /// Set frame ID for coordinate system
    pub fn set_frame_id(&mut self, frame_id: &str) {
        self.frame_id = frame_id.to_string();
    }

    /// Set IMU sample rate (Hz)
    pub fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate.max(1.0).min(1000.0);
    }

    /// Get actual sample rate (samples per second)
    pub fn get_actual_sample_rate(&self) -> f32 {
        if self.sample_count < 2 {
            return 0.0;
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let time_diff = current_time - self.last_sample_time;
        if time_diff > 0 {
            1000.0 / time_diff as f32
        } else {
            0.0
        }
    }

    fn initialize_imu(&mut self) -> bool {
        if self.is_initialized {
            return true;
        }

        // Initialize IMU hardware here
        self.is_initialized = true;
        true
    }

    fn read_imu_data(&mut self) -> Imu {
        // Generate synthetic IMU data for testing
        self.sim_angle += 0.01; // Slow rotation

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut imu = Imu::new();
        imu.orientation = [
            0.0,
            0.0,
            self.sim_angle.cos() as f64,
            self.sim_angle.sin() as f64,
        ];
        imu.angular_velocity = [0.01, 0.0, 0.0];
        imu.linear_acceleration = [0.0, 0.0, -9.81];
        imu.timestamp = current_time;
        imu
    }
}

impl Node for ImuNode {
    fn name(&self) -> &'static str {
        "ImuNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Initialize IMU on first tick
        if !self.is_initialized
            && !self.initialize_imu() {
                return;
            }

        // Read and publish IMU data
        let imu_data = self.read_imu_data();
        self.sample_count += 1;
        self.last_sample_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let _ = self.publisher.send(imu_data, None);
    }
}

impl Default for ImuNode {
    fn default() -> Self {
        Self::new()
    }
}
