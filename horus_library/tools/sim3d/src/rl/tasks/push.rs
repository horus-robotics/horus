use bevy::prelude::*;
use rand::Rng;

use crate::rl::{
    Action, EpisodeInfo, Observation, RLTask, StepResult, TaskConfig, TaskParameters,
    TerminationReason,
};
use crate::robot::Robot;
use crate::physics::rigid_body::RigidBodyComponent;
use crate::physics::world::PhysicsWorld;

/// Push task: Push an object to a target location
pub struct PushTask {
    config: TaskConfig,
    pusher_entity: Option<Entity>,
    object_entity: Option<Entity>,
    target_position: Vec3,
    episode_info: EpisodeInfo,
    current_step: usize,
    target_tolerance: f32,
    object_velocity_bonus: f32,
    previous_object_distance: f32,
    total_push_force: f32,
}

impl PushTask {
    pub fn new(obs_dim: usize, action_dim: usize) -> Self {
        let target_tolerance = 0.2;
        let object_velocity_bonus = 1.0;

        Self {
            config: TaskConfig {
                max_steps: 1000,
                dt: 0.02,
                obs_dim,
                action_dim,
                parameters: TaskParameters::Push {
                    target_tolerance,
                    object_velocity_bonus,
                },
            },
            pusher_entity: None,
            object_entity: None,
            target_position: Vec3::ZERO,
            episode_info: EpisodeInfo::default(),
            current_step: 0,
            target_tolerance,
            object_velocity_bonus,
            previous_object_distance: 0.0,
            total_push_force: 0.0,
        }
    }

    /// Sample target position for object
    fn sample_target(&mut self) {
        let mut rng = rand::thread_rng();
        let distance = rng.gen_range(2.0..5.0);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);

