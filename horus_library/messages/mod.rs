//! Message types for the HORUS robotics framework
//!
//! This module contains all standardized message types used for communication
//! between HORUS components (nodes, algorithms, and applications).
//!
//! # Message Organization
//!
//! Messages are organized by domain:
//! - Geometry: Spatial primitives (Twist, Pose2D, Transform, etc.)
//! - Sensor: Sensor data formats (LaserScan, Imu, Odometry, etc.)
//! - Control: Actuator commands (MotorCommand, ServoCommand, PID, etc.)
//! - Diagnostics: System health (Status, Heartbeat, EmergencyStop, etc.)
//! - Input: User input (KeyboardInput, JoystickInput)
//! - Application: App-specific messages (SnakeState, Direction, etc.)
//!
//! All message types are re-exported at the crate root for convenience.

// Core message modules
pub mod geometry;
pub mod sensor;
pub mod control;
pub mod diagnostics;
pub mod vision;
pub mod navigation;
pub mod force;
pub mod io;
pub mod perception;
pub mod coordination;
pub mod timing;

// Input messages
pub mod keyboard_input_msg;
pub mod joystick_msg;

// Application-specific messages
pub mod snake_state;
pub mod cmd_vel;

// Re-export all message types for convenience
// Geometry
pub use geometry::{
    Twist, Pose2D, Transform, Point3, Vector3, Quaternion
};

// Sensor
pub use sensor::{
    LaserScan, Imu, Odometry, Range, BatteryState
};

// Control
pub use control::{
    MotorCommand, DifferentialDriveCommand, ServoCommand,
    PidConfig, TrajectoryPoint, JointCommand
};

// Diagnostics
pub use diagnostics::{
    Heartbeat, Status, StatusLevel, EmergencyStop,
    ResourceUsage, DiagnosticValue, DiagnosticReport, SafetyStatus,
    NodeState, HealthStatus, NodeHeartbeat
};

// Vision
pub use vision::{
    Image, CompressedImage, CameraInfo, Detection, DetectionArray
};

// Navigation
pub use navigation::{
    Goal, Path, OccupancyGrid, CostMap, PathPlan
};

// Force
pub use force::{
    WrenchStamped, TactileArray, ImpedanceParameters, ForceCommand
};

// Industrial I/O
pub use io::{
    DigitalIO, AnalogIO, ModbusMessage, EtherNetIPMessage, NetworkStatus
};

// Perception
pub use perception::{
    PointCloud, BoundingBox3D, DepthImage, PlaneDetection
};

// Coordination
pub use coordination::{
    RobotState, FleetStatus, TaskAssignment, FormationControl
};

// Timing
pub use timing::{
    TimeSync, ScheduledEvent, Timeline, ClockStats
};

// Input (existing)
pub use keyboard_input_msg::KeyboardInput;
pub use joystick_msg::JoystickInput;

// Application (existing)
pub use snake_state::{Direction, SnakeState};
pub use cmd_vel::CmdVel;
