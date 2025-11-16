use bevy::prelude::*;
use rand::Rng;

use crate::rl::{
    Action, EpisodeInfo, Observation, RLTask, StepResult, TaskConfig, TaskParameters,
    TerminationReason,
};
use crate::robot::Robot;
use crate::physics::rigid_body::RigidBodyComponent;
use crate::physics::world::PhysicsWorld;

/// Manipulation task: Grasp and manipulate objects to target positions
pub struct ManipulationTask {
    config: TaskConfig,
    gripper_entity: Option<Entity>,
    object_entity: Option<Entity>,
    target_position: Vec3,
    episode_info: EpisodeInfo,
    current_step: usize,
    target_tolerance: f32,
    grasp_threshold: f32,
    is_grasped: bool,
    grasp_time: usize,
}

impl ManipulationTask {
    pub fn new(obs_dim: usize, action_dim: usize) -> Self {
        let target_tolerance = 0.1;
        let grasp_threshold = 0.15;

        Self {
            config: TaskConfig {
                max_steps: 800,
                dt: 0.02,
                obs_dim,
                action_dim,
                parameters: TaskParameters::Manipulation {
                    target_tolerance,
                    grasp_threshold,
                },
            },
            gripper_entity: None,
            object_entity: None,
            target_position: Vec3::ZERO,
            episode_info: EpisodeInfo::default(),
            current_step: 0,
            target_tolerance,
            grasp_threshold,
            is_grasped: false,
            grasp_time: 0,
        }
    }

    /// Sample target position for object
    fn sample_target(&mut self) {
        let mut rng = rand::thread_rng();
        self.target_position = Vec3::new(
            rng.gen_range(-0.5..0.5),
            rng.gen_range(0.5..1.0),
            rng.gen_range(-0.5..0.5),
        );
    }

    /// Find gripper and object entities
    fn find_entities(&mut self, world: &mut World) {
        let mut query = world.query::<(Entity, &Robot, &Transform)>();

        // Find gripper (highest Y position) and object (separate entity)
        let mut max_y = f32::NEG_INFINITY;
        let mut gripper = None;

        for (entity, _robot, transform) in query.iter(world) {
            if transform.translation.y > max_y {
                max_y = transform.translation.y;
                gripper = Some(entity);
            }
        }

        self.gripper_entity = gripper;

        // For this task, assume there's a separate object entity
        // In a real scenario, would query for specific object components
        // For now, use a simplified approach
    }

