//! HFrame integration for Sim3D
//!
//! This module provides Bevy-compatible wrappers around HFrame, the high-performance
//! lock-free transform system. All coordinate transforms use f64 precision internally
//! for robotics-grade accuracy.
//!
//! # Usage
//!
//! ```rust,ignore
//! use sim3d::hframe::{BevyHFrame, BevyTransform};
//!
//! // In Bevy system
//! fn my_system(mut hframe: ResMut<BevyHFrame>) {
//!     hframe.register_frame("camera", Some("base_link")).unwrap();
//!     hframe.update_transform("camera", &BevyTransform::from_translation([0.0, 0.0, 0.5]));
//!     let tf = hframe.tf("camera", "world").unwrap();
//! }
//! ```

pub mod bevy_wrapper;
pub mod urdf_parser;

// Re-export wrapper types for convenience
pub use bevy_wrapper::{
    bevy_transform_to_hframe, hframe_to_bevy_transform, hframe_to_isometry, isometry_to_hframe,
    render_tf_frames, urdf_origin_to_hframe, urdf_origin_to_isometry, BevyHFrame, BevyTransform,
    HFrameFilter, HFrameResource, HFrameVisualizer, HFrameVisualizerConfig,
};

// Backwards-compatible type aliases (for migration from tf/)
pub use bevy_wrapper::TFFilter;
pub use bevy_wrapper::TFTree;
pub use bevy_wrapper::TFVisualizer;

// Re-export URDF parser
pub use urdf_parser::{
    URDFCollision, URDFDynamics, URDFGeometry, URDFInertial, URDFJoint, URDFJointType, URDFLimit,
    URDFLink, URDFMaterial, URDFParser, URDFPose, URDFRobot, URDFVisual,
};

// Re-export core HFrame types for direct access
pub mod core {
    pub use horus_library::hframe::{
        frame_id_to_string, string_to_frame_id, timestamp_now, FrameId, FrameRegistry, FrameSlot,
        HFrame, HFrameConfig, HFrameCore, HFrameError, StaticTransformStamped, TFMessage,
        Transform, TransformStamped, FRAME_ID_SIZE, MAX_TRANSFORMS_PER_MESSAGE,
    };
}
