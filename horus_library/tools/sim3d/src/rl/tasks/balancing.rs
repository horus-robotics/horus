use bevy::prelude::*;
use rand::Rng;

use crate::physics::rigid_body::RigidBodyComponent;
use crate::physics::world::PhysicsWorld;
use crate::rl::{
    Action, EpisodeInfo, Observation, RLTask, StepResult, TaskConfig, TaskParameters,
    TerminationReason,
};
use crate::robot::Robot;

/// Balancing task: Balance an inverted pendulum or cart-pole system
pub struct BalancingTask {
    config: TaskConfig,
    cart_entity: Option<Entity>,
    pole_entity: Option<Entity>,
    episode_info: EpisodeInfo,
    current_step: usize,
    angle_limit: f32,
    position_limit: f32,
}

impl BalancingTask {
    pub fn new(obs_dim: usize, action_dim: usize) -> Self {
        let angle_limit = 0.4; // ~23 degrees
        let position_limit = 2.5; // meters

        Self {
            config: TaskConfig {
                max_steps: 500,
                dt: 0.02,
                obs_dim,
                action_dim,
                parameters: TaskParameters::Balancing {
                    angle_limit,
                    position_limit,
                },
            },
            cart_entity: None,
            pole_entity: None,
            episode_info: EpisodeInfo::default(),
            current_step: 0,
            angle_limit,
            position_limit,
        }
    }

    /// Find cart and pole entities
    fn find_entities(&mut self, world: &mut World) {
        let mut query = world.query::<(Entity, &Robot, &Transform)>();

        // Find cart (lowest Y position) and pole (highest Y position)
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut cart = None;
        let mut pole = None;

        for (entity, _robot, transform) in query.iter(world) {
            let y = transform.translation.y;
            if y < min_y {
                min_y = y;
                cart = Some(entity);
            }
            if y > max_y {
                max_y = y;
                pole = Some(entity);
            }
        }

        self.cart_entity = cart;
        self.pole_entity = pole;
    }

