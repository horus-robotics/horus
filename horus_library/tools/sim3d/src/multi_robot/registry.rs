//! Robot capability and metadata registry

use super::RobotId;
use bevy::prelude::*;
use std::collections::HashMap;

/// Robot capability definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RobotCapability {
    /// Mobile base
    Mobile,
    /// Manipulator arm
    Manipulation,
    /// Camera sensor
    Vision,
    /// Lidar sensor
    Lidar,
    /// Communication
    Communication,
    /// Custom capability
    Custom(String),
}

/// Robot metadata
#[derive(Debug, Clone)]
pub struct RobotMetadata {
    /// Robot ID
    pub id: RobotId,
    /// Robot type/model
    pub robot_type: String,
    /// Capabilities
    pub capabilities: Vec<RobotCapability>,
    /// Maximum payload (kg)
    pub max_payload: Option<f32>,
    /// Maximum speed (m/s)
    pub max_speed: Option<f32>,
    /// Battery capacity (Wh)
    pub battery_capacity: Option<f32>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

impl RobotMetadata {
    pub fn new(id: RobotId, robot_type: impl Into<String>) -> Self {
        Self {
            id,
            robot_type: robot_type.into(),
            capabilities: Vec::new(),
            max_payload: None,
            max_speed: None,
            battery_capacity: None,
            custom: HashMap::new(),
        }
    }

    /// Add a capability
    pub fn with_capability(mut self, capability: RobotCapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Add multiple capabilities
    pub fn with_capabilities(mut self, capabilities: Vec<RobotCapability>) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    /// Set maximum payload
    pub fn with_payload(mut self, payload: f32) -> Self {
        self.max_payload = Some(payload);
        self
    }

    /// Set maximum speed
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.max_speed = Some(speed);
        self
    }

    /// Set battery capacity
    pub fn with_battery(mut self, capacity: f32) -> Self {
        self.battery_capacity = Some(capacity);
        self
    }

    /// Add custom metadata
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// Check if robot has a capability
    pub fn has_capability(&self, capability: &RobotCapability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get custom metadata value
    pub fn get_custom(&self, key: &str) -> Option<&String> {
        self.custom.get(key)
    }
}

/// Robot registry resource
#[derive(Resource, Default)]
pub struct RobotRegistry {
    /// Metadata per robot
    metadata: HashMap<RobotId, RobotMetadata>,
    /// Capability index for fast lookup
    capability_index: HashMap<RobotCapability, Vec<RobotId>>,
}

impl RobotRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register robot metadata
    pub fn register(&mut self, metadata: RobotMetadata) {
        let robot_id = metadata.id.clone();

        // Update capability index
        for capability in &metadata.capabilities {
            self.capability_index
                .entry(capability.clone())
                .or_default()
                .push(robot_id.clone());
        }

        self.metadata.insert(robot_id, metadata);
    }

    /// Unregister a robot
    pub fn unregister(&mut self, robot_id: &RobotId) -> Option<RobotMetadata> {
        if let Some(metadata) = self.metadata.remove(robot_id) {
            // Remove from capability index
            for capability in &metadata.capabilities {
                if let Some(robots) = self.capability_index.get_mut(capability) {
                    robots.retain(|id| id != robot_id);
                }
            }
            Some(metadata)
        } else {
            None
        }
    }

    /// Get robot metadata
    pub fn get(&self, robot_id: &RobotId) -> Option<&RobotMetadata> {
        self.metadata.get(robot_id)
    }

    /// Get mutable robot metadata
    pub fn get_mut(&mut self, robot_id: &RobotId) -> Option<&mut RobotMetadata> {
        self.metadata.get_mut(robot_id)
    }

    /// Find robots with a specific capability
    pub fn find_by_capability(&self, capability: &RobotCapability) -> Vec<RobotId> {
        self.capability_index
            .get(capability)
            .cloned()
            .unwrap_or_default()
    }

    /// Find robots by type
    pub fn find_by_type(&self, robot_type: &str) -> Vec<RobotId> {
        self.metadata
            .values()
            .filter(|m| m.robot_type == robot_type)
            .map(|m| m.id.clone())
            .collect()
    }

    /// Get all registered robot IDs
    pub fn all_robots(&self) -> Vec<RobotId> {
        self.metadata.keys().cloned().collect()
    }

    /// Get robot count
    pub fn count(&self) -> usize {
        self.metadata.len()
    }

