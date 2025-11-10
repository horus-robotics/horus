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
//! - `GpsNode` - GPS/GNSS positioning for outdoor navigation
//!
//! ## Control & Actuation (Movement and Control)
//! - `DcMotorNode` - DC motor control with PWM (L298N, TB6612, etc.)
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
//! - `SerialNode` - UART/Serial communication (GPS, Arduino, sensors)
//! - `I2cBusNode` - I2C bus communication for sensors and peripherals
//!
//! ## Vision & Image Processing
//! - `ImageProcessorNode` - Image preprocessing and filtering
//!
//! ## Input Devices
//! - `KeyboardInputNode` - Keyboard input capture
//! - `JoystickInputNode` - Gamepad/joystick input
//!
//! # Usage Examples
//!
//! ```rust,ignore
//! use horus_library::nodes::*;
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

// Declare node modules (each in its own folder with README.md)
pub mod camera;
pub mod collision_detector;
pub mod dc_motor;
pub mod differential_drive;
pub mod digital_io;
pub mod emergency_stop;
pub mod encoder;
pub mod gps;
pub mod i2c_bus;
pub mod image_processor;
pub mod imu;
pub mod joystick;
pub mod keyboard_input;
pub mod lidar;
pub mod localization;
pub mod modbus;
pub mod path_planner;
pub mod pid_controller;
pub mod safety_monitor;
pub mod serial;
pub mod servo_controller;

// Re-export node types for convenience
// Safety & Monitoring Nodes
pub use emergency_stop::EmergencyStopNode;
pub use safety_monitor::SafetyMonitorNode;

// Sensor Interface Nodes
pub use camera::CameraNode;
pub use encoder::EncoderNode;
pub use gps::GpsNode;
pub use imu::ImuNode;
pub use lidar::LidarNode;

// Control & Actuation Nodes
pub use dc_motor::DcMotorNode;
pub use differential_drive::DifferentialDriveNode;
pub use pid_controller::PidControllerNode;
pub use servo_controller::ServoControllerNode;

// Navigation Nodes
pub use collision_detector::CollisionDetectorNode;
pub use localization::LocalizationNode;
pub use path_planner::PathPlannerNode;

// Industrial Integration Nodes
pub use digital_io::DigitalIONode;
pub use i2c_bus::I2cBusNode;
pub use modbus::ModbusNode;
pub use serial::SerialNode;

// Vision & Image Processing Nodes
pub use image_processor::ImageProcessorNode;

// Input Device Nodes
pub use joystick::JoystickInputNode;
pub use keyboard_input::KeyboardInputNode;

// Re-export core HORUS types for convenience
pub use horus_core::{Hub, Node, NodeInfo};
