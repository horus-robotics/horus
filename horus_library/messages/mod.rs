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
    StepperCommand, TrajectoryPoint,
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
    AnalogIO, CanFrame, DigitalIO, EtherNetIPMessage, I2cMessage, ModbusMessage, NetworkStatus,
    SerialData, SpiMessage,
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

// Imports for GenericMessage definition
use horus_core::core::LogSummary;
use serde::{Deserialize, Serialize};

/// Generic message type for cross-language communication
///
/// This message type provides a standardized way to communicate between
/// Rust and Python nodes when type-specific messages are not needed or
/// when dynamic typing is preferred.
///
/// The `data` field contains MessagePack-serialized payload, and the optional
/// `metadata` field can store additional information like message type or timestamp.
///
/// # Example (Rust)
///
/// ```rust,ignore
/// use horus::prelude::*;
/// extern crate rmp_serde;
///
/// let hub = Hub::<GenericMessage>::new("my_topic")?;
///
/// // Send a dict-like structure (requires rmp_serde dependency)
/// let data = rmp_serde::to_vec(&serde_json::json!({
///     "x": 1.0,
///     "y": 2.0
/// }))?;
///
/// let msg = GenericMessage::new(data);
/// hub.send(msg, None)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Example (Python)
///
/// ```python
/// from horus import PyHub
///
/// hub = PyHub("my_topic")
///
/// # Send automatically serializes to GenericMessage
/// hub.send({"x": 1.0, "y": 2.0}, node)
///
/// # Receive automatically deserializes
/// msg_bytes = hub.recv(node)
/// if msg_bytes:
///     import msgpack
///     data = msgpack.unpackb(msg_bytes, raw=False)
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenericMessage {
    /// MessagePack-serialized payload
    pub data: Vec<u8>,
    /// Optional metadata (e.g., message type, timestamp JSON)
    pub metadata: Option<String>,
}

impl GenericMessage {
    /// Create a new GenericMessage with raw bytes
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            metadata: None,
        }
    }

    /// Create a GenericMessage with metadata
    pub fn with_metadata(data: Vec<u8>, metadata: String) -> Self {
        Self {
            data,
            metadata: Some(metadata),
        }
    }

    /// Serialize any serde-compatible type into a GenericMessage
    ///
    /// This is the recommended way to create GenericMessage from structured data.
    /// Users don't need to handle MessagePack serialization directly.
    ///
    /// # Example
    /// ```rust,ignore
    /// use horus::prelude::*;
    /// use std::collections::HashMap;
    ///
    /// let mut data = HashMap::new();
    /// data.insert("x", 1.0);
    /// data.insert("y", 2.0);
    ///
    /// let msg = GenericMessage::from_value(&data)?;
    /// ```
    pub fn from_value<T: Serialize>(value: &T) -> Result<Self, String> {
        let data = rmp_serde::to_vec(value)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        Ok(Self::new(data))
    }

    /// Deserialize the data field into a typed value
    ///
    /// This is the recommended way to extract structured data from GenericMessage.
    /// Users don't need to handle MessagePack deserialization directly.
    ///
    /// # Example
    /// ```rust,ignore
    /// use horus::prelude::*;
    /// use std::collections::HashMap;
    ///
    /// if let Some(msg) = hub.recv(ctx) {
    ///     let data: HashMap<String, f64> = msg.to_value()?;
    ///     println!("x: {}, y: {}", data["x"], data["y"]);
    /// }
    /// ```
    pub fn to_value<'a, T: Deserialize<'a>>(&'a self) -> Result<T, String> {
        rmp_serde::from_slice(&self.data)
            .map_err(|e| format!("Failed to deserialize: {}", e))
    }
}

impl LogSummary for GenericMessage {
    fn log_summary(&self) -> String {
        if let Some(ref meta) = self.metadata {
            format!("<{} bytes, meta: {}>", self.data.len(), meta)
        } else {
            format!("<{} bytes>", self.data.len())
        }
    }
}
