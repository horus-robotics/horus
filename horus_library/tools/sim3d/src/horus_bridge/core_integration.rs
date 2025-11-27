//! Integration with horus_core for actual HORUS communication
//!
//! This module bridges sim3d's Bevy-based simulation with horus_core's
//! Node/Hub system for real inter-process communication.

use bevy::prelude::*;

use super::messages::*;
use crate::physics::rigid_body::Velocity as PhysicsVelocity;

/// Simulated robot node that can receive commands and publish data
/// Uses local message types for Bevy integration
#[derive(Resource, Default)]
pub struct SimulatedRobots {
    robots: Vec<SimulatedRobotData>,
}

/// Data for a simulated robot (Bevy-local)
pub struct SimulatedRobotData {
    pub name: String,
    pub last_cmd_vel: Twist,
    pub current_transform: Transform,
    pub current_velocity: Vec3,
}

impl SimulatedRobotData {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            last_cmd_vel: Twist::default(),
            current_transform: Transform::default(),
            current_velocity: Vec3::ZERO,
        }
    }

    /// Get the robot name
    pub fn robot_name(&self) -> &str {
        &self.name
    }

    /// Get the latest command velocity
    pub fn get_cmd_vel(&self) -> &Twist {
        &self.last_cmd_vel
    }

    /// Set command velocity (called by external bridge)
    pub fn set_cmd_vel(&mut self, twist: Twist) {
        self.last_cmd_vel = twist;
    }

    /// Update current transform
    pub fn update_transform(&mut self, transform: Transform) {
        self.current_transform = transform;
    }

    /// Update current velocity
    pub fn update_velocity(&mut self, velocity: Vec3) {
        self.current_velocity = velocity;
    }
}

impl SimulatedRobots {
    pub fn new() -> Self {
        Self { robots: Vec::new() }
    }

    /// Add a new simulated robot
    pub fn add_robot(&mut self, name: impl Into<String>) -> usize {
        let index = self.robots.len();
        self.robots.push(SimulatedRobotData::new(name));
        index
    }

    /// Get a robot by index
    pub fn get(&self, index: usize) -> Option<&SimulatedRobotData> {
        self.robots.get(index)
    }

    /// Get a mutable robot by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut SimulatedRobotData> {
        self.robots.get_mut(index)
    }

    /// Find a robot by name
    pub fn find_by_name(&self, name: &str) -> Option<&SimulatedRobotData> {
        self.robots.iter().find(|r| r.name == name)
    }

    /// Find a mutable robot by name
    pub fn find_by_name_mut(&mut self, name: &str) -> Option<&mut SimulatedRobotData> {
        self.robots.iter_mut().find(|r| r.name == name)
    }

    /// Iterate over all robots
    pub fn iter(&self) -> impl Iterator<Item = &SimulatedRobotData> {
        self.robots.iter()
    }

    /// Iterate mutably over all robots
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut SimulatedRobotData> {
        self.robots.iter_mut()
    }

    /// Get the number of robots
    pub fn len(&self) -> usize {
        self.robots.len()
    }

    /// Check if there are no robots
    pub fn is_empty(&self) -> bool {
        self.robots.is_empty()
    }
}

/// System to sync simulated robots with Bevy entities
pub fn sync_robots_system(
    mut robots: ResMut<SimulatedRobots>,
    query: Query<(&Transform, &Name), With<crate::robot::robot::Robot>>,
) {
    for (transform, name) in query.iter() {
        if let Some(robot) = robots.find_by_name_mut(name.as_str()) {
            robot.update_transform(*transform);
        }
    }
}

/// System to apply cmd_vel from simulated robots to Bevy entities
pub fn apply_cmd_vel_system(
    robots: Res<SimulatedRobots>,
    mut query: Query<(&Name, &mut PhysicsVelocity), With<crate::robot::robot::Robot>>,
) {
    for (name, mut velocity) in query.iter_mut() {
        if let Some(robot) = robots.find_by_name(name.as_str()) {
            let twist = robot.get_cmd_vel();
            velocity.linear = Vec3::new(twist.linear.x, twist.linear.y, twist.linear.z);
            velocity.angular = Vec3::new(twist.angular.x, twist.angular.y, twist.angular.z);
        }
    }
}

/// HORUS Core Bridge Plugin
/// Adds resources for tracking simulated robots and syncing with HORUS
pub struct HorusCorePlugin;

impl Plugin for HorusCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulatedRobots>()
            .add_systems(Update, sync_robots_system);
    }
}

/// Network bridge configuration for external HORUS communication
#[derive(Resource, Clone)]
pub struct HorusNetworkConfig {
    /// Enable network bridge
    pub enabled: bool,
    /// Multicast address for discovery
    pub multicast_addr: String,
    /// Port for communication
    pub port: u16,
}

impl Default for HorusNetworkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            multicast_addr: "239.255.255.250".to_string(),
            port: 5000,
        }
    }
}

/// Create a topic name following HORUS conventions (dot notation)
pub fn horus_topic(robot_name: &str, topic: &str) -> String {
    format!("{}.{}", robot_name, topic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulated_robot_data() {
        let robot = SimulatedRobotData::new("test_robot");
        assert_eq!(robot.robot_name(), "test_robot");
    }

    #[test]
    fn test_simulated_robots_resource() {
        let mut robots = SimulatedRobots::new();
        let idx = robots.add_robot("robot1");
        assert_eq!(idx, 0);
        assert!(robots.get(0).is_some());
        assert_eq!(robots.len(), 1);
    }

    #[test]
    fn test_robot_cmd_vel() {
        let mut robot = SimulatedRobotData::new("test_robot");
        let twist = Twist {
            linear: Vector3Message {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            angular: Vector3Message {
                x: 0.0,
                y: 0.0,
                z: 0.5,
            },
        };
        robot.set_cmd_vel(twist.clone());
        assert_eq!(robot.get_cmd_vel().linear.x, 1.0);
        assert_eq!(robot.get_cmd_vel().angular.z, 0.5);
    }

    #[test]
    fn test_horus_topic() {
        let topic = horus_topic("robot1", "cmd_vel");
        assert_eq!(topic, "robot1.cmd_vel");
    }
}