    /// Clear all registrations
    pub fn clear(&mut self) {
        self.metadata.clear();
        self.capability_index.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robot_metadata() {
        let metadata = RobotMetadata::new(RobotId::new("robot1"), "turtlebot3")
            .with_capability(RobotCapability::Mobile)
            .with_capability(RobotCapability::Vision)
            .with_speed(0.5)
            .with_payload(2.0);

        assert_eq!(metadata.robot_type, "turtlebot3");
        assert_eq!(metadata.capabilities.len(), 2);
        assert!(metadata.has_capability(&RobotCapability::Mobile));
        assert_eq!(metadata.max_speed, Some(0.5));
    }

    #[test]
    fn test_robot_metadata_custom() {
        let metadata = RobotMetadata::new(RobotId::new("robot1"), "custom")
            .with_custom("color", "red")
            .with_custom("version", "1.0");

        assert_eq!(metadata.get_custom("color"), Some(&"red".to_string()));
        assert_eq!(metadata.get_custom("version"), Some(&"1.0".to_string()));
        assert_eq!(metadata.get_custom("missing"), None);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = RobotRegistry::new();

        let metadata = RobotMetadata::new(RobotId::new("robot1"), "turtlebot3")
            .with_capability(RobotCapability::Mobile);

        registry.register(metadata);

        assert_eq!(registry.count(), 1);
        assert!(registry.get(&RobotId::new("robot1")).is_some());
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = RobotRegistry::new();

        let metadata = RobotMetadata::new(RobotId::new("robot1"), "turtlebot3");
        registry.register(metadata);

        assert_eq!(registry.count(), 1);

        let removed = registry.unregister(&RobotId::new("robot1"));
        assert!(removed.is_some());
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_find_by_capability() {
        let mut registry = RobotRegistry::new();

        let metadata1 = RobotMetadata::new(RobotId::new("robot1"), "turtlebot3")
            .with_capability(RobotCapability::Mobile);

        let metadata2 = RobotMetadata::new(RobotId::new("robot2"), "ur5e")
            .with_capability(RobotCapability::Manipulation);

        let metadata3 = RobotMetadata::new(RobotId::new("robot3"), "turtlebot3")
            .with_capability(RobotCapability::Mobile);

        registry.register(metadata1);
        registry.register(metadata2);
        registry.register(metadata3);

        let mobile_robots = registry.find_by_capability(&RobotCapability::Mobile);
        assert_eq!(mobile_robots.len(), 2);
        assert!(mobile_robots.contains(&RobotId::new("robot1")));
        assert!(mobile_robots.contains(&RobotId::new("robot3")));

        let manip_robots = registry.find_by_capability(&RobotCapability::Manipulation);
        assert_eq!(manip_robots.len(), 1);
        assert!(manip_robots.contains(&RobotId::new("robot2")));
    }

    #[test]
    fn test_registry_find_by_type() {
        let mut registry = RobotRegistry::new();

        registry.register(RobotMetadata::new(RobotId::new("robot1"), "turtlebot3"));
        registry.register(RobotMetadata::new(RobotId::new("robot2"), "ur5e"));
        registry.register(RobotMetadata::new(RobotId::new("robot3"), "turtlebot3"));

        let turtlebots = registry.find_by_type("turtlebot3");
        assert_eq!(turtlebots.len(), 2);

        let arms = registry.find_by_type("ur5e");
        assert_eq!(arms.len(), 1);
    }

    #[test]
    fn test_registry_all_robots() {
        let mut registry = RobotRegistry::new();

        registry.register(RobotMetadata::new(RobotId::new("robot1"), "type1"));
        registry.register(RobotMetadata::new(RobotId::new("robot2"), "type2"));
        registry.register(RobotMetadata::new(RobotId::new("robot3"), "type3"));

        let all = registry.all_robots();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_capability_custom() {
        let cap1 = RobotCapability::Custom("grasping".to_string());
        let cap2 = RobotCapability::Custom("grasping".to_string());
        let cap3 = RobotCapability::Custom("welding".to_string());

        assert_eq!(cap1, cap2);
        assert_ne!(cap1, cap3);
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = RobotRegistry::new();

        registry.register(RobotMetadata::new(RobotId::new("robot1"), "type1"));
        registry.register(RobotMetadata::new(RobotId::new("robot2"), "type2"));
        assert_eq!(registry.count(), 2);

        registry.clear();
        assert_eq!(registry.count(), 0);
    }
}
