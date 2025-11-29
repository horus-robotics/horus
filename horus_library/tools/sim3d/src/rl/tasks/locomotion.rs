use bevy::prelude::*;
use rand::Rng;

use crate::physics::rigid_body::RigidBodyComponent;
use crate::physics::world::PhysicsWorld;
use crate::rl::{
    Action, EpisodeInfo, Observation, RLTask, StepResult, TaskConfig, TaskParameters,
    TerminationReason,
};
use crate::robot::articulated::RobotJointCommands;
use crate::robot::state::{ArticulatedRobot, RobotJointStates};
use crate::robot::Robot;

/// Locomotion task: Learn to walk/run at a target velocity while staying upright
pub struct LocomotionTask {
    config: TaskConfig,
    robot_base_entity: Option<Entity>,
    /// Entity of the articulated robot (has RobotJointCommands)
    articulated_robot_entity: Option<Entity>,
    /// Names of joints to control (in action order)
    joint_names: Vec<String>,
    /// Maximum torque per joint (N·m for revolute, N for prismatic)
    max_torques: Vec<f32>,
    target_velocity: Vec3,
    episode_info: EpisodeInfo,
    current_step: usize,
    height_limit: f32,
    initial_height: f32,
    total_distance: f32,
}

impl LocomotionTask {
    /// Default maximum torque for joint motors (N·m)
    const DEFAULT_MAX_TORQUE: f32 = 50.0;

    pub fn new(obs_dim: usize, action_dim: usize) -> Self {
        let height_limit = 0.3; // Min height before falling

        Self {
            config: TaskConfig {
                max_steps: 1000,
                dt: 0.02,
                obs_dim,
                action_dim,
                parameters: TaskParameters::Locomotion {
                    target_velocity: Vec3::new(1.0, 0.0, 0.0),
                    height_limit,
                },
            },
            robot_base_entity: None,
            articulated_robot_entity: None,
            joint_names: Vec::new(),
            max_torques: Vec::new(),
            target_velocity: Vec3::new(1.0, 0.0, 0.0),
            episode_info: EpisodeInfo::default(),
            current_step: 0,
            height_limit,
            initial_height: 1.0,
            total_distance: 0.0,
        }
    }

    /// Configure joint names and maximum torques for the robot
    pub fn with_joints(mut self, joint_names: Vec<String>, max_torques: Option<Vec<f32>>) -> Self {
        let torques =
            max_torques.unwrap_or_else(|| vec![Self::DEFAULT_MAX_TORQUE; joint_names.len()]);
        self.joint_names = joint_names;
        self.max_torques = torques;
        self
    }

    /// Find robot base entity (torso/body) and articulated robot entity
    fn find_robot_base(&mut self, world: &mut World) {
        // Find articulated robot entity first
        let mut articulated_query = world.query::<(Entity, &ArticulatedRobot, &RobotJointStates)>();
        if let Some((entity, _robot, joint_states)) = articulated_query.iter(world).next() {
            self.articulated_robot_entity = Some(entity);
            // If joint names not configured, use joint order from robot
            if self.joint_names.is_empty() {
                self.joint_names = joint_states.joint_order.clone();
                self.max_torques = vec![Self::DEFAULT_MAX_TORQUE; self.joint_names.len()];
            }
        }

        // Find base entity (Robot component with Transform closest to initial height)
        let mut query = world.query::<(Entity, &Robot, &Transform)>();

        // Find entity closest to initial height (typically the torso)
        let mut best_diff = f32::INFINITY;
        let mut base = None;

        for (entity, _robot, transform) in query.iter(world) {
            let height_diff = (transform.translation.y - self.initial_height).abs();
            if height_diff < best_diff {
                best_diff = height_diff;
                base = Some(entity);
            }
        }

        self.robot_base_entity = base;
    }

