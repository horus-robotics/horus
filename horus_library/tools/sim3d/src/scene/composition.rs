//! Scene composition system for including and reusing scene components
//!
//! Allows scenes to include other scenes, enabling modular scene design.

// Scene composer stores base_dir for path resolution
#![allow(dead_code)]

use super::loader::SceneDefinition;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Scene with include support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposableScene {
    /// Scene includes
    #[serde(default)]
    pub includes: Vec<SceneInclude>,

    /// Parameters for substitution
    #[serde(default)]
    pub parameters: HashMap<String, serde_yaml::Value>,

    /// Base scene definition
    #[serde(flatten)]
    pub scene: SceneDefinition,
}

/// Scene include directive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneInclude {
    /// Path to scene file to include
    pub file: String,

    /// Optional namespace prefix for included objects/robots
    pub namespace: Option<String>,

    /// Position offset for all included entities
    #[serde(default)]
    pub position_offset: [f32; 3],

    /// Rotation offset (quaternion)
    #[serde(default)]
    pub rotation_offset: [f32; 4],

    /// Parameter overrides for the included scene
    #[serde(default)]
    pub parameters: HashMap<String, serde_yaml::Value>,
}

impl Default for SceneInclude {
    fn default() -> Self {
        Self {
            file: String::new(),
            namespace: None,
            position_offset: [0.0, 0.0, 0.0],
            rotation_offset: [1.0, 0.0, 0.0, 0.0], // Identity quaternion
            parameters: HashMap::new(),
        }
    }
}

/// Scene composition resolver
pub struct SceneComposer {
    /// Base directory for resolving relative paths
    base_dir: PathBuf,

    /// Cache of loaded scenes to prevent circular includes
    loaded_scenes: HashMap<PathBuf, SceneDefinition>,
}

