//! YCB (Yale-CMU-Berkeley) Object Loader
//!
//! This module provides functionality to load and spawn YCB objects
//! from the YCB Object and Model Set, commonly used for manipulation research.
//!
//! # Example
//! ```ignore
//! use sim3d::assets::YCBLoader;
//!
//! let loader = YCBLoader::from_yaml("assets/objects/ycb_objects.yaml")?;
//! let can = loader.spawn_object("002_master_chef_can", Vec3::new(0.0, 0.0, 0.5), &mut commands, &mut physics_world, &mut meshes, &mut materials)?;
//! ```

use anyhow::{Context, Result};
use bevy::prelude::*;
use nalgebra::{Translation3, UnitQuaternion};
use rand::Rng;
use rapier3d::prelude::{Isometry, RigidBodyBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::physics::collider::{ColliderBuilder, ColliderShape};
use crate::physics::rigid_body::{Damping, Mass, RigidBodyComponent, Velocity};
use crate::physics::world::PhysicsWorld;

/// Configuration for a YCB mesh object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YCBMeshObjectConfig {
    /// Object name (e.g., "002_master_chef_can")
    pub name: String,
    /// Category (e.g., "food", "kitchen", "tools")
    pub category: String,
    /// Path to mesh file relative to assets directory
    pub mesh: String,
    /// Mass in kg
    pub mass: f32,
    /// Bounding box dimensions [x, y, z] in meters
    pub dimensions: [f32; 3],
    /// Friction coefficient
    pub friction: f32,
    /// Restitution (bounciness) coefficient
    pub restitution: f32,
}

/// Configuration for a primitive shape object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YCBPrimitiveConfig {
    /// Object name
    pub name: String,
    /// Shape type ("box", "sphere", "cylinder", "capsule")
    #[serde(rename = "type")]
    pub shape_type: String,
    /// Dimensions for box [x, y, z]
    #[serde(default)]
    pub dimensions: Option<[f32; 3]>,
    /// Radius for sphere, cylinder, capsule
    #[serde(default)]
    pub radius: Option<f32>,
    /// Length for cylinder, capsule
    #[serde(default)]
    pub length: Option<f32>,
    /// Mass in kg
    pub mass: f32,
    /// Friction coefficient
    pub friction: f32,
    /// Restitution coefficient
    pub restitution: f32,
    /// RGBA color [r, g, b, a]
    #[serde(default = "default_color")]
    pub color: [f32; 4],
}

fn default_color() -> [f32; 4] {
    [0.8, 0.8, 0.8, 1.0]
}

/// Usage information from the YAML
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YCBUsageInfo {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub example_rust: String,
    #[serde(default)]
    pub example_python: String,
}

/// Root configuration structure for YCB objects YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YCBObjectConfig {
    /// List of mesh-based YCB objects
    #[serde(default)]
    pub objects: Vec<YCBMeshObjectConfig>,
    /// List of primitive shape objects
    #[serde(default)]
    pub primitives: Vec<YCBPrimitiveConfig>,
    /// Usage information
    #[serde(default)]
    pub usage: YCBUsageInfo,
}

/// Component to mark and track spawned YCB objects
#[derive(Component, Debug, Clone)]
pub struct YCBObject {
    /// Name of the YCB object
    pub name: String,
    /// Category of the object
    pub category: String,
    /// Whether this is a primitive or mesh object
    pub is_primitive: bool,
    /// Mass in kg
    pub mass: f32,
    /// Dimensions/bounds in meters
    pub dimensions: Vec3,
}

impl YCBObject {
    /// Create a new YCBObject component from mesh config
    pub fn from_mesh_config(config: &YCBMeshObjectConfig) -> Self {
        Self {
            name: config.name.clone(),
            category: config.category.clone(),
            is_primitive: false,
            mass: config.mass,
            dimensions: Vec3::from_array(config.dimensions),
        }
    }

    /// Create a new YCBObject component from primitive config
    pub fn from_primitive_config(config: &YCBPrimitiveConfig) -> Self {
        let dimensions = match config.shape_type.as_str() {
            "box" => {
                let dims = config.dimensions.unwrap_or([0.1, 0.1, 0.1]);
                Vec3::from_array(dims)
            }
            "sphere" => {
                let r = config.radius.unwrap_or(0.05);
                Vec3::splat(r * 2.0)
            }
            "cylinder" | "capsule" => {
                let r = config.radius.unwrap_or(0.05);
                let l = config.length.unwrap_or(0.1);
                Vec3::new(r * 2.0, l, r * 2.0)
            }
            _ => Vec3::splat(0.1),
        };

        Self {
            name: config.name.clone(),
            category: "primitive".to_string(),
            is_primitive: true,
            mass: config.mass,
            dimensions,
        }
    }
}

