use bevy::prelude::*;
use rand::Rng;

use crate::physics::rigid_body::RigidBodyComponent;
use crate::rl::{
    Action, EpisodeInfo, Observation, RLTask, StepResult, TaskConfig, TaskParameters,
    TerminationReason,
};
use crate::robot::Robot;

/// Reaching task: Robot must reach a target position in 3D space
pub struct ReachingTask {
    config: TaskConfig,
    target_position: Vec3,
    end_effector_entity: Option<Entity>,
    episode_info: EpisodeInfo,
    current_step: usize,
    initial_distance: f32,
    target_tolerance: f32,
    max_distance: f32,
}

impl ReachingTask {
    pub fn new(obs_dim: usize, action_dim: usize) -> Self {
        let target_tolerance = 0.05;
        let max_distance = 10.0;

        Self {
            config: TaskConfig {
                max_steps: 500,
                dt: 0.02,
                obs_dim,
                action_dim,
                parameters: TaskParameters::Reaching {
                    target_tolerance,
                    max_distance,
                },
            },
            target_position: Vec3::ZERO,
            end_effector_entity: None,
            episode_info: EpisodeInfo::default(),
            current_step: 0,
            initial_distance: 0.0,
            target_tolerance,
            max_distance,
        }
    }

    /// Sample a random target position within workspace
    fn sample_target(&mut self) {
        let mut rng = rand::thread_rng();
        self.target_position = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.3..1.5),
            rng.gen_range(-1.0..1.0),
        );
    }

    /// Find the end-effector entity (last link in robot chain)
    fn find_end_effector(&mut self, world: &mut World) {
        // Query for robot and its rigid bodies
        let mut query = world.query::<(Entity, &Robot, &RigidBodyComponent, &Transform)>();

        // Find the entity with the highest Y position (typically the end-effector)
        let mut max_y = f32::NEG_INFINITY;
        let mut end_effector = None;

        for (entity, _robot, _rb, transform) in query.iter(world) {
            if transform.translation.y > max_y {
                max_y = transform.translation.y;
                end_effector = Some(entity);
            }
        }

        self.end_effector_entity = end_effector;
    }

    /// Get current end-effector position
    fn get_end_effector_position(&self, world: &World) -> Option<Vec3> {
        if let Some(entity) = self.end_effector_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                return Some(transform.translation);
            }
        }
        None
    }

    /// Compute distance to target
    fn distance_to_target(&self, world: &World) -> f32 {
        if let Some(ee_pos) = self.get_end_effector_position(world) {
            ee_pos.distance(self.target_position)
        } else {
            self.max_distance
        }
    }
}

impl RLTask for ReachingTask {
    fn config(&self) -> &TaskConfig {
        &self.config
    }

    fn reset(&mut self, world: &mut World) -> Observation {
        // Reset episode info
        self.episode_info = EpisodeInfo::default();
        self.current_step = 0;

        // Sample new target
        self.sample_target();

        // Find end-effector if not already found
        if self.end_effector_entity.is_none() {
            self.find_end_effector(world);
        }

        // Reset robot to initial configuration
        let mut query = world.query::<(&Robot, &mut Transform, &RigidBodyComponent)>();
        for (_robot, mut transform, _rb) in query.iter_mut(world) {
            transform.translation = Vec3::new(0.0, 0.5, 0.0);
            transform.rotation = Quat::IDENTITY;
        }

        // Record initial distance
        self.initial_distance = self.distance_to_target(world);

        self.get_observation(world)
    }

