use bevy::prelude::*;
use bevy::pbr::StandardMaterial;
use rapier3d::prelude::*;

use crate::physics::diff_drive::{CmdVel, DifferentialDrive};
use crate::physics::rigid_body::{
    Damping, ExternalForce, ExternalImpulse, GravityScale, RigidBodyComponent, Velocity,
};
use crate::physics::world::PhysicsWorld;

/// Sync Rapier3D rigid body transforms to Bevy visual transforms
pub fn sync_physics_to_visual_system(
    physics_world: Res<PhysicsWorld>,
    mut query: Query<(&RigidBodyComponent, &mut Transform)>,
) {
    for (rb_component, mut transform) in query.iter_mut() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get(rb_component.handle) {
            let position = rigid_body.translation();
            let rotation = rigid_body.rotation();

            transform.translation = Vec3::new(position.x, position.y, position.z);
            transform.rotation = Quat::from_xyzw(rotation.i, rotation.j, rotation.k, rotation.w);
        }
    }
}

/// Debug system to check mesh components on physics entities
pub fn debug_mesh_components_system(
    physics_query: Query<(Entity, &Name, Option<&Mesh3d>, Option<&MeshMaterial3d<StandardMaterial>>, Option<&Visibility>, Option<&ViewVisibility>, Option<&GlobalTransform>, &Transform, Option<&bevy::render::primitives::Aabb>), With<RigidBodyComponent>>,
    all_mesh_query: Query<(Entity, &Name, &Mesh3d, Option<&MeshMaterial3d<StandardMaterial>>, Option<&Visibility>, Option<&ViewVisibility>, Option<&GlobalTransform>, &Transform, Option<&bevy::render::primitives::Aabb>), Without<RigidBodyComponent>>,
    mesh_assets: Res<Assets<Mesh>>,
    material_assets: Res<Assets<StandardMaterial>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    // Log on frames 1, 10, 50, and 100 to see if things change over time
    if *frame_count == 1 || *frame_count == 10 || *frame_count == 50 {
        info!("DEBUG_MESH Frame {}: Checking {} physics entities, {} non-physics mesh entities", *frame_count, physics_query.iter().count(), all_mesh_query.iter().count());

        // Check non-physics entities (like debug cube) that DO render
        for (entity, name, mesh, material, visibility, view_vis, global_transform, transform, aabb) in all_mesh_query.iter() {
            let vis_str = visibility.map(|v| format!("{:?}", v)).unwrap_or("None".to_string());
            let view_vis_str = view_vis.map(|v| format!("{}", v.get())).unwrap_or("None".to_string());
            let gt_str = global_transform.map(|gt| format!("{:?}", gt.translation())).unwrap_or("None".to_string());
            let mesh_valid = mesh_assets.contains(&mesh.0);
            let mat_valid = material.as_ref().map(|m| material_assets.contains(&m.0)).unwrap_or(false);
            let aabb_str = aabb.map(|a| format!("min={:?} max={:?}", a.min(), a.max())).unwrap_or("NO_AABB".to_string());

            // Get the actual mesh to check its attributes
            let mesh_info = mesh_assets.get(&mesh.0).map(|m| {
                let vertex_count = m.count_vertices();
                let has_positions = m.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION).is_some();
                let has_normals = m.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_NORMAL).is_some();
                format!("verts={} has_pos={} has_norm={}", vertex_count, has_positions, has_normals)
            }).unwrap_or("NO_MESH".to_string());

            info!("DEBUG_NON_PHYSICS: '{}' entity={:?} pos={:?} global={} vis={} view_vis={} mesh_valid={} mat_valid={} mesh_info={} aabb={}",
                  name.as_str(), entity, transform.translation, gt_str, vis_str, view_vis_str,
                  mesh_valid, mat_valid, mesh_info, aabb_str);
        }

        // Check physics entities that DON'T render
        for (entity, name, mesh, material, visibility, view_vis, global_transform, transform, aabb) in physics_query.iter() {
            let vis_str = visibility.map(|v| format!("{:?}", v)).unwrap_or("None".to_string());
            let view_vis_str = view_vis.map(|v| format!("{}", v.get())).unwrap_or("None".to_string());
            let gt_str = global_transform.map(|gt| format!("{:?}", gt.translation())).unwrap_or("None".to_string());
            let aabb_str = aabb.map(|a| format!("min={:?} max={:?}", a.min(), a.max())).unwrap_or("NO_AABB".to_string());

            // Check if mesh asset actually exists
            let mesh_valid = mesh.as_ref().map(|m| mesh_assets.contains(&m.0)).unwrap_or(false);
            let mat_valid = material.as_ref().map(|m| material_assets.contains(&m.0)).unwrap_or(false);

            // Get the actual mesh to check its attributes
            let mesh_info = mesh.as_ref().and_then(|m| mesh_assets.get(&m.0)).map(|m| {
                let vertex_count = m.count_vertices();
                let has_positions = m.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION).is_some();
                let has_normals = m.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_NORMAL).is_some();
                format!("verts={} has_pos={} has_norm={}", vertex_count, has_positions, has_normals)
            }).unwrap_or("NO_MESH".to_string());

            info!("DEBUG_PHYSICS: '{}' entity={:?} pos={:?} global={} vis={} view_vis={} mesh={} (valid={}) mat={} (valid={}) mesh_info={} aabb={}",
                  name.as_str(), entity, transform.translation, gt_str, vis_str, view_vis_str,
                  mesh.is_some(), mesh_valid, material.is_some(), mat_valid, mesh_info, aabb_str);
        }
    }
}

