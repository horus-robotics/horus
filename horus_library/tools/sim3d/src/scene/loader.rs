use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{EnhancedError, ErrorCategory, Result};
use crate::physics::world::PhysicsWorld;
use crate::robot::urdf_loader::URDFLoader;
use crate::scene::spawner::{ObjectSpawnConfig, ObjectSpawner, SpawnShape, SpawnedObjects};
use crate::scene::validation::SceneValidator;
use crate::tf::TFTree;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDefinition {
    pub name: String,
    pub description: Option<String>,
    pub gravity: Option<f32>,
    pub objects: Vec<SceneObject>,
    #[serde(default)]
    pub robots: Vec<SceneRobot>,
    pub lighting: Option<SceneLighting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneObject {
    pub name: String,
    pub shape: SceneShape,
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 4], // Quaternion (w, x, y, z)
    #[serde(default)]
    pub rotation_euler: Option<[f32; 3]>, // Alternative: euler angles (x, y, z) in degrees
    #[serde(default = "default_is_static")]
    pub is_static: bool,
    #[serde(default = "default_mass")]
    pub mass: f32,
    #[serde(default = "default_friction")]
    pub friction: f32,
    #[serde(default = "default_restitution")]
    pub restitution: f32,
    #[serde(default)]
    pub color: Option<[f32; 3]>, // RGB
    #[serde(default)]
    pub damping: Option<(f32, f32)>, // (linear, angular)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SceneShape {
    Box { size: [f32; 3] },
    Sphere { radius: f32 },
    Cylinder { radius: f32, height: f32 },
    Capsule { radius: f32, height: f32 },
    Ground { size_x: f32, size_z: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneRobot {
    pub name: String,
    pub urdf_path: String,
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 4], // Quaternion (w, x, y, z)
    #[serde(default)]
    pub rotation_euler: Option<[f32; 3]>, // Alternative: euler angles (x, y, z) in degrees
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneLighting {
    #[serde(default = "default_ambient")]
    pub ambient: [f32; 3],
    #[serde(default)]
    pub directional: Option<DirectionalLightConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionalLightConfig {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub illuminance: f32,
}

// Default values
fn default_is_static() -> bool {
    false
}

fn default_mass() -> f32 {
    1.0
}

fn default_friction() -> f32 {
    0.5
}

fn default_restitution() -> f32 {
    0.0
}

fn default_ambient() -> [f32; 3] {
    [0.3, 0.3, 0.3]
}

impl SceneDefinition {
    /// Load scene from YAML file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();

        // Read file with enhanced error
        let content = std::fs::read_to_string(&path_buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                EnhancedError::file_not_found(&path_buf)
            } else {
                EnhancedError::new(format!("Failed to read scene file: {}", e))
                    .with_file(&path_buf)
                    .with_category(ErrorCategory::FileNotFound)
                    .with_hint("Ensure the file exists and you have read permissions")
            }
        })?;

        // Validate before parsing
        let validator = SceneValidator::new().map_err(|e| {
            EnhancedError::new(format!("Failed to create validator: {}", e))
                .with_category(ErrorCategory::ConfigError)
        })?;

        let report = validator.validate_yaml(&content).map_err(|e| {
            EnhancedError::new(format!("Validation error: {}", e))
                .with_file(&path_buf)
                .with_category(ErrorCategory::ValidationError)
        })?;

        if !report.valid {
            let error_messages: Vec<String> =
                report.errors.iter().map(|e| format!("  - {}", e)).collect();

            return Err(EnhancedError::new(format!(
                "Scene validation failed:\n{}",
                error_messages.join("\n")
            ))
            .with_file(&path_buf)
            .with_category(ErrorCategory::ValidationError)
            .with_hint("Check the scene YAML against the schema requirements")
            .with_suggestion(
                "Common validation issues:\n  \
                 - Missing required field 'name'\n  \
                 - Invalid data types (e.g., string instead of number)\n  \
                 - Invalid array sizes (e.g., position must have exactly 3 elements)\n  \
                 - Out of range values (e.g., negative mass)",
            ));
        }

        // Parse YAML with enhanced error
        serde_yaml::from_str(&content).map_err(|e| {
            let mut error = EnhancedError::from(e).with_file(&path_buf);
            error.suggestion = Some(format!(
                "Scene file syntax error. Check:\n  \
                     - YAML indentation (use spaces, not tabs)\n  \
                     - Array syntax: [x, y, z] or list with dashes\n  \
                     - Field names match schema\n\
                     \nExample valid scene:\n  \
                     name: \"MyScene\"\n  \
                     gravity: [0.0, -9.81, 0.0]\n  \
                     objects:\n  \
                       - name: \"box1\"\n  \
                         shape: {{type: box, size: [1.0, 1.0, 1.0]}}\n  \
                         position: [0.0, 0.5, 0.0]"
            ));
            error
        })
    }

    /// Load scene from JSON file
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();

        let content = std::fs::read_to_string(&path_buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                EnhancedError::file_not_found(&path_buf)
            } else {
                EnhancedError::new(format!("Failed to read scene file: {}", e)).with_file(&path_buf)
            }
        })?;

        serde_json::from_str(&content).map_err(|e| EnhancedError::from(e).with_file(&path_buf))
    }

    /// Load scene from YAML string
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        // Validate before parsing
        let validator = SceneValidator::new()
            .map_err(|e| EnhancedError::new(format!("Failed to create validator: {}", e)))?;

        let report = validator.validate_yaml(yaml).map_err(|e| {
            EnhancedError::new(format!("Validation error: {}", e))
                .with_category(ErrorCategory::ValidationError)
        })?;

        if !report.valid {
            let error_messages: Vec<String> =
                report.errors.iter().map(|e| format!("  - {}", e)).collect();

            return Err(EnhancedError::new(format!(
                "Scene validation failed:\n{}",
                error_messages.join("\n")
            ))
            .with_category(ErrorCategory::ValidationError)
            .with_hint("Check the scene YAML against the schema requirements"));
        }

        serde_yaml::from_str(yaml).map_err(EnhancedError::from)
    }

    /// Load scene from JSON string
    pub fn from_json_str(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(EnhancedError::from)
    }

    /// Convert to spawn configs
    pub fn to_spawn_configs(&self) -> Vec<ObjectSpawnConfig> {
        self.objects
            .iter()
            .map(|obj| {
                let shape = match &obj.shape {
                    SceneShape::Box { size } => SpawnShape::Box {
                        size: Vec3::from(*size),
                    },
                    SceneShape::Sphere { radius } => SpawnShape::Sphere { radius: *radius },
                    SceneShape::Cylinder { radius, height } => SpawnShape::Cylinder {
                        radius: *radius,
                        height: *height,
                    },
                    SceneShape::Capsule { radius, height } => SpawnShape::Capsule {
                        radius: *radius,
                        height: *height,
                    },
                    SceneShape::Ground { size_x, size_z } => SpawnShape::Ground {
                        size_x: *size_x,
                        size_z: *size_z,
                    },
                };

                let rotation = if let Some(euler) = obj.rotation_euler {
                    // Convert euler angles from degrees to radians
                    Quat::from_euler(
                        EulerRot::XYZ,
                        euler[0].to_radians(),
                        euler[1].to_radians(),
                        euler[2].to_radians(),
                    )
                } else {
                    // Use quaternion directly
                    Quat::from_xyzw(
                        obj.rotation[1],
                        obj.rotation[2],
                        obj.rotation[3],
                        obj.rotation[0],
                    )
                };

                let color = obj
                    .color
                    .map(|c| Color::srgb(c[0], c[1], c[2]))
                    .unwrap_or(Color::srgb(0.8, 0.8, 0.8));

                let mut config = ObjectSpawnConfig::new(&obj.name, shape)
                    .at_position(Vec3::from(obj.position))
                    .with_rotation(rotation)
                    .with_mass(obj.mass)
                    .with_friction(obj.friction)
                    .with_restitution(obj.restitution)
                    .with_color(color);

                if obj.is_static {
                    config = config.as_static();
                }

                if let Some((linear, angular)) = obj.damping {
                    config = config.with_damping(linear, angular);
                }

                config
            })
            .collect()
    }
}

