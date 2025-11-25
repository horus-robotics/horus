//! TF Tree - Frame hierarchy and transform lookup
//!
//! Manages a tree of coordinate frames and provides efficient
//! transform lookup between any two frames.

use super::buffer::CircularBuffer;
use super::transform::Transform;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Default buffer duration in seconds
const DEFAULT_BUFFER_DURATION_SECS: f64 = 10.0;

/// Default buffer capacity (samples per frame)
const DEFAULT_BUFFER_CAPACITY: usize = 1000;

/// TF errors
#[derive(Debug, Error)]
pub enum TFError {
    #[error("Frame '{0}' not found")]
    FrameNotFound(String),

    #[error("Parent frame '{0}' does not exist")]
    ParentNotFound(String),

    #[error("No common ancestor found between '{0}' and '{1}'")]
    NoCommonAncestor(String, String),

    #[error("Adding transform would create a cycle")]
    CycleDetected,

    #[error("Transform not available at time {0}")]
    TransformNotAvailable(u64),

    #[error("Frame '{0}' already exists")]
    FrameAlreadyExists(String),
}

/// Result type for TF operations
pub type TFResult<T> = Result<T, TFError>;

/// A node in the transform tree
#[derive(Debug, Clone)]
pub struct FrameNode {
    /// Frame identifier
    pub name: String,
    /// Parent frame name (None for root)
    pub parent: Option<String>,
    /// Child frame names
    pub children: Vec<String>,
    /// Whether this is a static (unchanging) frame
    pub is_static: bool,
    /// Static transform (if is_static is true)
    pub static_transform: Option<Transform>,
    /// Time-based transform buffer (if is_static is false)
    pub transform_buffer: CircularBuffer<(u64, Transform)>,
}

impl FrameNode {
    /// Create a new root frame node
    pub fn new_root(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            is_static: true,
            static_transform: Some(Transform::identity()),
            transform_buffer: CircularBuffer::new(1),
        }
    }

    /// Create a new static frame node
    pub fn new_static(
        name: impl Into<String>,
        parent: impl Into<String>,
        transform: Transform,
    ) -> Self {
        Self {
            name: name.into(),
            parent: Some(parent.into()),
            children: Vec::new(),
            is_static: true,
            static_transform: Some(transform),
            transform_buffer: CircularBuffer::new(1),
        }
    }

    /// Create a new dynamic frame node
    pub fn new_dynamic(
        name: impl Into<String>,
        parent: impl Into<String>,
        buffer_capacity: usize,
    ) -> Self {
        Self {
            name: name.into(),
            parent: Some(parent.into()),
            children: Vec::new(),
            is_static: false,
            static_transform: None,
            transform_buffer: CircularBuffer::new(buffer_capacity),
        }
    }

    /// Get the transform at a specific time
    pub fn get_transform(&self, timestamp: u64) -> Option<Transform> {
        if self.is_static {
            self.static_transform
        } else {
            self.transform_buffer.get_interpolated(timestamp)
        }
    }

    /// Get the latest transform
    pub fn get_latest_transform(&self) -> Option<Transform> {
        if self.is_static {
            self.static_transform
        } else {
            self.transform_buffer.get_latest().map(|(_, tf)| tf)
        }
    }

    /// Update the transform
    pub fn update_transform(&mut self, transform: Transform, timestamp: u64) {
        if self.is_static {
            self.static_transform = Some(transform);
        } else {
            self.transform_buffer.push((timestamp, transform));
        }
    }
}

/// Transform tree for managing coordinate frames
#[derive(Debug)]
pub struct TFTree {
    /// All frames indexed by name
    frames: HashMap<String, FrameNode>,
    /// Root frame name
    root: String,
    /// Transform chain cache (source, target) -> path
    cache: HashMap<(String, String), Vec<String>>,
    /// Buffer duration in nanoseconds
    buffer_duration_ns: u64,
    /// Buffer capacity per frame
    buffer_capacity: usize,
}

impl Default for TFTree {
    fn default() -> Self {
        Self::new("world")
    }
}

impl TFTree {
    /// Create a new TF tree with the given root frame name
    pub fn new(root: impl Into<String>) -> Self {
        let root = root.into();
        let mut frames = HashMap::new();
        frames.insert(root.clone(), FrameNode::new_root(&root));

        Self {
            frames,
            root,
            cache: HashMap::new(),
            buffer_duration_ns: (DEFAULT_BUFFER_DURATION_SECS * 1e9) as u64,
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
        }
    }

    /// Set the buffer duration for dynamic transforms
    pub fn set_buffer_duration(&mut self, seconds: f64) {
        self.buffer_duration_ns = (seconds * 1e9) as u64;
    }

