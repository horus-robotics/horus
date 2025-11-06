use crate::{Imu, LaserScan, Odometry};
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

/// Localization Node - Robot position estimation using sensor fusion
///
/// Fuses odometry, IMU, and lidar data to estimate robot pose using
/// Extended Kalman Filter (EKF) for accurate localization.
pub struct LocalizationNode {
    pose_publisher: Hub<Odometry>,
    odometry_subscriber: Hub<Odometry>,
    imu_subscriber: Hub<Imu>,
    lidar_subscriber: Hub<LaserScan>,

    // State vector: [x, y, theta, vx, vy, omega]
    state: [f64; 6],
    covariance: [[f64; 6]; 6], // State covariance matrix

    // Sensor configurations
    process_noise: [[f64; 6]; 6],
    odometry_noise: [[f64; 3]; 3],
    imu_noise: [[f64; 3]; 3],

    // Localization parameters
    frame_id: String,
    child_frame_id: String,
    initial_pose_set: bool,

    // Timing
    last_update_time: u64,
    last_odometry_time: u64,
    last_imu_time: u64,

    // Reference landmarks for correction (simplified SLAM)
    landmarks: Vec<(f64, f64)>, // Known landmark positions
    landmark_detection_range: f64,
}

impl LocalizationNode {
    /// Create a new localization node with default topic "pose"
    pub fn new() -> Result<Self> {
        Self::new_with_topics("pose", "odom", "imu", "lidar_scan")
    }

    /// Create a new localization node with custom topics
    pub fn new_with_topics(
        pose_topic: &str,
        odom_topic: &str,
        imu_topic: &str,
        lidar_topic: &str,
    ) -> Result<Self> {
        let mut node = Self {
            pose_publisher: Hub::new(pose_topic)?,
            odometry_subscriber: Hub::new(odom_topic)?,
            imu_subscriber: Hub::new(imu_topic)?,
            lidar_subscriber: Hub::new(lidar_topic)?,

            state: [0.0; 6], // Initial state: all zeros
            covariance: [[0.0; 6]; 6],

            process_noise: [[0.0; 6]; 6],
            odometry_noise: [[0.0; 3]; 3],
            imu_noise: [[0.0; 3]; 3],

            frame_id: "map".to_string(),
            child_frame_id: "base_link".to_string(),
            initial_pose_set: false,

            last_update_time: 0,
            last_odometry_time: 0,
            last_imu_time: 0,

            landmarks: Vec::new(),
            landmark_detection_range: 10.0, // 10m detection range
        };

        // Initialize covariance matrix (high initial uncertainty)
        for i in 0..6 {
            node.covariance[i][i] = 1.0;
        }

        // Initialize process noise (motion model uncertainty)
        node.process_noise[0][0] = 0.1; // x position
        node.process_noise[1][1] = 0.1; // y position
        node.process_noise[2][2] = 0.05; // theta
        node.process_noise[3][3] = 0.2; // vx
        node.process_noise[4][4] = 0.2; // vy
        node.process_noise[5][5] = 0.1; // omega

        // Initialize odometry measurement noise
        node.odometry_noise[0][0] = 0.05; // x measurement noise
        node.odometry_noise[1][1] = 0.05; // y measurement noise
        node.odometry_noise[2][2] = 0.02; // theta measurement noise

        // Initialize IMU measurement noise
        node.imu_noise[0][0] = 0.1; // acceleration x
        node.imu_noise[1][1] = 0.1; // acceleration y
        node.imu_noise[2][2] = 0.05; // angular velocity z

        Ok(node)
    }

    /// Set initial pose estimate
    pub fn set_initial_pose(&mut self, x: f64, y: f64, theta: f64) {
        self.state[0] = x; // x position
        self.state[1] = y; // y position
        self.state[2] = theta; // orientation
        self.state[3] = 0.0; // vx
        self.state[4] = 0.0; // vy
        self.state[5] = 0.0; // omega

        // Reset covariance to moderate uncertainty
        for i in 0..6 {
            for j in 0..6 {
                self.covariance[i][j] = if i == j { 0.5 } else { 0.0 };
            }
        }

        self.initial_pose_set = true;
    }

    /// Set coordinate frame IDs
    pub fn set_frame_ids(&mut self, frame_id: &str, child_frame_id: &str) {
        self.frame_id = frame_id.to_string();
        self.child_frame_id = child_frame_id.to_string();
    }