impl SceneObject {
    /// Create a simple box object
    pub fn box_object(name: &str, position: [f32; 3], size: [f32; 3]) -> Self {
        Self {
            name: name.to_string(),
            shape: SceneShape::Box { size },
            position,
            rotation: [1.0, 0.0, 0.0, 0.0], // Identity quaternion
            rotation_euler: None,
            is_static: false,
            mass: 1.0,
            friction: 0.5,
            restitution: 0.0,
            color: None,
            damping: None,
        }
    }

    /// Create a simple sphere object
    pub fn sphere_object(name: &str, position: [f32; 3], radius: f32) -> Self {
        Self {
            name: name.to_string(),
            shape: SceneShape::Sphere { radius },
            position,
            rotation: [1.0, 0.0, 0.0, 0.0],
            rotation_euler: None,
            is_static: false,
            mass: 1.0,
            friction: 0.5,
            restitution: 0.0,
            color: None,
            damping: None,
        }
    }

    /// Create a ground plane
    pub fn ground_object(size_x: f32, size_z: f32) -> Self {
        Self {
            name: "ground".to_string(),
            shape: SceneShape::Ground { size_x, size_z },
            position: [0.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            rotation_euler: None,
            is_static: true,
            mass: 1.0,
            friction: 0.7,
            restitution: 0.0,
            color: Some([0.3, 0.5, 0.3]),
            damping: None,
        }
    }
}

#[derive(Resource)]
pub struct LoadedScene {
    pub definition: SceneDefinition,
    pub entities: Vec<Entity>,
}

impl LoadedScene {
    pub fn new(definition: SceneDefinition) -> Self {
        Self {
            definition,
            entities: Vec::new(),
        }
    }
}

pub struct SceneLoader;

impl SceneLoader {
    /// Load and spawn a scene from a file
    pub fn load_scene<P: AsRef<Path>>(
        path: P,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        spawned_objects: &mut SpawnedObjects,
        tf_tree: &mut TFTree,
    ) -> Result<LoadedScene> {
        // Determine file format from extension
        let path_ref = path.as_ref();
        let extension = path_ref
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                EnhancedError::new("Invalid file extension")
                    .with_file(path_ref)
                    .with_hint("Scene file must have a valid extension (.yaml, .yml, or .json)")
            })?;

