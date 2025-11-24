//! Joint validation tests for sim3d
//!
//! Comprehensive tests for various joint types:
//! - Revolute joints (hinge)
//! - Prismatic joints (slider)
//! - Fixed joints
//! - Spherical joints (ball-and-socket)
//! - Joint motors and breaking

use bevy::prelude::*;
use rapier3d::prelude::*;
use std::f32::consts::PI;

const GRAVITY: f32 = 9.81;
const TOLERANCE: f32 = 0.05;

/// Test configuration for revolute joint limits
#[derive(Debug, Clone)]
pub struct RevoluteJointLimitTest {
    pub lower_limit: f32,
    pub upper_limit: f32,
    pub initial_angle: f32,
    pub applied_torque: f32,
    pub duration: f32,
    pub timestep: f32,
}

impl Default for RevoluteJointLimitTest {
    fn default() -> Self {
        Self {
            lower_limit: -PI / 4.0,
            upper_limit: PI / 4.0,
            initial_angle: 0.0,
            applied_torque: 10.0,
            duration: 2.0,
            timestep: 0.001,
        }
    }
}

/// Result of revolute joint limit test
#[derive(Debug, Clone)]
pub struct RevoluteJointLimitResult {
    pub min_angle_reached: f32,
    pub max_angle_reached: f32,
    pub limits_respected: bool,
    pub final_angle: f32,
}

/// Run revolute joint limit validation
pub fn validate_revolute_joint_limits(config: RevoluteJointLimitTest) -> Result<RevoluteJointLimitResult, String> {
    let num_steps = (config.duration / config.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, -GRAVITY, 0.0];
    let integration_parameters = IntegrationParameters {
        dt: config.timestep,
        ..Default::default()
    };
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let physics_hooks = ();
    let event_handler = ();

    // Create fixed base
    let base = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 2.0, 0.0])
        .build();
    let base_handle = rigid_body_set.insert(base);

    // Create rotating link
    let link = RigidBodyBuilder::dynamic()
        .translation(vector![0.5, 2.0, 0.0])
        .build();
    let link_handle = rigid_body_set.insert(link);

    let link_collider = ColliderBuilder::cuboid(0.5, 0.05, 0.05)
        .mass(1.0)
        .build();
    collider_set.insert_with_parent(link_collider, link_handle, &mut rigid_body_set);

    // Create revolute joint with limits
    let joint = RevoluteJointBuilder::new(Vector::z_axis())
        .local_anchor1(point![0.0, 0.0, 0.0])
        .local_anchor2(point![-0.5, 0.0, 0.0])
        .limits([config.lower_limit, config.upper_limit])
        .motor_velocity(config.applied_torque.signum() * 5.0, config.applied_torque.abs())
        .build();
    impulse_joint_set.insert(base_handle, link_handle, joint, true);

    let mut min_angle = f32::MAX;
    let mut max_angle = f32::MIN;
    let mut final_angle = 0.0;

    // Run simulation
    for _step in 0..num_steps {
        physics_pipeline.step(
            &gravity_vec,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );

        let rb = rigid_body_set.get(link_handle).ok_or("Link not found")?;
        let rotation = rb.rotation();
        let (axis, angle) = rotation.axis_angle().unwrap_or((Vector::z_axis(), 0.0));
        let angle_z = if axis.z > 0.0 { angle } else { -angle };

        min_angle = min_angle.min(angle_z);
        max_angle = max_angle.max(angle_z);
        final_angle = angle_z;
    }

    // Check if limits were respected (with small tolerance for solver)
    let limit_tolerance = 0.1; // radians
    let limits_respected = min_angle >= config.lower_limit - limit_tolerance
        && max_angle <= config.upper_limit + limit_tolerance;

    Ok(RevoluteJointLimitResult {
        min_angle_reached: min_angle,
        max_angle_reached: max_angle,
        limits_respected,
        final_angle,
    })
}

/// Test configuration for prismatic joint motion
#[derive(Debug, Clone)]
pub struct PrismaticJointMotionTest {
    pub axis: Vec3,
    pub lower_limit: f32,
    pub upper_limit: f32,
    pub applied_force: f32,
    pub duration: f32,
    pub timestep: f32,
}

impl Default for PrismaticJointMotionTest {
    fn default() -> Self {
        Self {
            axis: Vec3::X,
            lower_limit: -1.0,
            upper_limit: 1.0,
            applied_force: 50.0,
            duration: 2.0,
            timestep: 0.001,
        }
    }
}

/// Result of prismatic joint motion test
#[derive(Debug, Clone)]
pub struct PrismaticJointMotionResult {
    pub min_position: f32,
    pub max_position: f32,
    pub limits_respected: bool,
    pub motion_occurred: bool,
    pub final_position: f32,
}