    fn step(&mut self, world: &mut World, action: &Action) -> StepResult {
        self.current_step += 1;

        // Apply action (continuous joint torques/velocities)
        if let Action::Continuous(actions) = action {
            // Apply actions to robot joints
            let mut query = world.query::<(&Robot, &mut Transform, &RigidBodyComponent)>();
            for (i, (_robot, mut transform, _rb)) in query.iter_mut(world).enumerate() {
                if i < actions.len() {
                    // Simple proportional control for demonstration
                    let action_value = actions[i].clamp(-1.0, 1.0);

                    // Apply rotation based on action
                    let rotation_delta = Quat::from_rotation_y(action_value * 0.1);
                    transform.rotation = transform.rotation * rotation_delta;
                }
            }
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
            let distance = self.distance_to_target(world);
            if distance < self.target_tolerance {
                self.episode_info.success = true;
                self.episode_info.termination_reason = TerminationReason::Success;
            } else if distance > self.max_distance {
                self.episode_info.termination_reason = TerminationReason::OutOfBounds;
            }
        } else if truncated {
            self.episode_info.termination_reason = TerminationReason::MaxSteps;
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

        // End-effector position (3D)
        if let Some(ee_pos) = self.get_end_effector_position(world) {
            obs_data.extend_from_slice(&[ee_pos.x, ee_pos.y, ee_pos.z]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0]);
        }

        // Target position (3D)
        obs_data.extend_from_slice(&[
            self.target_position.x,
            self.target_position.y,
            self.target_position.z,
        ]);

        // Distance to target (1D)
        let distance = self.distance_to_target(world);
        obs_data.push(distance);

        // Normalized direction vector (3D)
        if let Some(ee_pos) = self.get_end_effector_position(world) {
            let direction = (self.target_position - ee_pos).normalize_or_zero();
            obs_data.extend_from_slice(&[direction.x, direction.y, direction.z]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0]);
        }

        Observation::new(obs_data)
    }

    fn compute_reward(&self, world: &mut World) -> f32 {
        let distance = self.distance_to_target(world);

        // Dense reward based on distance
        let distance_reward = -distance;

        // Bonus for reaching target
        let reach_bonus = if distance < self.target_tolerance {
            10.0
        } else {
            0.0
        };

        // Progress reward (compared to initial distance)
        let progress = (self.initial_distance - distance).max(0.0);
        let progress_reward = progress * 0.5;

        // Penalty for going out of bounds
        let bounds_penalty = if distance > self.max_distance {
            -10.0
        } else {
            0.0
        };

        distance_reward + reach_bonus + progress_reward + bounds_penalty
    }

    fn is_done(&self, world: &mut World) -> bool {
        let distance = self.distance_to_target(world);

        // Success: reached target
        if distance < self.target_tolerance {
            return true;
        }

        // Failure: out of bounds
        if distance > self.max_distance {
            return true;
        }

        false
    }

    fn get_info(&self) -> EpisodeInfo {
        self.episode_info.clone()
    }

    fn render(&self, gizmos: &mut Gizmos, world: &mut World) {
        // Draw target position as green sphere
        gizmos.sphere(
            Isometry3d::new(self.target_position, Quat::IDENTITY),
            self.target_tolerance,
            Color::srgb(0.0, 1.0, 0.0).with_alpha(0.5),
        );

        // Draw target tolerance zone
        gizmos.sphere(
            Isometry3d::new(self.target_position, Quat::IDENTITY),
            self.target_tolerance,
            Color::srgb(0.0, 1.0, 0.0).with_alpha(0.2),
        );

        // Draw line from end-effector to target
        if let Some(ee_pos) = self.get_end_effector_position(world) {
            gizmos.line(
                ee_pos,
                self.target_position,
                Color::srgb(1.0, 1.0, 0.0).with_alpha(0.5),
            );

            // Draw end-effector position as yellow sphere
            gizmos.sphere(
                Isometry3d::new(ee_pos, Quat::IDENTITY),
                0.03,
                Color::srgb(1.0, 1.0, 0.0),
            );
        }

        // Draw workspace bounds
        let bounds_color = Color::srgb(0.5, 0.5, 0.5).with_alpha(0.1);
        gizmos.sphere(
            Isometry3d::new(Vec3::ZERO, Quat::IDENTITY),
            self.max_distance,
            bounds_color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reaching_task_creation() {
        let task = ReachingTask::new(10, 6);
        assert_eq!(task.config.obs_dim, 10);
        assert_eq!(task.config.action_dim, 6);
        assert_eq!(task.current_step, 0);
    }

    #[test]
    fn test_target_sampling() {
        let mut task = ReachingTask::new(10, 6);
        task.sample_target();

        // Check target is within reasonable bounds
        assert!(task.target_position.x.abs() <= 1.0);
        assert!(task.target_position.y >= 0.3 && task.target_position.y <= 1.5);
        assert!(task.target_position.z.abs() <= 1.0);
    }

    #[test]
    fn test_observation_size() {
        let task = ReachingTask::new(10, 6);
        let mut world = World::new();
        let obs = task.get_observation(&mut world);

        // Should have: ee_pos(3) + target(3) + distance(1) + direction(3) = 10
        assert_eq!(obs.len(), 10);
    }
}