        let definition = match extension {
            "yaml" | "yml" => SceneDefinition::from_yaml_file(path_ref)?,
            "json" => SceneDefinition::from_json_file(path_ref)?,
            _ => {
                return Err(
                    EnhancedError::new(format!("Unsupported file format: {}", extension))
                        .with_file(path_ref)
                        .with_hint("Supported formats: .yaml, .yml, .json")
                        .with_suggestion("Rename your scene file to use .yaml or .json extension"),
                )
            }
        };

        // Get the directory of the world file for resolving relative URDF paths
        let world_dir = path_ref.parent().unwrap_or(Path::new("."));

        Self::spawn_scene(
            definition,
            world_dir,
            commands,
            physics_world,
            meshes,
            materials,
            spawned_objects,
            tf_tree,
        )
    }

    /// Spawn a scene from a definition
    pub fn spawn_scene(
        definition: SceneDefinition,
        world_dir: &Path,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        spawned_objects: &mut SpawnedObjects,
        tf_tree: &mut TFTree,
    ) -> Result<LoadedScene> {
        let mut loaded_scene = LoadedScene::new(definition.clone());

        // Spawn objects
        let configs = definition.to_spawn_configs();
        for config in configs {
            let entity =
                ObjectSpawner::spawn_object(config, commands, physics_world, meshes, materials);
            loaded_scene.entities.push(entity);
            spawned_objects.add(entity);
        }

        // Spawn robots
        for robot_def in &definition.robots {
            // Resolve URDF path relative to world file directory
            let urdf_path = if Path::new(&robot_def.urdf_path).is_absolute() {
                robot_def.urdf_path.clone().into()
            } else {
                world_dir.join(&robot_def.urdf_path)
            };

            info!(
                "Loading robot '{}' from URDF: {}",
                robot_def.name,
                urdf_path.display()
            );

            let mut urdf_loader = URDFLoader::new().with_base_path(world_dir);
            match urdf_loader.load(
                &urdf_path,
                commands,
                physics_world,
                tf_tree,
                meshes,
                materials,
            ) {
                Ok(robot_entity) => {
                    // Apply position and rotation from scene definition
                    let position = Vec3::from(robot_def.position);
                    let rotation = if let Some(euler) = robot_def.rotation_euler {
                        Quat::from_euler(
                            EulerRot::XYZ,
                            euler[0].to_radians(),
                            euler[1].to_radians(),
                            euler[2].to_radians(),
                        )
                    } else {
                        Quat::from_array(robot_def.rotation)
                    };

                    commands
                        .entity(robot_entity)
                        .insert(Transform::from_translation(position).with_rotation(rotation));

                    loaded_scene.entities.push(robot_entity);
                    spawned_objects.add(robot_entity);
                    info!("Successfully spawned robot '{}'", robot_def.name);
                }
                Err(e) => {
                    warn!("Failed to load robot '{}': {}", robot_def.name, e);
                }
            }
        }

        info!(
            "Loaded scene '{}' with {} objects and {} robots",
            definition.name,
            loaded_scene.entities.len() - definition.robots.len(),
            definition.robots.len()
        );

        Ok(loaded_scene)
    }

    /// Clear the current scene
    pub fn clear_scene(commands: &mut Commands, spawned_objects: &mut SpawnedObjects) {
        for entity in spawned_objects.objects.drain(..) {
            commands.entity(entity).despawn_recursive();
        }
        info!("Scene cleared");
    }

    /// Reset scene (clear and reload)
    pub fn reset_scene(
        loaded_scene: &LoadedScene,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        spawned_objects: &mut SpawnedObjects,
        tf_tree: &mut TFTree,
    ) -> Result<LoadedScene> {
        Self::clear_scene(commands, spawned_objects);
        Self::spawn_scene(
            loaded_scene.definition.clone(),
            Path::new("."), // Use current directory for reset
            commands,
            physics_world,
            meshes,
            materials,
            spawned_objects,
            tf_tree,
        )
    }
}