    /// Add known landmark for localization correction
    pub fn add_landmark(&mut self, x: f64, y: f64) {
        self.landmarks.push((x, y));
    }

    /// Get current pose estimate
    pub fn get_pose(&self) -> (f64, f64, f64) {
        (self.state[0], self.state[1], self.state[2])
    }

    /// Get current velocity estimate
    pub fn get_velocity(&self) -> (f64, f64, f64) {
        (self.state[3], self.state[4], self.state[5])
    }

    /// Get pose uncertainty (position covariance)
    pub fn get_position_uncertainty(&self) -> f64 {
        (self.covariance[0][0] + self.covariance[1][1]).sqrt()
    }

    fn predict_step(&mut self, dt: f64) {
        // Predict next state using motion model
        // Simple kinematic model: x_{k+1} = x_k + vx*dt, y_{k+1} = y_k + vy*dt, etc.

        let old_state = self.state;

        // Position prediction
        self.state[0] += old_state[3] * dt; // x += vx * dt
        self.state[1] += old_state[4] * dt; // y += vy * dt
        self.state[2] += old_state[5] * dt; // theta += omega * dt

        // Normalize angle
        self.state[2] = self.normalize_angle(self.state[2]);

        // Velocities remain constant (no acceleration model)
        // state[3], state[4], state[5] unchanged

        // Predict covariance: P = F*P*F' + Q
        let mut predicted_cov = [[0.0; 6]; 6];

        // Simplified covariance prediction (identity motion model)
        for (i, row) in predicted_cov.iter_mut().enumerate() {
            for (j, val) in row.iter_mut().enumerate() {
                *val = self.covariance[i][j] + self.process_noise[i][j] * dt;
            }
        }

        self.covariance = predicted_cov;
    }

    fn update_with_odometry(&mut self, odom: &Odometry) {
        if !self.initial_pose_set {
            // Initialize pose from first odometry reading
            self.set_initial_pose(odom.pose.x, odom.pose.y, odom.pose.theta);
        }

        // Measurement vector: [x, y, theta]
        let measurement = [odom.pose.x, odom.pose.y, odom.pose.theta];

        // Expected measurement (predicted state)
        let predicted = [self.state[0], self.state[1], self.state[2]];

        // Innovation (measurement residual)
        let mut innovation = [0.0; 3];
        for i in 0..3 {
            innovation[i] = measurement[i] - predicted[i];
        }

        // Normalize angle innovation
        innovation[2] = self.normalize_angle(innovation[2]);

        // Simplified Kalman update (assuming direct observation of position/orientation)
        let kalman_gain = 0.3; // Simplified - should compute proper Kalman gain

        // Update state
        for (i, innov_val) in innovation.iter().enumerate().take(3) {
            self.state[i] += kalman_gain * innov_val;
        }

        // Update velocities from odometry twist
        self.state[3] = odom.twist.linear[0]; // vx
        self.state[4] = odom.twist.linear[1]; // vy
        self.state[5] = odom.twist.angular[2]; // omega

        // Normalize orientation
        self.state[2] = self.normalize_angle(self.state[2]);

        // Update covariance (simplified)
        for i in 0..3 {
            self.covariance[i][i] *= 1.0 - kalman_gain;
        }
    }

    fn update_with_imu(&mut self, imu: &Imu) {
        if !self.initial_pose_set {
            return; // Need initial pose before using IMU
        }

        // Use IMU angular velocity to refine orientation prediction
        let imu_omega = imu.angular_velocity[2];

        // Weighted fusion of odometry and IMU angular velocity
        let imu_weight = 0.3;
        self.state[5] = (1.0 - imu_weight) * self.state[5] + imu_weight * imu_omega;

        // Use IMU accelerations to validate velocity changes (simplified)
        let accel_x = imu.linear_acceleration[0];
        let accel_y = imu.linear_acceleration[1];

        // Simple acceleration-based velocity correction
        let dt = 0.01; // Assume ~100Hz IMU rate
        self.state[3] += accel_x * dt * 0.1; // Small correction factor
        self.state[4] += accel_y * dt * 0.1;
    }

