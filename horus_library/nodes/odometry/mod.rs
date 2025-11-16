use crate::{Odometry, Twist};
use horus_core::error::HorusResult;

type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo, NodeInfoExt};
use std::f64::consts::PI;
use std::time::{SystemTime, UNIX_EPOCH};

/// Odometry Calculation Node
///
/// Computes robot pose (position and orientation) from wheel encoder feedback.
/// Supports differential drive, mecanum, and ackermann steering kinematics.
///
/// # Kinematics Models
/// - **Differential Drive**: Two-wheeled robots (wheelbase and radius)
/// - **Mecanum Drive**: Four mecanum wheels for omni-directional motion
/// - **Ackermann**: Car-like steering (wheelbase and track width)
/// - **Skid Steer**: Tank-like with independent track control
///
/// # Features
/// - Dead reckoning from encoder ticks
/// - Velocity integration
/// - Configurable update rates
/// - Covariance estimation
/// - Frame transformation support
/// - Reset and calibration
///
/// # Example
/// ```rust,ignore
/// use horus_library::nodes::OdometryNode;
///
/// let mut odom = OdometryNode::new_differential_drive(0.5, 0.05)?;
/// odom.set_encoder_resolution(4096); // Ticks per revolution
/// odom.set_update_rate(50.0); // 50 Hz
/// ```
pub struct OdometryNode {
    // Input subscribers
    encoder_left_sub: Hub<i64>,
    encoder_right_sub: Hub<i64>,
    velocity_sub: Hub<Twist>,

    // Output publisher
    odom_publisher: Hub<Odometry>,

    // Robot configuration
    kinematic_model: KinematicModel,
    wheel_base: f64,      // Distance between wheels (m)
    wheel_radius: f64,    // Wheel radius (m)
    track_width: f64,     // Distance between left and right wheels (m)
    encoder_resolution: u32, // Ticks per revolution

    // Current state
    x: f64,               // Position x (m)
    y: f64,               // Position y (m)
    theta: f64,           // Orientation (radians)
    vx: f64,              // Linear velocity x (m/s)
    vy: f64,              // Linear velocity y (m/s)
    vtheta: f64,          // Angular velocity (rad/s)

    // Previous encoder values
    prev_left_ticks: i64,
    prev_right_ticks: i64,
    prev_time: u64,       // Nanoseconds

    // Covariance estimates
    position_variance: f64,
    orientation_variance: f64,

    // Configuration
    update_rate: f64,     // Hz
    use_encoder_input: bool,
    use_velocity_input: bool,
    frame_id: String,
    child_frame_id: String,
}

/// Kinematic model type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KinematicModel {
    DifferentialDrive,
    Mecanum,
    Ackermann,
    SkidSteer,
}

impl OdometryNode {
    /// Create odometry node for differential drive robot
    pub fn new_differential_drive(wheel_base: f64, wheel_radius: f64) -> Result<Self> {
        Ok(Self {
            encoder_left_sub: Hub::new("encoder/left")?,
            encoder_right_sub: Hub::new("encoder/right")?,
            velocity_sub: Hub::new("cmd_vel")?,
            odom_publisher: Hub::new("odom")?,
            kinematic_model: KinematicModel::DifferentialDrive,
            wheel_base,
            wheel_radius,
            track_width: wheel_base, // Same as wheelbase for diff drive
            encoder_resolution: 4096,
            x: 0.0,
            y: 0.0,
            theta: 0.0,
            vx: 0.0,
            vy: 0.0,
            vtheta: 0.0,
            prev_left_ticks: 0,
            prev_right_ticks: 0,
            prev_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            position_variance: 0.01,     // 1cm std dev
            orientation_variance: 0.001, // ~1.8 degree std dev
            update_rate: 50.0,
            use_encoder_input: true,
            use_velocity_input: false,
            frame_id: "odom".to_string(),
            child_frame_id: "base_link".to_string(),
        })
    }

    /// Create odometry node for mecanum drive robot
    pub fn new_mecanum_drive(wheel_base: f64, track_width: f64, wheel_radius: f64) -> Result<Self> {
        let mut node = Self::new_differential_drive(wheel_base, wheel_radius)?;
        node.kinematic_model = KinematicModel::Mecanum;
        node.track_width = track_width;
        Ok(node)
    }

    /// Create odometry node for ackermann steering
    pub fn new_ackermann(wheel_base: f64, track_width: f64, wheel_radius: f64) -> Result<Self> {
        let mut node = Self::new_differential_drive(wheel_base, wheel_radius)?;
        node.kinematic_model = KinematicModel::Ackermann;
        node.track_width = track_width;
        Ok(node)
    }

    /// Set encoder resolution (ticks per revolution)
    pub fn set_encoder_resolution(&mut self, resolution: u32) {
        self.encoder_resolution = resolution;
    }