/// Example scene builder for common scenarios
pub struct SceneBuilder;

impl SceneBuilder {
    /// Create an empty scene with just a ground plane
    pub fn empty(name: &str) -> SceneDefinition {
        SceneDefinition {
            name: name.to_string(),
            description: Some("Empty scene with ground plane".to_string()),
            gravity: Some(-9.81),
            objects: vec![SceneObject::ground_object(20.0, 20.0)],
            robots: vec![],
            lighting: Some(SceneLighting {
                ambient: [0.3, 0.3, 0.3],
                directional: Some(DirectionalLightConfig {
                    direction: [-0.5, -1.0, -0.5],
                    color: [1.0, 1.0, 1.0],
                    illuminance: 10000.0,
                }),
            }),
        }
    }

    /// Create a test scene with various objects
    pub fn test_objects() -> SceneDefinition {
        SceneDefinition {
            name: "test_objects".to_string(),
            description: Some("Test scene with various shapes".to_string()),
            gravity: Some(-9.81),
            objects: vec![
                SceneObject::ground_object(20.0, 20.0),
                SceneObject::box_object("box1", [0.0, 2.0, 0.0], [1.0, 1.0, 1.0]),
                SceneObject::sphere_object("sphere1", [2.0, 2.0, 0.0], 0.5),
                SceneObject {
                    name: "cylinder1".to_string(),
                    shape: SceneShape::Cylinder {
                        radius: 0.5,
                        height: 1.0,
                    },
                    position: [-2.0, 2.0, 0.0],
                    rotation: [1.0, 0.0, 0.0, 0.0],
                    rotation_euler: None,
                    is_static: false,
                    mass: 1.0,
                    friction: 0.5,
                    restitution: 0.3,
                    color: Some([0.8, 0.3, 0.3]),
                    damping: None,
                },
            ],
            robots: vec![],
            lighting: Some(SceneLighting {
                ambient: [0.3, 0.3, 0.3],
                directional: Some(DirectionalLightConfig {
                    direction: [-0.5, -1.0, -0.5],
                    color: [1.0, 1.0, 1.0],
                    illuminance: 10000.0,
                }),
            }),
        }
    }

    /// Create a box stacking scene
    pub fn box_stack() -> SceneDefinition {
        let mut objects = vec![SceneObject::ground_object(10.0, 10.0)];

        // Stack 5 boxes
        for i in 0..5 {
            let y = 0.5 + i as f32 * 1.0;
            objects.push(SceneObject::box_object(
                &format!("box_{}", i),
                [0.0, y, 0.0],
                [1.0, 1.0, 1.0],
            ));
        }

        SceneDefinition {
            name: "box_stack".to_string(),
            description: Some("Tower of stacked boxes".to_string()),
            gravity: Some(-9.81),
            objects,
            robots: vec![],
            lighting: Some(SceneLighting {
                ambient: [0.3, 0.3, 0.3],
                directional: Some(DirectionalLightConfig {
                    direction: [-0.5, -1.0, -0.5],
                    color: [1.0, 1.0, 1.0],
                    illuminance: 10000.0,
                }),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_serialization() {
        let scene = SceneBuilder::test_objects();

        // Test YAML serialization
        let yaml = serde_yaml::to_string(&scene).unwrap();
        let loaded = SceneDefinition::from_yaml_str(&yaml).unwrap();
        assert_eq!(loaded.name, "test_objects");
        assert_eq!(loaded.objects.len(), 4);

        // Test JSON serialization
        let json = serde_json::to_string_pretty(&scene).unwrap();
        let loaded = SceneDefinition::from_json_str(&json).unwrap();
        assert_eq!(loaded.name, "test_objects");
    }

    #[test]
    fn test_spawn_configs() {
        let scene = SceneBuilder::empty("test");
        let configs = scene.to_spawn_configs();
        assert_eq!(configs.len(), 1);
        assert!(configs[0].is_static);
    }
}
