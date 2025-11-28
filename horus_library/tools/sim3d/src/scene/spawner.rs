use bevy::prelude::*;
use rapier3d::prelude::*;

use crate::physics::collider::{ColliderBuilder, ColliderShape};
use crate::physics::rigid_body::{Damping, Mass, RigidBodyComponent, Velocity};
use crate::physics::world::PhysicsWorld;

#[derive(Clone)]
pub struct ObjectSpawnConfig {
    pub name: String,
    pub shape: SpawnShape,
    pub position: Vec3,
    pub rotation: Quat,
    pub is_static: bool,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub color: Color,
    pub damping: Option<(f32, f32)>,
}

impl Default for ObjectSpawnConfig {
    fn default() -> Self {
        Self {
            name: "object".to_string(),
            shape: SpawnShape::Box { size: Vec3::ONE },
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            is_static: false,
            mass: 1.0,
            friction: 0.5,
            restitution: 0.0,
            color: Color::srgb(0.8, 0.8, 0.8),
            damping: None,
        }
    }
}

impl ObjectSpawnConfig {
    pub fn new(name: impl Into<String>, shape: SpawnShape) -> Self {
        Self {
            name: name.into(),
            shape,
            ..default()
        }
    }

    pub fn at_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_rotation_euler(mut self, x: f32, y: f32, z: f32) -> Self {
        self.rotation = Quat::from_euler(EulerRot::XYZ, x, y, z);
        self
    }

    pub fn as_static(mut self) -> Self {
        self.is_static = true;
        self
    }

    pub fn as_dynamic(mut self) -> Self {
        self.is_static = false;
        self
    }

    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_damping(mut self, linear: f32, angular: f32) -> Self {
        self.damping = Some((linear, angular));
        self
    }
}

#[derive(Clone)]
pub enum SpawnShape {
    Box { size: Vec3 },
    Sphere { radius: f32 },
    Cylinder { radius: f32, height: f32 },
    Capsule { radius: f32, height: f32 },
    Ground { size_x: f32, size_z: f32 },
}

pub struct ObjectSpawner;

impl ObjectSpawner {
    pub fn spawn_object(
        config: ObjectSpawnConfig,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Entity {
        // Create rigid body
        // Build with position and rotation
        use nalgebra::{Translation3, UnitQuaternion};
        use rapier3d::prelude::Isometry;

        let translation =
            Translation3::new(config.position.x, config.position.y, config.position.z);
        let rotation = UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
            config.rotation.w,
            config.rotation.x,
            config.rotation.y,
            config.rotation.z,
        ));
        let position = Isometry::from_parts(translation, rotation);

        let rigid_body = if config.is_static {
            RigidBodyBuilder::fixed().position(position).build()
        } else {
            RigidBodyBuilder::dynamic()
                .position(position)
                .additional_mass(config.mass)
                .build()
        };

        // Create collider based on shape
        let collider = match &config.shape {
            SpawnShape::Box { size } => {
                let half_extents = *size / 2.0;
                ColliderBuilder::new(ColliderShape::Box { half_extents })
                    .friction(config.friction)
                    .restitution(config.restitution)
                    .build()
            }
            SpawnShape::Sphere { radius } => {
                ColliderBuilder::new(ColliderShape::Sphere { radius: *radius })
                    .friction(config.friction)
                    .restitution(config.restitution)
                    .build()
            }
            SpawnShape::Cylinder { radius, height } => {
                ColliderBuilder::new(ColliderShape::Cylinder {
                    half_height: height / 2.0,
                    radius: *radius,
                })
                .friction(config.friction)
                .restitution(config.restitution)
                .build()
            }
            SpawnShape::Capsule { radius, height } => {
                ColliderBuilder::new(ColliderShape::Capsule {
                    half_height: height / 2.0,
                    radius: *radius,
                })
                .friction(config.friction)
                .restitution(config.restitution)
                .build()
            }
            SpawnShape::Ground { size_x, size_z } => {
                let half_extents = Vec3::new(size_x / 2.0, 0.1, size_z / 2.0);
                ColliderBuilder::new(ColliderShape::Box { half_extents })
                    .friction(config.friction)
                    .restitution(config.restitution)
                    .build()
            }
        };