/// Read velocities from physics world and update Velocity components
pub fn sync_velocities_from_physics_system(
    physics_world: Res<PhysicsWorld>,
    mut query: Query<(&RigidBodyComponent, &mut Velocity)>,
) {
    for (rb_component, mut velocity) in query.iter_mut() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get(rb_component.handle) {
            let linvel = rigid_body.linvel();
            let angvel = rigid_body.angvel();

            velocity.linear = Vec3::new(linvel.x, linvel.y, linvel.z);
            velocity.angular = Vec3::new(angvel.x, angvel.y, angvel.z);
        }
    }
}

/// Apply external forces to rigid bodies
pub fn apply_external_forces_system(
    mut physics_world: ResMut<PhysicsWorld>,
    mut query: Query<(&RigidBodyComponent, &mut ExternalForce)>,
) {
    for (rb_component, mut ext_force) in query.iter_mut() {
        if ext_force.force == Vec3::ZERO && ext_force.torque == Vec3::ZERO {
            continue;
        }

        if let Some(rigid_body) = physics_world.rigid_body_set.get_mut(rb_component.handle) {
            let force =
                nalgebra::Vector3::new(ext_force.force.x, ext_force.force.y, ext_force.force.z);
            let torque =
                nalgebra::Vector3::new(ext_force.torque.x, ext_force.torque.y, ext_force.torque.z);

            rigid_body.add_force(force, true);
            rigid_body.add_torque(torque, true);
        }

        // Reset forces after application
        ext_force.force = Vec3::ZERO;
        ext_force.torque = Vec3::ZERO;
    }
}

/// Apply external impulses to rigid bodies
pub fn apply_external_impulses_system(
    mut physics_world: ResMut<PhysicsWorld>,
    mut query: Query<(&RigidBodyComponent, &mut ExternalImpulse)>,
) {
    for (rb_component, mut ext_impulse) in query.iter_mut() {
        if ext_impulse.impulse == Vec3::ZERO && ext_impulse.torque_impulse == Vec3::ZERO {
            continue;
        }

        if let Some(rigid_body) = physics_world.rigid_body_set.get_mut(rb_component.handle) {
            let impulse = nalgebra::Vector3::new(
                ext_impulse.impulse.x,
                ext_impulse.impulse.y,
                ext_impulse.impulse.z,
            );
            let torque_impulse = nalgebra::Vector3::new(
                ext_impulse.torque_impulse.x,
                ext_impulse.torque_impulse.y,
                ext_impulse.torque_impulse.z,
            );

            rigid_body.apply_impulse(impulse, true);
            rigid_body.apply_torque_impulse(torque_impulse, true);
        }

        // Reset impulses after application
        ext_impulse.impulse = Vec3::ZERO;
        ext_impulse.torque_impulse = Vec3::ZERO;
    }
}