/// Spawn options for YCB objects
#[derive(Debug, Clone)]
pub struct YCBSpawnOptions {
    /// Position in world space
    pub position: Vec3,
    /// Rotation as quaternion
    pub rotation: Quat,
    /// Whether the object is static (fixed) or dynamic
    pub is_static: bool,
    /// Linear damping
    pub linear_damping: f32,
    /// Angular damping
    pub angular_damping: f32,
    /// Scale factor applied to mesh and collider
    pub scale: f32,
    /// Override color (None uses default)
    pub color_override: Option<Color>,
}

impl Default for YCBSpawnOptions {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            is_static: false,
            linear_damping: 0.1,
            angular_damping: 0.1,
            scale: 1.0,
            color_override: None,
        }
    }
}

impl YCBSpawnOptions {
    /// Create spawn options with position
    pub fn at_position(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Set position
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    /// Set rotation from quaternion
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set rotation from Euler angles (radians)
    pub fn with_rotation_euler(mut self, x: f32, y: f32, z: f32) -> Self {
        self.rotation = Quat::from_euler(EulerRot::XYZ, x, y, z);
        self
    }

    /// Set rotation from axis-angle
    pub fn with_rotation_axis_angle(mut self, axis: Vec3, angle: f32) -> Self {
        self.rotation = Quat::from_axis_angle(axis, angle);
        self
    }

    /// Make the object static (fixed in space)
    pub fn as_static(mut self) -> Self {
        self.is_static = true;
        self
    }

    /// Make the object dynamic (affected by physics)
    pub fn as_dynamic(mut self) -> Self {
        self.is_static = false;
        self
    }

    /// Set damping values
    pub fn with_damping(mut self, linear: f32, angular: f32) -> Self {
        self.linear_damping = linear;
        self.angular_damping = angular;
        self
    }

    /// Set scale factor
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Override the default color
    pub fn with_color(mut self, color: Color) -> Self {
        self.color_override = Some(color);
        self
    }
}

/// Result of spawning a YCB object
#[derive(Debug, Clone)]
pub struct SpawnedYCBObject {
    /// The spawned entity
    pub entity: Entity,
    /// Name of the object
    pub name: String,
    /// Category of the object
    pub category: String,
    /// Position where it was spawned
    pub position: Vec3,
}

/// YCB Object Loader
///
/// Loads YCB object configurations from YAML and spawns them into Bevy ECS.
pub struct YCBLoader {
    /// Configuration loaded from YAML
    config: YCBObjectConfig,
    /// Base path for asset resolution
    assets_base_path: PathBuf,
    /// Index for fast object lookup by name
    object_index: HashMap<String, usize>,
    /// Index for fast primitive lookup by name
    primitive_index: HashMap<String, usize>,
    /// Category to object names mapping
    category_index: HashMap<String, Vec<String>>,
}

impl YCBLoader {
    /// Create a new YCBLoader from a YAML configuration file
    ///
    /// # Arguments
    /// * `yaml_path` - Path to the YCB objects YAML configuration file
    ///
    /// # Returns
    /// Result containing the loader or an error
    pub fn from_yaml<P: AsRef<Path>>(yaml_path: P) -> Result<Self> {
        let yaml_path = yaml_path.as_ref();
        let yaml_content = std::fs::read_to_string(yaml_path)
            .with_context(|| format!("Failed to read YCB config file: {}", yaml_path.display()))?;

        let config: YCBObjectConfig = serde_yaml::from_str(&yaml_content)
            .with_context(|| format!("Failed to parse YCB config YAML: {}", yaml_path.display()))?;

        // Determine assets base path (parent directory of yaml file, then up to assets root)
        let assets_base_path = yaml_path
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("assets"));

        Self::from_config(config, assets_base_path)
    }

