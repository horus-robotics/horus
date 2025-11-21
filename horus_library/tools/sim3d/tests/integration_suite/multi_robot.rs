//! Multi-robot coordination benchmarks

use bevy::prelude::*;

/// Multi-robot test scenario
pub struct MultiRobotScenario {
    pub name: String,
    pub robot_count: usize,
    pub formation_type: FormationType,
    pub communication_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormationType {
    Swarm,
    Formation,
    Independent,
}

impl MultiRobotScenario {
    pub fn small_swarm() -> Self {
        Self {
            name: "Small Swarm (5 robots)".to_string(),
            robot_count: 5,
            formation_type: FormationType::Swarm,
            communication_enabled: true,
        }
    }

    pub fn large_swarm() -> Self {
        Self {
            name: "Large Swarm (20 robots)".to_string(),
            robot_count: 20,
            formation_type: FormationType::Swarm,
            communication_enabled: true,
        }
    }

    pub fn formation_control() -> Self {
        Self {
            name: "Formation Control (10 robots)".to_string(),
            robot_count: 10,
            formation_type: FormationType::Formation,
            communication_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_robot_scenario_small() {
        let scenario = MultiRobotScenario::small_swarm();
        assert_eq!(scenario.robot_count, 5);
        assert_eq!(scenario.formation_type, FormationType::Swarm);
    }

    #[test]
    fn test_multi_robot_scenario_large() {
        let scenario = MultiRobotScenario::large_swarm();
        assert_eq!(scenario.robot_count, 20);
    }

    #[test]
    fn test_multi_robot_scenario_formation() {
        let scenario = MultiRobotScenario::formation_control();
        assert_eq!(scenario.robot_count, 10);
        assert_eq!(scenario.formation_type, FormationType::Formation);
    }
}
