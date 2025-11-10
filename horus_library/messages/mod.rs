// Message types for the HORUS robotics framework
//
// This module contains all standardized message types used for communication
// between HORUS components (nodes, algorithms, and applications).
//
// # Message Organization
//
// Messages are organized by domain:
// - Geometry: Spatial primitives (Twist, Pose2D, Transform, etc.)
// - Sensor: Sensor data formats (LaserScan, Imu, Odometry, etc.)
// - Control: Actuator commands (MotorCommand, ServoCommand, PID, etc.)
// - Diagnostics: System health (Status, Heartbeat, EmergencyStop, etc.)
// - Input: User input (KeyboardInput, JoystickInput)
// - Application: App-specific messages (SnakeState, Direction, etc.)
//
// All message types are re-exported at the crate root for convenience.

// Core message modules
pub mod control;
pub mod coordination;
pub mod diagnostics;
pub mod force;
pub mod geometry;
pub mod io;
pub mod navigation;
pub mod perception;
pub mod sensor;
pub mod timing;
pub mod vision;

// Input messages
pub mod joystick_msg;
pub mod keyboard_input_msg;

// Application-specific messages
pub mod cmd_vel;
pub mod snake_state;

// Re-export all message types for convenience
// Geometry
pub use geometry::{Point3, Pose2D, Quaternion, Transform, Twist, Vector3};

// Sensor
pub use sensor::{BatteryState, Imu, LaserScan, NavSatFix, Odometry, Range};

// Control
pub use control::{
    DifferentialDriveCommand, JointCommand, MotorCommand, PidConfig, PwmCommand, ServoCommand,
    TrajectoryPoint,
};

// Diagnostics
pub use diagnostics::{
    DiagnosticReport, DiagnosticValue, EmergencyStop, HealthStatus, Heartbeat, NodeHeartbeat,
    NodeState, ResourceUsage, SafetyStatus, Status, StatusLevel,
};

// Vision
pub use vision::{CameraInfo, CompressedImage, Detection, DetectionArray, Image};

// Navigation
pub use navigation::{CostMap, Goal, OccupancyGrid, Path, PathPlan};

// Force
pub use force::{ForceCommand, ImpedanceParameters, TactileArray, WrenchStamped};

// Industrial I/O
pub use io::{
    AnalogIO, DigitalIO, EtherNetIPMessage, I2cMessage, ModbusMessage, NetworkStatus, SerialData,
};

// Perception
pub use perception::{BoundingBox3D, DepthImage, PlaneDetection, PointCloud};

// Coordination
pub use coordination::{FleetStatus, FormationControl, RobotState, TaskAssignment};

// Timing
pub use timing::{ClockStats, ScheduledEvent, TimeSync, Timeline};

// Input (existing)
pub use joystick_msg::JoystickInput;
pub use keyboard_input_msg::KeyboardInput;

// Application (existing)
pub use cmd_vel::CmdVel;
pub use snake_state::{Direction, SnakeState};