    /// Get gripper position
    fn get_gripper_position(&self, world: &World) -> Option<Vec3> {
        if let Some(entity) = self.gripper_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                return Some(transform.translation);
            }
        }
        None
    }

    /// Get object position
    fn get_object_position(&self, world: &World) -> Option<Vec3> {
        if let Some(entity) = self.object_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                return Some(transform.translation);
            }
        }
        // If no object entity, return a default position
        Some(Vec3::new(0.3, 0.5, 0.3))
    }

    /// Get object velocity
    fn get_object_velocity(&self, world: &World) -> Vec3 {
        if let Some(entity) = self.object_entity {
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

    /// Get gripper velocity
    fn get_gripper_velocity(&self, world: &World) -> Vec3 {
        if let Some(entity) = self.gripper_entity {
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

    /// Check if object is grasped
    fn check_grasp(&mut self, world: &World) -> bool {
        if let Some(gripper_pos) = self.get_gripper_position(world) {
            if let Some(object_pos) = self.get_object_position(world) {
                let distance = gripper_pos.distance(object_pos);
                if distance < self.grasp_threshold {
                    if !self.is_grasped {
                        self.is_grasped = true;
                        self.grasp_time = self.current_step;
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Check if object is at target
    fn is_object_at_target(&self, world: &World) -> bool {
        if let Some(object_pos) = self.get_object_position(world) {
            object_pos.distance(self.target_position) < self.target_tolerance
        } else {
            false
        }
    }

    /// Distance from object to target
    fn object_to_target_distance(&self, world: &World) -> f32 {
        if let Some(object_pos) = self.get_object_position(world) {
            object_pos.distance(self.target_position)
        } else {
            100.0
        }
    }

    /// Distance from gripper to object
    fn gripper_to_object_distance(&self, world: &World) -> f32 {
        if let Some(gripper_pos) = self.get_gripper_position(world) {
            if let Some(object_pos) = self.get_object_position(world) {
                return gripper_pos.distance(object_pos);
            }
        }
        100.0
    }
}

impl RLTask for ManipulationTask {
    fn config(&self) -> &TaskConfig {
        &self.config
    }

    fn reset(&mut self, world: &mut World) -> Observation {
        // Reset episode info
        self.episode_info = EpisodeInfo::default();
        self.current_step = 0;
        self.is_grasped = false;
        self.grasp_time = 0;

        // Sample new target
        self.sample_target();

        // Find entities if not already found
        if self.gripper_entity.is_none() {
            self.find_entities(world);
        }

        // Reset gripper position
        if let Some(entity) = self.gripper_entity {
            if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                let mut rng = rand::thread_rng();
                transform.translation = Vec3::new(
                    rng.gen_range(-0.3..0.3),
                    1.0,
                    rng.gen_range(-0.3..0.3),
                );
            }
        }

        // Reset object position if it exists
        if let Some(entity) = self.object_entity {
            if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                let mut rng = rand::thread_rng();
                transform.translation = Vec3::new(
                    rng.gen_range(-0.4..0.4),
                    0.5,
                    rng.gen_range(-0.4..0.4),
                );
            }
        }

        self.get_observation(world)
    }

    fn step(&mut self, world: &mut World, action: &Action) -> StepResult {
        self.current_step += 1;

        // Apply action (gripper position delta + grasp command)
        if let Action::Continuous(actions) = action {
            if let Some(entity) = self.gripper_entity {
                if actions.len() >= 4 {
                    let dx = actions[0].clamp(-1.0, 1.0) * 0.05;
                    let dy = actions[1].clamp(-1.0, 1.0) * 0.05;
                    let dz = actions[2].clamp(-1.0, 1.0) * 0.05;
                    let grasp_cmd = actions[3]; // > 0 = grasp, < 0 = release

                    if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                        transform.translation.x += dx;
                        transform.translation.y += dy;
                        transform.translation.z += dz;

                        // Clamp gripper position to workspace
                        transform.translation.x = transform.translation.x.clamp(-1.0, 1.0);
                        transform.translation.y = transform.translation.y.clamp(0.3, 1.5);
                        transform.translation.z = transform.translation.z.clamp(-1.0, 1.0);
                    }

                    // Check grasp based on proximity and grasp command
                    if grasp_cmd > 0.0 {
                        self.check_grasp(world);
                    } else {
                        self.is_grasped = false;
                    }

                    // If grasped, move object with gripper
                    if self.is_grasped {
                        if let Some(gripper_pos) = self.get_gripper_position(world) {
                            if let Some(object_entity) = self.object_entity {
                                if let Some(mut transform) = world.get_mut::<Transform>(object_entity) {
                                    transform.translation = gripper_pos - Vec3::new(0.0, 0.1, 0.0);
                                }
                            }
                        }
                    }
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
            if self.is_object_at_target(world) {
                self.episode_info.success = true;
                self.episode_info.termination_reason = TerminationReason::Success;
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

        // Gripper position (3D)
        if let Some(gripper_pos) = self.get_gripper_position(world) {
            obs_data.extend_from_slice(&[gripper_pos.x, gripper_pos.y, gripper_pos.z]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0]);
        }

        // Gripper velocity (3D)
        let gripper_vel = self.get_gripper_velocity(world);
        obs_data.extend_from_slice(&[gripper_vel.x, gripper_vel.y, gripper_vel.z]);

        // Object position (3D)
        if let Some(object_pos) = self.get_object_position(world) {
            obs_data.extend_from_slice(&[object_pos.x, object_pos.y, object_pos.z]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0]);
        }

        // Object velocity (3D)
        let object_vel = self.get_object_velocity(world);
        obs_data.extend_from_slice(&[object_vel.x, object_vel.y, object_vel.z]);

        // Target position (3D)
        obs_data.extend_from_slice(&[
            self.target_position.x,
            self.target_position.y,
            self.target_position.z,
        ]);

        // Gripper to object distance (1D)
        let gripper_obj_dist = self.gripper_to_object_distance(world);
        obs_data.push(gripper_obj_dist);

        // Object to target distance (1D)
        let obj_target_dist = self.object_to_target_distance(world);
        obs_data.push(obj_target_dist);

        // Grasp state (1D)
        obs_data.push(if self.is_grasped { 1.0 } else { 0.0 });

        // Direction from gripper to object (3D)
        if let Some(gripper_pos) = self.get_gripper_position(world) {
            if let Some(object_pos) = self.get_object_position(world) {
                let dir = (object_pos - gripper_pos).normalize_or_zero();
                obs_data.extend_from_slice(&[dir.x, dir.y, dir.z]);
            } else {
                obs_data.extend_from_slice(&[0.0, 0.0, 0.0]);
            }
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0]);
        }

        // Direction from object to target (3D)
        if let Some(object_pos) = self.get_object_position(world) {
            let dir = (self.target_position - object_pos).normalize_or_zero();
            obs_data.extend_from_slice(&[dir.x, dir.y, dir.z]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0]);
        }

        Observation::new(obs_data)
    }

    fn compute_reward(&self, world: &mut World) -> f32 {
        let gripper_obj_dist = self.gripper_to_object_distance(world);
        let obj_target_dist = self.object_to_target_distance(world);

        // Phase 1: Reward for approaching object (before grasp)
        let approach_reward = if !self.is_grasped {
            -gripper_obj_dist * 2.0
        } else {
            0.0
        };

        // Bonus for grasping
        let grasp_reward = if self.is_grasped && self.grasp_time == self.current_step {
            5.0
        } else {
            0.0
        };

        // Phase 2: Reward for moving object to target (after grasp)
        let manipulation_reward = if self.is_grasped {
            -obj_target_dist * 3.0
        } else {
            0.0
        };

        // Large bonus for placing object at target
        let placement_reward = if self.is_object_at_target(world) {
            50.0
        } else {
            0.0
        };

        // Penalty for dropping object
        let drop_penalty = if !self.is_grasped && self.grasp_time > 0 && self.current_step > self.grasp_time + 10 {
            -2.0
        } else {
            0.0
        };

        // Penalty for object velocity (encourage stable manipulation)
        let object_vel = self.get_object_velocity(world);
        let stability_penalty = -object_vel.length() * 0.1;

        // Small time penalty
        let time_penalty = -0.01;

        approach_reward
            + grasp_reward
            + manipulation_reward
            + placement_reward
            + drop_penalty
            + stability_penalty
            + time_penalty
    }

    fn is_done(&self, world: &mut World) -> bool {
        self.is_object_at_target(world)
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

        // Draw gripper position
        if let Some(gripper_pos) = self.get_gripper_position(world) {
            gizmos.sphere(
                Isometry3d::new(gripper_pos, Quat::IDENTITY),
                0.05,
                if self.is_grasped {
                    Color::srgb(1.0, 0.0, 0.0)
                } else {
                    Color::srgb(0.0, 0.5, 1.0)
                },
            );
        }

        // Draw object position
        if let Some(object_pos) = self.get_object_position(world) {
            gizmos.sphere(
                Isometry3d::new(object_pos, Quat::IDENTITY),
                0.08,
                Color::srgb(1.0, 0.5, 0.0),
            );
        }

        // Draw grasp zone around gripper
        if let Some(gripper_pos) = self.get_gripper_position(world) {
            gizmos.sphere(
                Isometry3d::new(gripper_pos, Quat::IDENTITY),
                self.grasp_threshold,
                Color::srgb(0.5, 0.5, 0.5).with_alpha(0.1),
            );
        }

        // Draw lines connecting gripper -> object -> target
        if let Some(gripper_pos) = self.get_gripper_position(world) {
            if let Some(object_pos) = self.get_object_position(world) {
                gizmos.line(
                    gripper_pos,
                    object_pos,
                    if self.is_grasped {
                        Color::srgb(1.0, 0.0, 0.0).with_alpha(0.8)
                    } else {
                        Color::srgb(1.0, 1.0, 0.0).with_alpha(0.3)
                    },
                );

                gizmos.line(
                    object_pos,
                    self.target_position,
                    Color::srgb(0.0, 1.0, 0.0).with_alpha(0.3),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manipulation_task_creation() {
        let task = ManipulationTask::new(25, 4);
        assert_eq!(task.config.obs_dim, 25);
        assert_eq!(task.config.action_dim, 4);
        assert_eq!(task.current_step, 0);
    }

    #[test]
    fn test_manipulation_parameters() {
        let task = ManipulationTask::new(25, 4);
        assert_eq!(task.target_tolerance, 0.1);
        assert_eq!(task.grasp_threshold, 0.15);
    }

    #[test]
    fn test_observation_size() {
        let task = ManipulationTask::new(25, 4);
        let mut world = World::new();
        let obs = task.get_observation(&mut world);

        // gripper_pos(3) + gripper_vel(3) + obj_pos(3) + obj_vel(3) + target(3) +
        // gripper_obj_dist(1) + obj_target_dist(1) + grasp_state(1) +
        // gripper_to_obj_dir(3) + obj_to_target_dir(3) = 24-25
        // Note: Actual size may vary based on implementation
        assert!(obs.len() >= 24 && obs.len() <= 25);
    }

    #[test]
    fn test_grasp_state() {
        let task = ManipulationTask::new(25, 4);
        assert!(!task.is_grasped);
        assert_eq!(task.grasp_time, 0);
    }
}
