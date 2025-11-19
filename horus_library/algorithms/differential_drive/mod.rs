//! Differential Drive Kinematics
//!
//! Forward and inverse kinematics for differential drive mobile robots.
//!
//! # Features
//!
//! - Forward kinematics (wheel speeds → robot velocity)
//! - Inverse kinematics (robot velocity → wheel speeds)
//! - Odometry calculation
//! - Configurable wheel base and radius
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::differential_drive::DifferentialDrive;
//!
//! let dd = DifferentialDrive::new(0.5, 0.1);  // wheel_base, wheel_radius
//!
//! // Convert Twist to wheel speeds
//! let (left, right) = dd.inverse_kinematics(1.0, 0.5);  // linear, angular
//!
//! // Convert wheel speeds to Twist
//! let (linear, angular) = dd.forward_kinematics(1.2, 0.8);  // left, right
//! ```

/// Differential Drive Kinematics
pub struct DifferentialDrive {
    wheel_base: f64,     // Distance between wheels (m)
    wheel_radius: f64,   // Wheel radius (m)
}

impl DifferentialDrive {
    /// Create new differential drive kinematics
    ///
    /// # Arguments
    /// * `wheel_base` - Distance between left and right wheels (meters)
    /// * `wheel_radius` - Radius of wheels (meters)
    pub fn new(wheel_base: f64, wheel_radius: f64) -> Self {
        Self {
            wheel_base,
            wheel_radius,
        }
    }

    /// Inverse kinematics: Convert robot velocity to wheel speeds
    ///
    /// # Arguments
    /// * `linear` - Linear velocity (m/s)
    /// * `angular` - Angular velocity (rad/s)
    ///
    /// # Returns
    /// (left_wheel_speed, right_wheel_speed) in m/s
    pub fn inverse_kinematics(&self, linear: f64, angular: f64) -> (f64, f64) {
        let left = linear - (angular * self.wheel_base / 2.0);
        let right = linear + (angular * self.wheel_base / 2.0);
        (left, right)
    }

    /// Forward kinematics: Convert wheel speeds to robot velocity
    ///
    /// # Arguments
    /// * `left_speed` - Left wheel speed (m/s)
    /// * `right_speed` - Right wheel speed (m/s)
    ///
    /// # Returns
    /// (linear_velocity, angular_velocity)
    pub fn forward_kinematics(&self, left_speed: f64, right_speed: f64) -> (f64, f64) {
        let linear = (left_speed + right_speed) / 2.0;
        let angular = (right_speed - left_speed) / self.wheel_base;
        (linear, angular)
    }

    /// Convert wheel speeds (rad/s) to linear speeds (m/s)
    pub fn wheel_angular_to_linear(&self, angular_speed: f64) -> f64 {
        angular_speed * self.wheel_radius
    }

    /// Convert linear speed (m/s) to wheel angular speed (rad/s)
    pub fn wheel_linear_to_angular(&self, linear_speed: f64) -> f64 {
        linear_speed / self.wheel_radius
    }

    /// Update odometry from wheel speeds
    ///
    /// # Arguments
    /// * `pose` - Current pose (x, y, theta)
    /// * `left_speed` - Left wheel speed (m/s)
    /// * `right_speed` - Right wheel speed (m/s)
    /// * `dt` - Time step (seconds)
    ///
    /// # Returns
    /// Updated pose (x, y, theta)
    pub fn update_odometry(
        &self,
        pose: (f64, f64, f64),
        left_speed: f64,
        right_speed: f64,
        dt: f64,
    ) -> (f64, f64, f64) {
        let (linear, angular) = self.forward_kinematics(left_speed, right_speed);

        let (x, y, theta) = pose;

        let new_theta = theta + angular * dt;
        let new_x = x + linear * theta.cos() * dt;
        let new_y = y + linear * theta.sin() * dt;

        (new_x, new_y, new_theta)
    }

    /// Get wheel base
    pub fn get_wheel_base(&self) -> f64 {
        self.wheel_base
    }

