// Allow sensor-specific patterns - range loops are clearer for matrix operations
#![allow(clippy::needless_range_loop)]
#![allow(clippy::approx_constant)]

pub mod camera;
pub mod depth;
pub mod distortion;
pub mod encoder;
pub mod event_camera;
pub mod force_torque;
pub mod gps;
pub mod imu;
pub mod lidar3d;
pub mod noise;
pub mod radar;
pub mod rgbd;
pub mod segmentation;
pub mod sonar;
pub mod tactile;
pub mod thermal;

// Re-export sensor plugins

// Re-export noise models for external use

// Re-export segmentation semantic classes

// Re-export camera types

// Re-export IMU types

// Re-export LiDAR types

// Re-export GPS types

// Re-export encoder types

// Re-export force/torque sensor types

// Re-export tactile sensor types

// Re-export event camera types

// Re-export radar types

// Re-export sonar types

// Re-export thermal camera types

// Re-export distortion models