    /// Get robot base transform
    fn get_base_transform(&self, world: &World) -> Option<Transform> {
        if let Some(entity) = self.robot_base_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                return Some(*transform);
            }
        }
        None
    }

    /// Get robot base velocity
    fn get_base_velocity(&self, world: &World) -> Vec3 {
        if let Some(entity) = self.robot_base_entity {
            if let Some(rb) = world.get::<RigidBodyComponent>(entity) {
                if let Some(physics_world) = world.get_resource::<PhysicsWorld>() {
                    if let Some(rigid_body) = physics_world.rigid_body_set.get(rb.handle) {
                        let linvel = rigid_body.linvel();
                        return Vec3::new(linvel.x, linvel.y, linvel.z);
                    }
                }
            }
        }
        Vec3::ZERO
    }

    /// Get robot base angular velocity
    fn get_base_angular_velocity(&self, world: &World) -> Vec3 {
        if let Some(entity) = self.robot_base_entity {
            if let Some(rb) = world.get::<RigidBodyComponent>(entity) {
                if let Some(physics_world) = world.get_resource::<PhysicsWorld>() {
                    if let Some(rigid_body) = physics_world.rigid_body_set.get(rb.handle) {
                        let angvel = rigid_body.angvel();
                        return Vec3::new(angvel.x, angvel.y, angvel.z);
                    }
                }
            }
        }
        Vec3::ZERO
    }

    /// Check if robot has fallen
    fn has_fallen(&self, world: &World) -> bool {
        if let Some(transform) = self.get_base_transform(world) {
            // Check height
            if transform.translation.y < self.height_limit {
                return true;
            }

            // Check orientation (tilted too much)
            let up = transform.rotation * Vec3::Y;
            let angle_from_vertical = up.angle_between(Vec3::Y);
            if angle_from_vertical > std::f32::consts::FRAC_PI_3 {
                // More than 60 degrees
                return true;
            }
        }
        false
    }

    /// Sample random target velocity
    fn sample_target_velocity(&mut self) {
        let mut rng = rand::thread_rng();
        let speed = rng.gen_range(0.5..2.0);
        let direction = rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI);

        self.target_velocity = Vec3::new(speed * direction.cos(), 0.0, speed * direction.sin());
    }
}

impl RLTask for LocomotionTask {
    fn config(&self) -> &TaskConfig {
        &self.config
    }

    fn reset(&mut self, world: &mut World) -> Observation {
        // Reset episode info
        self.episode_info = EpisodeInfo::default();
        self.current_step = 0;
        self.total_distance = 0.0;

        // Sample new target velocity
        self.sample_target_velocity();

        // Find robot base if not already found
        if self.robot_base_entity.is_none() {
            self.find_robot_base(world);
        }

        // Reset robot to upright position
        let mut query = world.query::<(&Robot, &mut Transform)>();
        for (_robot, mut transform) in query.iter_mut(world) {
            transform.translation.y = self.initial_height;
            transform.rotation = Quat::IDENTITY;
        }

        self.get_observation(world)
    }

