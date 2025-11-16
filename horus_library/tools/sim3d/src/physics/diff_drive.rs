use bevy::prelude::*;
use nalgebra::Vector3;

#[derive(Component, Debug, Clone)]
pub struct DifferentialDrive {
    pub wheel_separation: f32,
    pub wheel_radius: f32,
    pub max_linear_velocity: f32,
    pub max_angular_velocity: f32,
}

impl DifferentialDrive {
    pub fn new(wheel_separation: f32, wheel_radius: f32) -> Self {
        Self {
            wheel_separation,
            wheel_radius,
            max_linear_velocity: 1.0,
            max_angular_velocity: 2.0,
        }
    }

    pub fn with_limits(mut self, max_linear: f32, max_angular: f32) -> Self {
        self.max_linear_velocity = max_linear;
        self.max_angular_velocity = max_angular;
        self
    }

    pub fn compute_wheel_velocities(&self, linear_vel: f32, angular_vel: f32) -> (f32, f32) {
        let linear_vel = linear_vel.clamp(-self.max_linear_velocity, self.max_linear_velocity);
        let angular_vel = angular_vel.clamp(-self.max_angular_velocity, self.max_angular_velocity);

        let left_vel = (linear_vel - angular_vel * self.wheel_separation / 2.0) / self.wheel_radius;
        let right_vel = (linear_vel + angular_vel * self.wheel_separation / 2.0) / self.wheel_radius;

        (left_vel, right_vel)
    }

    pub fn compute_body_velocity(&self, left_wheel_vel: f32, right_wheel_vel: f32) -> (f32, f32) {
        let linear_vel = (left_wheel_vel + right_wheel_vel) * self.wheel_radius / 2.0;
        let angular_vel = (right_wheel_vel - left_wheel_vel) * self.wheel_radius / self.wheel_separation;

        (linear_vel, angular_vel)
    }

    pub fn apply_velocity(
        &self,
        linear_vel: f32,
        angular_vel: f32,
        current_yaw: f32,
    ) -> (Vector3<f32>, Vector3<f32>) {
        let linear_vel = linear_vel.clamp(-self.max_linear_velocity, self.max_linear_velocity);
        let angular_vel = angular_vel.clamp(-self.max_angular_velocity, self.max_angular_velocity);

        let vel_x = linear_vel * current_yaw.cos();
        let vel_z = linear_vel * current_yaw.sin();

        let linvel = Vector3::new(vel_x, 0.0, vel_z);
        let angvel = Vector3::new(0.0, angular_vel, 0.0);

        (linvel, angvel)
    }
}

#[derive(Component)]
pub struct CmdVel {
    pub linear: f32,
    pub angular: f32,
}

impl Default for CmdVel {
    fn default() -> Self {
        Self {
            linear: 0.0,
            angular: 0.0,
        }
    }
}