    /// Create a new YCBLoader from an already-loaded configuration
    ///
    /// # Arguments
    /// * `config` - The YCB object configuration
    /// * `assets_base_path` - Base path for resolving asset paths
    pub fn from_config(config: YCBObjectConfig, assets_base_path: PathBuf) -> Result<Self> {
        let mut object_index = HashMap::new();
        let mut primitive_index = HashMap::new();
        let mut category_index: HashMap<String, Vec<String>> = HashMap::new();

        // Build object index
        for (i, obj) in config.objects.iter().enumerate() {
            object_index.insert(obj.name.clone(), i);
            category_index
                .entry(obj.category.clone())
                .or_default()
                .push(obj.name.clone());
        }

        // Build primitive index
        for (i, prim) in config.primitives.iter().enumerate() {
            primitive_index.insert(prim.name.clone(), i);
            category_index
                .entry("primitive".to_string())
                .or_default()
                .push(prim.name.clone());
        }

        Ok(Self {
            config,
            assets_base_path,
            object_index,
            primitive_index,
            category_index,
        })
    }

    /// Get an object configuration by name
    pub fn get_object(&self, name: &str) -> Option<&YCBMeshObjectConfig> {
        self.object_index
            .get(name)
            .map(|&i| &self.config.objects[i])
    }

    /// Get a primitive configuration by name
    pub fn get_primitive(&self, name: &str) -> Option<&YCBPrimitiveConfig> {
        self.primitive_index
            .get(name)
            .map(|&i| &self.config.primitives[i])
    }

    /// List all available object names
    pub fn list_objects(&self) -> Vec<&str> {
        self.config.objects.iter().map(|o| o.name.as_str()).collect()
    }

    /// List all available primitive names
    pub fn list_primitives(&self) -> Vec<&str> {
        self.config
            .primitives
            .iter()
            .map(|p| p.name.as_str())
            .collect()
    }

