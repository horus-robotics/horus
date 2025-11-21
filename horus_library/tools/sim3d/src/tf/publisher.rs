use bevy::prelude::*;
use std::collections::HashMap;

use crate::tf::tree::TFTree;

/// TF message for external communication (e.g., ROS compatibility)
#[derive(Clone, Debug)]
pub struct TFMessage {
    pub frame_id: String,
    pub child_frame_id: String,
    pub timestamp: f32,
    pub translation: Vec3,
    pub rotation: Quat,
}

impl TFMessage {
    pub fn new(
        frame_id: impl Into<String>,
        child_frame_id: impl Into<String>,
        timestamp: f32,
        transform: Transform,
    ) -> Self {
        Self {
            frame_id: frame_id.into(),
            child_frame_id: child_frame_id.into(),
            timestamp,
            translation: transform.translation,
            rotation: transform.rotation,
        }
    }

    pub fn to_transform(&self) -> Transform {
        Transform {
            translation: self.translation,
            rotation: self.rotation,
            scale: Vec3::ONE,
        }
    }
}

/// Batch of TF messages
#[derive(Clone, Debug, Default)]
pub struct TFMessageBatch {
    pub messages: Vec<TFMessage>,
}

impl TFMessageBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, message: TFMessage) {
        self.messages.push(message);
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

/// TF lookup utilities
pub struct TFLookup;

impl TFLookup {
    /// Look up transform from source frame to target frame
    pub fn lookup_transform(
        tf_tree: &TFTree,
        source_frame: &str,
        target_frame: &str,
    ) -> Option<Transform> {
        // Use TFTree's built-in lookup which computes the relative transform
        let isometry = tf_tree.lookup_transform(source_frame, target_frame).ok()?;

        // Convert Isometry3 to Bevy Transform
        let translation = Vec3::new(
            isometry.translation.x,
            isometry.translation.y,
            isometry.translation.z,
        );

        let rotation = Quat::from_xyzw(
            isometry.rotation.i,
            isometry.rotation.j,
            isometry.rotation.k,
            isometry.rotation.w,
        );

        Some(Transform::from_translation(translation).with_rotation(rotation))
    }