    fn update_with_landmarks(&mut self, lidar: &LaserScan) {
        if !self.initial_pose_set || self.landmarks.is_empty() {
            return;
        }

        // Simplified landmark-based correction
        let robot_x = self.state[0];
        let robot_y = self.state[1];
        let robot_theta = self.state[2];

        // Extract potential landmark observations from lidar
        for (i, &range) in lidar.ranges.iter().enumerate() {
            if range > 0.5 && range < self.landmark_detection_range as f32 {
                let angle =
                    lidar.angle_min as f64 + i as f64 * lidar.angle_increment as f64 + robot_theta;

                let observed_x = robot_x + range as f64 * angle.cos();
                let observed_y = robot_y + range as f64 * angle.sin();

                // Find closest known landmark
                let mut min_distance = f64::INFINITY;
                let mut closest_landmark = None;

                for &(lm_x, lm_y) in &self.landmarks {
                    let distance =
                        ((observed_x - lm_x).powi(2) + (observed_y - lm_y).powi(2)).sqrt();
                    if distance < min_distance && distance < 1.0 {
                        // 1m association threshold
                        min_distance = distance;
                        closest_landmark = Some((lm_x, lm_y));
                    }
                }

                // Apply landmark correction if association found
                if let Some((lm_x, lm_y)) = closest_landmark {
                    let correction_weight = 0.1;
                    let position_error_x = observed_x - lm_x;
                    let position_error_y = observed_y - lm_y;

                    // Correct robot position estimate
                    self.state[0] -= correction_weight * position_error_x;
                    self.state[1] -= correction_weight * position_error_y;
                }
            }
        }
    }

    fn normalize_angle(&self, angle: f64) -> f64 {
        let mut normalized = angle;
        while normalized > std::f64::consts::PI {
            normalized -= 2.0 * std::f64::consts::PI;
        }
        while normalized < -std::f64::consts::PI {
            normalized += 2.0 * std::f64::consts::PI;
        }
        normalized
    }

    fn publish_pose(&self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut localized_pose = Odometry::new();

        // Set frame information
        localized_pose.frame_id = self
            .frame_id
            .clone()
            .into_bytes()
            .try_into()
            .unwrap_or([0; 32]);
        localized_pose.child_frame_id = self
            .child_frame_id
            .clone()
            .into_bytes()
            .try_into()
            .unwrap_or([0; 32]);

        // Set pose
        localized_pose.pose.x = self.state[0];
        localized_pose.pose.y = self.state[1];
        localized_pose.pose.theta = self.state[2];

        // Set twist
        localized_pose.twist.linear[0] = self.state[3];
        localized_pose.twist.linear[1] = self.state[4];
        localized_pose.twist.angular[2] = self.state[5];

        // Set covariances (simplified - only diagonal elements)
        for i in 0..6 {
            localized_pose.pose_covariance[i * 6 + i] = self.covariance[i][i];
        }

        localized_pose.timestamp = current_time;

        let _ = self.pose_publisher.send(localized_pose, None);
    }

    /// Reset localization (useful for relocalization)
    pub fn reset(&mut self) {
        self.state = [0.0; 6];
        for i in 0..6 {
            for j in 0..6 {
                self.covariance[i][j] = if i == j { 1.0 } else { 0.0 };
            }
        }
        self.initial_pose_set = false;
    }

    /// Check if localization is well-converged
    pub fn is_converged(&self) -> bool {
        self.initial_pose_set && self.get_position_uncertainty() < 0.3 // 30cm uncertainty
    }
}

impl Node for LocalizationNode {
    fn name(&self) -> &'static str {
        "LocalizationNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Calculate time step
        let dt = if self.last_update_time > 0 {
            (current_time - self.last_update_time) as f64 / 1000.0
        } else {
            0.01 // 10ms default
        };

        if dt > 0.001 {
            // Minimum 1ms update interval
            // Prediction step
            self.predict_step(dt);
            self.last_update_time = current_time;
        }

        // Update with odometry data
        if let Some(odom) = self.odometry_subscriber.recv(None) {
            if odom.timestamp > self.last_odometry_time {
                self.update_with_odometry(&odom);
                self.last_odometry_time = odom.timestamp;
            }
        }

        // Update with IMU data
        if let Some(imu) = self.imu_subscriber.recv(None) {
            if imu.timestamp > self.last_imu_time {
                self.update_with_imu(&imu);
                self.last_imu_time = imu.timestamp;
            }
        }

        // Update with lidar landmarks
        if let Some(lidar) = self.lidar_subscriber.recv(None) {
            self.update_with_landmarks(&lidar);
        }

        // Publish localized pose
        if self.initial_pose_set {
            self.publish_pose();
        }
    }
}

// Default impl removed - use Node::new() instead which returns HorusResult