impl SceneComposer {
    /// Create a new scene composer
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            loaded_scenes: HashMap::new(),
        }
    }

    /// Load and compose a scene with all its includes
    pub fn load_and_compose(&mut self, scene_path: &Path) -> Result<SceneDefinition> {
        // Read the composable scene
        let content = std::fs::read_to_string(scene_path)
            .with_context(|| format!("Failed to read scene file: {}", scene_path.display()))?;

        let composable: ComposableScene = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse scene file: {}", scene_path.display()))?;

        // Apply parameter substitution
        let mut scene = composable.scene;
        self.apply_parameters(&mut scene, &composable.parameters)?;

        // Process includes
        for include in &composable.includes {
            let included_scene = self.load_include(include, scene_path.parent().unwrap())?;
            self.merge_scene(&mut scene, included_scene, include)?;
        }

        Ok(scene)
    }

    fn load_include(
        &mut self,
        include: &SceneInclude,
        base_path: &Path,
    ) -> Result<SceneDefinition> {
        // Resolve include path
        let include_path = if Path::new(&include.file).is_absolute() {
            PathBuf::from(&include.file)
        } else {
            base_path.join(&include.file)
        };

        // Check for circular includes
        if self.loaded_scenes.contains_key(&include_path) {
            return Ok(self.loaded_scenes[&include_path].clone());
        }

        // Load included scene
        let content = std::fs::read_to_string(&include_path).with_context(|| {
            format!("Failed to read included scene: {}", include_path.display())
        })?;

        let included: ComposableScene = serde_yaml::from_str(&content).with_context(|| {
            format!("Failed to parse included scene: {}", include_path.display())
        })?;

        // Apply parameter overrides
        let mut combined_params = included.parameters.clone();
        combined_params.extend(include.parameters.clone());

        let mut scene = included.scene;
        self.apply_parameters(&mut scene, &combined_params)?;

        // Recursively process includes
        for nested_include in &included.includes {
            let nested_scene = self.load_include(nested_include, include_path.parent().unwrap())?;
            self.merge_scene(&mut scene, nested_scene, nested_include)?;
        }

        // Cache the loaded scene
        self.loaded_scenes
            .insert(include_path.clone(), scene.clone());

        Ok(scene)
    }

    fn merge_scene(
        &self,
        target: &mut SceneDefinition,
        mut source: SceneDefinition,
        include: &SceneInclude,
    ) -> Result<()> {
        // Apply namespace to object/robot names
        if let Some(ns) = &include.namespace {
            for obj in &mut source.objects {
                obj.name = format!("{}.{}", ns, obj.name);
            }
            for robot in &mut source.robots {
                robot.name = format!("{}.{}", ns, robot.name);
            }
        }

        // Apply position and rotation offsets
        for obj in &mut source.objects {
            // Apply position offset
            obj.position[0] += include.position_offset[0];
            obj.position[1] += include.position_offset[1];
            obj.position[2] += include.position_offset[2];

            // Apply rotation offset via quaternion multiplication: q_result = q_offset * q_obj
            let q_obj = obj.rotation;
            let q_offset = include.rotation_offset;

            // Quaternion multiplication: (w1, x1, y1, z1) * (w2, x2, y2, z2)
            obj.rotation = [
                q_offset[0] * q_obj[0]
                    - q_offset[1] * q_obj[1]
                    - q_offset[2] * q_obj[2]
                    - q_offset[3] * q_obj[3],
                q_offset[0] * q_obj[1] + q_offset[1] * q_obj[0] + q_offset[2] * q_obj[3]
                    - q_offset[3] * q_obj[2],
                q_offset[0] * q_obj[2] - q_offset[1] * q_obj[3]
                    + q_offset[2] * q_obj[0]
                    + q_offset[3] * q_obj[1],
                q_offset[0] * q_obj[3] + q_offset[1] * q_obj[2] - q_offset[2] * q_obj[1]
                    + q_offset[3] * q_obj[0],
            ];
        }

        for robot in &mut source.robots {
            robot.position[0] += include.position_offset[0];
            robot.position[1] += include.position_offset[1];
            robot.position[2] += include.position_offset[2];

            // Apply rotation offset via quaternion multiplication
            let q_robot = robot.rotation;
            let q_offset = include.rotation_offset;

            robot.rotation = [
                q_offset[0] * q_robot[0]
                    - q_offset[1] * q_robot[1]
                    - q_offset[2] * q_robot[2]
                    - q_offset[3] * q_robot[3],
                q_offset[0] * q_robot[1] + q_offset[1] * q_robot[0] + q_offset[2] * q_robot[3]
                    - q_offset[3] * q_robot[2],
                q_offset[0] * q_robot[2] - q_offset[1] * q_robot[3]
                    + q_offset[2] * q_robot[0]
                    + q_offset[3] * q_robot[1],
                q_offset[0] * q_robot[3] + q_offset[1] * q_robot[2] - q_offset[2] * q_robot[1]
                    + q_offset[3] * q_robot[0],
            ];
        }

        // Merge objects and robots
        target.objects.extend(source.objects);
        target.robots.extend(source.robots);

        // Merge lighting if target doesn't have it
        if target.lighting.is_none() && source.lighting.is_some() {
            target.lighting = source.lighting;
        }

        Ok(())
    }

    fn apply_parameters(
        &self,
        _scene: &mut SceneDefinition,
        _parameters: &HashMap<String, serde_yaml::Value>,
    ) -> Result<()> {
        // Parameter substitution: Currently parameters are passed through to included scenes
        // Future enhancement: traverse scene and replace ${param_name} patterns with actual values
        // Example: position: [${x}, ${y}, ${z}] -> position: [1.0, 2.0, 3.0]
        // This would require serializing scene to YAML, doing regex replacement, then deserializing

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::loader::{SceneObject, SceneShape};

    #[test]
    fn test_scene_include_default() {
        let include = SceneInclude::default();
        assert_eq!(include.position_offset, [0.0, 0.0, 0.0]);
        assert_eq!(include.rotation_offset, [1.0, 0.0, 0.0, 0.0]);
        assert!(include.namespace.is_none());
    }

    #[test]
    fn test_merge_scene_with_namespace() {
        let composer = SceneComposer::new(".");
        let mut target = SceneDefinition {
            name: "target".to_string(),
            description: None,
            gravity: None,
            objects: vec![],
            robots: vec![],
            lighting: None,
        };

        let source = SceneDefinition {
            name: "source".to_string(),
            description: None,
            gravity: None,
            objects: vec![SceneObject {
                name: "box1".to_string(),
                shape: SceneShape::Box {
                    size: [1.0, 1.0, 1.0],
                },
                position: [0.0, 0.0, 0.0],
                rotation: [1.0, 0.0, 0.0, 0.0],
                rotation_euler: None,
                is_static: true,
                mass: 1.0,
                friction: 0.5,
                restitution: 0.0,
                color: None,
                damping: None,
            }],
            robots: vec![],
            lighting: None,
        };

        let include = SceneInclude {
            file: "test.yaml".to_string(),
            namespace: Some("env".to_string()),
            position_offset: [1.0, 0.0, 0.0],
            rotation_offset: [1.0, 0.0, 0.0, 0.0],
            parameters: HashMap::new(),
        };

        composer.merge_scene(&mut target, source, &include).unwrap();

        assert_eq!(target.objects.len(), 1);
        assert_eq!(target.objects[0].name, "env.box1");
        assert_eq!(target.objects[0].position[0], 1.0);
    }
}
