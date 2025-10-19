use crate::{DifferentialDriveCommand, Odometry, Twist};
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

/// Differential Drive Node - Mobile robot base controller
///
/// Subscribes to Twist velocity commands and converts them to differential drive
/// motor commands. Also publishes odometry based on wheel encoder feedback.
pub struct DifferentialDriveNode {
    // Publishers
    drive_publisher: Hub<DifferentialDriveCommand>,
    odom_publisher: Hub<Odometry>,

    // Subscribers
    cmd_subscriber: Hub<Twist>,

    // Configuration
    wheel_base: f32,      // Distance between wheels (m)
    wheel_radius: f32,    // Wheel radius (m)
    max_linear_vel: f32,  // Max linear velocity (m/s)
    max_angular_vel: f32, // Max angular velocity (rad/s)

    // State
    current_twist: Twist,
    position_x: f64,
    position_y: f64,
    orientation: f64,
    last_update_time: u64,
}

impl DifferentialDriveNode {
    /// Create a new differential drive node with default topics
    pub fn new() -> Self {
        Self::new_with_topics("cmd_vel", "drive_command", "odom")
    }

    /// Create a new differential drive node with custom topics
    pub fn new_with_topics(cmd_topic: &str, drive_topic: &str, odom_topic: &str) -> Self {
        Self {
            drive_publisher: Hub::new(drive_topic).expect("Failed to create drive command hub"),
            odom_publisher: Hub::new(odom_topic).expect("Failed to create odometry hub"),
            cmd_subscriber: Hub::new(cmd_topic).expect("Failed to subscribe to cmd_vel"),

            wheel_base: 0.5,       // 50cm wheel base
            wheel_radius: 0.1,     // 10cm wheel radius
            max_linear_vel: 2.0,   // 2 m/s max
            max_angular_vel: 3.14, // π rad/s max

            current_twist: Twist::default(),
            position_x: 0.0,
            position_y: 0.0,
            orientation: 0.0,
            last_update_time: 0,
        }
    }

    /// Set wheel base (distance between wheels in meters)
    pub fn set_wheel_base(&mut self, wheel_base: f32) {
        self.wheel_base = wheel_base.max(0.1);
    }

    /// Set wheel radius (in meters)
    pub fn set_wheel_radius(&mut self, radius: f32) {
        self.wheel_radius = radius.max(0.01);
    }

    /// Set maximum velocities
    pub fn set_velocity_limits(&mut self, max_linear: f32, max_angular: f32) {
        self.max_linear_vel = max_linear.max(0.1);
        self.max_angular_vel = max_angular.max(0.1);
    }

    /// Reset odometry to origin
    pub fn reset_odometry(&mut self) {
        self.position_x = 0.0;
        self.position_y = 0.0;
        self.orientation = 0.0;
    }

    /// Get current position
    pub fn get_position(&self) -> (f64, f64, f64) {
        (self.position_x, self.position_y, self.orientation)
    }

    fn clamp_twist(&self, mut twist: Twist) -> Twist {
        // Clamp linear velocity
        twist.linear[0] =
            twist.linear[0].clamp(-self.max_linear_vel as f64, self.max_linear_vel as f64);
        twist.linear[1] = 0.0; // Differential drive can't move sideways
        twist.linear[2] = 0.0;

        // Clamp angular velocity
        twist.angular[0] = 0.0;
        twist.angular[1] = 0.0;
        twist.angular[2] =
            twist.angular[2].clamp(-self.max_angular_vel as f64, self.max_angular_vel as f64);

        twist
    }

    fn twist_to_wheel_speeds(&self, twist: &Twist) -> (f32, f32) {
        // Convert twist to wheel speeds using differential drive kinematics
        let linear_vel = twist.linear[0];
        let angular_vel = twist.angular[2];

        // Calculate wheel speeds
        let left_wheel_speed = linear_vel - (angular_vel * self.wheel_base as f64 / 2.0);
        let right_wheel_speed = linear_vel + (angular_vel * self.wheel_base as f64 / 2.0);

        (left_wheel_speed as f32, right_wheel_speed as f32)
    }

    fn update_odometry(&mut self, dt: f32) {
        let linear_vel = self.current_twist.linear[0];
        let angular_vel = self.current_twist.angular[2];

        // Update position using simple integration
        let dx = linear_vel * (self.orientation.cos()) * dt as f64;
        let dy = linear_vel * (self.orientation.sin()) * dt as f64;
        let dtheta = angular_vel * dt as f64;

        self.position_x += dx;
        self.position_y += dy;
        self.orientation += dtheta;

        // Normalize orientation to [-π, π]
        while self.orientation > std::f64::consts::PI {
            self.orientation -= 2.0 * std::f64::consts::PI;
        }
        while self.orientation < -std::f64::consts::PI {
            self.orientation += 2.0 * std::f64::consts::PI;
        }
    }

    fn publish_drive_command(&self, left_speed: f32, right_speed: f32) {
        let cmd = DifferentialDriveCommand::new(left_speed as f64, right_speed as f64);
        let _ = self.drive_publisher.send(cmd, None);
    }

    fn publish_odometry(&self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut odom = Odometry::new();
        odom.pose.x = self.position_x;
        odom.pose.y = self.position_y;
        odom.pose.theta = self.orientation;
        odom.twist.linear[0] = self.current_twist.linear[0];
        odom.twist.angular[2] = self.current_twist.angular[2];

        // Set frame IDs
        let frame_id = "odom";
        let child_frame_id = "base_link";
        let frame_bytes = frame_id.as_bytes();
        let len = frame_bytes.len().min(31);
        odom.frame_id[..len].copy_from_slice(&frame_bytes[..len]);

        let child_frame_bytes = child_frame_id.as_bytes();
        let len = child_frame_bytes.len().min(31);
        odom.child_frame_id[..len].copy_from_slice(&child_frame_bytes[..len]);

        odom.timestamp = current_time;

        let _ = self.odom_publisher.send(odom, None);
    }
}

impl Node for DifferentialDriveNode {
    fn name(&self) -> &'static str {
        "DifferentialDriveNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Calculate delta time
        let dt = if self.last_update_time > 0 {
            (current_time - self.last_update_time) as f32 / 1000.0
        } else {
            0.01 // 10ms default
        };
        self.last_update_time = current_time;

        // Check for new velocity commands
        if let Some(twist_cmd) = self.cmd_subscriber.recv(None) {
            self.current_twist = self.clamp_twist(twist_cmd);
        }

        // Convert twist to wheel speeds and publish
        let (left_speed, right_speed) = self.twist_to_wheel_speeds(&self.current_twist);
        self.publish_drive_command(left_speed, right_speed);

        // Update and publish odometry
        self.update_odometry(dt);
        self.publish_odometry();
    }
}

impl Default for DifferentialDriveNode {
    fn default() -> Self {
        Self::new()
    }
}
