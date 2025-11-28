use bevy::prelude::*;
use rand::Rng;

use crate::physics::rigid_body::RigidBodyComponent;
use crate::physics::world::PhysicsWorld;
use crate::rl::{
    Action, EpisodeInfo, Observation, RLTask, StepResult, TaskConfig, TaskParameters,
    TerminationReason,
};
use crate::robot::Robot;

/// Navigation task: Navigate to a goal position while avoiding obstacles
pub struct NavigationTask {
    config: TaskConfig,
    robot_entity: Option<Entity>,
    goal_position: Vec3,
    episode_info: EpisodeInfo,
    current_step: usize,
    goal_tolerance: f32,
    max_distance: f32,
    previous_distance: f32,
    min_distance_achieved: f32,
    collision_count: u32,
}

impl NavigationTask {
    pub fn new(obs_dim: usize, action_dim: usize) -> Self {
        let goal_tolerance = 0.3;
        let max_distance = 20.0;

        Self {
            config: TaskConfig {
                max_steps: 1000,
                dt: 0.02,
                obs_dim,
                action_dim,
                parameters: TaskParameters::Navigation {
                    goal_tolerance,
                    max_distance,
                },
            },
            robot_entity: None,
            goal_position: Vec3::ZERO,
            episode_info: EpisodeInfo::default(),
            current_step: 0,
            goal_tolerance,
            max_distance,
            previous_distance: 0.0,
            min_distance_achieved: f32::INFINITY,
            collision_count: 0,
        }
    }

    /// Sample a random goal position
    fn sample_goal(&mut self) {
        let mut rng = rand::thread_rng();
        let distance = rng.gen_range(5.0..15.0);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);

