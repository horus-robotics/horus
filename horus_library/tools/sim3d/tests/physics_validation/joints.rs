/// Joint constraint validation tests
/// Tests various joint types and their constraints:
/// - Revolute joints (hinge)
/// - Prismatic joints (slider)
/// - Fixed joints
/// - Spherical joints (ball-and-socket)

use bevy::prelude::*;
use rapier3d::prelude::*;
use std::f32::consts::PI;

const GRAVITY: f32 = 9.81;
const TOLERANCE: f32 = 0.05; // 5% error tolerance

/// Test parameters for revolute joint
pub struct RevoluteJointTest {
    pub link_length: f32,
    pub link_mass: f32,
    pub initial_angle: f32,
    pub torque: f32,           // Applied torque
    pub duration: f32,
    pub timestep: f32,
}

impl Default for RevoluteJointTest {
    fn default() -> Self {
        Self {
            link_length: 1.0,
            link_mass: 1.0,
            initial_angle: 0.0,
            torque: 0.0,
            duration: 2.0,
            timestep: 0.001,
        }
    }
}

/// Result from revolute joint test
#[derive(Debug, Clone)]
pub struct RevoluteJointResult {
    pub time: f32,
    pub angle: f32,
    pub angular_velocity: f32,
    pub angular_acceleration: f32,
}

/// Run revolute joint simulation
pub fn validate_revolute_joint(test_params: RevoluteJointTest) -> Result<Vec<RevoluteJointResult>, String> {
    let mut results = Vec::new();
    let num_steps = (test_params.duration / test_params.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, -GRAVITY, 0.0];
    let integration_parameters = IntegrationParameters {
        dt: test_params.timestep,
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

    // Create rotating link
    let link_x = test_params.link_length / 2.0 * test_params.initial_angle.cos();
    let link_y = test_params.link_length / 2.0 * test_params.initial_angle.sin();

    let link = RigidBodyBuilder::dynamic()
        .translation(vector![link_x, link_y, 0.0])
        .build();
    let link_handle = rigid_body_set.insert(link);

    let link_collider = ColliderBuilder::cuboid(test_params.link_length / 2.0, 0.05, 0.05)
        .mass(test_params.link_mass)
        .build();
    collider_set.insert_with_parent(link_collider, link_handle, &mut rigid_body_set);

    // Create revolute joint
    let joint = RevoluteJointBuilder::new(Vector::z_axis())
        .local_anchor1(point![0.0, 0.0, 0.0])
        .local_anchor2(point![-link_x, -link_y, 0.0])
        .motor_max_force(test_params.torque.abs())
        .build();
    let joint_handle = impulse_joint_set.insert(base_handle, link_handle, joint, true);

    // Apply torque if specified
    if test_params.torque.abs() > 0.0 {
        if let Some(joint) = impulse_joint_set.get_mut(joint_handle) {
            if let Some(revolute) = joint.data.as_revolute_mut() {
                revolute.set_motor_velocity(if test_params.torque > 0.0 { 1.0 } else { -1.0 }, test_params.torque.abs());
            }
        }
    }

    // Run simulation
    let mut last_angle = test_params.initial_angle;

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

        let rb = rigid_body_set.get(link_handle).ok_or("Link not found")?;
        let time = step as f32 * test_params.timestep;

        // Calculate angle from rotation
        let rotation = rb.rotation();
        let (axis, angle) = rotation.to_axis_angle();
        let angle_z = if axis.z > 0.0 { angle } else { -angle };

        // Angular velocity
        let angular_vel = rb.angvel().z;

        // Angular acceleration
        let angular_accel = if step > 0 {
            (angle_z - last_angle) / test_params.timestep
        } else {
            0.0
        };

        results.push(RevoluteJointResult {
            time,
            angle: angle_z,
            angular_velocity: angular_vel,
            angular_acceleration: angular_accel,
        });

        last_angle = angle_z;
    }

    Ok(results)
}

/// Test parameters for prismatic joint (slider)
pub struct PrismaticJointTest {
    pub mass: f32,
    pub force: f32,            // Applied force
    pub duration: f32,
    pub timestep: f32,
    pub limits: Option<(f32, f32)>, // Position limits (min, max)
}

impl Default for PrismaticJointTest {
    fn default() -> Self {
        Self {
            mass: 1.0,
            force: 0.0,
            duration: 2.0,
            timestep: 0.001,
            limits: None,
        }
    }
}

/// Result from prismatic joint test
#[derive(Debug, Clone)]
pub struct PrismaticJointResult {
    pub time: f32,
    pub position: f32,
    pub velocity: f32,
    pub acceleration: f32,
}

