//! # HORUS Standard Library
//!
//! The official standard library for the HORUS robotics framework.
//!
//! ## Structure
//!
//! ```text
//! horus_library/
//! ├── messages/       # Shared memory-safe messages
//! ├── nodes/          # Reusable nodes
//! ├── algorithms/     # Common algorithms (future)
//! ├── apps/           # Complete demo applications
//! └── tools/          # Development utilities
//! ```
//!
//! ## Usage
//!
//! ```rust
//! // Message types and nodes are re-exported at the root for convenience
//! use horus_library::{
//!     // Messages
//!     KeyboardInput, JoystickInput, CmdVel, LaserScan, Image, Twist,
//!     // Nodes
//!     CameraNode, LidarNode, DifferentialDriveNode, EmergencyStopNode
//! };
//!
//! // Create and configure nodes with simple constructors
//! let camera = CameraNode::new();              // Uses "camera/image" topic
//! let lidar = LidarNode::new();               // Uses "scan" topic
//! let drive = DifferentialDriveNode::new();   // Subscribes to "cmd_vel"
//! let emergency = EmergencyStopNode::new();   // Emergency stop handler
//!
//! // Or import from specific modules
//! use horus_library::messages::{Direction, SnakeState};
//! use horus_library::nodes::{PidControllerNode, SafetyMonitorNode};
//! ```

pub mod messages;
pub mod nodes;

// Re-export message types at the crate root for convenience
pub use messages::*;

// Re-export commonly used nodes for convenience
pub use nodes::{
    // Safety & Monitoring
    EmergencyStopNode, SafetyMonitorNode,
    // Sensors
    CameraNode, LidarNode, ImuNode,
    // Control
    DifferentialDriveNode, PidControllerNode,
    // Industrial
    ModbusNode,
    // Input (existing)
    KeyboardInputNode, JoystickInputNode,
};