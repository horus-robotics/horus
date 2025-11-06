use horus_core::core::LogSummary;
// Geometric and spatial message types for robotics
//
// This module provides fundamental geometric primitives used throughout
// robotics applications for representing position, orientation, and motion.

use serde::{Deserialize, Serialize};

/// 3D velocity command with linear and angular components
///
/// Used for commanding robot motion in 3D space. For 2D robots,
/// only x (forward) and yaw (rotation) are typically used.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Twist {
    /// Linear velocity [x, y, z] in m/s
    pub linear: [f64; 3],
    /// Angular velocity [roll, pitch, yaw] in rad/s
    pub angular: [f64; 3],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Twist {
    /// Create a new Twist message
    pub fn new(linear: [f64; 3], angular: [f64; 3]) -> Self {
        Self {
            linear,
            angular,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a 2D twist (forward velocity and rotation)
    pub fn new_2d(linear_x: f64, angular_z: f64) -> Self {
        Self::new([linear_x, 0.0, 0.0], [0.0, 0.0, angular_z])
    }

    /// Stop command (all zeros)
    pub fn stop() -> Self {
        Self::new([0.0; 3], [0.0; 3])
    }

    /// Check if all values are finite
    pub fn is_valid(&self) -> bool {
        self.linear.iter().all(|v| v.is_finite()) && self.angular.iter().all(|v| v.is_finite())
    }
}

/// 2D pose representation (position and orientation)
///
/// Commonly used for mobile robots operating in planar environments.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Pose2D {
    /// X position in meters
    pub x: f64,
    /// Y position in meters
    pub y: f64,
    /// Orientation angle in radians
    pub theta: f64,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Pose2D {
    /// Create a new 2D pose
    pub fn new(x: f64, y: f64, theta: f64) -> Self {
        Self {
            x,
            y,
            theta,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create pose at origin
    pub fn origin() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Calculate euclidean distance to another pose
    pub fn distance_to(&self, other: &Pose2D) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Normalize theta to [-pi, pi]
    pub fn normalize_angle(&mut self) {
        while self.theta > std::f64::consts::PI {
            self.theta -= 2.0 * std::f64::consts::PI;
        }
        while self.theta < -std::f64::consts::PI {
            self.theta += 2.0 * std::f64::consts::PI;
        }
    }

    /// Check if values are finite
    pub fn is_valid(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.theta.is_finite()
    }
}

/// 3D transformation (translation and rotation)
///
/// Represents a full 3D transformation using translation vector and
/// quaternion rotation. Used for coordinate frame transformations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Transform {
    /// Translation [x, y, z] in meters
    pub translation: [f64; 3],
    /// Rotation as quaternion [x, y, z, w]
    pub rotation: [f64; 4],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Transform {
    /// Create a new transform
    pub fn new(translation: [f64; 3], rotation: [f64; 4]) -> Self {
        Self {
            translation,
            rotation,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Identity transform (no translation or rotation)
    pub fn identity() -> Self {
        Self::new([0.0; 3], [0.0, 0.0, 0.0, 1.0])
    }

    /// Create from 2D pose (z=0, only yaw rotation)
    pub fn from_pose_2d(pose: &Pose2D) -> Self {
        let half_theta = pose.theta / 2.0;
        Self::new(
            [pose.x, pose.y, 0.0],
            [0.0, 0.0, half_theta.sin(), half_theta.cos()],
        )
    }

    /// Check if quaternion is normalized and values are finite
    pub fn is_valid(&self) -> bool {
        // Check finite values
        if !self.translation.iter().all(|v| v.is_finite())
            || !self.rotation.iter().all(|v| v.is_finite())
        {
            return false;
        }

        // Check quaternion normalization
        let norm = self.rotation.iter().map(|v| v * v).sum::<f64>().sqrt();
        (norm - 1.0).abs() < 0.01
    }

    /// Normalize the quaternion component
    pub fn normalize_rotation(&mut self) {
        let norm = self.rotation.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 0.0 {
            for v in &mut self.rotation {
                *v /= norm;
            }
        }
    }
}

/// 3D point representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn origin() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn distance_to(&self, other: &Point3) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// 3D vector representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&mut self) {
        let mag = self.magnitude();
        if mag > 0.0 {
            self.x /= mag;
            self.y /= mag;
            self.z /= mag;
        }
    }

    pub fn dot(&self, other: &Vector3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Vector3) -> Vector3 {
        Vector3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }
}

/// Quaternion for 3D rotation representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Quaternion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::identity()
    }
}

impl Quaternion {
    pub fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    pub fn identity() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    pub fn from_euler(roll: f64, pitch: f64, yaw: f64) -> Self {
        let cr = (roll / 2.0).cos();
        let sr = (roll / 2.0).sin();
        let cp = (pitch / 2.0).cos();
        let sp = (pitch / 2.0).sin();
        let cy = (yaw / 2.0).cos();
        let sy = (yaw / 2.0).sin();

        Self {
            x: sr * cp * cy - cr * sp * sy,
            y: cr * sp * cy + sr * cp * sy,
            z: cr * cp * sy - sr * sp * cy,
            w: cr * cp * cy + sr * sp * sy,
        }
    }

    pub fn normalize(&mut self) {
        let norm = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if norm > 0.0 {
            self.x /= norm;
            self.y /= norm;
            self.z /= norm;
            self.w /= norm;
        }
    }

    pub fn is_valid(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }
}

impl LogSummary for Twist {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for Pose2D {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for Transform {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for Point3 {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for Vector3 {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for Quaternion {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twist_creation() {
        let twist = Twist::new_2d(1.0, 0.5);
        assert_eq!(twist.linear[0], 1.0);
        assert_eq!(twist.angular[2], 0.5);
        assert!(twist.is_valid());
    }

    #[test]
    fn test_pose2d_distance() {
        let pose1 = Pose2D::new(0.0, 0.0, 0.0);
        let pose2 = Pose2D::new(3.0, 4.0, 0.0);
        assert_eq!(pose1.distance_to(&pose2), 5.0);
    }

    #[test]
    fn test_quaternion_normalization() {
        let mut q = Quaternion::new(1.0, 0.0, 0.0, 1.0);
        q.normalize();
        let norm = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
        assert!((norm - 1.0).abs() < 1e-10);
    }
}
