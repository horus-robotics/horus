//! Scenario save/load system for sim2d
//!
//! This module provides functionality to save and load complete simulation states,
//! including world configuration, robot states, and simulation parameters.

use crate::{Obstacle, RobotConfig, WorldConfig};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Complete scenario definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Scenario format version
    pub version: String,

    /// Scenario name
    pub name: String,

    /// Description of the scenario
    pub description: String,

    /// World configuration
    pub world: WorldState,

    /// Robot states
    pub robots: Vec<RobotState>,

    /// Simulation parameters
    pub simulation: SimulationState,

    /// Optional: Recorded trajectory data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<HashMap<String, Vec<TrajectoryPoint>>>,
}

/// World state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// World width in meters
    pub width: f32,

    /// World height in meters
    pub height: f32,

    /// List of obstacles
    pub obstacles: Vec<Obstacle>,
}

/// Robot state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotState {
    /// Robot name
    pub name: String,

    /// Position [x, y] in meters
    pub position: [f32; 2],

    /// Heading angle in radians
    pub heading: f32,

    /// Velocity [linear, angular]
    pub velocity: [f32; 2],

    /// Path to robot configuration file
    pub config_file: Option<String>,

    /// Inline robot configuration (if no config file)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<RobotConfig>,
}

/// Simulation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationState {
    /// Simulation time in seconds
    pub time: f64,

    /// Timestep in seconds
    pub timestep: f64,

    /// Is simulation paused
    pub paused: bool,
}

/// Single point in a trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    /// Time in seconds
    pub time: f64,

    /// Pose [x, y, theta]
    pub pose: [f32; 3],

    /// Velocity [linear, angular]
    pub velocity: [f32; 2],
}

