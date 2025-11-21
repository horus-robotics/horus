//! Robot kinematics models for sim2d
//!
//! Provides different drive types: differential, Ackermann, omnidirectional, etc.

use horus_library::messages::CmdVel;
use nalgebra::Vector2;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};

/// Robot kinematics type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KinematicsType {
    /// Differential drive (default) - two independently driven wheels
    Differential,
    /// Ackermann steering - car-like steering
    Ackermann,
    /// Omnidirectional - holonomic drive (can move in any direction)
    Omnidirectional,
}

impl Default for KinematicsType {
    fn default() -> Self {
        Self::Differential
    }
}

/// Configuration for Ackermann steering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckermannConfig {
    /// Wheelbase (distance between front and rear axles)
    pub wheelbase: f32,
    /// Maximum steering angle in radians
    pub max_steering_angle: f32,
}

impl Default for AckermannConfig {
    fn default() -> Self {
        Self {
            wheelbase: 0.5,          // 0.5 meters
            max_steering_angle: 0.7, // ~40 degrees
        }
    }
}

/// Kinematics model for a robot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinematicsModel {
    pub kinematic_type: KinematicsType,
    #[serde(default)]
    pub ackermann: AckermannConfig,
}

impl Default for KinematicsModel {
    fn default() -> Self {
        Self {
            kinematic_type: KinematicsType::Differential,
            ackermann: AckermannConfig::default(),
        }
    }
}

impl KinematicsModel {
    /// Apply velocity command to rigid body based on kinetics model
    pub fn apply_velocity(&self, cmd_vel: &CmdVel, rigid_body: &mut RigidBody, max_speed: f32) {
        match self.kinematic_type {
            KinematicsType::Differential => self.apply_differential(cmd_vel, rigid_body, max_speed),
            KinematicsType::Ackermann => self.apply_ackermann(cmd_vel, rigid_body, max_speed),
            KinematicsType::Omnidirectional => {
                self.apply_omnidirectional(cmd_vel, rigid_body, max_speed)
            }
        }
    }

    /// Differential drive kinematics
    /// Linear velocity moves forward/backward, angular velocity rotates in place
    fn apply_differential(&self, cmd_vel: &CmdVel, rigid_body: &mut RigidBody, max_speed: f32) {
        let robot_angle = rigid_body.rotation().angle();
        let forward_dir = Vector2::new(robot_angle.cos(), robot_angle.sin());

        // Apply linear velocity in robot's forward direction
        let linear_vel = forward_dir * cmd_vel.linear.clamp(-max_speed, max_speed);
        let angular_vel = cmd_vel.angular.clamp(-3.0, 3.0); // Max angular speed

        rigid_body.set_linvel(vector![linear_vel.x, linear_vel.y], true);
        rigid_body.set_angvel(angular_vel, true);
    }

    /// Ackermann steering kinematics (car-like)
    /// Linear velocity is forward speed, angular velocity controls steering angle
    fn apply_ackermann(&self, cmd_vel: &CmdVel, rigid_body: &mut RigidBody, max_speed: f32) {
        let robot_angle = rigid_body.rotation().angle();

        // Interpret angular command as desired steering angle
        let steering_angle = cmd_vel.angular.clamp(
            -self.ackermann.max_steering_angle,
            self.ackermann.max_steering_angle,
        );

        // Linear velocity is forward/backward speed
        let forward_speed = cmd_vel.linear.clamp(-max_speed, max_speed);

        // Calculate turning radius using Ackermann geometry
        // R = wheelbase / tan(steering_angle)
        // angular_velocity = forward_speed / R = forward_speed * tan(steering_angle) / wheelbase
        let angular_vel = if steering_angle.abs() > 0.01 {
            forward_speed * steering_angle.tan() / self.ackermann.wheelbase
        } else {
            0.0 // Driving straight
        };

        // Calculate velocity in global frame
        let forward_dir = Vector2::new(robot_angle.cos(), robot_angle.sin());
        let linear_vel = forward_dir * forward_speed;

        rigid_body.set_linvel(vector![linear_vel.x, linear_vel.y], true);
        rigid_body.set_angvel(angular_vel, true);
    }

    /// Omnidirectional kinematics (holonomic)
    /// Can move in any direction - linear is forward/backward, angular adds strafing
    /// Interprets CmdVel as: linear = x velocity, angular = y velocity (in robot frame)
    fn apply_omnidirectional(&self, cmd_vel: &CmdVel, rigid_body: &mut RigidBody, max_speed: f32) {
        let robot_angle = rigid_body.rotation().angle();

        // CmdVel: linear = x (forward/backward), angular = y (left/right strafe)
        let local_vel = Vector2::new(
            cmd_vel.linear.clamp(-max_speed, max_speed),
            cmd_vel.angular.clamp(-max_speed, max_speed),
        );

        // Transform from robot frame to world frame
        let cos_angle = robot_angle.cos();
        let sin_angle = robot_angle.sin();

        let world_vel = Vector2::new(
            local_vel.x * cos_angle - local_vel.y * sin_angle,
            local_vel.x * sin_angle + local_vel.y * cos_angle,
        );

        // Omnidirectional robots don't rotate from velocity commands
        // (Could add separate rotation control if needed)
        rigid_body.set_linvel(vector![world_vel.x, world_vel.y], true);
        rigid_body.set_angvel(0.0, true);
    }

