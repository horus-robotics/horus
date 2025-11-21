use bevy::prelude::*;
use nalgebra::{Isometry3, Translation3, UnitQuaternion};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TransformFrame {
    pub name: String,
    pub parent: Option<String>,
    pub transform: Isometry3<f32>,
    pub children: Vec<String>,
}

#[derive(Resource, Default)]
pub struct TFTree {
    pub frames: HashMap<String, TransformFrame>,
    pub root: String,
}

impl TFTree {
    pub fn new(root: impl Into<String>) -> Self {
        let root = root.into();
        let mut frames = HashMap::new();

        frames.insert(
            root.clone(),
            TransformFrame {
                name: root.clone(),
                parent: None,
                transform: Isometry3::identity(),
                children: Vec::new(),
            },
        );

        Self { frames, root }
    }

    pub fn add_frame(
        &mut self,
        name: impl Into<String>,
        parent: impl Into<String>,
        transform: Isometry3<f32>,
    ) -> Result<(), String> {
        let name = name.into();
        let parent = parent.into();

        if !self.frames.contains_key(&parent) {
            return Err(format!("Parent frame '{}' does not exist", parent));
        }

        if let Some(parent_frame) = self.frames.get_mut(&parent) {
            parent_frame.children.push(name.clone());
        }

        self.frames.insert(
            name.clone(),
            TransformFrame {
                name: name.clone(),
                parent: Some(parent),
                transform,
                children: Vec::new(),
            },
        );

        Ok(())
    }

    pub fn update_frame(&mut self, name: &str, transform: Isometry3<f32>) -> Result<(), String> {
        self.frames
            .get_mut(name)
            .ok_or_else(|| format!("Frame '{}' not found", name))?
            .transform = transform;
        Ok(())
    }

    pub fn get_transform(&self, from: &str, to: &str) -> Result<Isometry3<f32>, String> {
        if from == to {
            return Ok(Isometry3::identity());
        }

        let mut from_path = vec![from.to_string()];
        let mut current = from;
        while let Some(frame) = self.frames.get(current) {
            if let Some(parent) = &frame.parent {
                from_path.push(parent.clone());
                current = parent;
            } else {
                break;
            }
        }

        let mut to_path = vec![to.to_string()];
        current = to;
        while let Some(frame) = self.frames.get(current) {
            if let Some(parent) = &frame.parent {
                to_path.push(parent.clone());
                current = parent;
            } else {
                break;
            }
        }

        let common_ancestor = from_path
            .iter()
            .find(|f| to_path.contains(f))
            .ok_or("No common ancestor found")?;

        let mut transform = Isometry3::identity();

        for frame_name in from_path.iter().take_while(|f| *f != common_ancestor) {
            let frame = &self.frames[frame_name];
            transform = frame.transform.inverse() * transform;
        }

        let to_ancestor_path: Vec<_> = to_path
            .iter()
            .take_while(|f| *f != common_ancestor)
            .collect();

        for frame_name in to_ancestor_path.iter().rev() {
            let frame = &self.frames[*frame_name];
            transform = frame.transform * transform;
        }

        Ok(transform)
    }

    /// Get a reference to a specific frame
    pub fn get_frame(&self, name: &str) -> Option<&TransformFrame> {
        self.frames.get(name)
    }

    /// Get all frame names in the tree
    pub fn get_all_frames(&self) -> Vec<&String> {
        self.frames.keys().collect()
    }

    /// Lookup transform between two frames (alias for get_transform)
    pub fn lookup_transform(&self, from: &str, to: &str) -> Result<Isometry3<f32>, String> {
        self.get_transform(from, to)
    }

    pub fn from_urdf(urdf: &urdf_rs::Robot) -> Self {
        let mut tree = TFTree::new("world");

        if let Some(base_link) = urdf.links.first() {
            tree.add_frame(&base_link.name, "world", Isometry3::identity())
                .ok();

            for joint in &urdf.joints {
                if tree.frames.contains_key(&joint.parent.link) {
                    let transform = urdf_origin_to_isometry(&joint.origin);
                    tree.add_frame(&joint.child.link, &joint.parent.link, transform)
                        .ok();
                }
            }
        }

        tree
    }
}

pub fn urdf_origin_to_isometry(origin: &urdf_rs::Pose) -> Isometry3<f32> {
    let translation = Translation3::new(
        origin.xyz[0] as f32,
        origin.xyz[1] as f32,
        origin.xyz[2] as f32,
    );

    let rotation = UnitQuaternion::from_euler_angles(
        origin.rpy[0] as f32,
        origin.rpy[1] as f32,
        origin.rpy[2] as f32,
    );

    Isometry3::from_parts(translation, rotation)
}