    /// Get chain of frames from source to target
    pub fn get_frame_chain(
        tf_tree: &TFTree,
        source_frame: &str,
        target_frame: &str,
    ) -> Option<Vec<String>> {
        let mut chain = Vec::new();

        // Trace from source to root
        let mut current = source_frame.to_string();
        let mut source_to_root = vec![current.clone()];

        while !current.is_empty() && current != "world" {
            if let Some(frame) = tf_tree.get_frame(&current) {
                if let Some(parent) = &frame.parent {
                    current = parent.clone();
                    source_to_root.push(current.clone());
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Trace from target to root
        current = target_frame.to_string();
        let mut target_to_root = vec![current.clone()];

        while !current.is_empty() && current != "world" {
            if let Some(frame) = tf_tree.get_frame(&current) {
                if let Some(parent) = &frame.parent {
                    current = parent.clone();
                    target_to_root.push(current.clone());
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Find common ancestor
        let mut common_ancestor_idx = None;
        for (i, source_frame) in source_to_root.iter().enumerate() {
            for (j, target_frame) in target_to_root.iter().enumerate() {
                if source_frame == target_frame {
                    common_ancestor_idx = Some((i, j));
                    break;
                }
            }
            if common_ancestor_idx.is_some() {
                break;
            }
        }

        if let Some((src_idx, tgt_idx)) = common_ancestor_idx {
            // Build chain: source -> common ancestor -> target
            for i in 0..src_idx {
                chain.push(source_to_root[i].clone());
            }
            for i in (0..=tgt_idx).rev() {
                if target_to_root[i] != source_to_root[src_idx] || i == 0 {
                    chain.push(target_to_root[i].clone());
                }
            }
            Some(chain)
        } else {
            None
        }
    }

    /// Check if transform from source to target is available
    pub fn can_transform(tf_tree: &TFTree, source_frame: &str, target_frame: &str) -> bool {
        if source_frame == target_frame {
            return true;
        }
        Self::get_frame_chain(tf_tree, source_frame, target_frame).is_some()
    }

    /// Get all child frames of a given frame
    pub fn get_child_frames(tf_tree: &TFTree, parent_frame: &str) -> Vec<String> {
        let mut children = Vec::new();

        for frame_name in tf_tree.get_all_frames() {
            if let Some(frame) = tf_tree.get_frame(frame_name) {
                if let Some(parent) = &frame.parent {
                    if parent == parent_frame {
                        children.push(frame_name.clone());
                    }
                }
            }
        }

        children
    }

    /// Get all descendant frames (recursive children)
    pub fn get_descendant_frames(tf_tree: &TFTree, parent_frame: &str) -> Vec<String> {
        let mut descendants = Vec::new();
        let children = Self::get_child_frames(tf_tree, parent_frame);

        for child in children {
            descendants.push(child.clone());
            descendants.extend(Self::get_descendant_frames(tf_tree, &child));
        }

        descendants
    }
}

/// TF interpolation utilities
pub struct TFInterpolator;

impl TFInterpolator {
    /// Linear interpolation between two transforms
    pub fn lerp(a: &Transform, b: &Transform, t: f32) -> Transform {
        let t = t.clamp(0.0, 1.0);

        Transform {
            translation: a.translation.lerp(b.translation, t),
            rotation: a.rotation.slerp(b.rotation, t),
            scale: a.scale.lerp(b.scale, t),
        }
    }

    /// Interpolate transform at given time from historical data
    pub fn interpolate_at_time(transforms: &[(f32, Transform)], time: f32) -> Option<Transform> {
        if transforms.is_empty() {
            return None;
        }

        if transforms.len() == 1 {
            return Some(transforms[0].1);
        }

        // Find surrounding timestamps
        let mut before = None;
        let mut after = None;

        for (i, (t, _)) in transforms.iter().enumerate() {
            if *t <= time {
                before = Some(i);
            }
            if *t >= time && after.is_none() {
                after = Some(i);
            }
        }

        match (before, after) {
            (Some(b), Some(a)) if b != a => {
                let (t0, transform0) = transforms[b];
                let (t1, transform1) = transforms[a];
                let alpha = (time - t0) / (t1 - t0);
                Some(Self::lerp(&transform0, &transform1, alpha))
            }
            (Some(i), _) | (_, Some(i)) => Some(transforms[i].1),
            _ => None,
        }
    }
}

/// TF broadcaster for exporting transforms to external systems
#[derive(Default)]
pub struct TFBroadcaster {
    pending_messages: Vec<TFMessage>,
}

impl TFBroadcaster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Send a transform
    pub fn send_transform(&mut self, message: TFMessage) {
        self.pending_messages.push(message);
    }

    /// Send multiple transforms
    pub fn send_transforms(&mut self, messages: Vec<TFMessage>) {
        self.pending_messages.extend(messages);
    }

    /// Get all pending messages (and clear the queue)
    pub fn take_pending(&mut self) -> Vec<TFMessage> {
        std::mem::take(&mut self.pending_messages)
    }

    /// Get number of pending messages
    pub fn pending_count(&self) -> usize {
        self.pending_messages.len()
    }

    /// Clear pending messages
    pub fn clear(&mut self) {
        self.pending_messages.clear();
    }
}

/// TF buffer for storing transform history
pub struct TFBuffer {
    /// History duration in seconds
    pub history_duration: f32,
    /// Transform history per frame
    frame_history: HashMap<String, Vec<(f32, Transform)>>,
}

impl TFBuffer {
    pub fn new(history_duration: f32) -> Self {
        Self {
            history_duration,
            frame_history: HashMap::new(),
        }
    }

    /// Add transform to buffer
    pub fn add_transform(&mut self, frame_id: String, time: f32, transform: Transform) {
        let history = self.frame_history.entry(frame_id).or_default();

        history.push((time, transform));

        // Remove old entries
        let cutoff_time = time - self.history_duration;
        history.retain(|(t, _)| *t >= cutoff_time);

        // Keep sorted by time
        history.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    /// Get transform at specific time (with interpolation)
    pub fn lookup_transform_at_time(&self, frame_id: &str, time: f32) -> Option<Transform> {
        let history = self.frame_history.get(frame_id)?;
        TFInterpolator::interpolate_at_time(history, time)
    }

    /// Get latest transform
    pub fn get_latest_transform(&self, frame_id: &str) -> Option<Transform> {
        let history = self.frame_history.get(frame_id)?;
        history.last().map(|(_, t)| *t)
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.frame_history.clear();
    }

    /// Get number of frames in buffer
    pub fn frame_count(&self) -> usize {
        self.frame_history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tf_message() {
        let transform = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let msg = TFMessage::new("world", "robot", 1.0, transform);

        assert_eq!(msg.frame_id, "world");
        assert_eq!(msg.child_frame_id, "robot");
        assert_eq!(msg.timestamp, 1.0);
        assert_eq!(msg.translation, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_tf_message_batch() {
        let mut batch = TFMessageBatch::new();
        assert!(batch.is_empty());

        let msg = TFMessage::new("world", "robot", 1.0, Transform::IDENTITY);
        batch.add(msg);

        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_tf_interpolator_lerp() {
        let a = Transform::from_translation(Vec3::new(0.0, 0.0, 0.0));
        let b = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0));

        let mid = TFInterpolator::lerp(&a, &b, 0.5);
        assert_eq!(mid.translation, Vec3::new(5.0, 0.0, 0.0));
    }

    #[test]
    fn test_tf_interpolator_at_time() {
        let transforms = vec![
            (0.0, Transform::from_translation(Vec3::new(0.0, 0.0, 0.0))),
            (1.0, Transform::from_translation(Vec3::new(10.0, 0.0, 0.0))),
        ];

        let result = TFInterpolator::interpolate_at_time(&transforms, 0.5).unwrap();
        assert_eq!(result.translation, Vec3::new(5.0, 0.0, 0.0));
    }

    #[test]
    fn test_tf_broadcaster() {
        let mut broadcaster = TFBroadcaster::new();
        assert_eq!(broadcaster.pending_count(), 0);

        let msg = TFMessage::new("world", "robot", 1.0, Transform::IDENTITY);
        broadcaster.send_transform(msg);

        assert_eq!(broadcaster.pending_count(), 1);

        let pending = broadcaster.take_pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(broadcaster.pending_count(), 0);
    }

    #[test]
    fn test_tf_buffer() {
        let mut buffer = TFBuffer::new(10.0);

        buffer.add_transform(
            "robot".to_string(),
            1.0,
            Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
        );
        buffer.add_transform(
            "robot".to_string(),
            2.0,
            Transform::from_translation(Vec3::new(2.0, 0.0, 0.0)),
        );

        let latest = buffer.get_latest_transform("robot").unwrap();
        assert_eq!(latest.translation, Vec3::new(2.0, 0.0, 0.0));

        let interp = buffer.lookup_transform_at_time("robot", 1.5).unwrap();
        assert_eq!(interp.translation, Vec3::new(1.5, 0.0, 0.0));
    }
}