        self.goal_position = Vec3::new(distance * angle.cos(), 0.5, distance * angle.sin());
    }

    /// Find robot entity
    fn find_robot(&mut self, world: &mut World) {
        let mut query = world.query::<(Entity, &Robot)>();
        if let Some((entity, _robot)) = query.iter(world).next() {
            self.robot_entity = Some(entity);
        }
    }

    /// Get robot position
    fn get_robot_position(&self, world: &World) -> Option<Vec3> {
        if let Some(entity) = self.robot_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                return Some(transform.translation);
            }
        }
        None
    }

    /// Get robot orientation
    fn get_robot_orientation(&self, world: &World) -> Option<Quat> {
        if let Some(entity) = self.robot_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                return Some(transform.rotation);
            }
        }
        None
    }

    /// Get robot velocity
    fn get_robot_velocity(&self, world: &World) -> Vec3 {
        if let Some(entity) = self.robot_entity {
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

    /// Get robot angular velocity
    fn get_robot_angular_velocity(&self, world: &World) -> Vec3 {
        if let Some(entity) = self.robot_entity {
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

    /// Compute distance to goal
    fn distance_to_goal(&self, world: &World) -> f32 {
        if let Some(pos) = self.get_robot_position(world) {
            pos.distance(self.goal_position)
        } else {
            self.max_distance
        }
    }

    /// Check if goal is reached
    fn is_goal_reached(&self, world: &World) -> bool {
        self.distance_to_goal(world) < self.goal_tolerance
    }

    /// Get direction to goal in robot's local frame
    fn goal_direction_local(&self, world: &World) -> Vec3 {
        if let Some(pos) = self.get_robot_position(world) {
            if let Some(rot) = self.get_robot_orientation(world) {
                let global_dir = (self.goal_position - pos).normalize_or_zero();
                return rot.inverse() * global_dir;
            }
        }
        Vec3::ZERO
    }
}

impl RLTask for NavigationTask {
    fn config(&self) -> &TaskConfig {
        &self.config
    }

    fn reset(&mut self, world: &mut World) -> Observation {
        // Reset episode info
        self.episode_info = EpisodeInfo::default();
        self.current_step = 0;
        self.collision_count = 0;

        // Sample new goal
        self.sample_goal();

        // Find robot if not already found
        if self.robot_entity.is_none() {
            self.find_robot(world);
        }

        // Reset robot to origin
        if let Some(entity) = self.robot_entity {
            if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                transform.translation = Vec3::new(0.0, 0.5, 0.0);
                transform.rotation = Quat::IDENTITY;
            }
        }

        // Initialize distance tracking
        self.previous_distance = self.distance_to_goal(world);
        self.min_distance_achieved = self.previous_distance;

        self.get_observation(world)
    }

    fn step(&mut self, world: &mut World, action: &Action) -> StepResult {
        self.current_step += 1;

        // Apply action (typically linear and angular velocity commands)
        if let Action::Continuous(actions) = action {
            if let Some(entity) = self.robot_entity {
                if actions.len() >= 2 {
                    let linear_vel = actions[0].clamp(-1.0, 1.0) * 2.0; // Max 2 m/s
                    let angular_vel = actions[1].clamp(-1.0, 1.0) * 2.0; // Max 2 rad/s

                    if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                        let dt = self.config.dt;

                        // Update rotation
                        let rotation_delta = Quat::from_rotation_y(angular_vel * dt);
                        transform.rotation *= rotation_delta;

                        // Update position in forward direction
                        let forward = transform.rotation * Vec3::X;
                        transform.translation += forward * linear_vel * dt;
                    }
                }
            }
        }

        // Update distance tracking
        let current_distance = self.distance_to_goal(world);
        if current_distance < self.min_distance_achieved {
            self.min_distance_achieved = current_distance;
        }

        // Get observation, reward, and check termination
        let observation = self.get_observation(world);
        let reward = self.compute_reward(world);
        let done = self.is_done(world);
        let truncated = self.current_step >= self.config.max_steps;

        // Update episode info
        self.episode_info.total_reward += reward;
        self.episode_info.steps = self.current_step;

        if done {
            if self.is_goal_reached(world) {
                self.episode_info.success = true;
                self.episode_info.termination_reason = TerminationReason::Success;
            } else if current_distance > self.max_distance {
                self.episode_info.termination_reason = TerminationReason::OutOfBounds;
            }
        } else if truncated {
            self.episode_info.termination_reason = TerminationReason::MaxSteps;
        }

        // Update previous distance for next step
        self.previous_distance = current_distance;

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

        // Robot position (3D)
        if let Some(pos) = self.get_robot_position(world) {
            obs_data.extend_from_slice(&[pos.x, pos.y, pos.z]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0]);
        }

        // Robot orientation (quaternion, 4D)
        if let Some(rot) = self.get_robot_orientation(world) {
            obs_data.extend_from_slice(&[rot.x, rot.y, rot.z, rot.w]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0, 1.0]);
        }

        // Linear velocity (3D)
        let velocity = self.get_robot_velocity(world);
        obs_data.extend_from_slice(&[velocity.x, velocity.y, velocity.z]);

        // Angular velocity (3D)
        let ang_vel = self.get_robot_angular_velocity(world);
        obs_data.extend_from_slice(&[ang_vel.x, ang_vel.y, ang_vel.z]);

        // Goal position (3D)
        obs_data.extend_from_slice(&[
            self.goal_position.x,
            self.goal_position.y,
            self.goal_position.z,
        ]);

        // Distance to goal (1D)
        let distance = self.distance_to_goal(world);
        obs_data.push(distance);

        // Direction to goal in robot frame (3D)
        let goal_dir_local = self.goal_direction_local(world);
        obs_data.extend_from_slice(&[goal_dir_local.x, goal_dir_local.y, goal_dir_local.z]);

        // Normalized progress (1D)
        let progress = 1.0 - (distance / self.max_distance).min(1.0);
        obs_data.push(progress);

        Observation::new(obs_data)
    }

    fn compute_reward(&self, world: &mut World) -> f32 {
        let distance = self.distance_to_goal(world);

        // Dense reward for reducing distance
        let progress_reward = (self.previous_distance - distance) * 10.0;

        // Sparse reward for reaching goal
        let goal_reward = if self.is_goal_reached(world) {
            100.0
        } else {
            0.0
        };

        // Distance-based reward (encourages getting closer)
        let distance_reward = -distance * 0.1;

        // Forward velocity reward (encourage moving forward)
        let velocity = self.get_robot_velocity(world);
        let goal_dir = if let Some(pos) = self.get_robot_position(world) {
            (self.goal_position - pos).normalize_or_zero()
        } else {
            Vec3::ZERO
        };
        let forward_reward = velocity.dot(goal_dir) * 0.5;

        // Penalty for excessive rotation (encourage efficient paths)
        let ang_vel = self.get_robot_angular_velocity(world);
        let rotation_penalty = -ang_vel.y.abs() * 0.05;

        // Penalty for going out of bounds
        let bounds_penalty = if distance > self.max_distance {
            -50.0
        } else {
            0.0
        };

        // Time penalty (encourage faster completion)
        let time_penalty = -0.01;

        progress_reward
            + goal_reward
            + distance_reward
            + forward_reward
            + rotation_penalty
            + bounds_penalty
            + time_penalty
    }

    fn is_done(&self, world: &mut World) -> bool {
        // Success: reached goal
        if self.is_goal_reached(world) {
            return true;
        }

        // Failure: out of bounds
        let distance = self.distance_to_goal(world);
        if distance > self.max_distance {
            return true;
        }

        false
    }

    fn get_info(&self) -> EpisodeInfo {
        self.episode_info.clone()
    }

    fn render(&self, gizmos: &mut Gizmos, world: &mut World) {
        // Draw goal position as green sphere
        gizmos.sphere(
            Isometry3d::new(self.goal_position, Quat::IDENTITY),
            self.goal_tolerance,
            Color::srgb(0.0, 1.0, 0.0).with_alpha(0.6),
        );

        // Draw goal tolerance zone
        gizmos.sphere(
            Isometry3d::new(self.goal_position, Quat::IDENTITY),
            self.goal_tolerance,
            Color::srgb(0.0, 1.0, 0.0).with_alpha(0.2),
        );

        // Draw path from robot to goal
        if let Some(robot_pos) = self.get_robot_position(world) {
            gizmos.line(
                robot_pos,
                self.goal_position,
                Color::srgb(1.0, 1.0, 0.0).with_alpha(0.3),
            );

            // Draw robot heading
            if let Some(rot) = self.get_robot_orientation(world) {
                let forward = rot * Vec3::X;
                let heading_end = robot_pos + forward * 0.5;
                gizmos.arrow(robot_pos, heading_end, Color::srgb(1.0, 0.0, 0.0));
            }

            // Draw velocity vector
            let velocity = self.get_robot_velocity(world);
            if velocity.length() > 0.01 {
                let vel_end = robot_pos + velocity * 0.5;
                gizmos.arrow(robot_pos, vel_end, Color::srgb(0.0, 0.5, 1.0));
            }
        }

        // Draw maximum distance boundary
        let bounds_color = Color::srgb(1.0, 0.0, 0.0).with_alpha(0.1);
        gizmos.circle(
            Isometry3d::new(
                Vec3::new(0.0, 0.5, 0.0),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            self.max_distance,
            bounds_color,
        );

        // Draw distance text indicator
        let distance = self.distance_to_goal(world);
        if let Some(robot_pos) = self.get_robot_position(world) {
            let text_pos = robot_pos + Vec3::new(0.0, 1.0, 0.0);
            gizmos.sphere(
                Isometry3d::new(text_pos, Quat::IDENTITY),
                0.05,
                if distance < self.goal_tolerance {
                    Color::srgb(0.0, 1.0, 0.0)
                } else {
                    Color::srgb(1.0, 1.0, 0.0)
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_task_creation() {
        let task = NavigationTask::new(21, 2);
        assert_eq!(task.config.obs_dim, 21);
        assert_eq!(task.config.action_dim, 2);
        assert_eq!(task.current_step, 0);
    }

    #[test]
    fn test_goal_tolerance() {
        let task = NavigationTask::new(21, 2);
        assert_eq!(task.goal_tolerance, 0.3);
        assert_eq!(task.max_distance, 20.0);
    }

    #[test]
    fn test_observation_size() {
        let task = NavigationTask::new(21, 2);
        let mut world = World::new();
        let obs = task.get_observation(&mut world);

        // pos(3) + quat(4) + vel(3) + ang_vel(3) + goal(3) + dist(1) + goal_dir(3) + progress(1) = 21
        assert_eq!(obs.len(), 21);
    }

    #[test]
    fn test_goal_sampling() {
        let mut task = NavigationTask::new(21, 2);
        task.sample_goal();

        // Check goal is within reasonable bounds
        let distance = task.goal_position.length();
        assert!(distance >= 5.0 && distance <= 15.0);
    }
}
