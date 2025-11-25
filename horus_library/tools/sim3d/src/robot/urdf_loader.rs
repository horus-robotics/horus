use anyhow::{Context, Result};
use bevy::prelude::*;
use bevy::render::mesh::Mesh;
use rapier3d::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use urdf_rs::{Geometry, Joint, Link, Robot as URDFRobot, Visual};

use crate::assets::mesh::{MeshLoadOptions, MeshLoader};
use crate::assets::resolver::PathResolver;
use crate::physics::collider::{ColliderBuilder, ColliderShape};
use crate::physics::joints::{
    create_fixed_joint, create_prismatic_joint, create_prismatic_joint_with_limits,
    create_revolute_joint, create_revolute_joint_with_limits, create_spherical_joint, JointType,
    PhysicsJoint,
};
use crate::physics::world::PhysicsWorld;
use crate::robot::robot::Robot;
use crate::tf::tree::TFTree;

pub struct URDFLoader {
    base_path: PathBuf,
    mesh_loader: MeshLoader,
    path_resolver: PathResolver,
}

impl URDFLoader {
    pub fn new() -> Self {
        let mut mesh_loader = MeshLoader::new();
        let mut path_resolver = PathResolver::new();

        // Add common robot model paths
        mesh_loader.add_base_path(PathBuf::from("assets/models"));
        mesh_loader.add_base_path(PathBuf::from("assets/robots"));
        path_resolver.add_base_path(PathBuf::from("assets/models"));
        path_resolver.add_base_path(PathBuf::from("assets/robots"));

        Self {
            base_path: PathBuf::from("."),
            mesh_loader,
            path_resolver,
        }
    }

    pub fn with_base_path(mut self, path: impl Into<PathBuf>) -> Self {
        let path_buf = path.into();
        self.base_path = path_buf.clone();
        self.mesh_loader.add_base_path(path_buf.clone());
        self.path_resolver.add_base_path(path_buf);
        self
    }