    /// Get description of this kinematics model
    pub fn description(&self) -> &'static str {
        match self.kinematic_type {
            KinematicsType::Differential => {
                "Differential Drive: Independent control of left/right wheels"
            }
            KinematicsType::Ackermann => {
                "Ackermann Steering: Car-like steering with front wheel control"
            }
            KinematicsType::Omnidirectional => {
                "Omnidirectional: Can move in any direction without rotation"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_differential_forward() {
        let model = KinematicsModel::default();
        let mut rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0])
            .build();

        let cmd_vel = CmdVel {
            linear: 1.0,
            angular: 0.0,
            stamp_nanos: 0,
        };

        model.apply_velocity(&cmd_vel, &mut rigid_body, 2.0);

        let linvel = rigid_body.linvel();
        assert!((linvel.x - 1.0).abs() < 0.01);
        assert!(linvel.y.abs() < 0.01);
        assert!(rigid_body.angvel().abs() < 0.01);
    }

    #[test]
    fn test_differential_rotation() {
        let model = KinematicsModel::default();
        let mut rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0])
            .build();

        let cmd_vel = CmdVel {
            linear: 0.0,
            angular: 1.0,
            stamp_nanos: 0,
        };

        model.apply_velocity(&cmd_vel, &mut rigid_body, 2.0);

        let linvel = rigid_body.linvel();
        assert!(linvel.x.abs() < 0.01);
        assert!(linvel.y.abs() < 0.01);
        assert!((rigid_body.angvel() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_ackermann_straight() {
        let model = KinematicsModel {
            kinematic_type: KinematicsType::Ackermann,
            ackermann: AckermannConfig::default(),
        };

        let mut rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0])
            .build();

        let cmd_vel = CmdVel {
            linear: 1.0,
            angular: 0.0, // No steering
            stamp_nanos: 0,
        };

        model.apply_velocity(&cmd_vel, &mut rigid_body, 2.0);

        let linvel = rigid_body.linvel();
        assert!((linvel.x - 1.0).abs() < 0.01);
        assert!(linvel.y.abs() < 0.01);
        assert!(rigid_body.angvel().abs() < 0.01);
    }

    #[test]
    fn test_ackermann_turning() {
        let model = KinematicsModel {
            kinematic_type: KinematicsType::Ackermann,
            ackermann: AckermannConfig {
                wheelbase: 0.5,
                max_steering_angle: 0.7,
            },
        };

        let mut rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0])
            .build();

        let cmd_vel = CmdVel {
            linear: 1.0,
            angular: 0.3, // Steering angle
            stamp_nanos: 0,
        };

        model.apply_velocity(&cmd_vel, &mut rigid_body, 2.0);

        // Should have both linear and angular velocity
        let linvel = rigid_body.linvel();
        assert!((linvel.x - 1.0).abs() < 0.01);
        assert!(rigid_body.angvel().abs() > 0.0);
    }

    #[test]
    fn test_omnidirectional() {
        let model = KinematicsModel {
            kinematic_type: KinematicsType::Omnidirectional,
            ackermann: AckermannConfig::default(),
        };

        let mut rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0])
            .build();

        // Move forward and right simultaneously
        let cmd_vel = CmdVel {
            linear: 1.0,  // Forward
            angular: 0.5, // Right strafe
            stamp_nanos: 0,
        };

        model.apply_velocity(&cmd_vel, &mut rigid_body, 2.0);

        let linvel = rigid_body.linvel();
        // Should move diagonally
        assert!(linvel.x > 0.0);
        assert!(linvel.y > 0.0);
        // Should not rotate
        assert!(rigid_body.angvel().abs() < 0.01);
    }

    #[test]
    fn test_max_speed_clamping() {
        let model = KinematicsModel::default();
        let mut rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0])
            .build();

        let cmd_vel = CmdVel {
            linear: 10.0, // Exceeds max
            angular: 0.0,
            stamp_nanos: 0,
        };

        model.apply_velocity(&cmd_vel, &mut rigid_body, 2.0);

        let linvel = rigid_body.linvel();
        // Should be clamped to max_speed
        assert!((linvel.x - 2.0).abs() < 0.01);
    }
}