    /// Get wheel radius
    pub fn get_wheel_radius(&self) -> f64 {
        self.wheel_radius
    }

    /// Set wheel base
    pub fn set_wheel_base(&mut self, wheel_base: f64) {
        self.wheel_base = wheel_base;
    }

    /// Set wheel radius
    pub fn set_wheel_radius(&mut self, wheel_radius: f64) {
        self.wheel_radius = wheel_radius;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_motion() {
        let dd = DifferentialDrive::new(0.5, 0.1);

        // Both wheels same speed = straight line
        let (linear, angular) = dd.forward_kinematics(1.0, 1.0);

        assert_eq!(linear, 1.0);
        assert_eq!(angular, 0.0);
    }

    #[test]
    fn test_rotation_in_place() {
        let dd = DifferentialDrive::new(0.5, 0.1);

        // Opposite wheel speeds = rotation
        let (linear, angular) = dd.forward_kinematics(-0.5, 0.5);

        assert_eq!(linear, 0.0);
        assert!(angular > 0.0);
    }

    #[test]
    fn test_arc_motion() {
        let dd = DifferentialDrive::new(0.5, 0.1);

        // Different wheel speeds = arc
        let (linear, angular) = dd.forward_kinematics(0.8, 1.2);

        assert!(linear > 0.0);
        assert!(angular > 0.0);
    }

    #[test]
    fn test_inverse_kinematics() {
        let dd = DifferentialDrive::new(0.5, 0.1);

        // Forward motion
        let (left, right) = dd.inverse_kinematics(1.0, 0.0);
        assert_eq!(left, 1.0);
        assert_eq!(right, 1.0);

        // Rotation
        let (left, right) = dd.inverse_kinematics(0.0, 1.0);
        assert!(left < 0.0);
        assert!(right > 0.0);
    }

    #[test]
    fn test_roundtrip() {
        let dd = DifferentialDrive::new(0.5, 0.1);

        let linear = 0.8;
        let angular = 0.3;

        let (left, right) = dd.inverse_kinematics(linear, angular);
        let (linear2, angular2) = dd.forward_kinematics(left, right);

        assert!((linear - linear2).abs() < 0.001);
        assert!((angular - angular2).abs() < 0.001);
    }

    #[test]
    fn test_wheel_conversions() {
        let dd = DifferentialDrive::new(0.5, 0.1);

        let angular = 10.0;  // rad/s
        let linear = dd.wheel_angular_to_linear(angular);
        let angular2 = dd.wheel_linear_to_angular(linear);

        assert!((angular - angular2).abs() < 0.001);
    }

    #[test]
    fn test_odometry_forward() {
        let dd = DifferentialDrive::new(0.5, 0.1);

        let pose = (0.0, 0.0, 0.0);
        let new_pose = dd.update_odometry(pose, 1.0, 1.0, 1.0);

        // Moving forward at 1 m/s for 1 second
        assert!((new_pose.0 - 1.0).abs() < 0.01);
        assert!(new_pose.1.abs() < 0.01);
        assert!(new_pose.2.abs() < 0.01);
    }

    #[test]
    fn test_odometry_rotation() {
        let dd = DifferentialDrive::new(0.5, 0.1);

        let pose = (0.0, 0.0, 0.0);
        let new_pose = dd.update_odometry(pose, -0.5, 0.5, 1.0);

        // Rotating in place
        assert!(new_pose.0.abs() < 0.01);
        assert!(new_pose.1.abs() < 0.01);
        assert!(new_pose.2 > 0.0);
    }

    #[test]
    fn test_getters_setters() {
        let mut dd = DifferentialDrive::new(0.5, 0.1);

        assert_eq!(dd.get_wheel_base(), 0.5);
        assert_eq!(dd.get_wheel_radius(), 0.1);

        dd.set_wheel_base(0.6);
        dd.set_wheel_radius(0.12);

        assert_eq!(dd.get_wheel_base(), 0.6);
        assert_eq!(dd.get_wheel_radius(), 0.12);
    }
}