        // Create visual mesh
        let mesh = match &config.shape {
            SpawnShape::Box { size } => {
                meshes.add(bevy::prelude::Cuboid::new(size.x, size.y, size.z))
            }
            SpawnShape::Sphere { radius } => meshes.add(bevy::prelude::Sphere { radius: *radius }),
            SpawnShape::Cylinder { radius, height } => meshes.add(bevy::prelude::Cylinder {
                radius: *radius,
                half_height: height / 2.0,
            }),
            SpawnShape::Capsule { radius, height } => meshes.add(Capsule3d {
                radius: *radius,
                half_length: height / 2.0,
            }),
            SpawnShape::Ground { size_x, size_z } => {
                meshes.add(bevy::prelude::Cuboid::new(*size_x, 0.2, *size_z))
            }
        };

        let material = materials.add(StandardMaterial {
            base_color: config.color,
            ..default()
        });

        // First, create the physics objects with a placeholder entity
        // We need the rb_handle before spawning the Bevy entity
        let placeholder = Entity::PLACEHOLDER;
        let rb_handle = physics_world.spawn_rigid_body(rigid_body, placeholder);
        physics_world.spawn_collider(collider, rb_handle);

        // Spawn Bevy entity with ALL components in a SINGLE spawn call
        // This is critical - Bevy 0.15's rendering pipeline may not properly initialize
        // entities when components are added via deferred insert after spawn
        let entity = if let Some((linear, angular)) = config.damping {
            commands
                .spawn((
                    Name::new(config.name.clone()),
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(config.position).with_rotation(config.rotation),
                    RigidBodyComponent::new(rb_handle),
                    Velocity::zero(),
                    Mass::new(config.mass),
                    Damping::new(linear, angular),
                ))
                .id()
        } else {
            commands
                .spawn((
                    Name::new(config.name.clone()),
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(config.position).with_rotation(config.rotation),
                    RigidBodyComponent::new(rb_handle),
                    Velocity::zero(),
                    Mass::new(config.mass),
                ))
                .id()
        };

        // Update the physics world with the actual entity
        physics_world.update_rigid_body_entity(rb_handle, entity);

        entity
    }

    pub fn spawn_box(
        name: impl Into<String>,
        position: Vec3,
        size: Vec3,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Entity {
        let config = ObjectSpawnConfig::new(name, SpawnShape::Box { size }).at_position(position);
        Self::spawn_object(config, commands, physics_world, meshes, materials)
    }

    pub fn spawn_sphere(
        name: impl Into<String>,
        position: Vec3,
        radius: f32,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Entity {
        let config =
            ObjectSpawnConfig::new(name, SpawnShape::Sphere { radius }).at_position(position);
        Self::spawn_object(config, commands, physics_world, meshes, materials)
    }

    pub fn spawn_cylinder(
        name: impl Into<String>,
        position: Vec3,
        radius: f32,
        height: f32,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Entity {
        let config = ObjectSpawnConfig::new(name, SpawnShape::Cylinder { radius, height })
            .at_position(position);
        Self::spawn_object(config, commands, physics_world, meshes, materials)
    }

    pub fn spawn_ground(
        size_x: f32,
        size_z: f32,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Entity {
        let config = ObjectSpawnConfig::new("ground", SpawnShape::Ground { size_x, size_z })
            .as_static()
            .with_color(Color::srgb(0.3, 0.5, 0.3));
        Self::spawn_object(config, commands, physics_world, meshes, materials)
    }

    pub fn spawn_wall(
        name: impl Into<String>,
        position: Vec3,
        width: f32,
        height: f32,
        thickness: f32,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Entity {
        let config = ObjectSpawnConfig::new(
            name,
            SpawnShape::Box {
                size: Vec3::new(width, height, thickness),
            },
        )
        .at_position(position)
        .as_static()
        .with_color(Color::srgb(0.7, 0.7, 0.7));
        Self::spawn_object(config, commands, physics_world, meshes, materials)
    }
}

#[derive(Resource, Default)]
pub struct SpawnedObjects {
    pub objects: Vec<Entity>,
}

impl SpawnedObjects {
    pub fn add(&mut self, entity: Entity) {
        self.objects.push(entity);
    }

    pub fn clear(&mut self) {
        self.objects.clear();
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

pub fn despawn_all_objects_system(
    mut commands: Commands,
    mut spawned_objects: ResMut<SpawnedObjects>,
) {
    for entity in spawned_objects.objects.drain(..) {
        commands.entity(entity).despawn_recursive();
    }
}