    /// Set the buffer capacity for dynamic transforms
    pub fn set_buffer_capacity(&mut self, capacity: usize) {
        self.buffer_capacity = capacity;
    }

    /// Get the root frame name
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Check if a frame exists
    pub fn has_frame(&self, name: &str) -> bool {
        self.frames.contains_key(name)
    }

    /// Get a frame by name
    pub fn get_frame(&self, name: &str) -> Option<&FrameNode> {
        self.frames.get(name)
    }

    /// Get all frame names
    pub fn get_all_frames(&self) -> Vec<String> {
        self.frames.keys().cloned().collect()
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Add a static transform (fixed, never changes)
    pub fn add_static_transform(
        &mut self,
        parent: &str,
        child: &str,
        transform: Transform,
    ) -> TFResult<()> {
        if !self.frames.contains_key(parent) {
            return Err(TFError::ParentNotFound(parent.to_string()));
        }

        if self.frames.contains_key(child) {
            // Update existing frame
            if let Some(frame) = self.frames.get_mut(child) {
                frame.is_static = true;
                frame.static_transform = Some(transform);
                frame.parent = Some(parent.to_string());
            }
        } else {
            // Create new frame
            let frame = FrameNode::new_static(child, parent, transform);
            self.frames.insert(child.to_string(), frame);

            // Add to parent's children
            if let Some(parent_frame) = self.frames.get_mut(parent) {
                if !parent_frame.children.contains(&child.to_string()) {
                    parent_frame.children.push(child.to_string());
                }
            }
        }

        // Invalidate cache
        self.cache.clear();

        Ok(())
    }

    /// Add a dynamic transform (changes over time)
    pub fn add_transform(
        &mut self,
        parent: &str,
        child: &str,
        transform: Transform,
        timestamp: u64,
    ) -> TFResult<()> {
        if !self.frames.contains_key(parent) {
            return Err(TFError::ParentNotFound(parent.to_string()));
        }

        if let Some(frame) = self.frames.get_mut(child) {
            // Update existing frame
            frame.update_transform(transform, timestamp);
        } else {
            // Create new dynamic frame
            let mut frame = FrameNode::new_dynamic(child, parent, self.buffer_capacity);
            frame.update_transform(transform, timestamp);
            self.frames.insert(child.to_string(), frame);

            // Add to parent's children
            if let Some(parent_frame) = self.frames.get_mut(parent) {
                if !parent_frame.children.contains(&child.to_string()) {
                    parent_frame.children.push(child.to_string());
                }
            }

            // Invalidate cache for new frames
            self.cache.clear();
        }

        Ok(())
    }

    /// Update an existing frame's transform
    pub fn update_transform(
        &mut self,
        name: &str,
        transform: Transform,
        timestamp: u64,
    ) -> TFResult<()> {
        let frame = self
            .frames
            .get_mut(name)
            .ok_or_else(|| TFError::FrameNotFound(name.to_string()))?;

        frame.update_transform(transform, timestamp);
        Ok(())
    }

    /// Lookup transform from source frame to target frame at specific time
    pub fn lookup_transform(
        &self,
        source: &str,
        target: &str,
        timestamp: u64,
    ) -> TFResult<Transform> {
        if source == target {
            return Ok(Transform::identity());
        }

        // Find path from source to target through common ancestor
        let (source_path, target_path) = self.find_paths_to_common_ancestor(source, target)?;

        // Compose transforms along the path
        let mut transform = Transform::identity();

        // Go from source up to common ancestor (invert transforms)
        // Each frame stores transform from parent->child, so going child->parent requires inverse
        // Skip the common ancestor (last element in source_path)
        let source_frames = if source_path.len() > 1 {
            &source_path[..source_path.len() - 1]
        } else {
            &source_path[..0]
        };

        for frame_name in source_frames {
            let frame = self
                .frames
                .get(frame_name)
                .ok_or_else(|| TFError::FrameNotFound(frame_name.clone()))?;

            let frame_tf = frame
                .get_transform(timestamp)
                .ok_or(TFError::TransformNotAvailable(timestamp))?;

            transform = transform.compose(&frame_tf.inverse());
        }

        // Go from common ancestor down to target
        // Skip the common ancestor (last element in target_path)
        let target_frames = if target_path.len() > 1 {
            &target_path[..target_path.len() - 1]
        } else {
            &target_path[..0]
        };

        for frame_name in target_frames.iter().rev() {
            let frame = self
                .frames
                .get(frame_name)
                .ok_or_else(|| TFError::FrameNotFound(frame_name.clone()))?;

            let frame_tf = frame
                .get_transform(timestamp)
                .ok_or(TFError::TransformNotAvailable(timestamp))?;

            transform = transform.compose(&frame_tf);
        }

        Ok(transform)
    }

    /// Lookup the latest transform between two frames
    pub fn lookup_latest_transform(&self, source: &str, target: &str) -> TFResult<Transform> {
        // Use a very large timestamp to get latest
        self.lookup_transform(source, target, u64::MAX)
    }

    /// Check if transform is available between two frames
    pub fn can_transform(&self, source: &str, target: &str) -> bool {
        if source == target {
            return true;
        }
        self.find_paths_to_common_ancestor(source, target).is_ok()
    }

    /// Get the chain of frames from source to target
    pub fn get_frame_chain(&self, source: &str, target: &str) -> TFResult<Vec<String>> {
        if source == target {
            return Ok(vec![source.to_string()]);
        }

        let (mut source_path, target_path) = self.find_paths_to_common_ancestor(source, target)?;

        // Combine paths: source -> common ancestor -> target
        for frame in target_path.into_iter().rev().skip(1) {
            source_path.push(frame);
        }

        Ok(source_path)
    }

    /// Find paths from source and target to their common ancestor
    fn find_paths_to_common_ancestor(
        &self,
        source: &str,
        target: &str,
    ) -> TFResult<(Vec<String>, Vec<String>)> {
        // Build path from source to root
        let mut source_path = vec![source.to_string()];
        let mut current = source.to_string();

        while let Some(frame) = self.frames.get(&current) {
            if let Some(parent) = &frame.parent {
                source_path.push(parent.clone());
                current = parent.clone();
            } else {
                break;
            }
        }

        // Build path from target to root
        let mut target_path = vec![target.to_string()];
        current = target.to_string();

        while let Some(frame) = self.frames.get(&current) {
            if let Some(parent) = &frame.parent {
                target_path.push(parent.clone());
                current = parent.clone();
            } else {
                break;
            }
        }

        // Find common ancestor
        for (i, source_frame) in source_path.iter().enumerate() {
            if let Some(j) = target_path.iter().position(|f| f == source_frame) {
                // Found common ancestor
                return Ok((
                    source_path[..=i].to_vec(),
                    target_path[..=j].to_vec(),
                ));
            }
        }

        Err(TFError::NoCommonAncestor(
            source.to_string(),
            target.to_string(),
        ))
    }

    /// Get all child frames of a given frame
    pub fn get_children(&self, parent: &str) -> Vec<String> {
        self.frames
            .get(parent)
            .map(|f| f.children.clone())
            .unwrap_or_default()
    }

    /// Get all descendant frames (recursive)
    pub fn get_descendants(&self, parent: &str) -> Vec<String> {
        let mut descendants = Vec::new();
        let children = self.get_children(parent);

        for child in children {
            descendants.push(child.clone());
            descendants.extend(self.get_descendants(&child));
        }

        descendants
    }

    /// Remove a frame from the tree
    pub fn remove_frame(&mut self, name: &str) -> TFResult<()> {
        if name == self.root {
            return Err(TFError::FrameNotFound("Cannot remove root frame".to_string()));
        }

        let frame = self
            .frames
            .remove(name)
            .ok_or_else(|| TFError::FrameNotFound(name.to_string()))?;

        // Remove from parent's children list
        if let Some(parent_name) = &frame.parent {
            if let Some(parent) = self.frames.get_mut(parent_name) {
                parent.children.retain(|c| c != name);
            }
        }

        // Recursively remove all children
        for child in frame.children {
            let _ = self.remove_frame(&child);
        }

        // Invalidate cache
        self.cache.clear();

        Ok(())
    }

    /// Clear all frames except root
    pub fn clear(&mut self) {
        let root_frame = self.frames.remove(&self.root);
        self.frames.clear();

        if let Some(mut root) = root_frame {
            root.children.clear();
            self.frames.insert(self.root.clone(), root);
        } else {
            self.frames
                .insert(self.root.clone(), FrameNode::new_root(&self.root));
        }

        self.cache.clear();
    }

    /// Validate the tree structure
    pub fn validate(&self) -> TFResult<()> {
        // Check that all parents exist
        for (name, frame) in &self.frames {
            if let Some(parent) = &frame.parent {
                if !self.frames.contains_key(parent) {
                    return Err(TFError::ParentNotFound(parent.clone()));
                }
            } else if name != &self.root {
                return Err(TFError::ParentNotFound(format!(
                    "Frame '{}' has no parent but is not root",
                    name
                )));
            }
        }

        // Check for cycles (each frame should reach root)
        for name in self.frames.keys() {
            let mut visited = std::collections::HashSet::new();
            let mut current = name.clone();

            while let Some(frame) = self.frames.get(&current) {
                if !visited.insert(current.clone()) {
                    return Err(TFError::CycleDetected);
                }
                if let Some(parent) = &frame.parent {
                    current = parent.clone();
                } else {
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Thread-safe wrapper for TFTree
#[allow(dead_code)]
pub type SharedTFTree = Arc<RwLock<TFTree>>;

/// Create a new shared TF tree
#[allow(dead_code)]
pub fn create_shared_tree(root: impl Into<String>) -> SharedTFTree {
    Arc::new(RwLock::new(TFTree::new(root)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_creation() {
        let tree = TFTree::new("world");
        assert!(tree.has_frame("world"));
        assert_eq!(tree.root(), "world");
        assert_eq!(tree.frame_count(), 1);
    }

    #[test]
    fn test_add_static_transform() {
        let mut tree = TFTree::new("world");

        tree.add_static_transform("world", "base_link", Transform::from_translation([1.0, 0.0, 0.0]))
            .unwrap();

        assert!(tree.has_frame("base_link"));
        assert_eq!(tree.frame_count(), 2);
    }

    #[test]
    fn test_lookup_identity() {
        let tree = TFTree::new("world");
        let tf = tree.lookup_transform("world", "world", 0).unwrap();
        assert!(tf.is_identity(1e-10));
    }

    #[test]
    fn test_lookup_direct_transform() {
        let mut tree = TFTree::new("world");

        tree.add_static_transform("world", "robot", Transform::from_translation([1.0, 2.0, 3.0]))
            .unwrap();

        let tf = tree.lookup_transform("world", "robot", 0).unwrap();
        assert!((tf.translation[0] - 1.0).abs() < 1e-6);
        assert!((tf.translation[1] - 2.0).abs() < 1e-6);
        assert!((tf.translation[2] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_lookup_chain_transform() {
        let mut tree = TFTree::new("world");

        tree.add_static_transform("world", "base", Transform::from_translation([1.0, 0.0, 0.0]))
            .unwrap();
        tree.add_static_transform("base", "camera", Transform::from_translation([0.5, 0.0, 0.2]))
            .unwrap();

        let tf = tree.lookup_transform("world", "camera", 0).unwrap();
        assert!((tf.translation[0] - 1.5).abs() < 1e-6);
        assert!((tf.translation[2] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_lookup_inverse_transform() {
        let mut tree = TFTree::new("world");

        tree.add_static_transform("world", "robot", Transform::from_translation([1.0, 0.0, 0.0]))
            .unwrap();

        let tf = tree.lookup_transform("robot", "world", 0).unwrap();
        assert!((tf.translation[0] - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_get_frame_chain() {
        let mut tree = TFTree::new("world");

        tree.add_static_transform("world", "base", Transform::identity())
            .unwrap();
        tree.add_static_transform("base", "arm", Transform::identity())
            .unwrap();
        tree.add_static_transform("arm", "gripper", Transform::identity())
            .unwrap();

        let chain = tree.get_frame_chain("world", "gripper").unwrap();
        assert_eq!(chain, vec!["world", "base", "arm", "gripper"]);
    }

    #[test]
    fn test_parent_not_found() {
        let mut tree = TFTree::new("world");

        let result = tree.add_static_transform("nonexistent", "child", Transform::identity());
        assert!(matches!(result, Err(TFError::ParentNotFound(_))));
    }

    #[test]
    fn test_can_transform() {
        let mut tree = TFTree::new("world");

        tree.add_static_transform("world", "robot", Transform::identity())
            .unwrap();

        assert!(tree.can_transform("world", "robot"));
        assert!(tree.can_transform("robot", "world"));
        assert!(!tree.can_transform("world", "nonexistent"));
    }

    #[test]
    fn test_get_children() {
        let mut tree = TFTree::new("world");

        tree.add_static_transform("world", "robot1", Transform::identity())
            .unwrap();
        tree.add_static_transform("world", "robot2", Transform::identity())
            .unwrap();

        let children = tree.get_children("world");
        assert_eq!(children.len(), 2);
        assert!(children.contains(&"robot1".to_string()));
        assert!(children.contains(&"robot2".to_string()));
    }

    #[test]
    fn test_validate() {
        let mut tree = TFTree::new("world");

        tree.add_static_transform("world", "base", Transform::identity())
            .unwrap();
        tree.add_static_transform("base", "camera", Transform::identity())
            .unwrap();

        assert!(tree.validate().is_ok());
    }

    #[test]
    fn test_clear() {
        let mut tree = TFTree::new("world");

        tree.add_static_transform("world", "robot", Transform::identity())
            .unwrap();
        tree.add_static_transform("robot", "sensor", Transform::identity())
            .unwrap();

        assert_eq!(tree.frame_count(), 3);

        tree.clear();

        assert_eq!(tree.frame_count(), 1);
        assert!(tree.has_frame("world"));
    }
}