    fn step(&mut self, world: &mut World, action: &Action) -> StepResult {
        self.current_step += 1;

        // Apply action (joint torques for locomotion) through proper joint motor system
        if let Action::Continuous(actions) = action {
            // Try to use articulated robot's joint command system (proper physics-based control)
            if let Some(robot_entity) = self.articulated_robot_entity {
                if let Some(mut commands) = world.get_mut::<RobotJointCommands>(robot_entity) {
                    // Apply actions as effort commands to joints
                    for (i, joint_name) in self.joint_names.iter().enumerate() {
                        if i < actions.len() {
                            // Scale action [-1, 1] to torque range [-max_torque, max_torque]
                            let action_value = actions[i].clamp(-1.0, 1.0);
                            let max_torque = self
                                .max_torques
                                .get(i)
                                .copied()
                                .unwrap_or(Self::DEFAULT_MAX_TORQUE);
                            let torque = action_value * max_torque;
                            commands.set_effort(joint_name, torque);
                        }
                    }
                }
            } else {
                // Fallback: Apply torques directly to rigid bodies through physics world
                // This handles cases where the robot doesn't have RobotJointCommands
                // First collect handles without borrowing world mutably
                let mut handles = Vec::new();
                {
                    let mut query = world.query::<(&Robot, &RigidBodyComponent)>();
                    for (_, rb) in query.iter(world).take(actions.len()) {
                        handles.push(rb.handle);
                    }
                }

                // Now apply torques through physics world
                if let Some(mut physics_world) = world.get_resource_mut::<PhysicsWorld>() {
                    for (i, handle) in handles.into_iter().enumerate() {
                        if i < actions.len() {
                            if let Some(body) = physics_world.rigid_body_set.get_mut(handle) {
                                let action_value = actions[i].clamp(-1.0, 1.0);
                                let max_torque = self
                                    .max_torques
                                    .get(i)
                                    .copied()
                                    .unwrap_or(Self::DEFAULT_MAX_TORQUE);
                                let torque = action_value * max_torque;

                                // Apply torque around the Y axis (vertical) for locomotion
                                body.apply_torque_impulse(
                                    rapier3d::prelude::Vector::new(
                                        0.0,
                                        torque * self.config.dt,
                                        0.0,
                                    ),
                                    true,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Track distance traveled
        let velocity = self.get_base_velocity(world);
        self.total_distance += velocity.length() * self.config.dt;

        // Get observation, reward, and check termination
        let observation = self.get_observation(world);
        let reward = self.compute_reward(world);
        let done = self.is_done(world);
        let truncated = self.current_step >= self.config.max_steps;

        // Update episode info
        self.episode_info.total_reward += reward;
        self.episode_info.steps = self.current_step;

        if done {
            if self.has_fallen(world) {
                self.episode_info.termination_reason = TerminationReason::Failure;
            }
        } else if truncated {
            self.episode_info.termination_reason = TerminationReason::MaxSteps;
            // Success if walked for full episode without falling
            self.episode_info.success = true;
        }

        StepResult {
            observation,
            reward,
            done,
            truncated,
            info: self.episode_info.clone(),
        }
    }

    fn get_observation(&self, world: &mut World) -> Observation {
        let mut obs_data = Vec::new();

        // Base position (3D)
        if let Some(transform) = self.get_base_transform(world) {
            obs_data.extend_from_slice(&[
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
            ]);
        } else {
            obs_data.extend_from_slice(&[0.0, self.initial_height, 0.0]);
        }

        // Base orientation (quaternion, 4D)
        if let Some(transform) = self.get_base_transform(world) {
            obs_data.extend_from_slice(&[
                transform.rotation.x,
                transform.rotation.y,
                transform.rotation.z,
                transform.rotation.w,
            ]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0, 1.0]);
        }

        // Linear velocity (3D)
        let velocity = self.get_base_velocity(world);
        obs_data.extend_from_slice(&[velocity.x, velocity.y, velocity.z]);

        // Angular velocity (3D)
        let ang_vel = self.get_base_angular_velocity(world);
        obs_data.extend_from_slice(&[ang_vel.x, ang_vel.y, ang_vel.z]);

        // Target velocity (3D)
        obs_data.extend_from_slice(&[
            self.target_velocity.x,
            self.target_velocity.y,
            self.target_velocity.z,
        ]);

        // Velocity error (3D)
        let vel_error = self.target_velocity - velocity;
        obs_data.extend_from_slice(&[vel_error.x, vel_error.y, vel_error.z]);

        // Joint positions and velocities (if articulated robot is available)
        if let Some(robot_entity) = self.articulated_robot_entity {
            if let Some(joint_states) = world.get::<RobotJointStates>(robot_entity) {
                // Add joint positions (normalized to [-1, 1] assuming ±π limits)
                let positions = joint_states.get_positions();
                for pos in &positions {
                    obs_data.push(pos / std::f32::consts::PI);
                }

                // Add joint velocities (normalized, assuming max velocity of 10 rad/s)
                let velocities = joint_states.get_velocities();
                for vel in &velocities {
                    obs_data.push(vel / 10.0);
                }
            }
        }

        Observation::new(obs_data)
    }

    fn compute_reward(&self, world: &mut World) -> f32 {
        let velocity = self.get_base_velocity(world);

        // Reward for matching target velocity
        let vel_error = (velocity - self.target_velocity).length();
        let velocity_reward = -vel_error;

        // Reward for staying upright
        let height_reward = if let Some(transform) = self.get_base_transform(world) {
            let height_diff = (transform.translation.y - self.initial_height).abs();
            -(height_diff * 2.0)
        } else {
            -5.0
        };

        // Orientation reward (stay upright)
        let orientation_reward = if let Some(transform) = self.get_base_transform(world) {
            let up = transform.rotation * Vec3::Y;
            up.dot(Vec3::Y) // Maximized when upright
        } else {
            0.0
        };

        // Forward progress reward
        let forward_reward = velocity.dot(self.target_velocity.normalize_or_zero()) * 0.1;

        // Penalty for excessive angular velocity (encourage stability)
        let ang_vel = self.get_base_angular_velocity(world);
        let stability_penalty = -ang_vel.length() * 0.01;

        // Heavy penalty for falling
        let fall_penalty = if self.has_fallen(world) { -10.0 } else { 0.0 };

        velocity_reward
            + height_reward
            + orientation_reward
            + forward_reward
            + stability_penalty
            + fall_penalty
    }

    fn is_done(&self, world: &mut World) -> bool {
        self.has_fallen(world)
    }

    fn get_info(&self) -> EpisodeInfo {
        self.episode_info.clone()
    }

    fn render(&self, gizmos: &mut Gizmos, world: &mut World) {
        // Draw target velocity vector
        if let Some(transform) = self.get_base_transform(world) {
            let start = transform.translation;
            let end = start + self.target_velocity;
            gizmos.arrow(start, end, Color::srgb(0.0, 1.0, 0.0));
        }

        // Draw current velocity vector
        if let Some(transform) = self.get_base_transform(world) {
            let velocity = self.get_base_velocity(world);
            let start = transform.translation;
            let end = start + velocity;
            gizmos.arrow(start, end, Color::srgb(0.0, 0.5, 1.0));
        }

        // Draw height limit plane
        let grid_size = 10.0;
        let limit_color = Color::srgb(1.0, 0.0, 0.0).with_alpha(0.1);
        gizmos.rect(
            Isometry3d::new(
                Vec3::new(0.0, self.height_limit, 0.0),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            Vec2::new(grid_size * 2.0, grid_size * 2.0),
            limit_color,
        );

        // Draw trajectory (path traveled)
        if let Some(transform) = self.get_base_transform(world) {
            gizmos.sphere(
                Isometry3d::new(transform.translation, Quat::IDENTITY),
                0.1,
                Color::srgb(1.0, 1.0, 0.0),
            );
        }

        // Draw orientation indicator
        if let Some(transform) = self.get_base_transform(world) {
            let forward = transform.rotation * Vec3::X;
            let forward_end = transform.translation + forward * 0.5;
            gizmos.arrow(
                transform.translation,
                forward_end,
                Color::srgb(1.0, 0.0, 0.0),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locomotion_task_creation() {
        let task = LocomotionTask::new(22, 12);
        assert_eq!(task.config.obs_dim, 22);
        assert_eq!(task.config.action_dim, 12);
        assert_eq!(task.current_step, 0);
    }

    #[test]
    fn test_target_velocity() {
        let task = LocomotionTask::new(22, 12);
        assert_eq!(task.target_velocity, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_observation_size() {
        let task = LocomotionTask::new(22, 12);
        let mut world = World::new();
        let obs = task.get_observation(&mut world);

        // pos(3) + quat(4) + vel(3) + ang_vel(3) + target_vel(3) + vel_error(3) = 19
        // Note: Actual size may vary based on implementation
        assert!(obs.len() >= 19 && obs.len() <= 22);
    }

    #[test]
    fn test_height_limit() {
        let task = LocomotionTask::new(22, 12);
        assert_eq!(task.height_limit, 0.3);
    }
}
