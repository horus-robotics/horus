//! HORUS Transform (TF) System
//!
//! Coordinate frame management system for tracking relationships between
//! different coordinate frames in a robot over time.
//!
//! # Overview
//!
//! The TF system provides:
//! - Transform math (composition, inverse, interpolation)
//! - TF tree structure for frame hierarchy
//! - Time-based transform buffering
//! - TF messages for inter-node communication
//!
//! # Example
//!
//! ```rust,ignore
//! use horus_library::tf::{Transform, TFTree};
//!
//! // Create TF tree with root frame
//! let mut tree = TFTree::new("world");
//!
//! // Add robot base relative to world
//! tree.add_static_transform(
//!     "world",
//!     "base_link",
//!     Transform::from_translation([1.0, 0.0, 0.0])
//! )?;
//!
//! // Add camera relative to base
//! tree.add_static_transform(
//!     "base_link",
//!     "camera_frame",
//!     Transform::from_translation([0.5, 0.0, 0.2])
//! )?;
//!
//! // Lookup transform from camera to world
//! let tf = tree.lookup_transform("camera_frame", "world", 0)?;
//! let point_world = tf.transform_point([0.0, 0.0, 1.0]);
//! ```

mod buffer;
mod messages;
mod transform;
mod tree;

// Re-export all public types
pub use buffer::{CircularBuffer, TFBuffer};
pub use messages::{StaticTransformStamped, TFMessage, TransformStamped};
pub use transform::Transform;
pub use tree::{FrameNode, TFTree, TFError};

/// Get current timestamp in nanoseconds
pub fn timestamp_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Convert frame ID bytes to string
pub fn frame_id_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string()
}

/// Copy string to fixed-size frame ID buffer
pub fn string_to_frame_id(s: &str, buffer: &mut [u8]) {
    let bytes = s.as_bytes();
    let len = bytes.len().min(buffer.len() - 1);
    buffer[..len].copy_from_slice(&bytes[..len]);
    buffer[len..].fill(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_now() {
        let ts = timestamp_now();
        assert!(ts > 0);
    }

    #[test]
    fn test_frame_id_conversion() {
        let mut buffer = [0u8; 64];
        string_to_frame_id("base_link", &mut buffer);
        assert_eq!(frame_id_to_string(&buffer), "base_link");
    }

    #[test]
    fn test_frame_id_truncation() {
        let mut buffer = [0u8; 8];
        string_to_frame_id("very_long_frame_name", &mut buffer);
        assert_eq!(frame_id_to_string(&buffer), "very_lo");
    }
}