    /// Set wheel parameters
    pub fn set_wheel_parameters(&mut self, base: f64, radius: f64) {
        self.wheel_base = base;
        self.wheel_radius = radius;
    }

    /// Set update rate in Hz
    pub fn set_update_rate(&mut self, rate: f64) {
        self.update_rate = rate.max(0.1);
    }

    /// Set covariance parameters
    pub fn set_covariance(&mut self, position_var: f64, orientation_var: f64) {
        self.position_variance = position_var;
        self.orientation_variance = orientation_var;
    }

    /// Set frame IDs
    pub fn set_frames(&mut self, frame_id: &str, child_frame_id: &str) {
        self.frame_id = frame_id.to_string();
        self.child_frame_id = child_frame_id.to_string();
    }

    /// Enable encoder-based odometry
    pub fn enable_encoder_input(&mut self, enable: bool) {
        self.use_encoder_input = enable;
    }

    /// Enable velocity-based odometry
    pub fn enable_velocity_input(&mut self, enable: bool) {
        self.use_velocity_input = enable;
    }

    /// Reset odometry to origin
    pub fn reset(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
        self.theta = 0.0;
        self.vx = 0.0;
        self.vy = 0.0;
        self.vtheta = 0.0;
    }

    /// Set current pose
    pub fn set_pose(&mut self, x: f64, y: f64, theta: f64) {
        self.x = x;
        self.y = y;
        self.theta = theta;
    }

    /// Get current pose
    pub fn get_pose(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.theta)
    }

    /// Get current velocity
    pub fn get_velocity(&self) -> (f64, f64, f64) {
        (self.vx, self.vy, self.vtheta)
    }

    /// Update odometry from encoder ticks
    fn update_from_encoders(&mut self, left_ticks: i64, right_ticks: i64, current_time: u64) {
        // Calculate time delta
        let dt = (current_time - self.prev_time) as f64 / 1_000_000_000.0;
        if dt <= 0.0 || dt > 1.0 {
            // Invalid time delta, skip update
            self.prev_time = current_time;
            return;
        }

        // Calculate tick deltas
        let dleft_ticks = left_ticks - self.prev_left_ticks;
        let dright_ticks = right_ticks - self.prev_right_ticks;

        // Convert ticks to distance
        let meters_per_tick = (2.0 * PI * self.wheel_radius) / self.encoder_resolution as f64;
        let dleft = dleft_ticks as f64 * meters_per_tick;
        let dright = dright_ticks as f64 * meters_per_tick;

        // Update based on kinematic model
        match self.kinematic_model {
            KinematicModel::DifferentialDrive | KinematicModel::SkidSteer => {
                // Differential drive kinematics
                let dc = (dleft + dright) / 2.0; // Center displacement
                let dtheta = (dright - dleft) / self.wheel_base; // Change in orientation

                // Update pose
                if dtheta.abs() < 0.0001 {
                    // Straight line motion
                    self.x += dc * self.theta.cos();
                    self.y += dc * self.theta.sin();
                } else {
                    // Arc motion
                    let radius = dc / dtheta;
                    self.x += radius * (self.theta + dtheta).sin() - radius * self.theta.sin();
                    self.y += -radius * (self.theta + dtheta).cos() + radius * self.theta.cos();
                    self.theta += dtheta;
                }

                // Normalize theta to [-pi, pi]
                while self.theta > PI {
                    self.theta -= 2.0 * PI;
                }
                while self.theta < -PI {
                    self.theta += 2.0 * PI;
                }

                // Calculate velocities
                self.vx = dc / dt;
                self.vy = 0.0; // Differential drive can't move sideways
                self.vtheta = dtheta / dt;
            }
            KinematicModel::Mecanum => {
                // Simplified mecanum kinematics (would need 4 encoders for full implementation)
                let dc = (dleft + dright) / 2.0;
                let dtheta = (dright - dleft) / self.track_width;

                self.x += dc * self.theta.cos();
                self.y += dc * self.theta.sin();
                self.theta += dtheta;

                self.vx = dc / dt;
                self.vy = 0.0; // Would need additional encoders
                self.vtheta = dtheta / dt;
            }
            KinematicModel::Ackermann => {
                // Ackermann steering kinematics
                let dc = (dleft + dright) / 2.0;
                // For full Ackermann, would need steering angle input
                let dtheta = (dright - dleft) / self.track_width;

                self.x += dc * self.theta.cos();
                self.y += dc * self.theta.sin();
                self.theta += dtheta;

                self.vx = dc / dt;
                self.vy = 0.0;
                self.vtheta = dtheta / dt;
            }
        }

        // Store current values for next iteration
        self.prev_left_ticks = left_ticks;
        self.prev_right_ticks = right_ticks;
        self.prev_time = current_time;
    }

    /// Update odometry from velocity command (dead reckoning)
    fn update_from_velocity(&mut self, twist: Twist, current_time: u64) {
        let dt = (current_time - self.prev_time) as f64 / 1_000_000_000.0;
        if dt <= 0.0 || dt > 1.0 {
            self.prev_time = current_time;
            return;
        }

        // Extract velocities
        let vx_body = twist.linear[0]; // x component
        let vy_body = twist.linear[1]; // y component
        let vtheta = twist.angular[2]; // z component (yaw)

        // Integrate velocities to get pose
        if vtheta.abs() < 0.0001 {
            // Straight line motion
            self.x += (vx_body * self.theta.cos() - vy_body * self.theta.sin()) * dt;
            self.y += (vx_body * self.theta.sin() + vy_body * self.theta.cos()) * dt;
        } else {
            // Curved motion
            let dtheta = vtheta * dt;
            self.x += (vx_body * (self.theta + dtheta / 2.0).cos() - vy_body * (self.theta + dtheta / 2.0).sin()) * dt;
            self.y += (vx_body * (self.theta + dtheta / 2.0).sin() + vy_body * (self.theta + dtheta / 2.0).cos()) * dt;
            self.theta += dtheta;
        }

        // Normalize theta
        while self.theta > PI {
            self.theta -= 2.0 * PI;
        }
        while self.theta < -PI {
            self.theta += 2.0 * PI;
        }

        // Store velocities
        self.vx = vx_body;
        self.vy = vy_body;
        self.vtheta = vtheta;

        self.prev_time = current_time;
    }

    /// Publish odometry message
    fn publish_odometry(&mut self, mut ctx: Option<&mut NodeInfo>) {
        let mut odom = Odometry::new();

        // Set pose
        odom.pose.x = self.x;
        odom.pose.y = self.y;
        odom.pose.theta = self.theta;

        // Set twist (velocity in body frame)
        odom.twist.linear[0] = self.vx;  // x component
        odom.twist.linear[1] = self.vy;  // y component
        odom.twist.angular[2] = self.vtheta; // z component (yaw)

        // Set covariance (simplified - diagonal only)
        // Pose covariance (6x6): [x, y, z, roll, pitch, yaw]
        odom.pose_covariance[0] = self.position_variance;     // x variance
        odom.pose_covariance[7] = self.position_variance;     // y variance
        odom.pose_covariance[14] = 1e9;                       // z (not used)
        odom.pose_covariance[21] = 1e9;                       // roll (not used)
        odom.pose_covariance[28] = 1e9;                       // pitch (not used)
        odom.pose_covariance[35] = self.orientation_variance; // yaw variance

        // Twist covariance
        odom.twist_covariance[0] = 0.001;  // vx variance
        odom.twist_covariance[7] = 0.001;  // vy variance
        odom.twist_covariance[35] = 0.001; // vtheta variance

        // Set frame IDs
        odom.set_frames(&self.frame_id, &self.child_frame_id);

        // Publish
        if let Err(e) = self.odom_publisher.send(odom, &mut None) {
            ctx.log_error(&format!("Failed to publish odometry: {:?}", e));
        }
    }
}