    pub fn load(
        &mut self,
        urdf_path: impl AsRef<Path>,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        tf_tree: &mut TFTree,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<Entity> {
        let urdf_path = urdf_path.as_ref();
        let urdf_str = std::fs::read_to_string(urdf_path)
            .with_context(|| format!("Failed to read URDF file: {}", urdf_path.display()))?;

        let urdf = urdf_rs::read_from_string(&urdf_str)
            .with_context(|| format!("Failed to parse URDF file: {}", urdf_path.display()))?;

        self.spawn_robot(urdf, commands, physics_world, tf_tree, meshes, materials)
    }

    pub fn spawn_robot(
        &mut self,
        urdf: URDFRobot,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        tf_tree: &mut TFTree,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Result<Entity> {
        let robot_name = urdf.name.clone();

        // Create root entity for the robot
        let root_entity = commands
            .spawn((
                Robot::new(&robot_name),
                Transform::default(),
                Visibility::default(),
            ))
            .id();

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
                meshes,
                materials,
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

        // Parent links according to joint hierarchy
        for joint in &urdf.joints {
            if let (Some(&parent_entity), Some(&child_entity)) = (
                link_entities.get(&joint.parent.link),
                link_entities.get(&joint.child.link),
            ) {
                // Apply joint transform to child
                let origin = &joint.origin;
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

                commands
                    .entity(child_entity)
                    .insert(Transform::from_translation(translation).with_rotation(rotation));

                // Parent child to parent
                commands.entity(parent_entity).add_child(child_entity);
            }
        }

        // Parent the base link (or any unparented links) to root
        for (link_name, &link_entity) in &link_entities {
            // Check if this link is a child in any joint
            let is_child = urdf.joints.iter().any(|j| j.child.link == *link_name);
            if !is_child {
                commands.entity(root_entity).add_child(link_entity);
            }
        }

        Ok(root_entity)
    }

    fn spawn_link(
        &mut self,
        link: &Link,
        robot_name: &str,
        commands: &mut Commands,
        physics_world: &mut PhysicsWorld,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
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
            RigidBodyBuilder::fixed().build()
        };

        // Create entity and spawn rigid body
        let entity = commands
            .spawn((
                Name::new(format!("{}::{}", robot_name, link.name)),
                Transform::default(),
                Visibility::default(),
            ))
            .id();

        let rb_handle = physics_world.spawn_rigid_body(rigid_body, entity);

        // Add colliders from collision geometry
        for collision in &link.collision {
            if let Some(collider) = self.create_collider_from_geometry(&collision.geometry)? {
                physics_world.spawn_collider(collider, rb_handle);
            }
        }

        // Add visual meshes
        for visual in &link.visual {
            self.spawn_visual_mesh(visual, entity, commands, meshes, materials)?;
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
        let _parent_entity = link_entities
            .get(&joint.parent.link)
            .context(format!("Parent link '{}' not found", joint.parent.link))?;

        let child_entity = link_entities
            .get(&joint.child.link)
            .context(format!("Child link '{}' not found", joint.child.link))?;

        let parent_rb = link_rb_handles.get(&joint.parent.link).context(format!(
            "Parent rigid body for '{}' not found",
            joint.parent.link
        ))?;

        let child_rb = link_rb_handles.get(&joint.child.link).context(format!(
            "Child rigid body for '{}' not found",
            joint.child.link
        ))?;

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
                        create_revolute_joint_with_limits(
                            anchor, anchor, axis, min_angle, max_angle,
                        ),
                        JointType::Revolute,
                    )
                } else {
                    (
                        create_revolute_joint(anchor, anchor, axis),
                        JointType::Revolute,
                    )
                }
            }
            urdf_rs::JointType::Continuous => (
                create_revolute_joint(anchor, anchor, axis),
                JointType::Revolute,
            ),
            urdf_rs::JointType::Prismatic => {
                let limit = &joint.limit;
                if limit.upper != 0.0 || limit.lower != 0.0 {
                    let min_dist = limit.lower as f32;
                    let max_dist = limit.upper as f32;
                    (
                        create_prismatic_joint_with_limits(
                            anchor, anchor, axis, min_dist, max_dist,
                        ),
                        JointType::Prismatic,
                    )
                } else {
                    (
                        create_prismatic_joint(anchor, anchor, axis),
                        JointType::Prismatic,
                    )
                }
            }
            urdf_rs::JointType::Fixed => (create_fixed_joint(anchor, anchor), JointType::Fixed),
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
        let joint_handle =
            physics_world
                .impulse_joint_set
                .insert(*parent_rb, *child_rb, physics_joint, true);

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
                Some(
                    ColliderBuilder::new(ColliderShape::Cylinder {
                        half_height,
                        radius,
                    })
                    .build(),
                )
            }
            Geometry::Capsule { radius, length } => {
                let half_height = *length as f32 / 2.0;
                let radius = *radius as f32;
                Some(
                    ColliderBuilder::new(ColliderShape::Capsule {
                        half_height,
                        radius,
                    })
                    .build(),
                )
            }
            Geometry::Sphere { radius } => {
                let radius = *radius as f32;
                Some(ColliderBuilder::new(ColliderShape::Sphere { radius }).build())
            }
            Geometry::Mesh { filename, scale: _ } => {
                // Load mesh file and create trimesh collider
                let mesh_path = self.resolve_mesh_path(filename);

                // For now, we'll skip mesh colliders and let them be implemented later
                // with proper mesh loading infrastructure
                warn!(
                    "Mesh colliders not yet fully implemented, skipping: {}",
                    mesh_path.display()
                );
                None
            }
        };

        Ok(collider)
    }

    fn spawn_visual_mesh(
        &mut self,
        visual: &Visual,
        parent_entity: Entity,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
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
                let cuboid =
                    bevy::prelude::Cuboid::new(size[0] as f32, size[1] as f32, size[2] as f32);
                commands.entity(parent_entity).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(meshes.add(Mesh::from(cuboid))),
                        MeshMaterial3d(materials.add(StandardMaterial {
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
                        Mesh3d(meshes.add(Mesh::from(cylinder))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: material,
                            ..default()
                        })),
                        transform,
                    ));
                });
            }
            Geometry::Sphere { radius } => {
                let sphere = bevy::prelude::Sphere {
                    radius: *radius as f32,
                };
                commands.entity(parent_entity).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(meshes.add(Mesh::from(sphere))),
                        MeshMaterial3d(materials.add(StandardMaterial {
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
                        Mesh3d(meshes.add(Mesh::from(capsule))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: material,
                            ..default()
                        })),
                        transform,
                    ));
                });
            }
            Geometry::Mesh { filename, scale } => {
                // Load mesh file using the mesh loader
                let resolved_path = match self.resolve_mesh_path_full(filename) {
                    Ok(path) => path,
                    Err(e) => {
                        warn!("Failed to resolve mesh path '{}': {}", filename, e);
                        warn!("Spawning fallback cube geometry instead");
                        // Spawn fallback cube when mesh path cannot be resolved
                        let cuboid = bevy::prelude::Cuboid::new(0.1, 0.1, 0.1);
                        commands.entity(parent_entity).with_children(|parent| {
                            parent.spawn((
                                Mesh3d(meshes.add(Mesh::from(cuboid))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::srgb(1.0, 0.0, 1.0), // Magenta for "missing mesh"
                                    ..default()
                                })),
                                transform,
                            ));
                        });
                        return Ok(());
                    }
                };

                // Set up mesh load options with scale
                let mesh_scale = scale
                    .as_ref()
                    .map(|s| Vec3::new(s[0] as f32, s[1] as f32, s[2] as f32))
                    .unwrap_or(Vec3::ONE);

                let load_options = MeshLoadOptions::default()
                    .with_scale(mesh_scale)
                    .generate_normals(true)
                    .generate_tangents(false)
                    .validate(true);

                // Load the mesh
                match self.mesh_loader.load(&resolved_path, load_options) {
                    Ok(loaded_mesh) => {
                        info!(
                            "Successfully loaded mesh: {} ({} vertices, {} triangles)",
                            resolved_path.display(),
                            loaded_mesh.vertex_count,
                            loaded_mesh.triangle_count
                        );

                        // Add mesh to assets
                        let mesh_handle = meshes.add(loaded_mesh.mesh);

                        // Use material from URDF if available, otherwise use mesh material
                        let material_handle = materials.add(StandardMaterial {
                            base_color: material,
                            ..default()
                        });

                        // Spawn the loaded mesh
                        commands.entity(parent_entity).with_children(|parent| {
                            parent.spawn((
                                Mesh3d(mesh_handle),
                                MeshMaterial3d(material_handle),
                                transform,
                            ));
                        });
                    }
                    Err(e) => {
                        warn!("Failed to load mesh '{}': {}", resolved_path.display(), e);
                        warn!("Spawning fallback cube geometry instead");
                        // Spawn fallback cube when mesh loading fails
                        let cuboid = bevy::prelude::Cuboid::new(0.1, 0.1, 0.1);
                        commands.entity(parent_entity).with_children(|parent| {
                            parent.spawn((
                                Mesh3d(meshes.add(Mesh::from(cuboid))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::srgb(1.0, 0.0, 1.0), // Magenta for "missing mesh"
                                    ..default()
                                })),
                                transform,
                            ));
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn resolve_mesh_path(&self, filename: &str) -> PathBuf {
        // Simple fallback for collider mesh paths (not used for visual meshes)
        if filename.starts_with("package://") {
            let relative_path = filename.strip_prefix("package://").unwrap();
            self.base_path.join(relative_path)
        } else if filename.starts_with("file://") {
            PathBuf::from(filename.strip_prefix("file://").unwrap())
        } else {
            self.base_path.join(filename)
        }
    }

    fn resolve_mesh_path_full(&self, filename: &str) -> Result<PathBuf> {
        // Use the PathResolver for comprehensive URI resolution
        self.path_resolver
            .resolve(filename)
            .with_context(|| format!("Failed to resolve mesh URI: {}", filename))
    }

    fn get_rb_handle_for_entity(
        &self,
        entity: Entity,
        physics_world: &PhysicsWorld,
    ) -> Option<RigidBodyHandle> {
        // Search through rigid body set to find handle matching entity
        physics_world
            .rigid_body_set
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