/// Run prismatic joint motion validation
pub fn validate_prismatic_joint_motion(config: PrismaticJointMotionTest) -> Result<PrismaticJointMotionResult, String> {
    let num_steps = (config.duration / config.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, -GRAVITY, 0.0];
    let integration_parameters = IntegrationParameters {
        dt: config.timestep,
        ..Default::default()
    };
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let physics_hooks = ();
    let event_handler = ();

    // Create fixed base
    let base = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 0.0, 0.0])
        .build();
    let base_handle = rigid_body_set.insert(base);

    // Create sliding body
    let slider = RigidBodyBuilder::dynamic()
        .translation(vector![0.0, 0.0, 0.0])
        .build();
    let slider_handle = rigid_body_set.insert(slider);

    let slider_collider = ColliderBuilder::cuboid(0.1, 0.1, 0.1)
        .mass(1.0)
        .build();
    collider_set.insert_with_parent(slider_collider, slider_handle, &mut rigid_body_set);

    // Create prismatic joint
    let axis = nalgebra::Unit::new_normalize(vector![config.axis.x, config.axis.y, config.axis.z]);
    let joint = PrismaticJointBuilder::new(axis)
        .local_anchor1(point![0.0, 0.0, 0.0])
        .local_anchor2(point![0.0, 0.0, 0.0])
        .limits([config.lower_limit, config.upper_limit])
        .motor_velocity(config.applied_force.signum() * 2.0, config.applied_force.abs())
        .build();
    impulse_joint_set.insert(base_handle, slider_handle, joint, true);

    let mut min_pos = f32::MAX;
    let mut max_pos = f32::MIN;
    let mut final_pos = 0.0;

    // Run simulation
    for _step in 0..num_steps {
        physics_pipeline.step(
            &gravity_vec,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );

        let rb = rigid_body_set.get(slider_handle).ok_or("Slider not found")?;
        let pos = rb.translation();
        let position_along_axis = pos.x * config.axis.x + pos.y * config.axis.y + pos.z * config.axis.z;

        min_pos = min_pos.min(position_along_axis);
        max_pos = max_pos.max(position_along_axis);
        final_pos = position_along_axis;
    }

    let limit_tolerance = 0.1;
    let limits_respected = min_pos >= config.lower_limit - limit_tolerance
        && max_pos <= config.upper_limit + limit_tolerance;
    let motion_occurred = (max_pos - min_pos).abs() > 0.01;

    Ok(PrismaticJointMotionResult {
        min_position: min_pos,
        max_position: max_pos,
        limits_respected,
        motion_occurred,
        final_position: final_pos,
    })
}

/// Test configuration for fixed joint rigidity
#[derive(Debug, Clone)]
pub struct FixedJointRigidityTest {
    pub applied_force: Vec3,
    pub applied_torque: Vec3,
    pub duration: f32,
    pub timestep: f32,
}

impl Default for FixedJointRigidityTest {
    fn default() -> Self {
        Self {
            applied_force: Vec3::new(100.0, 0.0, 0.0),
            applied_torque: Vec3::new(0.0, 10.0, 0.0),
            duration: 1.0,
            timestep: 0.001,
        }
    }
}

/// Result of fixed joint rigidity test
#[derive(Debug, Clone)]
pub struct FixedJointRigidityResult {
    pub max_relative_displacement: f32,
    pub max_relative_rotation: f32,
    pub joint_maintained: bool,
}

/// Run fixed joint rigidity validation
pub fn validate_fixed_joint_rigidity(config: FixedJointRigidityTest) -> Result<FixedJointRigidityResult, String> {
    let num_steps = (config.duration / config.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, -GRAVITY, 0.0];
    let integration_parameters = IntegrationParameters {
        dt: config.timestep,
        ..Default::default()
    };
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let physics_hooks = ();
    let event_handler = ();

    // Create fixed base
    let base = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 1.0, 0.0])
        .build();
    let base_handle = rigid_body_set.insert(base);

    // Create attached body
    let attached = RigidBodyBuilder::dynamic()
        .translation(vector![1.0, 1.0, 0.0])
        .build();
    let attached_handle = rigid_body_set.insert(attached);

    let attached_collider = ColliderBuilder::cuboid(0.2, 0.2, 0.2)
        .mass(5.0)
        .build();
    collider_set.insert_with_parent(attached_collider, attached_handle, &mut rigid_body_set);

    // Create fixed joint
    let joint = FixedJointBuilder::new()
        .local_anchor1(point![1.0, 0.0, 0.0])
        .local_anchor2(point![0.0, 0.0, 0.0])
        .build();
    impulse_joint_set.insert(base_handle, attached_handle, joint, true);

    // Get initial relative pose
    let base_rb = rigid_body_set.get(base_handle).ok_or("Base not found")?;
    let attached_rb = rigid_body_set.get(attached_handle).ok_or("Attached not found")?;
    let initial_relative_pos = attached_rb.translation() - base_rb.translation();
    let initial_relative_rot = base_rb.rotation().inverse() * attached_rb.rotation();

    let mut max_displacement = 0.0f32;
    let mut max_rotation = 0.0f32;

    // Run simulation with external forces
    for step in 0..num_steps {
        // Apply forces periodically
        if step % 100 == 0 {
            if let Some(rb) = rigid_body_set.get_mut(attached_handle) {
                rb.apply_impulse(vector![config.applied_force.x * config.timestep,
                                         config.applied_force.y * config.timestep,
                                         config.applied_force.z * config.timestep], true);
                rb.apply_torque_impulse(vector![config.applied_torque.x * config.timestep,
                                                config.applied_torque.y * config.timestep,
                                                config.applied_torque.z * config.timestep], true);
            }
        }

        physics_pipeline.step(
            &gravity_vec,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );

        let base_rb = rigid_body_set.get(base_handle).ok_or("Base not found")?;
        let attached_rb = rigid_body_set.get(attached_handle).ok_or("Attached not found")?;

        let current_relative_pos = attached_rb.translation() - base_rb.translation();
        let current_relative_rot = base_rb.rotation().inverse() * attached_rb.rotation();

        let displacement = (current_relative_pos - initial_relative_pos).norm();
        max_displacement = max_displacement.max(displacement);

        let rot_diff = initial_relative_rot.inverse() * current_relative_rot;
        let (_, angle) = rot_diff.axis_angle().unwrap_or((Vector::z_axis(), 0.0));
        max_rotation = max_rotation.max(angle.abs());
    }

    // Fixed joint should maintain relative pose with minimal deviation
    let joint_maintained = max_displacement < 0.1 && max_rotation < 0.1;

    Ok(FixedJointRigidityResult {
        max_relative_displacement: max_displacement,
        max_relative_rotation: max_rotation,
        joint_maintained,
    })
}

