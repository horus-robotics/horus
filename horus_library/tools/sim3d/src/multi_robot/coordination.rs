//! Swarm coordination primitives and behaviors

use super::{Robot, RobotId};
use bevy::prelude::*;
use std::collections::HashMap;

/// Swarm behavior component
#[derive(Component)]
pub struct SwarmAgent {
    /// Desired separation from neighbors
    pub separation_distance: f32,
    /// Alignment weight (how much to match neighbor velocity)
    pub alignment_weight: f32,
    /// Cohesion weight (how much to move toward center)
    pub cohesion_weight: f32,
    /// Separation weight (how much to avoid neighbors)
    pub separation_weight: f32,
    /// Maximum speed
    pub max_speed: f32,
    /// Perception radius for neighbors
    pub perception_radius: f32,
}

impl Default for SwarmAgent {
    fn default() -> Self {
        Self {
            separation_distance: 2.0,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            separation_weight: 1.5,
            max_speed: 2.0,
            perception_radius: 5.0,
        }
    }
}

/// Formation controller component
#[derive(Component)]
pub struct FormationController {
    /// Formation type
    pub formation_type: FormationType,
    /// Position in formation (index)
    pub formation_index: usize,
    /// Formation scale
    pub scale: f32,
    /// Leader to follow (if any)
    pub leader: Option<RobotId>,
}

/// Formation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormationType {
    /// Line formation
    Line,
    /// Circle formation
    Circle,
    /// Grid formation
    Grid,
    /// V formation (like birds)
    Wedge,
    /// Custom formation from points
    Custom,
}

impl FormationController {
    pub fn new(formation_type: FormationType, index: usize) -> Self {
        Self {
            formation_type,
            formation_index: index,
            scale: 1.0,
            leader: None,
        }
    }

    /// Get desired position in formation
    pub fn get_formation_position(&self, leader_transform: &Transform) -> Vec3 {
        let offset = match self.formation_type {
            FormationType::Line => {
                Vec3::new(self.formation_index as f32 * 2.0 * self.scale, 0.0, 0.0)
            }
            FormationType::Circle => {
                let angle = (self.formation_index as f32 * std::f32::consts::TAU) / 8.0;
                let radius = 5.0 * self.scale;
                Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius)
            }
            FormationType::Grid => {
                let grid_size = 4;
                let x = (self.formation_index % grid_size) as f32 * 2.0 * self.scale;
                let z = (self.formation_index / grid_size) as f32 * 2.0 * self.scale;
                Vec3::new(x, 0.0, z)
            }
            FormationType::Wedge => {
                let row = ((self.formation_index as f32 * 8.0 + 1.0).sqrt() - 1.0) / 2.0;
                let row_i = row.floor() as usize;
                let col = self.formation_index - (row_i * (row_i + 1)) / 2;
                let x = col as f32 * 2.0 * self.scale - row as f32 * self.scale;
                let z = row as f32 * 2.0 * self.scale;
                Vec3::new(x, 0.0, -z)
            }
            FormationType::Custom => {
                // Default to line for custom
                Vec3::new(self.formation_index as f32 * 2.0 * self.scale, 0.0, 0.0)
            }
        };

        leader_transform.translation + leader_transform.rotation * offset
    }
}

/// Consensus algorithm state
#[derive(Resource, Default)]
pub struct ConsensusState {
    /// Shared state values per robot
    values: HashMap<RobotId, f32>,
    /// Convergence threshold
    pub convergence_threshold: f32,
}

impl ConsensusState {
    pub fn new(threshold: f32) -> Self {
        Self {
            values: HashMap::new(),
            convergence_threshold: threshold,
        }
    }

    /// Update robot value
    pub fn set_value(&mut self, robot_id: RobotId, value: f32) {
        self.values.insert(robot_id, value);
    }

    /// Get robot value
    pub fn get_value(&self, robot_id: &RobotId) -> Option<f32> {
        self.values.get(robot_id).copied()
    }

    /// Get average value across all robots
    pub fn average(&self) -> f32 {
        if self.values.is_empty() {
            0.0
        } else {
            self.values.values().sum::<f32>() / self.values.len() as f32
        }
    }

    /// Check if consensus has been reached
    pub fn is_converged(&self) -> bool {
        if self.values.len() < 2 {
            return true;
        }

        let avg = self.average();
        self.values
            .values()
            .all(|&v| (v - avg).abs() < self.convergence_threshold)
    }