impl Node for OdometryNode {
    fn name(&self) -> &'static str {
        "OdometryNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        // Process encoder inputs
        if self.use_encoder_input {
            let left_ticks = self.encoder_left_sub.recv(&mut None);
            let right_ticks = self.encoder_right_sub.recv(&mut None);

            if let (Some(left), Some(right)) = (left_ticks, right_ticks) {
                self.update_from_encoders(left, right, current_time);
            }
        }

        // Process velocity inputs (fallback or additional source)
        if self.use_velocity_input {
            if let Some(twist) = self.velocity_sub.recv(&mut None) {
                self.update_from_velocity(twist, current_time);
            }
        }

        // Publish odometry at configured rate
        static mut LAST_PUBLISH_TIME: u64 = 0;
        let publish_interval = (1_000_000_000.0 / self.update_rate) as u64;

        unsafe {
            if current_time - LAST_PUBLISH_TIME >= publish_interval {
                self.publish_odometry(ctx.as_deref_mut());
                LAST_PUBLISH_TIME = current_time;

                // Periodic detailed logging
                static mut LOG_COUNTER: u32 = 0;
                LOG_COUNTER += 1;
                if LOG_COUNTER % 100 == 0 {
                    // Log every 100 publishes (2 sec at 50Hz)
                    ctx.log_debug(&format!(
                        "Odom: pos=({:.3}, {:.3})m theta={:.2}° vel=({:.2}, {:.2})m/s omega={:.2}rad/s",
                        self.x,
                        self.y,
                        self.theta.to_degrees(),
                        self.vx,
                        self.vy,
                        self.vtheta
                    ));
                }
            }
        }
    }
}