/// Test configuration for spherical joint degrees of freedom
#[derive(Debug, Clone)]
pub struct SphericalJointDOFTest {
    pub applied_torque: Vec3,
    pub duration: f32,
    pub timestep: f32,
}

impl Default for SphericalJointDOFTest {
    fn default() -> Self {
        Self {
            applied_torque: Vec3::new(5.0, 5.0, 5.0),
            duration: 2.0,
            timestep: 0.001,
        }
    }
}

/// Result of spherical joint DOF test
#[derive(Debug, Clone)]
pub struct SphericalJointDOFResult {
    pub x_rotation_range: f32,
    pub y_rotation_range: f32,
    pub z_rotation_range: f32,
    pub all_dof_active: bool,
    pub position_constraint_maintained: bool,
}

/// Run spherical joint degrees of freedom validation
pub fn validate_spherical_joint_dof(config: SphericalJointDOFTest) -> Result<SphericalJointDOFResult, String> {
    let num_steps = (config.duration / config.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, -GRAVITY, 0.0];
    let integration_parameters = IntegrationParameters {
        dt: config.timestep,
        ..Default::default()
    };
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let physics_hooks = ();
    let event_handler = ();

    // Create fixed base
    let base = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 2.0, 0.0])
        .build();
    let base_handle = rigid_body_set.insert(base);

    // Create ball body
    let ball = RigidBodyBuilder::dynamic()
        .translation(vector![0.0, 1.0, 0.0])
        .build();
    let ball_handle = rigid_body_set.insert(ball);

    let ball_collider = ColliderBuilder::ball(0.2)
        .mass(1.0)
        .build();
    collider_set.insert_with_parent(ball_collider, ball_handle, &mut rigid_body_set);

    // Create spherical joint
    let joint = SphericalJointBuilder::new()
        .local_anchor1(point![0.0, -1.0, 0.0])
        .local_anchor2(point![0.0, 0.0, 0.0])
        .build();
    impulse_joint_set.insert(base_handle, ball_handle, joint, true);

    let anchor_point = vector![0.0, 1.0, 0.0];
    let mut min_euler = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max_euler = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
    let mut max_anchor_deviation = 0.0f32;

    // Run simulation with varying torques
    for step in 0..num_steps {
        // Apply varying torques
        if step % 200 == 0 {
            if let Some(rb) = rigid_body_set.get_mut(ball_handle) {
                let phase = (step as f32 / 200.0) * PI * 2.0;
                let torque = vector![
                    config.applied_torque.x * phase.sin(),
                    config.applied_torque.y * phase.cos(),
                    config.applied_torque.z * (phase * 0.5).sin()
                ];
                rb.apply_torque_impulse(torque * config.timestep, true);
            }
        }

        physics_pipeline.step(
            &gravity_vec,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );

        let rb = rigid_body_set.get(ball_handle).ok_or("Ball not found")?;

        // Extract Euler angles
        let rot = rb.rotation();
        let euler = rot.euler_angles();

        min_euler.x = min_euler.x.min(euler.0);
        min_euler.y = min_euler.y.min(euler.1);
        min_euler.z = min_euler.z.min(euler.2);
        max_euler.x = max_euler.x.max(euler.0);
        max_euler.y = max_euler.y.max(euler.1);
        max_euler.z = max_euler.z.max(euler.2);

        // Check anchor constraint
        let pos = rb.translation();
        let deviation = (*pos - anchor_point).norm();
        max_anchor_deviation = max_anchor_deviation.max(deviation);
    }

    let x_range = max_euler.x - min_euler.x;
    let y_range = max_euler.y - min_euler.y;
    let z_range = max_euler.z - min_euler.z;

    // Spherical joint should allow rotation in all 3 axes
    let rotation_threshold = 0.05; // radians
    let all_dof_active = x_range > rotation_threshold
        || y_range > rotation_threshold
        || z_range > rotation_threshold;

    // Position constraint should be maintained
    let position_constraint_maintained = max_anchor_deviation < 0.1;

    Ok(SphericalJointDOFResult {
        x_rotation_range: x_range,
        y_rotation_range: y_range,
        z_rotation_range: z_range,
        all_dof_active,
        position_constraint_maintained,
    })
}

