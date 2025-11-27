//! # HORUS Standard Library
//!
//! The official standard library for the HORUS robotics framework.
//!
//! ## Structure
//!
//! ```text
//! horus_library/
//! ── messages/       # Shared memory-safe messages
//! ── nodes/          # Reusable nodes
//! ── algorithms/     # Common algorithms (future)
//! ── hframe/         # HFrame - High-performance transform system
//! ── apps/           # Complete demo applications
//! ── tools/          # Development utilities (sim2d, sim3d)
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Message types, traits, and nodes are re-exported at the root for convenience
//! use horus_library::{
//!     // Core traits
//!     LogSummary,
//!     // Messages
//!     KeyboardInput, JoystickInput, CmdVel, LaserScan, Image, Twist,
//!     // Nodes (feature-gated)
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
//!
//! // Use HFrame for coordinate transforms
//! use horus_library::hframe::{HFrame, Transform};
//!
//! // Use simulators (separate crates to avoid cyclic deps)
//! use sim2d::{Sim2DBuilder, RobotConfig};
//! use sim3d::rl::{RLTask, Action, Observation};
//! ```

pub mod algorithms;
pub mod hframe;
pub mod messages;
pub mod nodes;

// Note: sim2d and sim3d are separate crates to avoid cyclic dependencies.
// Access them directly via:
//   - Rust: use sim2d::*; or use sim3d::*;
//   - Python: from horus.library.sim2d import Sim2D
//             from horus.library.sim3d import make_env

// Re-export core traits needed for message types
pub use horus_core::core::LogSummary;

// Re-export message types at the crate root for convenience
pub use messages::*;

// Re-export commonly used nodes for convenience
// Always available (hardware-independent)
pub use nodes::{DifferentialDriveNode, EmergencyStopNode, PidControllerNode, SafetyMonitorNode};

// Feature-gated hardware nodes
#[cfg(any(
    feature = "opencv-backend",
    feature = "v4l2-backend",
    feature = "realsense",
    feature = "zed"
))]
pub use nodes::CameraNode;

#[cfg(any(feature = "bno055-imu", feature = "mpu6050-imu"))]
pub use nodes::ImuNode;

#[cfg(feature = "gilrs")]
pub use nodes::JoystickInputNode;

#[cfg(feature = "crossterm")]
pub use nodes::KeyboardInputNode;

#[cfg(feature = "rplidar")]
pub use nodes::LidarNode;

#[cfg(feature = "modbus-hardware")]
pub use nodes::ModbusNode;

/// Prelude module for convenient imports
///
/// # Usage
/// ```rust,ignore
/// use horus_library::prelude::*;
///
/// // For simulation, import sim2d/sim3d directly:
/// use sim2d::{Sim2DBuilder, RobotConfig};
/// use sim3d::rl::{RLTask, make_env};  // separate crate
/// ```
pub mod prelude {
    // Core traits
    pub use crate::LogSummary;

    // Common message types
    pub use crate::messages::{
        cmd_vel::CmdVel,
        geometry::{Point3, Pose2D, Quaternion, Twist, Vector3},
        sensor::{BatteryState, Imu, LaserScan, NavSatFix, Odometry},
    };

    // HFrame - High-performance transform system
    pub use crate::hframe::{
        timestamp_now, FrameId, FrameRegistry, FrameSlot, HFrame, HFrameConfig, HFrameCore,
        HFrameError, StaticTransformStamped, TFMessage, Transform, TransformStamped,
    };

    // Common nodes
    pub use crate::nodes::{
        DifferentialDriveNode, EmergencyStopNode, PidControllerNode, SafetyMonitorNode,
    };

    // Note: sim2d and sim3d are separate crates to avoid cyclic dependencies.
    // Import them directly: use sim2d::*; or use sim3d::*;
}