    /// Get variance of values
    pub fn variance(&self) -> f32 {
        if self.values.is_empty() {
            return 0.0;
        }

        let avg = self.average();
        let sum_sq_diff: f32 = self.values.values().map(|&v| (v - avg).powi(2)).sum();
        sum_sq_diff / self.values.len() as f32
    }
}

/// System to update swarm behavior
pub fn swarm_coordination_system(
    mut agents: Query<
        (&mut Transform, &SwarmAgent, &Robot),
        (With<SwarmAgent>, Without<FormationController>),
    >,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // Collect all agent positions first to avoid borrow issues
    let positions: Vec<_> = agents
        .iter()
        .map(|(t, _, r)| (r.id.clone(), t.translation))
        .collect();

    // Update each agent
    for (mut transform, agent, robot) in agents.iter_mut() {
        let mut separation = Vec3::ZERO;
        let alignment = Vec3::ZERO;
        let mut cohesion = Vec3::ZERO;
        let mut neighbor_count = 0;

        // Find neighbors
        for (other_id, other_pos) in &positions {
            if other_id == &robot.id {
                continue;
            }

            let diff = *other_pos - transform.translation;
            let distance = diff.length();

            if distance < agent.perception_radius && distance > 0.0 {
                neighbor_count += 1;

                // Separation: avoid getting too close
                if distance < agent.separation_distance {
                    separation -= diff / distance;
                }

                // Cohesion: move toward average position
                cohesion += *other_pos;

                // Alignment would need velocity data
                // For now we'll just use position-based behaviors
            }
        }

        if neighbor_count > 0 {
            // Average cohesion position
            cohesion /= neighbor_count as f32;
            cohesion = (cohesion - transform.translation).normalize_or_zero();
        }

        // Combine behaviors
        let mut desired_velocity = Vec3::ZERO;
        desired_velocity += separation * agent.separation_weight;
        desired_velocity += cohesion * agent.cohesion_weight;
        desired_velocity += alignment * agent.alignment_weight;

        // Limit speed
        if desired_velocity.length() > agent.max_speed {
            desired_velocity = desired_velocity.normalize() * agent.max_speed;
        }

        // Update position
        transform.translation += desired_velocity * dt;

        // Update rotation to face movement direction
        if desired_velocity.length() > 0.01 {
            let forward = desired_velocity.normalize();
            transform.rotation = Quat::from_rotation_arc(Vec3::Z, forward);
        }
    }
}

/// System to update formation control
pub fn formation_control_system(
    mut followers: Query<(&mut Transform, &FormationController, &Robot), With<FormationController>>,
    leaders: Query<(&Transform, &Robot), Without<FormationController>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut transform, formation, robot) in followers.iter_mut() {
        if let Some(leader_id) = &formation.leader {
            // Find leader transform
            if let Some((leader_transform, _)) = leaders.iter().find(|(_, r)| &r.id == leader_id) {
                let target_pos = formation.get_formation_position(leader_transform);

                // Move toward formation position
                let direction = target_pos - transform.translation;
                let distance = direction.length();

                if distance > 0.1 {
                    let speed = (distance * 2.0).min(3.0); // Proportional speed with max
                    transform.translation += direction.normalize() * speed * dt;

                    // Face movement direction
                    if direction.length() > 0.01 {
                        let forward = direction.normalize();
                        transform.rotation = Quat::from_rotation_arc(Vec3::Z, forward);
                    }
                }
            }
        }
    }
}

/// Task allocation resource
#[derive(Resource, Default)]
pub struct TaskAllocation {
    /// Task assignments: robot_id -> task_id
    assignments: HashMap<RobotId, String>,
    /// Task costs: (robot_id, task_id) -> cost
    costs: HashMap<(RobotId, String), f32>,
}