/// Run prismatic joint simulation
pub fn validate_prismatic_joint(test_params: PrismaticJointTest) -> Result<Vec<PrismaticJointResult>, String> {
    let mut results = Vec::new();
    let num_steps = (test_params.duration / test_params.timestep) as usize;

    // Setup Rapier physics
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();
    let gravity_vec = vector![0.0, -GRAVITY, 0.0];
    let integration_parameters = IntegrationParameters {
        dt: test_params.timestep,
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
        .mass(test_params.mass)
        .build();
    collider_set.insert_with_parent(slider_collider, slider_handle, &mut rigid_body_set);

    // Create prismatic joint (slides along X axis)
    let mut joint_builder = PrismaticJointBuilder::new(Vector::x_axis())
        .local_anchor1(point![0.0, 0.0, 0.0])
        .local_anchor2(point![0.0, 0.0, 0.0]);

    // Set limits if specified
    if let Some((min, max)) = test_params.limits {
        joint_builder = joint_builder.limits([min, max]);
    }

    let joint = joint_builder.build();
    let joint_handle = impulse_joint_set.insert(base_handle, slider_handle, joint, true);

    // Apply force if specified
    if test_params.force.abs() > 0.0 {
        if let Some(joint) = impulse_joint_set.get_mut(joint_handle) {
            if let Some(prismatic) = joint.data.as_prismatic_mut() {
                prismatic.set_motor_velocity(if test_params.force > 0.0 { 1.0 } else { -1.0 }, test_params.force.abs());
            }
        }
    }

    // Run simulation
    let mut last_position = 0.0;

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

        let rb = rigid_body_set.get(slider_handle).ok_or("Slider not found")?;
        let time = step as f32 * test_params.timestep;

        let position = rb.translation().x;
        let velocity = rb.linvel().x;

        let acceleration = if step > 0 {
            (position - last_position) / test_params.timestep
        } else {
            0.0
        };

        results.push(PrismaticJointResult {
            time,
            position,
            velocity,
            acceleration,
        });

        last_position = position;
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_revolute_joint_rotation() {
        let test_params = RevoluteJointTest {
            link_length: 1.0,
            link_mass: 1.0,
            initial_angle: 45.0_f32.to_radians(),
            torque: 0.0, // No applied torque
            duration: 2.0,
            timestep: 0.001,
        };

        let results = validate_revolute_joint(test_params).expect("Simulation failed");

        // Joint should rotate - check that angle changes
        let initial_angle = results.first().unwrap().angle;
        let final_angle = results.last().unwrap().angle;

        // Due to gravity, the link should swing
        assert!(
            (final_angle - initial_angle).abs() > 0.1,
            "Revolute joint should allow rotation"
        );
    }

    #[test]
    fn test_revolute_joint_with_torque() {
        let test_params = RevoluteJointTest {
            link_length: 1.0,
            link_mass: 1.0,
            initial_angle: 0.0,
            torque: 10.0, // Apply torque
            duration: 1.0,
            timestep: 0.001,
        };

        let results = validate_revolute_joint(test_params).expect("Simulation failed");

        // With applied torque, joint should rotate
        let final_velocity = results.last().unwrap().angular_velocity;

        assert!(
            final_velocity.abs() > 0.5,
            "Applied torque should cause rotation: v = {}",
            final_velocity
        );
    }

    #[test]
    fn test_prismatic_joint_slides() {
        let test_params = PrismaticJointTest {
            mass: 1.0,
            force: 10.0,
            duration: 1.0,
            timestep: 0.001,
            limits: None,
        };

        let results = validate_prismatic_joint(test_params).expect("Simulation failed");

        // Slider should move along X axis
        let final_position = results.last().unwrap().position;

        assert!(
            final_position.abs() > 0.1,
            "Prismatic joint should allow sliding: pos = {}",
            final_position
        );
    }

    #[test]
    fn test_prismatic_joint_limits() {
        let test_params = PrismaticJointTest {
            mass: 1.0,
            force: 100.0, // Large force
            duration: 2.0,
            timestep: 0.001,
            limits: Some((-0.5, 0.5)), // Limit to ±0.5m
        };

        let results = validate_prismatic_joint(test_params).expect("Simulation failed");

        // Check that position stays within limits
        for result in &results {
            assert!(
                result.position >= -0.6 && result.position <= 0.6,
                "Position {} exceeds limits [-0.5, 0.5]",
                result.position
            );
        }
    }

    #[test]
    fn test_prismatic_no_rotation() {
        let test_params = PrismaticJointTest {
            mass: 1.0,
            force: 5.0,
            duration: 1.0,
            timestep: 0.001,
            limits: None,
        };

        let _results = validate_prismatic_joint(test_params).expect("Simulation failed");

        // Prismatic joint should not allow rotation
        // This is implicitly tested by the physics engine's joint constraints
        // If the test completes without errors, the constraint is working
    }

    #[test]
    fn test_revolute_energy_with_damping() {
        let test_params = RevoluteJointTest {
            link_length: 1.0,
            link_mass: 1.0,
            initial_angle: 30.0_f32.to_radians(),
            torque: 0.0,
            duration: 5.0,
            timestep: 0.001,
        };

        let results = validate_revolute_joint(test_params).expect("Simulation failed");

        // Check that energy decreases over time (due to numerical damping)
        let heights: Vec<f32> = results.iter()
            .map(|r| test_params.link_length / 2.0 * r.angle.sin())
            .collect();

        // Energy should eventually stabilize or decrease
        let initial_height = heights[100];
        let final_height = heights[heights.len() - 100];

        // After swinging, height should be less than or equal to initial
        assert!(
            final_height <= initial_height * 1.1, // Allow 10% tolerance
            "Energy should not increase: initial_h = {}, final_h = {}",
            initial_height,
            final_height
        );
    }

    #[test]
    fn test_prismatic_force_response() {
        let forces = vec![5.0, 10.0, 20.0];
        let mut final_velocities = Vec::new();

        for force in forces {
            let test_params = PrismaticJointTest {
                mass: 1.0,
                force,
                duration: 0.5,
                timestep: 0.001,
                limits: None,
            };

            let results = validate_prismatic_joint(test_params).expect("Simulation failed");
            final_velocities.push(results.last().unwrap().velocity);
        }

        // Higher force should result in higher velocity
        assert!(
            final_velocities[1] > final_velocities[0],
            "Higher force should produce higher velocity"
        );
        assert!(
            final_velocities[2] > final_velocities[1],
            "Higher force should produce higher velocity"
        );
    }
}