        self.target_position = Vec3::new(
            distance * angle.cos(),
            0.5,
            distance * angle.sin(),
        );
    }

    /// Find pusher and object entities
    fn find_entities(&mut self, world: &mut World) {
        let mut query = world.query::<(Entity, &Robot, &Transform)>();

        // Find pusher (robot entity)
        if let Some((entity, _robot, _transform)) = query.iter(world).next() {
            self.pusher_entity = Some(entity);
        }

        // In a real scenario, would query for a specific object entity
        // For now, use simplified approach
    }

    /// Get pusher position
    fn get_pusher_position(&self, world: &World) -> Option<Vec3> {
        if let Some(entity) = self.pusher_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                return Some(transform.translation);
            }
        }
        None
    }

    /// Get pusher orientation
    fn get_pusher_orientation(&self, world: &World) -> Option<Quat> {
        if let Some(entity) = self.pusher_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                return Some(transform.rotation);
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
        // Default object position if not found
        Some(Vec3::new(1.0, 0.5, 1.0))
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

    /// Get pusher velocity
    fn get_pusher_velocity(&self, world: &World) -> Vec3 {
        if let Some(entity) = self.pusher_entity {
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

    /// Distance from object to target
    fn object_to_target_distance(&self, world: &World) -> f32 {
        if let Some(object_pos) = self.get_object_position(world) {
            object_pos.distance(self.target_position)
        } else {
            100.0
        }
    }

    /// Distance from pusher to object
    fn pusher_to_object_distance(&self, world: &World) -> f32 {
        if let Some(pusher_pos) = self.get_pusher_position(world) {
            if let Some(object_pos) = self.get_object_position(world) {
                return pusher_pos.distance(object_pos);
            }
        }
        100.0
    }

    /// Check if object is at target
    fn is_object_at_target(&self, world: &World) -> bool {
        self.object_to_target_distance(world) < self.target_tolerance
    }

    /// Optimal push direction (from object towards target)
    fn optimal_push_direction(&self, world: &World) -> Vec3 {
        if let Some(object_pos) = self.get_object_position(world) {
            (self.target_position - object_pos).normalize_or_zero()
        } else {
            Vec3::ZERO
        }
    }

    /// Check if pusher is behind object (good pushing position)
    fn is_good_push_position(&self, world: &World) -> bool {
        if let Some(pusher_pos) = self.get_pusher_position(world) {
            if let Some(object_pos) = self.get_object_position(world) {
                let optimal_dir = self.optimal_push_direction(world);
                let pusher_to_object = (object_pos - pusher_pos).normalize_or_zero();

                // Good position if pusher is roughly behind object relative to target
                let alignment = pusher_to_object.dot(optimal_dir);
                return alignment > 0.7; // Within ~45 degrees
            }
        }
        false
    }
}

impl RLTask for PushTask {
    fn config(&self) -> &TaskConfig {
        &self.config
    }

    fn reset(&mut self, world: &mut World) -> Observation {
        // Reset episode info
        self.episode_info = EpisodeInfo::default();
        self.current_step = 0;
        self.total_push_force = 0.0;

        // Sample new target
        self.sample_target();

        // Find entities if not already found
        if self.pusher_entity.is_none() {
            self.find_entities(world);
        }

        // Reset pusher position
        if let Some(entity) = self.pusher_entity {
            if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                transform.translation = Vec3::new(0.0, 0.5, 0.0);
                transform.rotation = Quat::IDENTITY;
            }
        }

        // Reset object position
        if let Some(entity) = self.object_entity {
            if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                let mut rng = rand::thread_rng();
                transform.translation = Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    0.5,
                    rng.gen_range(-1.0..1.0),
                );
            }
        }

        // Initialize distance tracking
        self.previous_object_distance = self.object_to_target_distance(world);

        self.get_observation(world)
    }

    fn step(&mut self, world: &mut World, action: &Action) -> StepResult {
        self.current_step += 1;

        // Apply action (pusher velocity commands)
        if let Action::Continuous(actions) = action {
            if let Some(entity) = self.pusher_entity {
                if actions.len() >= 2 {
                    let vx = actions[0].clamp(-1.0, 1.0) * 1.5; // Max 1.5 m/s
                    let vz = actions[1].clamp(-1.0, 1.0) * 1.5;

                    if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                        let dt = self.config.dt;

                        // Update pusher position
                        transform.translation.x += vx * dt;
                        transform.translation.z += vz * dt;

                        // Update orientation to face movement direction
                        if vx.abs() > 0.01 || vz.abs() > 0.01 {
                            let angle = vz.atan2(vx);
                            transform.rotation = Quat::from_rotation_y(angle);
                        }

                        // Simple collision: if pusher is close to object, push it
                        if let Some(object_entity) = self.object_entity {
                            let pusher_pos = transform.translation;
                            if let Some(mut object_transform) = world.get_mut::<Transform>(object_entity) {
                                let object_pos = object_transform.translation;
                                let distance = pusher_pos.distance(object_pos);

                                if distance < 0.3 {
                                    // Contact threshold
                                    let push_dir = (object_pos - pusher_pos).normalize_or_zero();
                                    let push_force = (vx * vx + vz * vz).sqrt();
                                    let push_magnitude = push_force * 0.1;

                                    object_transform.translation += push_dir * push_magnitude;
                                    self.total_push_force += push_magnitude;
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

        // Update previous distance for next step
        self.previous_object_distance = self.object_to_target_distance(world);

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

        // Pusher position (3D)
        if let Some(pusher_pos) = self.get_pusher_position(world) {
            obs_data.extend_from_slice(&[pusher_pos.x, pusher_pos.y, pusher_pos.z]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0]);
        }

        // Pusher orientation (quaternion, 4D)
        if let Some(rot) = self.get_pusher_orientation(world) {
            obs_data.extend_from_slice(&[rot.x, rot.y, rot.z, rot.w]);
        } else {
            obs_data.extend_from_slice(&[0.0, 0.0, 0.0, 1.0]);
        }

        // Pusher velocity (3D)
        let pusher_vel = self.get_pusher_velocity(world);
        obs_data.extend_from_slice(&[pusher_vel.x, pusher_vel.y, pusher_vel.z]);

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

        // Pusher to object distance (1D)
        let pusher_obj_dist = self.pusher_to_object_distance(world);
        obs_data.push(pusher_obj_dist);

        // Object to target distance (1D)
        let obj_target_dist = self.object_to_target_distance(world);
        obs_data.push(obj_target_dist);

        // Direction from pusher to object (3D)
        if let Some(pusher_pos) = self.get_pusher_position(world) {
            if let Some(object_pos) = self.get_object_position(world) {
                let dir = (object_pos - pusher_pos).normalize_or_zero();
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

        // Optimal push direction (3D)
        let optimal_dir = self.optimal_push_direction(world);
        obs_data.extend_from_slice(&[optimal_dir.x, optimal_dir.y, optimal_dir.z]);

        Observation::new(obs_data)
    }

    fn compute_reward(&self, world: &mut World) -> f32 {
        let obj_target_dist = self.object_to_target_distance(world);
        let pusher_obj_dist = self.pusher_to_object_distance(world);

        // Dense reward for object getting closer to target
        let progress_reward = (self.previous_object_distance - obj_target_dist) * 10.0;

        // Distance-based reward (encourage pushing object towards target)
        let distance_reward = -obj_target_dist * 0.5;

        // Large bonus for reaching target
        let success_reward = if self.is_object_at_target(world) {
            100.0
        } else {
            0.0
        };

        // Reward for being in good pushing position
        let position_reward = if self.is_good_push_position(world) {
            1.0
        } else {
            0.0
        };

        // Reward for object velocity in correct direction
        let object_vel = self.get_object_velocity(world);
        let optimal_dir = self.optimal_push_direction(world);
        let velocity_alignment = object_vel.dot(optimal_dir);
        let velocity_reward = velocity_alignment * self.object_velocity_bonus;

        // Penalty for pusher being too far from object (encourage staying close)
        let proximity_penalty = if pusher_obj_dist > 2.0 {
            -(pusher_obj_dist - 2.0) * 0.5
        } else {
            0.0
        };

        // Small penalty for excessive object velocity (encourage controlled pushing)
        let control_penalty = if object_vel.length() > 3.0 {
            -(object_vel.length() - 3.0) * 0.2
        } else {
            0.0
        };

        // Time penalty (encourage efficiency)
        let time_penalty = -0.01;

        progress_reward
            + distance_reward
            + success_reward
            + position_reward
            + velocity_reward
            + proximity_penalty
            + control_penalty
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

        // Draw object position
        if let Some(object_pos) = self.get_object_position(world) {
            gizmos.sphere(
                Isometry3d::new(object_pos, Quat::IDENTITY),
                0.15,
                Color::srgb(1.0, 0.5, 0.0),
            );

            // Draw object velocity vector
            let object_vel = self.get_object_velocity(world);
            if object_vel.length() > 0.01 {
                let vel_end = object_pos + object_vel * 0.5;
                gizmos.arrow(
                    object_pos,
                    vel_end,
                    Color::srgb(1.0, 0.0, 1.0),
                );
            }
        }

        // Draw pusher position and orientation
        if let Some(pusher_pos) = self.get_pusher_position(world) {
            gizmos.sphere(
                Isometry3d::new(pusher_pos, Quat::IDENTITY),
                0.1,
                Color::srgb(0.0, 0.5, 1.0),
            );

            // Draw pusher heading
            if let Some(rot) = self.get_pusher_orientation(world) {
                let forward = rot * Vec3::X;
                let heading_end = pusher_pos + forward * 0.5;
                gizmos.arrow(
                    pusher_pos,
                    heading_end,
                    Color::srgb(1.0, 0.0, 0.0),
                );
            }
        }

        // Draw optimal push direction
        if let Some(object_pos) = self.get_object_position(world) {
            let optimal_dir = self.optimal_push_direction(world);
            let dir_end = object_pos + optimal_dir * 0.8;
            gizmos.arrow(
                object_pos,
                dir_end,
                Color::srgb(0.0, 1.0, 1.0).with_alpha(0.5),
            );
        }

        // Draw line from object to target
        if let Some(object_pos) = self.get_object_position(world) {
            gizmos.line(
                object_pos,
                self.target_position,
                Color::srgb(1.0, 1.0, 0.0).with_alpha(0.3),
            );
        }

        // Draw line from pusher to object
        if let Some(pusher_pos) = self.get_pusher_position(world) {
            if let Some(object_pos) = self.get_object_position(world) {
                let color = if self.is_good_push_position(world) {
                    Color::srgb(0.0, 1.0, 0.0).with_alpha(0.5)
                } else {
                    Color::srgb(1.0, 0.0, 0.0).with_alpha(0.3)
                };
                gizmos.line(pusher_pos, object_pos, color);
            }
        }

        // Draw contact zone around pusher
        if let Some(pusher_pos) = self.get_pusher_position(world) {
            gizmos.sphere(
                Isometry3d::new(pusher_pos, Quat::IDENTITY),
                0.3,
                Color::srgb(0.5, 0.5, 0.5).with_alpha(0.1),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_task_creation() {
        let task = PushTask::new(30, 2);
        assert_eq!(task.config.obs_dim, 30);
        assert_eq!(task.config.action_dim, 2);
        assert_eq!(task.current_step, 0);
    }

    #[test]
    fn test_push_parameters() {
        let task = PushTask::new(30, 2);
        assert_eq!(task.target_tolerance, 0.2);
        assert_eq!(task.object_velocity_bonus, 1.0);
    }

    #[test]
    fn test_observation_size() {
        let task = PushTask::new(30, 2);
        let mut world = World::new();
        let obs = task.get_observation(&mut world);

        // pusher_pos(3) + pusher_quat(4) + pusher_vel(3) + obj_pos(3) + obj_vel(3) +
        // target(3) + pusher_obj_dist(1) + obj_target_dist(1) +
        // pusher_to_obj_dir(3) + obj_to_target_dir(3) + optimal_dir(3) = 30
        assert_eq!(obs.len(), 30);
    }

    #[test]
    fn test_target_sampling() {
        let mut task = PushTask::new(30, 2);
        task.sample_target();

        // Check target is within reasonable bounds
        let distance = Vec3::new(task.target_position.x, 0.0, task.target_position.z).length();
        assert!(distance >= 2.0 && distance <= 5.0);
    }
}