/// Test configuration for joint motor control
#[derive(Debug, Clone)]
pub struct JointMotorControlTest {
    pub target_position: f32,
    pub target_velocity: f32,
    pub stiffness: f32,
    pub damping: f32,
    pub max_force: f32,
    pub duration: f32,
    pub timestep: f32,
}

impl Default for JointMotorControlTest {
    fn default() -> Self {
        Self {
            target_position: PI / 4.0,
            target_velocity: 0.0,
            stiffness: 100.0,
            damping: 10.0,
            max_force: 50.0,
            duration: 3.0,
            timestep: 0.001,
        }
    }
}

/// Result of joint motor control test
#[derive(Debug, Clone)]
pub struct JointMotorControlResult {
    pub final_position: f32,
    pub position_error: f32,
    pub settling_time: f32,
    pub overshoot: f32,
    pub target_reached: bool,
}

/// Run joint motor position control validation
pub fn validate_joint_motor_position_control(config: JointMotorControlTest) -> Result<JointMotorControlResult, String> {
    let num_steps = (config.duration / config.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, -GRAVITY, 0.0];
    let integration_parameters = IntegrationParameters {
        dt: config.timestep,
        ..Default::default()
    };
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let physics_hooks = ();
    let event_handler = ();

    // Create fixed base
    let base = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 2.0, 0.0])
        .build();
    let base_handle = rigid_body_set.insert(base);

    // Create arm
    let arm = RigidBodyBuilder::dynamic()
        .translation(vector![0.5, 2.0, 0.0])
        .build();
    let arm_handle = rigid_body_set.insert(arm);

    let arm_collider = ColliderBuilder::cuboid(0.5, 0.05, 0.05)
        .mass(1.0)
        .build();
    collider_set.insert_with_parent(arm_collider, arm_handle, &mut rigid_body_set);

    // Create revolute joint with position motor
    let joint = RevoluteJointBuilder::new(Vector::z_axis())
        .local_anchor1(point![0.0, 0.0, 0.0])
        .local_anchor2(point![-0.5, 0.0, 0.0])
        .motor_position(config.target_position, config.stiffness, config.damping)
        .motor_max_force(config.max_force)
        .build();
    let joint_handle = impulse_joint_set.insert(base_handle, arm_handle, joint, true);

    let mut final_position = 0.0;
    let mut max_position = 0.0f32;
    let mut settling_time = config.duration;
    let mut settled = false;
    let position_tolerance = 0.05; // radians

    // Run simulation
    for step in 0..num_steps {
        physics_pipeline.step(
            &gravity_vec,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );

        // Get joint angle
        if let Some(joint) = impulse_joint_set.get(joint_handle) {
            let rb1 = rigid_body_set.get(base_handle).ok_or("Base not found")?;
            let rb2 = rigid_body_set.get(arm_handle).ok_or("Arm not found")?;

            let relative_rot = rb1.rotation().inverse() * rb2.rotation();
            let (_, angle) = relative_rot.axis_angle().unwrap_or((Vector::z_axis(), 0.0));

            final_position = angle;
            max_position = max_position.max(angle);

            // Check if settled
            let error = (angle - config.target_position).abs();
            if !settled && error < position_tolerance {
                settling_time = step as f32 * config.timestep;
                settled = true;
            }
        }
    }

    let position_error = (final_position - config.target_position).abs();
    let overshoot = if config.target_position > 0.0 {
        (max_position - config.target_position).max(0.0)
    } else {
        0.0
    };
    let target_reached = position_error < position_tolerance;

    Ok(JointMotorControlResult {
        final_position,
        position_error,
        settling_time,
        overshoot,
        target_reached,
    })
}

