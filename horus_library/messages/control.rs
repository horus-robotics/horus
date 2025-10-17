//! Control message types for robotics
//!
//! This module provides messages for controlling actuators,
//! motors, servos, and other controllable components.

use serde::{Deserialize, Serialize};
use serde_arrays;

/// Motor command for direct motor control
///
/// Supports various control modes including velocity, position, and torque control.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct MotorCommand {
    /// Motor ID (for multi-motor systems)
    pub motor_id: u8,
    /// Control mode (0=velocity, 1=position, 2=torque, 3=voltage)
    pub mode: u8,
    /// Target value (units depend on mode)
    pub target: f64,
    /// Maximum velocity (for position mode)
    pub max_velocity: f64,
    /// Maximum acceleration
    pub max_acceleration: f64,
    /// Feed-forward term
    pub feed_forward: f64,
    /// Enable motor (false = brake/coast depending on config)
    pub enable: bool,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl MotorCommand {
    pub const MODE_VELOCITY: u8 = 0;
    pub const MODE_POSITION: u8 = 1;
    pub const MODE_TORQUE: u8 = 2;
    pub const MODE_VOLTAGE: u8 = 3;

    /// Create a velocity command
    pub fn velocity(motor_id: u8, velocity: f64) -> Self {
        Self {
            motor_id,
            mode: Self::MODE_VELOCITY,
            target: velocity,
            max_velocity: f64::INFINITY,
            max_acceleration: f64::INFINITY,
            feed_forward: 0.0,
            enable: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a position command
    pub fn position(motor_id: u8, position: f64, max_velocity: f64) -> Self {
        Self {
            motor_id,
            mode: Self::MODE_POSITION,
            target: position,
            max_velocity,
            max_acceleration: f64::INFINITY,
            feed_forward: 0.0,
            enable: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a stop command
    pub fn stop(motor_id: u8) -> Self {
        Self {
            motor_id,
            mode: Self::MODE_VELOCITY,
            target: 0.0,
            max_velocity: f64::INFINITY,
            max_acceleration: f64::INFINITY,
            feed_forward: 0.0,
            enable: false,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Check if values are valid
    pub fn is_valid(&self) -> bool {
        self.target.is_finite() &&
        self.max_velocity.is_finite() &&
        self.max_acceleration.is_finite() &&
        self.feed_forward.is_finite()
    }
}

/// Differential drive motor commands
///
/// Commands for a two-wheeled differential drive robot.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct DifferentialDriveCommand {
    /// Left wheel velocity in rad/s
    pub left_velocity: f64,
    /// Right wheel velocity in rad/s
    pub right_velocity: f64,
    /// Maximum acceleration in rad/s²
    pub max_acceleration: f64,
    /// Enable motors
    pub enable: bool,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl DifferentialDriveCommand {
    /// Create a new differential drive command
    pub fn new(left: f64, right: f64) -> Self {
        Self {
            left_velocity: left,
            right_velocity: right,
            max_acceleration: f64::INFINITY,
            enable: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a stop command
    pub fn stop() -> Self {
        Self {
            left_velocity: 0.0,
            right_velocity: 0.0,
            max_acceleration: f64::INFINITY,
            enable: false,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create from linear and angular velocities
    pub fn from_twist(linear: f64, angular: f64, wheel_base: f64, wheel_radius: f64) -> Self {
        let left = (linear - angular * wheel_base / 2.0) / wheel_radius;
        let right = (linear + angular * wheel_base / 2.0) / wheel_radius;
        Self::new(left, right)
    }

    /// Check if values are valid
    pub fn is_valid(&self) -> bool {
        self.left_velocity.is_finite() &&
        self.right_velocity.is_finite() &&
        self.max_acceleration.is_finite()
    }
}

/// Servo command for position-controlled servos
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ServoCommand {
    /// Servo ID (for multi-servo systems)
    pub servo_id: u8,
    /// Target position in radians
    pub position: f32,
    /// Movement speed (0-1, 0=max speed)
    pub speed: f32,
    /// Torque enable
    pub enable: bool,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl ServoCommand {
    /// Create a new servo command
    pub fn new(servo_id: u8, position: f32) -> Self {
        Self {
            servo_id,
            position,
            speed: 0.5,
            enable: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a command with specific speed
    pub fn with_speed(servo_id: u8, position: f32, speed: f32) -> Self {
        Self {
            servo_id,
            position,
            speed: speed.clamp(0.0, 1.0),
            enable: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Disable servo (remove torque)
    pub fn disable(servo_id: u8) -> Self {
        Self {
            servo_id,
            position: 0.0,
            speed: 0.0,
            enable: false,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Convert position from degrees to radians
    pub fn from_degrees(servo_id: u8, degrees: f32) -> Self {
        Self::new(servo_id, degrees.to_radians())
    }
}

/// PID gains configuration message
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PidConfig {
    /// Controller ID
    pub controller_id: u8,
    /// Proportional gain
    pub kp: f64,
    /// Integral gain
    pub ki: f64,
    /// Derivative gain
    pub kd: f64,
    /// Integral windup limit
    pub integral_limit: f64,
    /// Output limit
    pub output_limit: f64,
    /// Enable anti-windup
    pub anti_windup: bool,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl PidConfig {
    /// Create a new PID configuration
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            controller_id: 0,
            kp,
            ki,
            kd,
            integral_limit: f64::INFINITY,
            output_limit: f64::INFINITY,
            anti_windup: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a P-only controller
    pub fn proportional(kp: f64) -> Self {
        Self::new(kp, 0.0, 0.0)
    }

    /// Create a PI controller
    pub fn pi(kp: f64, ki: f64) -> Self {
        Self::new(kp, ki, 0.0)
    }

    /// Create a PD controller
    pub fn pd(kp: f64, kd: f64) -> Self {
        Self::new(kp, 0.0, kd)
    }

    /// Set limits
    pub fn with_limits(mut self, integral_limit: f64, output_limit: f64) -> Self {
        self.integral_limit = integral_limit;
        self.output_limit = output_limit;
        self
    }

    /// Check if gains are valid
    pub fn is_valid(&self) -> bool {
        self.kp.is_finite() &&
        self.ki.is_finite() &&
        self.kd.is_finite() &&
        self.integral_limit.is_finite() &&
        self.output_limit.is_finite() &&
        self.kp >= 0.0 &&
        self.ki >= 0.0 &&
        self.kd >= 0.0
    }
}

/// Trajectory point for path following
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct TrajectoryPoint {
    /// Position [x, y, z]
    pub position: [f64; 3],
    /// Velocity [vx, vy, vz]
    pub velocity: [f64; 3],
    /// Acceleration [ax, ay, az]
    pub acceleration: [f64; 3],
    /// Orientation as quaternion [x, y, z, w]
    pub orientation: [f64; 4],
    /// Angular velocity [wx, wy, wz]
    pub angular_velocity: [f64; 3],
    /// Time from trajectory start in seconds
    pub time_from_start: f64,
}

impl TrajectoryPoint {
    /// Create a simple 2D trajectory point
    pub fn new_2d(x: f64, y: f64, vx: f64, vy: f64, time: f64) -> Self {
        Self {
            position: [x, y, 0.0],
            velocity: [vx, vy, 0.0],
            acceleration: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
            angular_velocity: [0.0; 3],
            time_from_start: time,
        }
    }

    /// Create a stationary point
    pub fn stationary(x: f64, y: f64, z: f64) -> Self {
        Self {
            position: [x, y, z],
            velocity: [0.0; 3],
            acceleration: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
            angular_velocity: [0.0; 3],
            time_from_start: 0.0,
        }
    }
}

/// Joint command for multi-DOF systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointCommand {
    /// Joint names (max 16 joints)
    #[serde(with = "serde_arrays")]
    pub joint_names: [[u8; 32]; 16],
    /// Number of active joints
    pub joint_count: u8,
    /// Position commands in radians
    #[serde(with = "serde_arrays")]
    pub positions: [f64; 16],
    /// Velocity commands in rad/s
    #[serde(with = "serde_arrays")]
    pub velocities: [f64; 16],
    /// Effort/torque commands in Nm
    #[serde(with = "serde_arrays")]
    pub efforts: [f64; 16],
    /// Control mode per joint (0=position, 1=velocity, 2=effort)
    #[serde(with = "serde_arrays")]
    pub modes: [u8; 16],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Default for JointCommand {
    fn default() -> Self {
        Self {
            joint_names: [[0; 32]; 16],
            joint_count: 0,
            positions: [0.0; 16],
            velocities: [0.0; 16],
            efforts: [0.0; 16],
            modes: [0; 16],
            timestamp: 0,
        }
    }
}

impl JointCommand {
    pub const MODE_POSITION: u8 = 0;
    pub const MODE_VELOCITY: u8 = 1;
    pub const MODE_EFFORT: u8 = 2;

    /// Create a new joint command
    pub fn new() -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        }
    }

    /// Add a joint position command
    pub fn add_position(&mut self, name: &str, position: f64) -> Result<(), &'static str> {
        if self.joint_count >= 16 {
            return Err("Maximum 16 joints supported");
        }

        let idx = self.joint_count as usize;

        // Copy joint name
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(31);
        self.joint_names[idx][..len].copy_from_slice(&name_bytes[..len]);
        self.joint_names[idx][len] = 0;

        self.positions[idx] = position;
        self.modes[idx] = Self::MODE_POSITION;
        self.joint_count += 1;

        Ok(())
    }

    /// Add a joint velocity command
    pub fn add_velocity(&mut self, name: &str, velocity: f64) -> Result<(), &'static str> {
        if self.joint_count >= 16 {
            return Err("Maximum 16 joints supported");
        }

        let idx = self.joint_count as usize;

        // Copy joint name
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(31);
        self.joint_names[idx][..len].copy_from_slice(&name_bytes[..len]);
        self.joint_names[idx][len] = 0;

        self.velocities[idx] = velocity;
        self.modes[idx] = Self::MODE_VELOCITY;
        self.joint_count += 1;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motor_command() {
        let cmd = MotorCommand::velocity(1, 10.0);
        assert_eq!(cmd.motor_id, 1);
        assert_eq!(cmd.mode, MotorCommand::MODE_VELOCITY);
        assert_eq!(cmd.target, 10.0);
        assert!(cmd.enable);
    }

    #[test]
    fn test_differential_drive_from_twist() {
        let cmd = DifferentialDriveCommand::from_twist(1.0, 0.5, 0.3, 0.05);
        assert!(cmd.left_velocity < cmd.right_velocity);
        assert!(cmd.is_valid());
    }

    #[test]
    fn test_pid_config() {
        let config = PidConfig::pi(1.0, 0.1).with_limits(10.0, 100.0);
        assert_eq!(config.kp, 1.0);
        assert_eq!(config.ki, 0.1);
        assert_eq!(config.kd, 0.0);
        assert_eq!(config.integral_limit, 10.0);
        assert!(config.is_valid());
    }

    #[test]
    fn test_joint_command() {
        let mut cmd = JointCommand::new();
        cmd.add_position("shoulder", 1.57).unwrap();
        cmd.add_velocity("elbow", 0.5).unwrap();

        assert_eq!(cmd.joint_count, 2);
        assert_eq!(cmd.positions[0], 1.57);
        assert_eq!(cmd.velocities[1], 0.5);
        assert_eq!(cmd.modes[0], JointCommand::MODE_POSITION);
        assert_eq!(cmd.modes[1], JointCommand::MODE_VELOCITY);
    }
}