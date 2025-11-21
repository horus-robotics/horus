//! Navigation benchmarks in cluttered environments

use bevy::prelude::*;

/// Navigation test scenario
pub struct NavigationScenario {
    pub name: String,
    pub obstacle_count: usize,
    pub path_length: f32,
    pub robot_speed: f32,
}

impl NavigationScenario {
    pub fn simple() -> Self {
        Self {
            name: "Simple Navigation".to_string(),
            obstacle_count: 10,
            path_length: 10.0,
            robot_speed: 1.0,
        }
    }

    pub fn cluttered() -> Self {
        Self {
            name: "Cluttered Environment".to_string(),
            obstacle_count: 50,
            path_length: 20.0,
            robot_speed: 0.5,
        }
    }

    pub fn maze() -> Self {
        Self {
            name: "Maze Navigation".to_string(),
            obstacle_count: 100,
            path_length: 50.0,
            robot_speed: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_scenario_simple() {
        let scenario = NavigationScenario::simple();
        assert_eq!(scenario.obstacle_count, 10);
        assert_eq!(scenario.path_length, 10.0);
    }

    #[test]
    fn test_navigation_scenario_cluttered() {
        let scenario = NavigationScenario::cluttered();
        assert_eq!(scenario.obstacle_count, 50);
    }

    #[test]
    fn test_navigation_scenario_maze() {
        let scenario = NavigationScenario::maze();
        assert_eq!(scenario.obstacle_count, 100);
    }
}