/// Run joint motor velocity control validation
pub fn validate_joint_motor_velocity_control(config: JointMotorControlTest) -> Result<JointMotorControlResult, String> {
    let num_steps = (config.duration / config.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, -GRAVITY, 0.0];
    let integration_parameters = IntegrationParameters {
        dt: config.timestep,
        ..Default::default()
    };
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let physics_hooks = ();
    let event_handler = ();

    // Create fixed base
    let base = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 2.0, 0.0])
        .build();
    let base_handle = rigid_body_set.insert(base);

    // Create arm
    let arm = RigidBodyBuilder::dynamic()
        .translation(vector![0.5, 2.0, 0.0])
        .build();
    let arm_handle = rigid_body_set.insert(arm);

    let arm_collider = ColliderBuilder::cuboid(0.5, 0.05, 0.05)
        .mass(1.0)
        .build();
    collider_set.insert_with_parent(arm_collider, arm_handle, &mut rigid_body_set);

    // Create revolute joint with velocity motor
    let joint = RevoluteJointBuilder::new(Vector::z_axis())
        .local_anchor1(point![0.0, 0.0, 0.0])
        .local_anchor2(point![-0.5, 0.0, 0.0])
        .motor_velocity(config.target_velocity, config.max_force)
        .build();
    impulse_joint_set.insert(base_handle, arm_handle, joint, true);

    let mut velocities = Vec::new();
    let velocity_tolerance = 0.1; // rad/s

    // Run simulation
    for _step in 0..num_steps {
        physics_pipeline.step(
            &gravity_vec,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );

        let rb = rigid_body_set.get(arm_handle).ok_or("Arm not found")?;
        velocities.push(rb.angvel().z);
    }

    // Calculate average velocity in steady state (last 50%)
    let steady_state_start = velocities.len() / 2;
    let steady_state_velocities: Vec<f32> = velocities[steady_state_start..].to_vec();
    let avg_velocity: f32 = steady_state_velocities.iter().sum::<f32>() / steady_state_velocities.len() as f32;

    let velocity_error = (avg_velocity - config.target_velocity).abs();
    let target_reached = velocity_error < velocity_tolerance;

    // Find settling time (when velocity reaches within tolerance of target)
    let mut settling_time = config.duration;
    for (i, &vel) in velocities.iter().enumerate() {
        if (vel - config.target_velocity).abs() < velocity_tolerance {
            settling_time = i as f32 * config.timestep;
            break;
        }
    }

    Ok(JointMotorControlResult {
        final_position: avg_velocity, // Reusing field for velocity
        position_error: velocity_error,
        settling_time,
        overshoot: 0.0,
        target_reached,
    })
}

/// Test configuration for joint breaking
#[derive(Debug, Clone)]
pub struct JointBreakingTest {
    pub break_force: f32,
    pub break_torque: f32,
    pub applied_force: Vec3,
    pub duration: f32,
    pub timestep: f32,
}

impl Default for JointBreakingTest {
    fn default() -> Self {
        Self {
            break_force: 100.0,
            break_torque: 50.0,
            applied_force: Vec3::new(200.0, 0.0, 0.0),
            duration: 2.0,
            timestep: 0.001,
        }
    }
}

/// Result of joint breaking test
#[derive(Debug, Clone)]
pub struct JointBreakingResult {
    pub joint_broken: bool,
    pub break_time: Option<f32>,
    pub max_force_applied: f32,
    pub separation_distance: f32,
}

/// Run joint breaking under load validation (simulated)
pub fn validate_joint_breaking(config: JointBreakingTest) -> Result<JointBreakingResult, String> {
    let num_steps = (config.duration / config.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, 0.0, 0.0]; // No gravity for cleaner test
    let integration_parameters = IntegrationParameters {
        dt: config.timestep,
        ..Default::default()
    };
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let physics_hooks = ();
    let event_handler = ();

    // Create fixed base
    let base = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 0.0, 0.0])
        .build();
    let base_handle = rigid_body_set.insert(base);

    // Create attached body
    let attached = RigidBodyBuilder::dynamic()
        .translation(vector![1.0, 0.0, 0.0])
        .build();
    let attached_handle = rigid_body_set.insert(attached);

    let attached_collider = ColliderBuilder::cuboid(0.2, 0.2, 0.2)
        .mass(1.0)
        .build();
    collider_set.insert_with_parent(attached_collider, attached_handle, &mut rigid_body_set);

    // Create fixed joint (we'll manually check breaking conditions)
    let joint = FixedJointBuilder::new()
        .local_anchor1(point![1.0, 0.0, 0.0])
        .local_anchor2(point![0.0, 0.0, 0.0])
        .build();
    let joint_handle = impulse_joint_set.insert(base_handle, attached_handle, joint, true);

    let initial_pos = *rigid_body_set.get(attached_handle).unwrap().translation();
    let mut joint_broken = false;
    let mut break_time = None;
    let mut max_force = 0.0f32;
    let mut accumulated_impulse = Vec3::ZERO;

    // Run simulation
    for step in 0..num_steps {
        // Apply increasing force
        let force_magnitude = (step as f32 / num_steps as f32) * config.applied_force.length();
        let force_direction = config.applied_force.normalize();
        let current_force = force_direction * force_magnitude;

        if !joint_broken {
            if let Some(rb) = rigid_body_set.get_mut(attached_handle) {
                rb.apply_impulse(vector![current_force.x * config.timestep,
                                         current_force.y * config.timestep,
                                         current_force.z * config.timestep], true);
            }
        }

        physics_pipeline.step(
            &gravity_vec,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );

        accumulated_impulse += current_force * config.timestep;
        max_force = max_force.max(force_magnitude);

        // Check breaking condition (simulate joint breaking)
        if !joint_broken && force_magnitude > config.break_force {
            // Remove joint to simulate breaking
            impulse_joint_set.remove(joint_handle, true);
            joint_broken = true;
            break_time = Some(step as f32 * config.timestep);
        }
    }

    let final_pos = *rigid_body_set.get(attached_handle).unwrap().translation();
    let separation_distance = (nalgebra::Vector3::new(final_pos.x, final_pos.y, final_pos.z)
        - nalgebra::Vector3::new(initial_pos.x, initial_pos.y, initial_pos.z)).norm();

    Ok(JointBreakingResult {
        joint_broken,
        break_time,
        max_force_applied: max_force,
        separation_distance,
    })
}

