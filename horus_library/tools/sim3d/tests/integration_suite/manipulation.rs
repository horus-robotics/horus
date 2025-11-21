//! Manipulation benchmarks (pick and place)

use bevy::prelude::*;

/// Manipulation test scenario
pub struct ManipulationScenario {
    pub name: String,
    pub object_count: usize,
    pub object_mass: f32,
    pub grasp_accuracy: f32,
}

impl ManipulationScenario {
    pub fn simple_pick() -> Self {
        Self {
            name: "Simple Pick and Place".to_string(),
            object_count: 1,
            object_mass: 0.1,
            grasp_accuracy: 0.01,
        }
    }

    pub fn multi_object() -> Self {
        Self {
            name: "Multi-Object Sorting".to_string(),
            object_count: 10,
            object_mass: 0.2,
            grasp_accuracy: 0.005,
        }
    }

    pub fn heavy_object() -> Self {
        Self {
            name: "Heavy Object Manipulation".to_string(),
            object_count: 1,
            object_mass: 5.0,
            grasp_accuracy: 0.02,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manipulation_scenario_simple() {
        let scenario = ManipulationScenario::simple_pick();
        assert_eq!(scenario.object_count, 1);
        assert_eq!(scenario.object_mass, 0.1);
    }

    #[test]
    fn test_manipulation_scenario_multi() {
        let scenario = ManipulationScenario::multi_object();
        assert_eq!(scenario.object_count, 10);
    }

    #[test]
    fn test_manipulation_scenario_heavy() {
        let scenario = ManipulationScenario::heavy_object();
        assert_eq!(scenario.object_mass, 5.0);
    }
}