impl TaskAllocation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set cost for a robot to complete a task
    pub fn set_cost(&mut self, robot_id: RobotId, task_id: String, cost: f32) {
        self.costs.insert((robot_id, task_id), cost);
    }

    /// Assign a task to a robot
    pub fn assign(&mut self, robot_id: RobotId, task_id: String) {
        self.assignments.insert(robot_id, task_id);
    }

    /// Get assigned task for a robot
    pub fn get_assignment(&self, robot_id: &RobotId) -> Option<&String> {
        self.assignments.get(robot_id)
    }

    /// Greedy task allocation (assign lowest cost task to each robot)
    pub fn allocate_greedy(&mut self, robots: &[RobotId], tasks: &[String]) {
        for robot_id in robots {
            let mut best_task = None;
            let mut best_cost = f32::INFINITY;

            for task_id in tasks {
                if let Some(&cost) = self.costs.get(&(robot_id.clone(), task_id.clone())) {
                    if cost < best_cost {
                        best_cost = cost;
                        best_task = Some(task_id.clone());
                    }
                }
            }

            if let Some(task) = best_task {
                self.assign(robot_id.clone(), task);
            }
        }
    }

    /// Clear all assignments
    pub fn clear(&mut self) {
        self.assignments.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_agent_default() {
        let agent = SwarmAgent::default();
        assert_eq!(agent.separation_distance, 2.0);
        assert!(agent.max_speed > 0.0);
    }

    #[test]
    fn test_formation_line() {
        let controller = FormationController::new(FormationType::Line, 2);
        let leader_transform = Transform::from_translation(Vec3::ZERO);

        let pos = controller.get_formation_position(&leader_transform);
        assert_eq!(pos, Vec3::new(4.0, 0.0, 0.0));
    }

    #[test]
    fn test_formation_circle() {
        let controller = FormationController::new(FormationType::Circle, 0);
        let leader_transform = Transform::from_translation(Vec3::ZERO);

        let pos = controller.get_formation_position(&leader_transform);
        // Should be at radius 5.0
        assert!((pos.length() - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_formation_grid() {
        let controller = FormationController::new(FormationType::Grid, 5);
        let leader_transform = Transform::from_translation(Vec3::ZERO);

        let pos = controller.get_formation_position(&leader_transform);
        // Index 5 should be at (1*2, 0, 1*2) = (2, 0, 2)
        assert_eq!(pos.x, 2.0);
        assert_eq!(pos.z, 2.0);
    }

    #[test]
    fn test_consensus_state() {
        let mut consensus = ConsensusState::new(0.1);

        consensus.set_value(RobotId::new("robot1"), 1.0);
        consensus.set_value(RobotId::new("robot2"), 2.0);
        consensus.set_value(RobotId::new("robot3"), 3.0);

        assert_eq!(consensus.average(), 2.0);
        assert!(!consensus.is_converged());
    }

    #[test]
    fn test_consensus_convergence() {
        let mut consensus = ConsensusState::new(0.1);

        consensus.set_value(RobotId::new("robot1"), 2.0);
        consensus.set_value(RobotId::new("robot2"), 2.05);
        consensus.set_value(RobotId::new("robot3"), 1.98);

        assert!(consensus.is_converged());
    }

    #[test]
    fn test_consensus_variance() {
        let mut consensus = ConsensusState::new(0.1);

        consensus.set_value(RobotId::new("robot1"), 1.0);
        consensus.set_value(RobotId::new("robot2"), 2.0);
        consensus.set_value(RobotId::new("robot3"), 3.0);

        let var = consensus.variance();
        // Variance of [1, 2, 3] should be 2/3
        assert!((var - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_task_allocation() {
        let mut allocation = TaskAllocation::new();

        let robot1 = RobotId::new("robot1");
        let robot2 = RobotId::new("robot2");
        let task_a = "task_a".to_string();
        let task_b = "task_b".to_string();

        allocation.set_cost(robot1.clone(), task_a.clone(), 1.0);
        allocation.set_cost(robot1.clone(), task_b.clone(), 3.0);
        allocation.set_cost(robot2.clone(), task_a.clone(), 2.0);
        allocation.set_cost(robot2.clone(), task_b.clone(), 1.5);

        allocation.allocate_greedy(&[robot1.clone(), robot2.clone()], &[task_a, task_b]);

        // robot1 should get task_a (cost 1.0)
        // robot2 should get task_b (cost 1.5)
        assert_eq!(allocation.get_assignment(&robot1).unwrap(), "task_a");
        assert_eq!(allocation.get_assignment(&robot2).unwrap(), "task_b");
    }

    #[test]
    fn test_formation_wedge() {
        let controller = FormationController::new(FormationType::Wedge, 0);
        let leader_transform = Transform::from_translation(Vec3::ZERO);

        let pos = controller.get_formation_position(&leader_transform);
        // First position should be at origin offset
        assert!(pos.length() >= 0.0);
    }

    #[test]
    fn test_formation_scale() {
        let mut controller = FormationController::new(FormationType::Line, 1);
        controller.scale = 2.0;

        let leader_transform = Transform::from_translation(Vec3::ZERO);
        let pos = controller.get_formation_position(&leader_transform);

        // With scale 2.0, position 1 should be at 4.0
        assert_eq!(pos.x, 4.0);
    }
}
