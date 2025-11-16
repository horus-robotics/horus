use bevy::prelude::*;
use bevy::render::mesh::Mesh;
use rapier3d::prelude::*;
use urdf_rs::{Geometry, Joint, Link, Robot as URDFRobot, Visual};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

use crate::physics::world::PhysicsWorld;
use crate::physics::collider::{ColliderBuilder, ColliderShape};
use crate::physics::joints::{create_revolute_joint, create_revolute_joint_with_limits,
                              create_prismatic_joint, create_prismatic_joint_with_limits,
                              create_fixed_joint, create_spherical_joint, PhysicsJoint, JointType};
use crate::robot::robot::Robot;
use crate::tf::tree::TFTree;

pub struct URDFLoader {
    base_path: PathBuf,
}

impl URDFLoader {
    pub fn new() -> Self {
        Self {
            base_path: PathBuf::from("."),
        }
    }

    pub fn with_base_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.base_path = path.into();
        self
    }

    pub fn load(
        &self,
        urdf_path: impl AsRef<Path>,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        tf_tree: &mut TFTree,
        asset_server: &AssetServer,
    ) -> Result<Entity> {
        let urdf_path = urdf_path.as_ref();
        let urdf_str = std::fs::read_to_string(urdf_path)
            .with_context(|| format!("Failed to read URDF file: {}", urdf_path.display()))?;

        let urdf = urdf_rs::read_from_string(&urdf_str)
            .with_context(|| format!("Failed to parse URDF file: {}", urdf_path.display()))?;

        self.spawn_robot(urdf, commands, physics_world, tf_tree, asset_server)
    }

    pub fn spawn_robot(
        &self,
        urdf: URDFRobot,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        tf_tree: &mut TFTree,
        asset_server: &AssetServer,
    ) -> Result<Entity> {
        let robot_name = urdf.name.clone();

        // Create root entity for the robot
        let root_entity = commands.spawn((
            Robot::new(&robot_name),
            Transform::default(),
            Visibility::default(),
        )).id();

        // Map link names to entity IDs
        let mut link_entities: HashMap<String, Entity> = HashMap::new();
        let mut link_rb_handles: HashMap<String, RigidBodyHandle> = HashMap::new();

        // Spawn all links
        for link in &urdf.links {
            let link_entity = self.spawn_link(
                link,
                &robot_name,
                commands,
                physics_world,
                asset_server,
            )?;

            link_entities.insert(link.name.clone(), link_entity);

            // Get the rigid body handle if it exists
            if let Some(rb_handle) = self.get_rb_handle_for_entity(link_entity, physics_world) {
                link_rb_handles.insert(link.name.clone(), rb_handle);
            }
        }

        // Spawn all joints
        for joint in &urdf.joints {
            self.spawn_joint(
                joint,
                &link_entities,
                &link_rb_handles,
                commands,
                physics_world,
            )?;
        }

        // Build TF tree from URDF
        *tf_tree = TFTree::from_urdf(&urdf);

        // Parent all link entities to root
        for entity in link_entities.values() {
            commands.entity(root_entity).add_child(*entity);
        }

        Ok(root_entity)
    }

    fn spawn_link(
        &self,
        link: &Link,
        robot_name: &str,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        asset_server: &AssetServer,
    ) -> Result<Entity> {
        // Create rigid body based on inertial properties
        let rigid_body = if !link.inertial.mass.value.is_nan() && link.inertial.mass.value > 0.0 {
            let mass = link.inertial.mass.value as f32;
            let origin = &link.inertial.origin;

            let translation = vector![
                origin.xyz[0] as f32,
                origin.xyz[1] as f32,
                origin.xyz[2] as f32
            ];

            let rotation = nalgebra::UnitQuaternion::from_euler_angles(
                origin.rpy[0] as f32,
                origin.rpy[1] as f32,
                origin.rpy[2] as f32,
            );

            let com_isometry = Isometry::from_parts(translation.into(), rotation);

            RigidBodyBuilder::dynamic()
                .position(com_isometry)
                .additional_mass(mass)
                .build()
        } else {
            // Static body for links without inertia
            RigidBodyBuilder::fixed()
                .build()
        };

        // Create entity and spawn rigid body
        let entity = commands.spawn((
            Name::new(format!("{}::{}", robot_name, link.name)),
            Transform::default(),
            Visibility::default(),
        )).id();

        let rb_handle = physics_world.spawn_rigid_body(rigid_body, entity);

        // Add colliders from collision geometry
        for collision in &link.collision {
            if let Some(collider) = self.create_collider_from_geometry(&collision.geometry)? {
                physics_world.spawn_collider(collider, rb_handle);
            }
        }

        // Add visual meshes
        for visual in &link.visual {
            self.spawn_visual_mesh(visual, entity, commands, asset_server)?;
        }

        Ok(entity)
    }

    fn spawn_joint(
        &self,
        joint: &Joint,
        link_entities: &HashMap<String, Entity>,
        link_rb_handles: &HashMap<String, RigidBodyHandle>,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
    ) -> Result<()> {
        let parent_entity = link_entities.get(&joint.parent.link)
            .context(format!("Parent link '{}' not found", joint.parent.link))?;

        let child_entity = link_entities.get(&joint.child.link)
            .context(format!("Child link '{}' not found", joint.child.link))?;

        let parent_rb = link_rb_handles.get(&joint.parent.link)
            .context(format!("Parent rigid body for '{}' not found", joint.parent.link))?;

        let child_rb = link_rb_handles.get(&joint.child.link)
            .context(format!("Child rigid body for '{}' not found", joint.child.link))?;

        // Convert origin to anchor points
        let origin = &joint.origin;
        let anchor = Vec3::new(
            origin.xyz[0] as f32,
            origin.xyz[1] as f32,
            origin.xyz[2] as f32,
        );

        let axis = Vec3::new(
            joint.axis.xyz[0] as f32,
            joint.axis.xyz[1] as f32,
            joint.axis.xyz[2] as f32,
        );

        // Create appropriate joint type
        let (physics_joint, joint_type) = match joint.joint_type {
            urdf_rs::JointType::Revolute => {
                let limit = &joint.limit;
                if limit.upper != 0.0 || limit.lower != 0.0 {
                    let min_angle = limit.lower as f32;
                    let max_angle = limit.upper as f32;
                    (
                        create_revolute_joint_with_limits(anchor, anchor, axis, min_angle, max_angle),
                        JointType::Revolute
                    )
                } else {
                    (create_revolute_joint(anchor, anchor, axis), JointType::Revolute)
                }
            }
            urdf_rs::JointType::Continuous => {
                (create_revolute_joint(anchor, anchor, axis), JointType::Revolute)
            }
            urdf_rs::JointType::Prismatic => {
                let limit = &joint.limit;
                if limit.upper != 0.0 || limit.lower != 0.0 {
                    let min_dist = limit.lower as f32;
                    let max_dist = limit.upper as f32;
                    (
                        create_prismatic_joint_with_limits(anchor, anchor, axis, min_dist, max_dist),
                        JointType::Prismatic
                    )
                } else {
                    (create_prismatic_joint(anchor, anchor, axis), JointType::Prismatic)
                }
            }
            urdf_rs::JointType::Fixed => {
                (create_fixed_joint(anchor, anchor), JointType::Fixed)
            }
            urdf_rs::JointType::Floating => {
                // Floating joints are represented by no joint constraint
                return Ok(());
            }
            urdf_rs::JointType::Planar => {
                // Planar joints not directly supported, use fixed for now
                (create_fixed_joint(anchor, anchor), JointType::Fixed)
            }
            urdf_rs::JointType::Spherical => {
                // Spherical joint (ball joint)
                (create_spherical_joint(anchor, anchor), JointType::Spherical)
            }
        };

        // Insert joint into physics world
        let joint_handle = physics_world.impulse_joint_set.insert(
            *parent_rb,
            *child_rb,
            physics_joint,
            true,
        );

        // Add PhysicsJoint component to child entity
        commands.entity(*child_entity).insert(PhysicsJoint {
            handle: joint_handle,
            joint_type,
        });

        Ok(())
    }

    fn create_collider_from_geometry(&self, geometry: &Geometry) -> Result<Option<Collider>> {
        let collider = match geometry {
            Geometry::Box { size } => {
                let half_extents = Vec3::new(
                    size[0] as f32 / 2.0,
                    size[1] as f32 / 2.0,
                    size[2] as f32 / 2.0,
                );
                Some(ColliderBuilder::new(ColliderShape::Box { half_extents }).build())
            }
            Geometry::Cylinder { radius, length } => {
                let half_height = *length as f32 / 2.0;
                let radius = *radius as f32;
                Some(ColliderBuilder::new(ColliderShape::Cylinder { half_height, radius }).build())
            }
            Geometry::Capsule { radius, length } => {
                let half_height = *length as f32 / 2.0;
                let radius = *radius as f32;
                Some(ColliderBuilder::new(ColliderShape::Capsule { half_height, radius }).build())
            }
            Geometry::Sphere { radius } => {
                let radius = *radius as f32;
                Some(ColliderBuilder::new(ColliderShape::Sphere { radius }).build())
            }
            Geometry::Mesh { filename, scale } => {
                // Load mesh file and create trimesh collider
                let mesh_path = self.resolve_mesh_path(filename);
                let scale = scale.as_ref().map(|s| Vec3::new(s[0] as f32, s[1] as f32, s[2] as f32))
                    .unwrap_or(Vec3::ONE);

                // For now, we'll skip mesh colliders and let them be implemented later
                // with proper mesh loading infrastructure
                warn!("Mesh colliders not yet fully implemented, skipping: {}", mesh_path.display());
                None
            }
        };

        Ok(collider)
    }

    fn spawn_visual_mesh(
        &self,
        visual: &Visual,
        parent_entity: Entity,
        commands: &mut Commands,
        asset_server: &AssetServer,
    ) -> Result<()> {
        // Create transform from visual origin
        let origin = &visual.origin;
        let translation = Vec3::new(
            origin.xyz[0] as f32,
            origin.xyz[1] as f32,
            origin.xyz[2] as f32,
        );
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            origin.rpy[0] as f32,
            origin.rpy[1] as f32,
            origin.rpy[2] as f32,
        );

        let transform = Transform::from_translation(translation).with_rotation(rotation);

        // Create material
        let material = if let Some(mat) = &visual.material {
            if let Some(color) = &mat.color {
                Color::srgba(
                    color.rgba[0] as f32,
                    color.rgba[1] as f32,
                    color.rgba[2] as f32,
                    color.rgba[3] as f32,
                )
            } else {
                Color::srgb(0.8, 0.8, 0.8)
            }
        } else {
            Color::srgb(0.8, 0.8, 0.8)
        };

        // Spawn visual based on geometry type
        match &visual.geometry {
            Geometry::Box { size } => {
                let cuboid = bevy::prelude::Cuboid::new(
                    size[0] as f32,
                    size[1] as f32,
                    size[2] as f32,
                );
                commands.entity(parent_entity).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(asset_server.add(Mesh::from(cuboid))),
                        MeshMaterial3d(asset_server.add(StandardMaterial {
                            base_color: material,
                            ..default()
                        })),
                        transform,
                    ));
                });
            }
            Geometry::Cylinder { radius, length } => {
                let cylinder = bevy::prelude::Cylinder {
                    radius: *radius as f32,
                    half_height: *length as f32 / 2.0,
                };
                commands.entity(parent_entity).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(asset_server.add(Mesh::from(cylinder))),
                        MeshMaterial3d(asset_server.add(StandardMaterial {
                            base_color: material,
                            ..default()
                        })),
                        transform,
                    ));
                });
            }
            Geometry::Sphere { radius } => {
                let sphere = bevy::prelude::Sphere { radius: *radius as f32 };
                commands.entity(parent_entity).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(asset_server.add(Mesh::from(sphere))),
                        MeshMaterial3d(asset_server.add(StandardMaterial {
                            base_color: material,
                            ..default()
                        })),
                        transform,
                    ));
                });
            }
            Geometry::Capsule { radius, length } => {
                let capsule = bevy::prelude::Capsule3d {
                    radius: *radius as f32,
                    half_length: *length as f32 / 2.0,
                };
                commands.entity(parent_entity).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(asset_server.add(Mesh::from(capsule))),
                        MeshMaterial3d(asset_server.add(StandardMaterial {
                            base_color: material,
                            ..default()
                        })),
                        transform,
                    ));
                });
            }
            Geometry::Mesh { filename, scale } => {
                let mesh_path = self.resolve_mesh_path(filename);
                let scale_vec = scale.as_ref()
                    .map(|s| Vec3::new(s[0] as f32, s[1] as f32, s[2] as f32))
                    .unwrap_or(Vec3::ONE);

                let final_transform = transform.with_scale(scale_vec);

                // Load mesh file based on extension
                let mesh_handle = if mesh_path.extension().and_then(|e| e.to_str()) == Some("gltf")
                    || mesh_path.extension().and_then(|e| e.to_str()) == Some("glb") {
                    asset_server.load(format!("{}#Scene0", mesh_path.display()))
                } else {
                    // For STL, OBJ, etc., load directly
                    asset_server.load(mesh_path.to_string_lossy().to_string())
                };

                commands.entity(parent_entity).with_children(|parent| {
                    parent.spawn((
                        SceneRoot(mesh_handle),
                        final_transform,
                    ));
                });
            }
        }

        Ok(())
    }

    fn resolve_mesh_path(&self, filename: &str) -> PathBuf {
        // Handle package:// URIs
        if filename.starts_with("package://") {
            let relative_path = filename.strip_prefix("package://").unwrap();
            self.base_path.join(relative_path)
        } else if filename.starts_with("file://") {
            PathBuf::from(filename.strip_prefix("file://").unwrap())
        } else {
            self.base_path.join(filename)
        }
    }

    fn get_rb_handle_for_entity(
        &self,
        entity: Entity,
        physics_world: &PhysicsWorld,
    ) -> Option<RigidBodyHandle> {
        // Search through rigid body set to find handle matching entity
        physics_world.rigid_body_set
            .iter()
            .find(|(_, rb)| Entity::from_bits(rb.user_data as u64) == entity)
            .map(|(handle, _)| handle)
    }
}

impl Default for URDFLoader {
    fn default() -> Self {
        Self::new()
    }
}