    /// List all object names in a specific category
    pub fn list_by_category(&self, category: &str) -> Vec<&str> {
        self.category_index
            .get(category)
            .map(|names| names.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// List all available categories
    pub fn list_categories(&self) -> Vec<&str> {
        self.category_index.keys().map(|s| s.as_str()).collect()
    }

    /// Get the total number of objects (mesh + primitives)
    pub fn object_count(&self) -> usize {
        self.config.objects.len() + self.config.primitives.len()
    }

    /// Spawn a YCB mesh object by name
    ///
    /// # Arguments
    /// * `name` - Name of the object to spawn
    /// * `options` - Spawn options (position, rotation, etc.)
    /// * `commands` - Bevy Commands
    /// * `physics_world` - Physics world resource
    /// * `meshes` - Bevy mesh assets
    /// * `materials` - Bevy material assets
    pub fn spawn_object(
        &self,
        name: &str,
        options: YCBSpawnOptions,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<SpawnedYCBObject> {
        let obj_config = self
            .get_object(name)
            .with_context(|| format!("YCB object not found: {}", name))?
            .clone();

        self.spawn_mesh_object(&obj_config, options, commands, physics_world, meshes, materials)
    }

    /// Spawn a primitive shape by name
    pub fn spawn_primitive(
        &self,
        name: &str,
        options: YCBSpawnOptions,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<SpawnedYCBObject> {
        let prim_config = self
            .get_primitive(name)
            .with_context(|| format!("YCB primitive not found: {}", name))?
            .clone();

        self.spawn_primitive_object(
            &prim_config,
            options,
            commands,
            physics_world,
            meshes,
            materials,
        )
    }

    /// Spawn an object by name (automatically detects if it's a mesh or primitive)
    pub fn spawn_by_name(
        &self,
        name: &str,
        options: YCBSpawnOptions,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<SpawnedYCBObject> {
        if self.object_index.contains_key(name) {
            self.spawn_object(name, options, commands, physics_world, meshes, materials)
        } else if self.primitive_index.contains_key(name) {
            self.spawn_primitive(name, options, commands, physics_world, meshes, materials)
        } else {
            anyhow::bail!(
                "Object '{}' not found. Available objects: {:?}",
                name,
                self.list_objects()
            )
        }
    }

    /// Spawn all objects in a category
    ///
    /// Objects are placed in a grid pattern starting from the given position.
    pub fn spawn_category(
        &self,
        category: &str,
        start_position: Vec3,
        spacing: f32,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<Vec<SpawnedYCBObject>> {
        let names = self.list_by_category(category);
        if names.is_empty() {
            anyhow::bail!(
                "Category '{}' not found or empty. Available categories: {:?}",
                category,
                self.list_categories()
            );
        }

        let grid_size = (names.len() as f32).sqrt().ceil() as usize;
        let mut spawned = Vec::with_capacity(names.len());

        for (i, name) in names.iter().enumerate() {
            let row = i / grid_size;
            let col = i % grid_size;
            let position = start_position
                + Vec3::new(col as f32 * spacing, 0.0, row as f32 * spacing);

            let options = YCBSpawnOptions::at_position(position);
            let result =
                self.spawn_by_name(name, options, commands, physics_world, meshes, materials)?;
            spawned.push(result);
        }

        Ok(spawned)
    }

    /// Spawn all objects
    pub fn spawn_all(
        &self,
        start_position: Vec3,
        spacing: f32,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<Vec<SpawnedYCBObject>> {
        let mut spawned = Vec::new();
        let total_count = self.object_count();
        let grid_size = (total_count as f32).sqrt().ceil() as usize;
        let mut index = 0;

        // Spawn mesh objects
        for obj in &self.config.objects {
            let row = index / grid_size;
            let col = index % grid_size;
            let position = start_position
                + Vec3::new(col as f32 * spacing, 0.0, row as f32 * spacing);

            let options = YCBSpawnOptions::at_position(position);
            let result = self.spawn_mesh_object(
                obj,
                options,
                commands,
                physics_world,
                meshes,
                materials,
            )?;
            spawned.push(result);
            index += 1;
        }

        // Spawn primitives
        for prim in &self.config.primitives {
            let row = index / grid_size;
            let col = index % grid_size;
            let position = start_position
                + Vec3::new(col as f32 * spacing, 0.0, row as f32 * spacing);

            let options = YCBSpawnOptions::at_position(position);
            let result = self.spawn_primitive_object(
                prim,
                options,
                commands,
                physics_world,
                meshes,
                materials,
            )?;
            spawned.push(result);
            index += 1;
        }

        Ok(spawned)
    }

    /// Spawn multiple random objects for a cluttered scene
    ///
    /// # Arguments
    /// * `count` - Number of objects to spawn
    /// * `bounds_min` - Minimum position bounds
    /// * `bounds_max` - Maximum position bounds
    /// * `categories` - Optional list of categories to sample from (None = all)
    pub fn spawn_cluttered_scene(
        &self,
        count: usize,
        bounds_min: Vec3,
        bounds_max: Vec3,
        categories: Option<&[&str]>,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<Vec<SpawnedYCBObject>> {
        let mut rng = rand::thread_rng();

        // Collect eligible object names
        let eligible_names: Vec<String> = match categories {
            Some(cats) => cats
                .iter()
                .flat_map(|c| self.list_by_category(c))
                .map(|s| s.to_string())
                .collect(),
            None => self
                .list_objects()
                .iter()
                .chain(self.list_primitives().iter())
                .map(|s| s.to_string())
                .collect(),
        };

        if eligible_names.is_empty() {
            anyhow::bail!("No objects available for spawning");
        }

        let mut spawned = Vec::with_capacity(count);

        for _ in 0..count {
            // Random object selection
            let name_idx = rng.gen_range(0..eligible_names.len());
            let name = &eligible_names[name_idx];

            // Random position within bounds
            let position = Vec3::new(
                rng.gen_range(bounds_min.x..bounds_max.x),
                rng.gen_range(bounds_min.y..bounds_max.y),
                rng.gen_range(bounds_min.z..bounds_max.z),
            );

            // Random rotation around Y axis
            let rotation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));

            let options = YCBSpawnOptions::default()
                .with_position(position)
                .with_rotation(rotation);

            let result =
                self.spawn_by_name(name, options, commands, physics_world, meshes, materials)?;
            spawned.push(result);
        }

        Ok(spawned)
    }

    /// Spawn a YCB mesh object with configuration
    fn spawn_mesh_object(
        &self,
        config: &YCBMeshObjectConfig,
        options: YCBSpawnOptions,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<SpawnedYCBObject> {
        let scaled_dims = Vec3::from_array(config.dimensions) * options.scale;
        let half_extents = scaled_dims / 2.0;

        // Create physics position
        let translation =
            Translation3::new(options.position.x, options.position.y, options.position.z);
        let rotation = UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
            options.rotation.w,
            options.rotation.x,
            options.rotation.y,
            options.rotation.z,
        ));
        let physics_position = Isometry::from_parts(translation.into(), rotation);

        // Create rigid body
        let rigid_body = if options.is_static {
            RigidBodyBuilder::fixed()
                .position(physics_position)
                .build()
        } else {
            RigidBodyBuilder::dynamic()
                .position(physics_position)
                .additional_mass(config.mass * options.scale.powi(3))
                .linear_damping(options.linear_damping)
                .angular_damping(options.angular_damping)
                .build()
        };

        // Create collider (use box approximation for YCB objects)
        let collider = ColliderBuilder::new(ColliderShape::Box { half_extents })
            .friction(config.friction)
            .restitution(config.restitution)
            .build();

        // Create visual mesh (box approximation - actual mesh loading would use MeshLoader)
        let visual_mesh = meshes.add(Cuboid::new(scaled_dims.x, scaled_dims.y, scaled_dims.z));

        // Create material with distinct color based on category
        let color = options.color_override.unwrap_or_else(|| {
            match config.category.as_str() {
                "food" => Color::srgb(0.9, 0.6, 0.3),
                "kitchen" => Color::srgb(0.7, 0.7, 0.8),
                "tools" => Color::srgb(0.5, 0.5, 0.6),
                "shapes" => Color::srgb(0.8, 0.7, 0.5),
                "office" => Color::srgb(0.4, 0.4, 0.5),
                _ => Color::srgb(0.8, 0.8, 0.8),
            }
        });

        let material = materials.add(StandardMaterial {
            base_color: color,
            metallic: 0.1,
            perceptual_roughness: 0.6,
            ..default()
        });

        // Spawn entity
        let entity = commands
            .spawn((
                Name::new(config.name.clone()),
                Transform::from_translation(options.position).with_rotation(options.rotation),
                Visibility::default(),
            ))
            .id();

        // Add to physics world
        let rb_handle = physics_world.spawn_rigid_body(rigid_body, entity);
        physics_world.spawn_collider(collider, rb_handle);

        // Add components
        commands.entity(entity).insert((
            RigidBodyComponent::new(rb_handle),
            Velocity::zero(),
            Mass::new(config.mass * options.scale.powi(3)),
            Damping::new(options.linear_damping, options.angular_damping),
            YCBObject::from_mesh_config(config),
            Mesh3d(visual_mesh),
            MeshMaterial3d(material),
        ));

        Ok(SpawnedYCBObject {
            entity,
            name: config.name.clone(),
            category: config.category.clone(),
            position: options.position,
        })
    }

    /// Spawn a primitive shape with configuration
    fn spawn_primitive_object(
        &self,
        config: &YCBPrimitiveConfig,
        options: YCBSpawnOptions,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<SpawnedYCBObject> {
        // Create physics position
        let translation =
            Translation3::new(options.position.x, options.position.y, options.position.z);
        let rotation = UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
            options.rotation.w,
            options.rotation.x,
            options.rotation.y,
            options.rotation.z,
        ));
        let physics_position = Isometry::from_parts(translation.into(), rotation);

        // Create rigid body
        let scaled_mass = config.mass * options.scale.powi(3);
        let rigid_body = if options.is_static {
            RigidBodyBuilder::fixed()
                .position(physics_position)
                .build()
        } else {
            RigidBodyBuilder::dynamic()
                .position(physics_position)
                .additional_mass(scaled_mass)
                .linear_damping(options.linear_damping)
                .angular_damping(options.angular_damping)
                .build()
        };

        // Create collider and mesh based on shape type
        let (collider, visual_mesh) = match config.shape_type.as_str() {
            "box" => {
                let dims = config.dimensions.unwrap_or([0.1, 0.1, 0.1]);
                let scaled_dims = Vec3::from_array(dims) * options.scale;
                let half_extents = scaled_dims / 2.0;

                let collider = ColliderBuilder::new(ColliderShape::Box { half_extents })
                    .friction(config.friction)
                    .restitution(config.restitution)
                    .build();

                let mesh = meshes.add(Cuboid::new(scaled_dims.x, scaled_dims.y, scaled_dims.z));
                (collider, mesh)
            }
            "sphere" => {
                let radius = config.radius.unwrap_or(0.05) * options.scale;

                let collider = ColliderBuilder::new(ColliderShape::Sphere { radius })
                    .friction(config.friction)
                    .restitution(config.restitution)
                    .build();

                let mesh = meshes.add(Sphere { radius });
                (collider, mesh)
            }
            "cylinder" => {
                let radius = config.radius.unwrap_or(0.05) * options.scale;
                let length = config.length.unwrap_or(0.1) * options.scale;
                let half_height = length / 2.0;

                let collider = ColliderBuilder::new(ColliderShape::Cylinder { half_height, radius })
                    .friction(config.friction)
                    .restitution(config.restitution)
                    .build();

                let mesh = meshes.add(Cylinder {
                    radius,
                    half_height,
                });
                (collider, mesh)
            }
            "capsule" => {
                let radius = config.radius.unwrap_or(0.05) * options.scale;
                let length = config.length.unwrap_or(0.1) * options.scale;
                let half_height = length / 2.0;

                let collider = ColliderBuilder::new(ColliderShape::Capsule { half_height, radius })
                    .friction(config.friction)
                    .restitution(config.restitution)
                    .build();

                let mesh = meshes.add(Capsule3d {
                    radius,
                    half_length: half_height,
                });
                (collider, mesh)
            }
            unknown => {
                anyhow::bail!("Unknown primitive shape type: {}", unknown);
            }
        };

        // Create material
        let color = options.color_override.unwrap_or_else(|| {
            Color::srgba(config.color[0], config.color[1], config.color[2], config.color[3])
        });

        let material = materials.add(StandardMaterial {
            base_color: color,
            metallic: 0.0,
            perceptual_roughness: 0.5,
            ..default()
        });

        // Spawn entity
        let entity = commands
            .spawn((
                Name::new(config.name.clone()),
                Transform::from_translation(options.position).with_rotation(options.rotation),
                Visibility::default(),
            ))
            .id();

        // Add to physics world
        let rb_handle = physics_world.spawn_rigid_body(rigid_body, entity);
        physics_world.spawn_collider(collider, rb_handle);

        // Add components
        commands.entity(entity).insert((
            RigidBodyComponent::new(rb_handle),
            Velocity::zero(),
            Mass::new(scaled_mass),
            Damping::new(options.linear_damping, options.angular_damping),
            YCBObject::from_primitive_config(config),
            Mesh3d(visual_mesh),
            MeshMaterial3d(material),
        ));

        Ok(SpawnedYCBObject {
            entity,
            name: config.name.clone(),
            category: "primitive".to_string(),
            position: options.position,
        })
    }

    /// Get the assets base path
    pub fn assets_base_path(&self) -> &Path {
        &self.assets_base_path
    }

    /// Resolve a mesh path relative to the assets base
    pub fn resolve_mesh_path(&self, mesh_path: &str) -> PathBuf {
        self.assets_base_path.join(mesh_path)
    }

    /// Get the raw configuration
    pub fn config(&self) -> &YCBObjectConfig {
        &self.config
    }
}

/// Helper function to spawn a single YCB object at a position
pub fn spawn_ycb_object_at(
    loader: &YCBLoader,
    name: &str,
    position: Vec3,
    commands: &mut Commands,
    physics_world: &mut PhysicsWorld,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Result<Entity> {
    let options = YCBSpawnOptions::at_position(position);
    let result = loader.spawn_by_name(name, options, commands, physics_world, meshes, materials)?;
    Ok(result.entity)
}

/// Helper function to spawn YCB object with full transform
pub fn spawn_ycb_object_with_transform(
    loader: &YCBLoader,
    name: &str,
    position: Vec3,
    rotation: Quat,
    scale: f32,
    commands: &mut Commands,
    physics_world: &mut PhysicsWorld,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Result<Entity> {
    let options = YCBSpawnOptions::default()
        .with_position(position)
        .with_rotation(rotation)
        .with_scale(scale);
    let result = loader.spawn_by_name(name, options, commands, physics_world, meshes, materials)?;
    Ok(result.entity)
}

/// Helper to create a random cluttered scene with YCB objects
pub fn create_ycb_clutter(
    loader: &YCBLoader,
    count: usize,
    table_height: f32,
    table_size: Vec3,
    commands: &mut Commands,
    physics_world: &mut PhysicsWorld,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Result<Vec<Entity>> {
    let bounds_min = Vec3::new(-table_size.x / 2.0, table_height + 0.05, -table_size.z / 2.0);
    let bounds_max = Vec3::new(table_size.x / 2.0, table_height + 0.3, table_size.z / 2.0);

    let spawned = loader.spawn_cluttered_scene(
        count,
        bounds_min,
        bounds_max,
        None,
        commands,
        physics_world,
        meshes,
        materials,
    )?;

    Ok(spawned.into_iter().map(|s| s.entity).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> YCBObjectConfig {
        YCBObjectConfig {
            objects: vec![
                YCBMeshObjectConfig {
                    name: "002_master_chef_can".to_string(),
                    category: "food".to_string(),
                    mesh: "meshes/ycb/002_master_chef_can/textured.obj".to_string(),
                    mass: 0.414,
                    dimensions: [0.102, 0.102, 0.1398],
                    friction: 0.5,
                    restitution: 0.1,
                },
                YCBMeshObjectConfig {
                    name: "003_cracker_box".to_string(),
                    category: "food".to_string(),
                    mesh: "meshes/ycb/003_cracker_box/textured.obj".to_string(),
                    mass: 0.411,
                    dimensions: [0.16, 0.21, 0.07],
                    friction: 0.6,
                    restitution: 0.1,
                },
                YCBMeshObjectConfig {
                    name: "025_mug".to_string(),
                    category: "kitchen".to_string(),
                    mesh: "meshes/ycb/025_mug/textured.obj".to_string(),
                    mass: 0.118,
                    dimensions: [0.093, 0.093, 0.095],
                    friction: 0.4,
                    restitution: 0.3,
                },
            ],
            primitives: vec![
                YCBPrimitiveConfig {
                    name: "cube_5cm".to_string(),
                    shape_type: "box".to_string(),
                    dimensions: Some([0.05, 0.05, 0.05]),
                    radius: None,
                    length: None,
                    mass: 0.125,
                    friction: 0.5,
                    restitution: 0.3,
                    color: [0.8, 0.2, 0.2, 1.0],
                },
                YCBPrimitiveConfig {
                    name: "sphere_5cm".to_string(),
                    shape_type: "sphere".to_string(),
                    dimensions: None,
                    radius: Some(0.025),
                    length: None,
                    mass: 0.065,
                    friction: 0.3,
                    restitution: 0.6,
                    color: [0.2, 0.8, 0.2, 1.0],
                },
            ],
            usage: YCBUsageInfo::default(),
        }
    }

    #[test]
    fn test_loader_creation() {
        let config = create_test_config();
        let loader = YCBLoader::from_config(config, PathBuf::from("assets")).unwrap();

        assert_eq!(loader.object_count(), 5);
        assert_eq!(loader.list_objects().len(), 3);
        assert_eq!(loader.list_primitives().len(), 2);
    }

    #[test]
    fn test_get_object() {
        let config = create_test_config();
        let loader = YCBLoader::from_config(config, PathBuf::from("assets")).unwrap();

        let obj = loader.get_object("002_master_chef_can");
        assert!(obj.is_some());
        let obj = obj.unwrap();
        assert_eq!(obj.name, "002_master_chef_can");
        assert_eq!(obj.category, "food");
        assert!((obj.mass - 0.414).abs() < 0.001);

        assert!(loader.get_object("nonexistent").is_none());
    }

    #[test]
    fn test_get_primitive() {
        let config = create_test_config();
        let loader = YCBLoader::from_config(config, PathBuf::from("assets")).unwrap();

        let prim = loader.get_primitive("cube_5cm");
        assert!(prim.is_some());
        let prim = prim.unwrap();
        assert_eq!(prim.name, "cube_5cm");
        assert_eq!(prim.shape_type, "box");

        assert!(loader.get_primitive("nonexistent").is_none());
    }

    #[test]
    fn test_list_by_category() {
        let config = create_test_config();
        let loader = YCBLoader::from_config(config, PathBuf::from("assets")).unwrap();

        let food = loader.list_by_category("food");
        assert_eq!(food.len(), 2);
        assert!(food.contains(&"002_master_chef_can"));
        assert!(food.contains(&"003_cracker_box"));

        let kitchen = loader.list_by_category("kitchen");
        assert_eq!(kitchen.len(), 1);
        assert!(kitchen.contains(&"025_mug"));

        let primitives = loader.list_by_category("primitive");
        assert_eq!(primitives.len(), 2);

        let empty = loader.list_by_category("nonexistent");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_list_categories() {
        let config = create_test_config();
        let loader = YCBLoader::from_config(config, PathBuf::from("assets")).unwrap();

        let categories = loader.list_categories();
        assert!(categories.contains(&"food"));
        assert!(categories.contains(&"kitchen"));
        assert!(categories.contains(&"primitive"));
    }

    #[test]
    fn test_spawn_options_builder() {
        let options = YCBSpawnOptions::default()
            .with_position(Vec3::new(1.0, 2.0, 3.0))
            .with_rotation_euler(0.0, std::f32::consts::FRAC_PI_2, 0.0)
            .with_damping(0.5, 0.3)
            .with_scale(2.0)
            .as_static();

        assert_eq!(options.position, Vec3::new(1.0, 2.0, 3.0));
        assert!(options.is_static);
        assert!((options.linear_damping - 0.5).abs() < 0.001);
        assert!((options.angular_damping - 0.3).abs() < 0.001);
        assert!((options.scale - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_spawn_options_at_position() {
        let options = YCBSpawnOptions::at_position(Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(options.position, Vec3::new(5.0, 6.0, 7.0));
        assert!(!options.is_static);
    }

    #[test]
    fn test_ycb_object_from_mesh_config() {
        let config = YCBMeshObjectConfig {
            name: "test_object".to_string(),
            category: "test".to_string(),
            mesh: "mesh.obj".to_string(),
            mass: 1.0,
            dimensions: [0.1, 0.2, 0.3],
            friction: 0.5,
            restitution: 0.1,
        };

        let ycb_obj = YCBObject::from_mesh_config(&config);
        assert_eq!(ycb_obj.name, "test_object");
        assert_eq!(ycb_obj.category, "test");
        assert!(!ycb_obj.is_primitive);
        assert!((ycb_obj.mass - 1.0).abs() < 0.001);
        assert_eq!(ycb_obj.dimensions, Vec3::new(0.1, 0.2, 0.3));
    }

    #[test]
    fn test_ycb_object_from_primitive_config() {
        let config = YCBPrimitiveConfig {
            name: "test_sphere".to_string(),
            shape_type: "sphere".to_string(),
            dimensions: None,
            radius: Some(0.05),
            length: None,
            mass: 0.5,
            friction: 0.3,
            restitution: 0.6,
            color: [1.0, 0.0, 0.0, 1.0],
        };

        let ycb_obj = YCBObject::from_primitive_config(&config);
        assert_eq!(ycb_obj.name, "test_sphere");
        assert_eq!(ycb_obj.category, "primitive");
        assert!(ycb_obj.is_primitive);
        assert!((ycb_obj.mass - 0.5).abs() < 0.001);
        // Sphere with radius 0.05 should have dimensions 0.1 x 0.1 x 0.1
        assert!((ycb_obj.dimensions.x - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_resolve_mesh_path() {
        let config = create_test_config();
        let loader = YCBLoader::from_config(config, PathBuf::from("/home/user/assets")).unwrap();

        let resolved = loader.resolve_mesh_path("meshes/test.obj");
        assert_eq!(resolved, PathBuf::from("/home/user/assets/meshes/test.obj"));
    }

    #[test]
    fn test_yaml_deserialization() {
        let yaml = r#"
objects:
  - name: "test_can"
    category: "food"
    mesh: "meshes/test.obj"
    mass: 0.5
    dimensions: [0.1, 0.1, 0.15]
    friction: 0.5
    restitution: 0.1

primitives:
  - name: "test_box"
    type: "box"
    dimensions: [0.05, 0.05, 0.05]
    mass: 0.125
    friction: 0.5
    restitution: 0.3
    color: [0.8, 0.2, 0.2, 1.0]

usage:
  description: "Test configuration"
"#;

        let config: YCBObjectConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.objects.len(), 1);
        assert_eq!(config.objects[0].name, "test_can");
        assert_eq!(config.primitives.len(), 1);
        assert_eq!(config.primitives[0].name, "test_box");
    }

    #[test]
    fn test_default_color() {
        let color = default_color();
        assert_eq!(color, [0.8, 0.8, 0.8, 1.0]);
    }

    #[test]
    fn test_spawned_ycb_object_struct() {
        let spawned = SpawnedYCBObject {
            entity: Entity::from_raw(42),
            name: "test".to_string(),
            category: "food".to_string(),
            position: Vec3::new(1.0, 2.0, 3.0),
        };

        assert_eq!(spawned.name, "test");
        assert_eq!(spawned.category, "food");
        assert_eq!(spawned.position, Vec3::new(1.0, 2.0, 3.0));
    }
}