    /// Get cart position
    fn get_cart_position(&self, world: &World) -> Option<Vec3> {
        if let Some(entity) = self.cart_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                return Some(transform.translation);
            }
        }
        None
    }

    /// Get pole angle from vertical (radians)
    fn get_pole_angle(&self, world: &World) -> f32 {
        if let Some(entity) = self.pole_entity {
            if let Some(transform) = world.get::<Transform>(entity) {
                // Calculate angle from vertical (Y-up)
                let forward = transform.rotation * Vec3::Y;
                let vertical = Vec3::Y;
                return forward.angle_between(vertical);
            }
        }
        0.0
    }

    /// Get cart velocity
    fn get_cart_velocity(&self, world: &World) -> Vec3 {
        if let Some(entity) = self.cart_entity {
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

    /// Get pole angular velocity
    fn get_pole_angular_velocity(&self, world: &World) -> Vec3 {
        if let Some(entity) = self.pole_entity {
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

    /// Check if system is balanced (within limits)
    fn is_balanced(&self, world: &World) -> bool {
        let angle = self.get_pole_angle(world);
        let cart_pos = self.get_cart_position(world).unwrap_or(Vec3::ZERO);

        angle.abs() < self.angle_limit && cart_pos.x.abs() < self.position_limit
    }
}

impl RLTask for BalancingTask {
    fn config(&self) -> &TaskConfig {
        &self.config
    }

    fn reset(&mut self, world: &mut World) -> Observation {
        // Reset episode info
        self.episode_info = EpisodeInfo::default();
        self.current_step = 0;

        // Find entities if not already found
        if self.cart_entity.is_none() || self.pole_entity.is_none() {
            self.find_entities(world);
        }

        // Reset cart to center
        if let Some(entity) = self.cart_entity {
            if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                let mut rng = rand::thread_rng();
                // Small random initial position
                transform.translation.x = rng.gen_range(-0.1..0.1);
                transform.translation.y = 0.5;
                transform.translation.z = 0.0;
            }
        }

        // Reset pole to near-vertical with small random perturbation
        if let Some(entity) = self.pole_entity {
            if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                let mut rng = rand::thread_rng();
                let small_angle = rng.gen_range(-0.05..0.05); // ~3 degrees
                transform.rotation = Quat::from_rotation_z(small_angle);
            }
        }

        self.get_observation(world)
    }

    fn step(&mut self, world: &mut World, action: &Action) -> StepResult {
        self.current_step += 1;

        // Apply action (force on cart)
        if let Action::Continuous(actions) = action {
            if let Some(entity) = self.cart_entity {
                if !actions.is_empty() {
                    let force = actions[0].clamp(-1.0, 1.0) * 10.0; // Scale force

                    // Apply force to cart
                    if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                        // Simple integration: velocity += force * dt / mass
                        let dt = self.config.dt;
                        let mass = 1.0;
                        let acceleration = force / mass;
                        let velocity_change = acceleration * dt;

                        // Update position
                        transform.translation.x += velocity_change * dt;
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
            if self.is_balanced(world) {
                self.episode_info.success = true;
                self.episode_info.termination_reason = TerminationReason::Success;
            } else {
                self.episode_info.termination_reason = TerminationReason::Failure;
            }
        } else if truncated {
            self.episode_info.termination_reason = TerminationReason::MaxSteps;
            // If still balanced at max steps, consider it a success
            if self.is_balanced(world) {
                self.episode_info.success = true;
            }
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

        // Cart position (1D, along X)
        let cart_pos = self.get_cart_position(world).unwrap_or(Vec3::ZERO);
        obs_data.push(cart_pos.x);

        // Cart velocity (1D, along X)
        let cart_vel = self.get_cart_velocity(world);
        obs_data.push(cart_vel.x);

        // Pole angle from vertical (1D)
        let pole_angle = self.get_pole_angle(world);
        obs_data.push(pole_angle);

        // Pole angular velocity (1D, around Z axis for 2D balancing)
        let pole_ang_vel = self.get_pole_angular_velocity(world);
        obs_data.push(pole_ang_vel.z);

        // Sine and cosine of pole angle (helps with angle wrapping)
        obs_data.push(pole_angle.sin());
        obs_data.push(pole_angle.cos());

        Observation::new(obs_data)
    }

    fn compute_reward(&self, world: &mut World) -> f32 {
        let angle = self.get_pole_angle(world);
        let cart_pos = self.get_cart_position(world).unwrap_or(Vec3::ZERO);

        // Reward for staying upright
        let angle_reward = (self.angle_limit - angle.abs()) / self.angle_limit;

        // Reward for staying centered
        let position_reward = (self.position_limit - cart_pos.x.abs()) / self.position_limit;

        // Small penalty for cart velocity (encourage smooth control)
        let cart_vel = self.get_cart_velocity(world);
        let velocity_penalty = -cart_vel.x.abs() * 0.01;

        // Small penalty for angular velocity (encourage stable balancing)
        let ang_vel = self.get_pole_angular_velocity(world);
        let ang_vel_penalty = -ang_vel.z.abs() * 0.01;

        // Heavy penalty for falling
        let failure_penalty = if !self.is_balanced(world) { -10.0 } else { 0.0 };

        angle_reward + position_reward * 0.5 + velocity_penalty + ang_vel_penalty + failure_penalty
    }

    fn is_done(&self, world: &mut World) -> bool {
        !self.is_balanced(world)
    }

    fn get_info(&self) -> EpisodeInfo {
        self.episode_info.clone()
    }

    fn render(&self, gizmos: &mut Gizmos, world: &mut World) {
        // Draw cart position limits
        let limit_color = Color::srgb(1.0, 0.0, 0.0).with_alpha(0.3);

        // Left limit
        gizmos.line(
            Vec3::new(-self.position_limit, 0.0, -1.0),
            Vec3::new(-self.position_limit, 2.0, -1.0),
            limit_color,
        );

        // Right limit
        gizmos.line(
            Vec3::new(self.position_limit, 0.0, -1.0),
            Vec3::new(self.position_limit, 2.0, -1.0),
            limit_color,
        );

        // Draw angle limits (as cones)
        if let Some(cart_pos) = self.get_cart_position(world) {
            let pole_height = 2.0;

            // Left angle limit
            let left_end = cart_pos
                + Vec3::new(
                    -pole_height * self.angle_limit.sin(),
                    pole_height * self.angle_limit.cos(),
                    0.0,
                );
            gizmos.line(cart_pos, left_end, limit_color);

            // Right angle limit
            let right_end = cart_pos
                + Vec3::new(
                    pole_height * self.angle_limit.sin(),
                    pole_height * self.angle_limit.cos(),
                    0.0,
                );
            gizmos.line(cart_pos, right_end, limit_color);

            // Draw vertical reference
            let vertical_end = cart_pos + Vec3::new(0.0, pole_height, 0.0);
            gizmos.line(
                cart_pos,
                vertical_end,
                Color::srgb(0.0, 1.0, 0.0).with_alpha(0.3),
            );
        }

        // Highlight cart
        if let Some(cart_pos) = self.get_cart_position(world) {
            gizmos.sphere(
                Isometry3d::new(cart_pos, Quat::IDENTITY),
                0.1,
                Color::srgb(0.0, 0.5, 1.0),
            );
        }

        // Draw pole angle indicator
        let _angle = self.get_pole_angle(world);
        let balanced = self.is_balanced(world);
        let angle_color = if balanced {
            Color::srgb(0.0, 1.0, 0.0) // Green if balanced
        } else {
            Color::srgb(1.0, 0.0, 0.0) // Red if falling
        };

        if let Some(cart_pos) = self.get_cart_position(world) {
            let angle_display_pos = cart_pos + Vec3::new(-1.5, 1.0, 0.0);
            gizmos.sphere(
                Isometry3d::new(angle_display_pos, Quat::IDENTITY),
                0.05,
                angle_color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balancing_task_creation() {
        let task = BalancingTask::new(6, 1);
        assert_eq!(task.config.obs_dim, 6);
        assert_eq!(task.config.action_dim, 1);
        assert_eq!(task.current_step, 0);
    }

    #[test]
    fn test_balancing_limits() {
        let task = BalancingTask::new(6, 1);
        assert_eq!(task.angle_limit, 0.4);
        assert_eq!(task.position_limit, 2.5);
    }

    #[test]
    fn test_observation_size() {
        let task = BalancingTask::new(6, 1);
        let mut world = World::new();
        let obs = task.get_observation(&mut world);

        // Should have: cart_pos(1) + cart_vel(1) + angle(1) + ang_vel(1) + sin(1) + cos(1) = 6
        assert_eq!(obs.len(), 6);
    }
}