impl Scenario {
    /// Create a new scenario with default values
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            version: "1.0".to_string(),
            name: name.into(),
            description: description.into(),
            world: WorldState {
                width: 20.0,
                height: 15.0,
                obstacles: Vec::new(),
            },
            robots: Vec::new(),
            simulation: SimulationState {
                time: 0.0,
                timestep: 0.016, // ~60 Hz
                paused: false,
            },
            trajectory: None,
        }
    }

    /// Save scenario to a YAML file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let yaml = serde_yaml::to_string(self).context("Failed to serialize scenario to YAML")?;

        std::fs::write(path, yaml).context(format!("Failed to write scenario to {:?}", path))?;

        Ok(())
    }

    /// Load scenario from a YAML file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let yaml = std::fs::read_to_string(path)
            .context(format!("Failed to read scenario from {:?}", path))?;

        let scenario: Scenario =
            serde_yaml::from_str(&yaml).context("Failed to deserialize scenario from YAML")?;

        // Validate version
        if scenario.version != "1.0" {
            anyhow::bail!("Unsupported scenario version: {}", scenario.version);
        }

        Ok(scenario)
    }

    /// Create scenario from current simulation state
    pub fn from_current_state(
        name: String,
        description: String,
        world_config: &WorldConfig,
        robot_configs: &[RobotConfig],
        time: f64,
    ) -> Self {
        let world = WorldState {
            width: world_config.width,
            height: world_config.height,
            obstacles: world_config.obstacles.clone(),
        };

        let robots = robot_configs
            .iter()
            .map(|config| {
                RobotState {
                    name: config.name.clone(),
                    position: config.position,
                    heading: 0.0, // Would need to extract from physics
                    velocity: [0.0, 0.0],
                    config_file: None,
                    config: Some(config.clone()),
                }
            })
            .collect();

        Self {
            version: "1.0".to_string(),
            name,
            description,
            world,
            robots,
            simulation: SimulationState {
                time,
                timestep: 0.016,
                paused: false,
            },
            trajectory: None,
        }
    }

    /// Convert scenario to WorldConfig
    pub fn to_world_config(&self) -> WorldConfig {
        WorldConfig {
            width: self.world.width,
            height: self.world.height,
            obstacles: self.world.obstacles.clone(),
        }
    }

    /// Convert scenario to robot configs
    pub fn to_robot_configs(&self) -> Vec<RobotConfig> {
        self.robots
            .iter()
            .filter_map(|robot_state| {
                robot_state.config.as_ref().map(|config| {
                    let mut cfg = config.clone();
                    cfg.name = robot_state.name.clone();
                    cfg.position = robot_state.position;
                    cfg
                })
            })
            .collect()
    }

    /// Add trajectory data for a robot
    pub fn add_trajectory(&mut self, robot_name: String, points: Vec<TrajectoryPoint>) {
        if self.trajectory.is_none() {
            self.trajectory = Some(HashMap::new());
        }

        if let Some(ref mut trajectories) = self.trajectory {
            trajectories.insert(robot_name, points);
        }
    }

    /// Get trajectory for a robot
    pub fn get_trajectory(&self, robot_name: &str) -> Option<&Vec<TrajectoryPoint>> {
        self.trajectory.as_ref()?.get(robot_name)
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Self::new("Untitled Scenario", "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObstacleShape;

    #[test]
    fn test_scenario_creation() {
        let scenario = Scenario::new("Test Scenario", "A test scenario");
        assert_eq!(scenario.name, "Test Scenario");
        assert_eq!(scenario.version, "1.0");
        assert!(scenario.robots.is_empty());
    }

    #[test]
    fn test_scenario_serialization() {
        let mut scenario = Scenario::new("Test", "Description");
        scenario.world.obstacles.push(Obstacle {
            pos: [5.0, 5.0],
            shape: ObstacleShape::Rectangle,
            size: [2.0, 1.0],
            color: Some([0.5, 0.5, 0.5]),
        });

        let yaml = serde_yaml::to_string(&scenario).unwrap();
        assert!(yaml.contains("Test"));
        assert!(yaml.contains("obstacles"));
    }

    #[test]
    fn test_scenario_deserialization() {
        let yaml = r#"
version: "1.0"
name: "Test Scenario"
description: "A test"
world:
  width: 20.0
  height: 15.0
  obstacles: []
robots: []
simulation:
  time: 0.0
  timestep: 0.016
  paused: false
"#;

        let scenario: Scenario = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(scenario.name, "Test Scenario");
        assert_eq!(scenario.world.width, 20.0);
    }

    #[test]
    fn test_scenario_save_load_cycle() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_scenario.yaml");

        // Create a scenario with realistic data
        let mut scenario = Scenario::new("Test Save/Load", "Testing scenario persistence");

        // Add obstacles
        scenario.world.obstacles.push(Obstacle {
            pos: [5.0, 5.0],
            shape: ObstacleShape::Rectangle,
            size: [2.0, 1.5],
            color: Some([0.8, 0.2, 0.2]),
        });
        scenario.world.obstacles.push(Obstacle {
            pos: [10.0, 8.0],
            shape: ObstacleShape::Circle,
            size: [1.0, 1.0],
            color: Some([0.2, 0.8, 0.2]),
        });

        // Add robot state
        let robot_config = RobotConfig {
            name: "test_robot".to_string(),
            topic_prefix: "/test".to_string(),
            position: [3.0, 4.0],
            width: 0.6,
            length: 0.9,
            max_speed: 2.5,
            color: [0.0, 0.5, 1.0],
            visual: crate::VisualComponents::default(),
            kinematics: crate::kinematics::KinematicsModel::default(),
            lidar: crate::LidarConfig {
                enabled: true,
                range_max: 10.0,
                range_min: 0.1,
                num_rays: 360,
                angle_min: -std::f32::consts::PI,
                angle_max: std::f32::consts::PI,
            },
            camera: crate::camera::CameraConfig::default(),
        };

        scenario.robots.push(RobotState {
            name: "test_robot".to_string(),
            position: [3.0, 4.0],
            heading: 1.57, // 90 degrees
            velocity: [0.5, 0.1],
            config_file: None,
            config: Some(robot_config.clone()),
        });

        // Set simulation state
        scenario.simulation.time = 5.5;
        scenario.simulation.paused = true;

        // Add trajectory data
        let trajectory_points = vec![
            TrajectoryPoint {
                time: 0.0,
                pose: [0.0, 0.0, 0.0],
                velocity: [0.0, 0.0],
            },
            TrajectoryPoint {
                time: 1.0,
                pose: [1.0, 0.5, 0.3],
                velocity: [0.5, 0.1],
            },
            TrajectoryPoint {
                time: 2.0,
                pose: [2.0, 1.0, 0.6],
                velocity: [0.5, 0.1],
            },
        ];
        scenario.add_trajectory("test_robot".to_string(), trajectory_points);

        // Save the scenario
        scenario
            .save_to_file(&test_path)
            .expect("Failed to save scenario");

        // Verify file exists
        assert!(test_path.exists(), "Scenario file should exist");

        // Load the scenario back
        let loaded_scenario =
            Scenario::load_from_file(&test_path).expect("Failed to load scenario");

        // Verify all fields match
        assert_eq!(loaded_scenario.version, "1.0");
        assert_eq!(loaded_scenario.name, "Test Save/Load");
        assert_eq!(loaded_scenario.description, "Testing scenario persistence");

        // Verify world state
        assert_eq!(loaded_scenario.world.width, 20.0);
        assert_eq!(loaded_scenario.world.height, 15.0);
        assert_eq!(loaded_scenario.world.obstacles.len(), 2);
        assert_eq!(loaded_scenario.world.obstacles[0].pos, [5.0, 5.0]);
        assert_eq!(
            loaded_scenario.world.obstacles[1].shape,
            ObstacleShape::Circle
        );

        // Verify robot state
        assert_eq!(loaded_scenario.robots.len(), 1);
        assert_eq!(loaded_scenario.robots[0].name, "test_robot");
        assert_eq!(loaded_scenario.robots[0].position, [3.0, 4.0]);
        assert_eq!(loaded_scenario.robots[0].heading, 1.57);
        assert_eq!(loaded_scenario.robots[0].velocity, [0.5, 0.1]);

        // Verify robot config
        let loaded_config = loaded_scenario.robots[0].config.as_ref().unwrap();
        assert_eq!(loaded_config.name, "test_robot");
        assert_eq!(loaded_config.max_speed, 2.5);
        assert_eq!(loaded_config.color, [0.0, 0.5, 1.0]);
        assert_eq!(loaded_config.lidar.range_max, 10.0);
        assert_eq!(loaded_config.lidar.range_min, 0.1);

        // Verify simulation state
        assert_eq!(loaded_scenario.simulation.time, 5.5);
        assert_eq!(loaded_scenario.simulation.timestep, 0.016);
        assert_eq!(loaded_scenario.simulation.paused, true);

        // Verify trajectory
        let loaded_trajectory = loaded_scenario
            .get_trajectory("test_robot")
            .expect("Trajectory should exist");
        assert_eq!(loaded_trajectory.len(), 3);
        assert_eq!(loaded_trajectory[0].time, 0.0);
        assert_eq!(loaded_trajectory[1].pose, [1.0, 0.5, 0.3]);
        assert_eq!(loaded_trajectory[2].velocity, [0.5, 0.1]);

        // Test conversion methods
        let world_config = loaded_scenario.to_world_config();
        assert_eq!(world_config.width, 20.0);
        assert_eq!(world_config.obstacles.len(), 2);

        let robot_configs = loaded_scenario.to_robot_configs();
        assert_eq!(robot_configs.len(), 1);
        assert_eq!(robot_configs[0].name, "test_robot");
        assert_eq!(robot_configs[0].position, [3.0, 4.0]);

        println!("✓ Scenario save/load cycle test passed successfully!");
    }

    #[test]
    fn test_scenario_from_current_state() {
        let world_config = WorldConfig {
            width: 25.0,
            height: 20.0,
            obstacles: vec![Obstacle {
                pos: [12.0, 10.0],
                shape: ObstacleShape::Rectangle,
                size: [3.0, 2.0],
                color: None,
            }],
        };

        let robot_configs = vec![RobotConfig {
            name: "robot1".to_string(),
            topic_prefix: "/robot1".to_string(),
            position: [5.0, 5.0],
            width: 0.5,
            length: 0.8,
            max_speed: 2.0,
            color: [1.0, 0.0, 0.0],
            visual: crate::VisualComponents::default(),
            kinematics: crate::kinematics::KinematicsModel::default(),
            lidar: crate::LidarConfig::default(),
            camera: crate::camera::CameraConfig::default(),
        }];

        let scenario = Scenario::from_current_state(
            "Generated Scenario".to_string(),
            "Auto-generated from current state".to_string(),
            &world_config,
            &robot_configs,
            10.5,
        );

        assert_eq!(scenario.name, "Generated Scenario");
        assert_eq!(scenario.world.width, 25.0);
        assert_eq!(scenario.world.obstacles.len(), 1);
        assert_eq!(scenario.robots.len(), 1);
        assert_eq!(scenario.robots[0].name, "robot1");
        assert_eq!(scenario.robots[0].position, [5.0, 5.0]);
        assert_eq!(scenario.simulation.time, 10.5);
    }
}