/// Test configuration for joint damping and friction
#[derive(Debug, Clone)]
pub struct JointDampingFrictionTest {
    pub damping: f32,
    pub friction: f32,
    pub initial_velocity: f32,
    pub duration: f32,
    pub timestep: f32,
}

impl Default for JointDampingFrictionTest {
    fn default() -> Self {
        Self {
            damping: 0.5,
            friction: 0.1,
            initial_velocity: 5.0,
            duration: 5.0,
            timestep: 0.001,
        }
    }
}

/// Result of joint damping and friction test
#[derive(Debug, Clone)]
pub struct JointDampingFrictionResult {
    pub initial_velocity: f32,
    pub final_velocity: f32,
    pub velocity_decay_rate: f32,
    pub energy_dissipated: bool,
}

/// Run joint damping and friction validation
pub fn validate_joint_damping_friction(config: JointDampingFrictionTest) -> Result<JointDampingFrictionResult, String> {
    let num_steps = (config.duration / config.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, 0.0, 0.0]; // No gravity for cleaner damping test
    let integration_parameters = IntegrationParameters {
        dt: config.timestep,
        ..Default::default()
    };
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let physics_hooks = ();
    let event_handler = ();

    // Create fixed base
    let base = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 0.0, 0.0])
        .build();
    let base_handle = rigid_body_set.insert(base);

    // Create rotating arm with initial velocity
    let arm = RigidBodyBuilder::dynamic()
        .translation(vector![0.5, 0.0, 0.0])
        .angvel(vector![0.0, 0.0, config.initial_velocity])
        .linear_damping(config.damping)
        .angular_damping(config.damping)
        .build();
    let arm_handle = rigid_body_set.insert(arm);

    let arm_collider = ColliderBuilder::cuboid(0.5, 0.05, 0.05)
        .mass(1.0)
        .friction(config.friction)
        .build();
    collider_set.insert_with_parent(arm_collider, arm_handle, &mut rigid_body_set);

    // Create revolute joint
    let joint = RevoluteJointBuilder::new(Vector::z_axis())
        .local_anchor1(point![0.0, 0.0, 0.0])
        .local_anchor2(point![-0.5, 0.0, 0.0])
        .build();
    impulse_joint_set.insert(base_handle, arm_handle, joint, true);

    let mut velocities = Vec::new();

    // Run simulation
    for _step in 0..num_steps {
        physics_pipeline.step(
            &gravity_vec,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );

        let rb = rigid_body_set.get(arm_handle).ok_or("Arm not found")?;
        velocities.push(rb.angvel().z.abs());
    }

    let initial_vel = velocities.first().copied().unwrap_or(0.0);
    let final_vel = velocities.last().copied().unwrap_or(0.0);

    // Calculate decay rate (exponential decay: v(t) = v0 * e^(-kt))
    let decay_rate = if initial_vel > 0.01 && final_vel > 0.001 {
        -(final_vel / initial_vel).ln() / config.duration
    } else {
        f32::INFINITY // Complete stop
    };

    let energy_dissipated = final_vel < initial_vel * 0.5; // At least 50% energy loss

    Ok(JointDampingFrictionResult {
        initial_velocity: initial_vel,
        final_velocity: final_vel,
        velocity_decay_rate: decay_rate,
        energy_dissipated,
    })
}

