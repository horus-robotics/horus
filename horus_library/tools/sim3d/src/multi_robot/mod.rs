//! Multi-robot simulation support with namespaces and coordination

pub mod communication;
pub mod coordination;
pub mod network;
pub mod registry;
pub mod sync;

use bevy::prelude::*;
use std::collections::HashMap;

/// Robot namespace identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct RobotId(pub String);

impl RobotId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for RobotId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for RobotId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Component marking a robot entity
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Robot {
    /// Unique robot identifier
    pub id: RobotId,
    /// Robot type/model name
    pub robot_type: String,
    /// Whether this robot is active
    pub active: bool,
}

impl Robot {
    pub fn new(id: impl Into<RobotId>, robot_type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            robot_type: robot_type.into(),
            active: true,
        }
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

/// Multi-robot scene manager resource
#[derive(Resource, Default)]
pub struct MultiRobotManager {
    /// Map of robot IDs to entity IDs
    robots: HashMap<RobotId, Entity>,
    /// Maximum number of robots allowed
    max_robots: usize,
}

impl MultiRobotManager {
    pub fn new(max_robots: usize) -> Self {
        Self {
            robots: HashMap::new(),
            max_robots,
        }
    }

    /// Register a robot
    pub fn register(&mut self, id: RobotId, entity: Entity) -> Result<(), String> {
        if self.robots.len() >= self.max_robots {
            return Err(format!("Maximum robots ({}) reached", self.max_robots));
        }

        if self.robots.contains_key(&id) {
            return Err(format!("Robot ID {:?} already exists", id));
        }

        self.robots.insert(id, entity);
        Ok(())
    }

    /// Unregister a robot
    pub fn unregister(&mut self, id: &RobotId) -> Option<Entity> {
        self.robots.remove(id)
    }

    /// Get robot entity by ID
    pub fn get_robot(&self, id: &RobotId) -> Option<Entity> {
        self.robots.get(id).copied()
    }

    /// Get all robot IDs
    pub fn get_all_ids(&self) -> Vec<RobotId> {
        self.robots.keys().cloned().collect()
    }

    /// Get number of registered robots
    pub fn count(&self) -> usize {
        self.robots.len()
    }

    /// Check if robot exists
    pub fn contains(&self, id: &RobotId) -> bool {
        self.robots.contains_key(id)
    }

    /// Clear all robots
    pub fn clear(&mut self) {
        self.robots.clear();
    }
}

/// System to automatically register robots
pub fn register_robots_system(
    mut manager: ResMut<MultiRobotManager>,
    robots: Query<(Entity, &Robot), Added<Robot>>,
) {
    for (entity, robot) in robots.iter() {
        if let Err(e) = manager.register(robot.id.clone(), entity) {
            warn!("Failed to register robot {:?}: {}", robot.id, e);
        } else {
            info!("Registered robot: {:?}", robot.id);
        }
    }
}

/// System to unregister removed robots
pub fn unregister_robots_system(
    mut manager: ResMut<MultiRobotManager>,
    mut removed: RemovedComponents<Robot>,
    robots: Query<&Robot>,
) {
    for entity in removed.read() {
        // Find robot ID by entity (since component is removed, we need to search)
        let robot_id = manager
            .robots
            .iter()
            .find(|(_, &e)| e == entity)
            .map(|(id, _)| id.clone());

        if let Some(id) = robot_id {
            manager.unregister(&id);
            info!("Unregistered robot: {:?}", id);
        }
    }
}

/// Multi-robot plugin
pub struct MultiRobotPlugin {
    pub max_robots: usize,
}

impl Default for MultiRobotPlugin {
    fn default() -> Self {
        Self { max_robots: 100 }
    }
}

impl Plugin for MultiRobotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MultiRobotManager::new(self.max_robots))
            .insert_resource(communication::CommunicationManager::default())
            .insert_resource(network::NetworkSimulator::default())
            .insert_resource(sync::SynchronizationManager::default())
            .insert_resource(registry::RobotRegistry::default())
            .insert_resource(coordination::ConsensusState::default())
            .insert_resource(coordination::TaskAllocation::default())
            .add_systems(
                Update,
                (
                    register_robots_system,
                    unregister_robots_system,
                    communication::communication_system,
                    network::network_simulation_system,
                    coordination::swarm_coordination_system,
                    coordination::formation_control_system,
                    sync::lock_step_sync_system,
                ),
            )
            .register_type::<Robot>()
            .register_type::<RobotId>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robot_id_creation() {
        let id1 = RobotId::new("robot_1");
        let id2 = RobotId::from("robot_2");
        let id3 = RobotId::from(String::from("robot_3"));

        assert_eq!(id1.as_str(), "robot_1");
        assert_eq!(id2.as_str(), "robot_2");
        assert_eq!(id3.as_str(), "robot_3");
    }

    #[test]
    fn test_robot_component() {
        let robot = Robot::new("robot_1", "turtlebot3");
        assert_eq!(robot.id.as_str(), "robot_1");
        assert_eq!(robot.robot_type, "turtlebot3");
        assert!(robot.active);

        let inactive = Robot::new("robot_2", "ur5e").with_active(false);
        assert!(!inactive.active);
    }

    #[test]
    fn test_multi_robot_manager() {
        let mut manager = MultiRobotManager::new(10);
        let entity = Entity::from_raw(1);
        let id = RobotId::new("robot_1");

        assert_eq!(manager.count(), 0);

        manager.register(id.clone(), entity).unwrap();
        assert_eq!(manager.count(), 1);
        assert!(manager.contains(&id));
        assert_eq!(manager.get_robot(&id), Some(entity));

        manager.unregister(&id);
        assert_eq!(manager.count(), 0);
        assert!(!manager.contains(&id));
    }

    #[test]
    fn test_manager_max_robots() {
        let mut manager = MultiRobotManager::new(2);

        manager
            .register(RobotId::new("robot_1"), Entity::from_raw(1))
            .unwrap();
        manager
            .register(RobotId::new("robot_2"), Entity::from_raw(2))
            .unwrap();

        let result = manager.register(RobotId::new("robot_3"), Entity::from_raw(3));
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_duplicate_id() {
        let mut manager = MultiRobotManager::new(10);
        let id = RobotId::new("robot_1");

        manager.register(id.clone(), Entity::from_raw(1)).unwrap();
        let result = manager.register(id, Entity::from_raw(2));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_all_ids() {
        let mut manager = MultiRobotManager::new(10);

        manager
            .register(RobotId::new("robot_1"), Entity::from_raw(1))
            .unwrap();
        manager
            .register(RobotId::new("robot_2"), Entity::from_raw(2))
            .unwrap();

        let ids = manager.get_all_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&RobotId::new("robot_1")));
        assert!(ids.contains(&RobotId::new("robot_2")));
    }
}