/// Apply damping to rigid bodies
pub fn apply_damping_system(
    mut physics_world: ResMut<PhysicsWorld>,
    query: Query<(&RigidBodyComponent, &Damping), Changed<Damping>>,
) {
    for (rb_component, damping) in query.iter() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get_mut(rb_component.handle) {
            rigid_body.set_linear_damping(damping.linear_damping);
            rigid_body.set_angular_damping(damping.angular_damping);
        }
    }
}

/// Apply gravity scale to rigid bodies
pub fn apply_gravity_scale_system(
    mut physics_world: ResMut<PhysicsWorld>,
    query: Query<(&RigidBodyComponent, &GravityScale), Changed<GravityScale>>,
) {
    for (rb_component, gravity_scale) in query.iter() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get_mut(rb_component.handle) {
            rigid_body.set_gravity_scale(gravity_scale.scale, true);
        }
    }
}

/// Apply differential drive commands to rigid bodies
pub fn apply_differential_drive_system(
    _time: Res<Time>,
    mut physics_world: ResMut<PhysicsWorld>,
    mut query: Query<(&RigidBodyComponent, &DifferentialDrive, &CmdVel, &Transform)>,
) {
    for (rb_component, diff_drive, cmd_vel, transform) in query.iter_mut() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get_mut(rb_component.handle) {
            // Get current yaw from transform
            let (_, _, yaw) = transform.rotation.to_euler(EulerRot::XYZ);

            // Compute desired velocities
            let (linvel, angvel) = diff_drive.apply_velocity(cmd_vel.linear, cmd_vel.angular, yaw);

            // Apply velocities to rigid body
            rigid_body.set_linvel(linvel, true);
            rigid_body.set_angvel(angvel, true);
        }
    }
}

/// Alternative: Apply differential drive using forces (more realistic)
pub fn apply_differential_drive_forces_system(
    time: Res<Time>,
    mut physics_world: ResMut<PhysicsWorld>,
    query: Query<(
        &RigidBodyComponent,
        &DifferentialDrive,
        &CmdVel,
        &Transform,
        &Velocity,
    )>,
) {
    let _dt = time.delta_secs();

    for (rb_component, diff_drive, cmd_vel, transform, current_vel) in query.iter() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get_mut(rb_component.handle) {
            // Get current yaw
            let (_, _, yaw) = transform.rotation.to_euler(EulerRot::XYZ);

            // Compute target velocities
            let (target_linvel, target_angvel) =
                diff_drive.apply_velocity(cmd_vel.linear, cmd_vel.angular, yaw);

            // Convert to bevy vectors for comparison
            let target_linvel_bevy = Vec3::new(target_linvel.x, target_linvel.y, target_linvel.z);
            let target_angvel_bevy = Vec3::new(target_angvel.x, target_angvel.y, target_angvel.z);

            // Compute force needed (simple P controller)
            let kp_linear = 10.0;
            let kp_angular = 5.0;

            let linear_error = target_linvel_bevy - current_vel.linear;
            let angular_error = target_angvel_bevy - current_vel.angular;

            let force = linear_error * kp_linear;
            let torque = angular_error * kp_angular;

            // Get mass for force scaling
            let mass = rigid_body.mass();

            let force_na = nalgebra::Vector3::new(force.x, force.y, force.z) * mass;
            let torque_na = nalgebra::Vector3::new(torque.x, torque.y, torque.z);

            rigid_body.add_force(force_na, true);
            rigid_body.add_torque(torque_na, true);
        }
    }
}

/// Set velocities directly on rigid bodies (for kinematic control)
pub fn apply_velocity_system(
    mut physics_world: ResMut<PhysicsWorld>,
    query: Query<(&RigidBodyComponent, &Velocity), Changed<Velocity>>,
) {
    for (rb_component, velocity) in query.iter() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get_mut(rb_component.handle) {
            let linvel =
                nalgebra::Vector3::new(velocity.linear.x, velocity.linear.y, velocity.linear.z);
            let angvel =
                nalgebra::Vector3::new(velocity.angular.x, velocity.angular.y, velocity.angular.z);

            rigid_body.set_linvel(linvel, true);
            rigid_body.set_angvel(angvel, true);
        }
    }
}
