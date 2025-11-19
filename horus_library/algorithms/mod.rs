//! Pure computational algorithms for robotics
//!
//! This module contains pure algorithmic implementations with no I/O dependencies.
//! All algorithms are fully tested and can be reused across different nodes or applications.
//!
//! # Architecture
//!
//! - **No I/O**: Algorithms contain only computation logic
//! - **Fully tested**: Each algorithm has comprehensive test coverage (107 tests total)
//! - **Reusable**: Can be used by any node or external code
//! - **Well-documented**: Each includes usage examples and API documentation
//!
//! # Available Algorithms
//!
//! ## Motion Planning
//! - **astar**: A* grid-based optimal pathfinding
//! - **rrt**: Rapidly-exploring Random Tree sampling-based planning
//! - **pure_pursuit**: Path tracking controller for mobile robots
//!
//! ## Localization & State Estimation
//! - **ekf**: Extended Kalman Filter for 2D robot localization
//! - **kalman_filter**: Linear Kalman Filter for 1D state estimation
//! - **sensor_fusion**: Multi-sensor fusion with variance weighting
//!
//! ## Control
//! - **pid**: PID feedback control with anti-windup
//! - **differential_drive**: Differential drive kinematics and odometry
//!
//! ## Mapping
//! - **occupancy_grid**: 2D occupancy grid with ray tracing
//!
//! ## Safety & Collision Detection
//! - **aabb**: Axis-Aligned Bounding Box collision detection
//! - **safety_layer**: Multi-level safety monitoring and enforcement

pub mod aabb;
pub mod astar;
pub mod differential_drive;
pub mod ekf;
pub mod kalman_filter;
pub mod occupancy_grid;
pub mod pid;
pub mod pure_pursuit;
pub mod rrt;
pub mod safety_layer;
pub mod sensor_fusion;
