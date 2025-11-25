//! TF (Transform Frame) module for sim3d
//!
//! This module provides two levels of TF functionality:
//!
//! 1. **Bevy-specific TF** (this module): Uses f32, Bevy Transform types, and
//!    nalgebra's Isometry3. Optimized for game engine integration.
//!
//! 2. **horus_library TF** (re-exported): Uses f64, engine-agnostic, and
//!    designed for high-precision robotics applications and inter-process
//!    communication via shared memory.
//!
//! # Usage
//!
//! ```rust,ignore
//! use sim3d::tf::{TFTree, TFLookup, TFBuffer};  // Bevy-specific
//! use sim3d::tf::core::{Transform as TFTransform, TFTree as CoreTFTree};  // Engine-agnostic
//! ```

pub mod publisher;
pub mod tree;
pub mod urdf_parser;
pub mod visualizer;

// Bevy-specific TF exports
pub use tree::{TFTree, TransformFrame, urdf_origin_to_isometry};
pub use publisher::{
    TFMessage, TFMessageBatch, TFLookup, TFInterpolator, TFBroadcaster,
    TFBuffer as BevyTFBuffer,
};

// URDF parser exports
pub use urdf_parser::{
    URDFParser, URDFRobot, URDFLink, URDFJoint, URDFJointType,
    URDFVisual, URDFCollision, URDFInertial, URDFGeometry,
    URDFMaterial, URDFLimit, URDFDynamics, URDFPose,
};

/// Re-export of horus_library's engine-agnostic TF types
///
/// Use these for high-precision computation and inter-process communication.
/// These types use f64 and are compatible with HORUS Hub shared memory.
pub mod core {
    pub use horus_library::tf::{
        // Core transform math
        Transform,
        // TF tree for frame hierarchy
        TFTree, TFError, FrameNode,
        // Message types (Pod-safe for shared memory)
        TransformStamped, StaticTransformStamped, TFMessage,
        // Buffering utilities
        CircularBuffer, TFBuffer,
        // Helper functions
        timestamp_now, frame_id_to_string, string_to_frame_id,
    };
}
