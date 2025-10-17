//! HORUS Library Nodes
//!
//! This module contains pre-built, high-level nodes for common robotics applications.
//! All nodes follow the same simple API pattern: `NodeName::new()` for default configuration
//! or `NodeName::new_with_topic()` for custom topic names.
//!
//! # MVP Node Categories
//!
//! ## Safety & Monitoring (Critical for Industrial Use)
//! - `EmergencyStopNode` - Hardware emergency stop handler
//! - `SafetyMonitorNode` - Critical safety system monitoring
//!
//! ## Sensor Interfaces (Essential Building Blocks)
//! - `CameraNode` - Vision input from cameras
//! - `LidarNode` - LiDAR scanning for mapping/obstacles
//! - `ImuNode` - Inertial measurement unit for orientation
//! - `EncoderNode` - Wheel encoder feedback
//!
//! ## Control & Actuation (Movement and Control)
//! - `DifferentialDriveNode` - Mobile robot base control
//! - `PidControllerNode` - Generic PID control
//! - `ServoControllerNode` - Industrial servo control
//!
//! ## Navigation (Path Planning and Localization)
//! - `PathPlannerNode` - A*/RRT path planning algorithms
//! - `LocalizationNode` - Robot position estimation
//! - `CollisionDetectorNode` - Real-time collision avoidance
//!
//! ## Industrial Integration (Production Ready)
//! - `ModbusNode` - Modbus TCP/RTU protocol handler
//! - `DigitalIONode` - Digital I/O interface
//!
//! ## Input Devices (Existing)
//! - `KeyboardInputNode` - Keyboard input capture
//! - `JoystickInputNode` - Gamepad/joystick input
//!
//! # Usage Examples
//!
//! ```rust
//! use crate::nodes::*;
//!
//! // Create nodes with simple constructors
//! let camera = CameraNode::new();                    // Uses "camera/image" topic
//! let lidar = LidarNode::new();                      // Uses "scan" topic
//! let drive = DifferentialDriveNode::new();          // Subscribes to "cmd_vel"
//! let pid = PidControllerNode::new();                // Generic PID control
//! let emergency = EmergencyStopNode::new();          // Emergency stop handler
//! let safety = SafetyMonitorNode::new();             // Safety monitoring
//!
//! // Or with custom topics
//! let front_camera = CameraNode::new_with_topic("front_camera");
//! let motor_pid = PidControllerNode::new_with_topics("motor_setpoint", "encoder_feedback", "motor_output", "pid_config");
//!
//! // Configure as needed
//! let mut camera = CameraNode::new();
//! camera.set_resolution(1920, 1080);
//! camera.set_fps(30);
//!
//! let mut drive = DifferentialDriveNode::new();
//! drive.set_wheel_base(0.5);
//! drive.set_velocity_limits(2.0, 3.14);
//! ```

// Declare node modules
pub mod emergency_stop_node;
pub mod safety_monitor_node;
pub mod camera_node;
pub mod lidar_node;
pub mod imu_node;
pub mod differential_drive_node;
pub mod pid_controller_node;
pub mod modbus_node;
pub mod keyboard_input_node;
pub mod joystick_node;
pub mod encoder_node;
pub mod servo_controller_node;
pub mod digital_io_node;
pub mod path_planner_node;
pub mod localization_node;
pub mod collision_detector_node;

// Re-export node types for convenience
// Safety & Monitoring Nodes
pub use emergency_stop_node::EmergencyStopNode;
pub use safety_monitor_node::SafetyMonitorNode;

// Sensor Interface Nodes
pub use camera_node::CameraNode;
pub use lidar_node::LidarNode;
pub use imu_node::ImuNode;
pub use encoder_node::EncoderNode;

// Control & Actuation Nodes
pub use differential_drive_node::DifferentialDriveNode;
pub use pid_controller_node::PidControllerNode;
pub use servo_controller_node::ServoControllerNode;

// Navigation Nodes
pub use path_planner_node::PathPlannerNode;
pub use localization_node::LocalizationNode;
pub use collision_detector_node::CollisionDetectorNode;

// Industrial Integration Nodes
pub use modbus_node::ModbusNode;
pub use digital_io_node::DigitalIONode;

// Input Device Nodes (Existing)
pub use keyboard_input_node::KeyboardInputNode;
pub use joystick_node::JoystickInputNode;

// Re-export core HORUS types for convenience
pub use horus_core::{Node, NodeInfo, Hub};