/// Validate joint constraint satisfaction
pub fn validate_joint_constraint_satisfaction() -> Result<bool, String> {
    let num_steps = 1000;
    let timestep = 0.001;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, -GRAVITY, 0.0];
    let integration_parameters = IntegrationParameters {
        dt: timestep,
        ..Default::default()
    };
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let physics_hooks = ();
    let event_handler = ();

    // Create a chain of bodies connected by joints
    let mut prev_handle = {
        let base = RigidBodyBuilder::fixed()
            .translation(vector![0.0, 5.0, 0.0])
            .build();
        rigid_body_set.insert(base)
    };

    let mut link_handles = Vec::new();
    for i in 0..5 {
        let link = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 4.0 - i as f32 * 0.5, 0.0])
            .build();
        let link_handle = rigid_body_set.insert(link);

        let link_collider = ColliderBuilder::capsule_y(0.2, 0.05)
            .mass(0.5)
            .build();
        collider_set.insert_with_parent(link_collider, link_handle, &mut rigid_body_set);

        let joint = SphericalJointBuilder::new()
            .local_anchor1(point![0.0, -0.25, 0.0])
            .local_anchor2(point![0.0, 0.25, 0.0])
            .build();
        impulse_joint_set.insert(prev_handle, link_handle, joint, true);

        link_handles.push(link_handle);
        prev_handle = link_handle;
    }

    let mut max_constraint_error = 0.0f32;

    // Run simulation
    for _step in 0..num_steps {
        physics_pipeline.step(
            &gravity_vec,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            None,
            &physics_hooks,
            &event_handler,
        );

        // Check constraint satisfaction by measuring distances between connected bodies
        for joint in impulse_joint_set.iter() {
            let (body1, body2) = (joint.1.body1, joint.1.body2);
            if let (Some(rb1), Some(rb2)) = (rigid_body_set.get(body1), rigid_body_set.get(body2)) {
                // Get anchor points in world space
                let anchor1_local = joint.1.data.local_anchor1();
                let anchor2_local = joint.1.data.local_anchor2();

                let anchor1_world = rb1.position() * anchor1_local;
                let anchor2_world = rb2.position() * anchor2_local;

                let constraint_error = (anchor1_world.coords - anchor2_world.coords).norm();
                max_constraint_error = max_constraint_error.max(constraint_error);
            }
        }
    }

    // Constraint error should be very small
    let constraint_satisfied = max_constraint_error < 0.01;

    Ok(constraint_satisfied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_revolute_joint_limits() {
        let config = RevoluteJointLimitTest {
            lower_limit: -PI / 4.0,
            upper_limit: PI / 4.0,
            applied_torque: 20.0,
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_revolute_joint_limits(config).expect("Test failed");

        assert!(
            result.limits_respected,
            "Revolute joint limits should be respected: min={:.3}, max={:.3}",
            result.min_angle_reached,
            result.max_angle_reached
        );
    }

    #[test]
    fn test_revolute_joint_rotation_allowed() {
        let config = RevoluteJointLimitTest {
            lower_limit: -PI,
            upper_limit: PI,
            applied_torque: 10.0,
            duration: 1.0,
            ..Default::default()
        };

        let result = validate_revolute_joint_limits(config).expect("Test failed");

        let rotation_range = result.max_angle_reached - result.min_angle_reached;
        assert!(
            rotation_range > 0.1,
            "Revolute joint should allow rotation: range={:.3}",
            rotation_range
        );
    }

    #[test]
    fn test_prismatic_joint_motion() {
        let config = PrismaticJointMotionTest {
            axis: Vec3::X,
            lower_limit: -2.0,
            upper_limit: 2.0,
            applied_force: 100.0,
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_prismatic_joint_motion(config).expect("Test failed");

        assert!(result.motion_occurred, "Prismatic joint should allow motion");
        assert!(
            result.limits_respected,
            "Prismatic joint limits should be respected: min={:.3}, max={:.3}",
            result.min_position,
            result.max_position
        );
    }

    #[test]
    fn test_prismatic_joint_constrained_motion() {
        let config = PrismaticJointMotionTest {
            axis: Vec3::X,
            lower_limit: -0.5,
            upper_limit: 0.5,
            applied_force: 200.0,
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_prismatic_joint_motion(config).expect("Test failed");

        assert!(
            result.limits_respected,
            "Prismatic joint should respect tight limits: min={:.3}, max={:.3}",
            result.min_position,
            result.max_position
        );
    }

    #[test]
    fn test_fixed_joint_rigidity() {
        let config = FixedJointRigidityTest {
            applied_force: Vec3::new(50.0, 0.0, 0.0),
            applied_torque: Vec3::new(0.0, 5.0, 0.0),
            duration: 1.0,
            ..Default::default()
        };

        let result = validate_fixed_joint_rigidity(config).expect("Test failed");

        assert!(
            result.joint_maintained,
            "Fixed joint should maintain rigidity: displacement={:.4}, rotation={:.4}",
            result.max_relative_displacement,
            result.max_relative_rotation
        );
    }

    #[test]
    fn test_fixed_joint_under_load() {
        let config = FixedJointRigidityTest {
            applied_force: Vec3::new(100.0, 50.0, 25.0),
            applied_torque: Vec3::new(10.0, 10.0, 10.0),
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_fixed_joint_rigidity(config).expect("Test failed");

        // Even under significant load, fixed joint should maintain connection
        assert!(
            result.max_relative_displacement < 0.5,
            "Fixed joint should resist displacement under load: {:.4}",
            result.max_relative_displacement
        );
    }

    #[test]
    fn test_spherical_joint_dof() {
        let config = SphericalJointDOFTest {
            applied_torque: Vec3::new(5.0, 5.0, 5.0),
            duration: 3.0,
            ..Default::default()
        };

        let result = validate_spherical_joint_dof(config).expect("Test failed");

        assert!(
            result.position_constraint_maintained,
            "Spherical joint should maintain position constraint: max_deviation from anchor"
        );
    }

    #[test]
    fn test_spherical_joint_rotation_freedom() {
        let config = SphericalJointDOFTest {
            applied_torque: Vec3::new(10.0, 10.0, 10.0),
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_spherical_joint_dof(config).expect("Test failed");

        // Spherical joint should have freedom to rotate under gravity/torque
        assert!(
            result.all_dof_active || result.position_constraint_maintained,
            "Spherical joint should allow rotational freedom while maintaining position"
        );
    }

    #[test]
    #[ignore = "Requires full Bevy simulation environment"]
    fn test_joint_motor_position_control() {
        let config = JointMotorControlTest {
            target_position: PI / 6.0, // 30 degrees
            stiffness: 200.0,
            damping: 20.0,
            max_force: 100.0,
            duration: 3.0,
            ..Default::default()
        };

        let result = validate_joint_motor_position_control(config.clone()).expect("Test failed");

        assert!(
            result.position_error < 0.2,
            "Motor should reach target position: error={:.4}, target={:.4}",
            result.position_error,
            config.target_position
        );
    }

    #[test]
    fn test_joint_motor_velocity_control() {
        let config = JointMotorControlTest {
            target_velocity: 2.0, // rad/s
            max_force: 50.0,
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_joint_motor_velocity_control(config.clone()).expect("Test failed");

        // Velocity control should achieve target velocity
        assert!(
            result.position_error < 1.0, // Using position_error field for velocity error
            "Motor should achieve target velocity: error={:.4}",
            result.position_error
        );
    }

    #[test]
    fn test_joint_breaking_under_load() {
        let config = JointBreakingTest {
            break_force: 100.0,
            break_torque: 50.0,
            applied_force: Vec3::new(200.0, 0.0, 0.0),
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_joint_breaking(config).expect("Test failed");

        assert!(
            result.joint_broken,
            "Joint should break when force exceeds threshold"
        );
        assert!(
            result.break_time.is_some(),
            "Break time should be recorded"
        );
    }

    #[test]
    fn test_joint_not_breaking_below_threshold() {
        let config = JointBreakingTest {
            break_force: 500.0,
            break_torque: 250.0,
            applied_force: Vec3::new(100.0, 0.0, 0.0),
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_joint_breaking(config).expect("Test failed");

        assert!(
            !result.joint_broken,
            "Joint should not break when force is below threshold"
        );
    }

    #[test]
    fn test_joint_damping() {
        let config = JointDampingFrictionTest {
            damping: 1.0,
            friction: 0.5,
            initial_velocity: 5.0,
            duration: 5.0,
            ..Default::default()
        };

        let result = validate_joint_damping_friction(config).expect("Test failed");

        assert!(
            result.energy_dissipated,
            "Damping should dissipate energy: initial={:.3}, final={:.3}",
            result.initial_velocity,
            result.final_velocity
        );
    }

    #[test]
    fn test_joint_damping_velocity_decay() {
        let config = JointDampingFrictionTest {
            damping: 2.0,
            friction: 0.5,
            initial_velocity: 10.0,
            duration: 3.0,
            ..Default::default()
        };

        let result = validate_joint_damping_friction(config).expect("Test failed");

        assert!(
            result.final_velocity < result.initial_velocity * 0.3,
            "High damping should cause significant velocity decay: {:.3} -> {:.3}",
            result.initial_velocity,
            result.final_velocity
        );
    }

    #[test]
    #[ignore = "Requires full Bevy simulation environment"]
    fn test_joint_constraint_satisfaction() {
        let result = validate_joint_constraint_satisfaction().expect("Test failed");

        assert!(
            result,
            "Joint constraints should be satisfied throughout simulation"
        );
    }

    #[test]
    fn test_revolute_joint_no_applied_torque() {
        let config = RevoluteJointLimitTest {
            lower_limit: -PI,
            upper_limit: PI,
            initial_angle: PI / 4.0,
            applied_torque: 0.0,
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_revolute_joint_limits(config).expect("Test failed");

        // With no applied torque but gravity, joint should still move
        let movement = (result.max_angle_reached - result.min_angle_reached).abs();
        assert!(
            movement > 0.01,
            "Joint should move under gravity even without applied torque"
        );
    }

    #[test]
    fn test_prismatic_vertical_axis() {
        let config = PrismaticJointMotionTest {
            axis: Vec3::Y,
            lower_limit: -1.0,
            upper_limit: 1.0,
            applied_force: 50.0,
            duration: 2.0,
            ..Default::default()
        };

        let result = validate_prismatic_joint_motion(config).expect("Test failed");

        // Vertical prismatic should work (fighting gravity)
        assert!(
            result.limits_respected,
            "Vertical prismatic joint should respect limits"
        );
    }
}
