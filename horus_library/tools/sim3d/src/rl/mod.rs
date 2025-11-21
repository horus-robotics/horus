use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod adversarial;
pub mod controllers;
pub mod curriculum;
pub mod domain_randomization;
pub mod reward_shaping;
pub mod tasks;

#[cfg(feature = "python")]
pub mod python;

/// Observation space for RL tasks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    /// Flat vector of observations
    pub data: Vec<f32>,
    /// Dimensions of observation (for reshaping)
    pub shape: Vec<usize>,
}

impl Observation {
    pub fn new(data: Vec<f32>) -> Self {
        let len = data.len();
        Self {
            data,
            shape: vec![len],
        }
    }

    pub fn with_shape(data: Vec<f32>, shape: Vec<usize>) -> Self {
        Self { data, shape }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Action space for RL tasks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    /// Continuous actions (e.g., joint torques, velocities)
    Continuous(Vec<f32>),
    /// Discrete actions (e.g., button presses)
    Discrete(usize),
    /// Multi-discrete actions (e.g., multiple independent discrete choices)
    MultiDiscrete(Vec<usize>),
}

impl Action {
    pub fn continuous(data: Vec<f32>) -> Self {
        Action::Continuous(data)
    }

    pub fn discrete(action: usize) -> Self {
        Action::Discrete(action)
    }

    pub fn multi_discrete(actions: Vec<usize>) -> Self {
        Action::MultiDiscrete(actions)
    }
}

/// RL task configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Maximum episode steps
    pub max_steps: usize,
    /// Time step duration (seconds)
    pub dt: f32,
    /// Observation dimension
    pub obs_dim: usize,
    /// Action dimension
    pub action_dim: usize,
    /// Task-specific parameters
    pub parameters: TaskParameters,
}

/// Task-specific parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskParameters {
    Reaching {
        target_tolerance: f32,
        max_distance: f32,
    },
    Balancing {
        angle_limit: f32,
        position_limit: f32,
    },
    Locomotion {
        target_velocity: Vec3,
        height_limit: f32,
    },
    Navigation {
        goal_tolerance: f32,
        max_distance: f32,
    },
    Manipulation {
        target_tolerance: f32,
        grasp_threshold: f32,
    },
    Push {
        target_tolerance: f32,
        object_velocity_bonus: f32,
    },
}

/// Episode information
#[derive(Clone, Debug, Default)]
pub struct EpisodeInfo {
    pub total_reward: f32,
    pub steps: usize,
    pub success: bool,
    pub termination_reason: TerminationReason,
}

#[derive(Clone, Debug, Default)]
pub enum TerminationReason {
    #[default]
    None,
    Success,
    MaxSteps,
    Failure,
    OutOfBounds,
}

/// Trait for RL tasks
pub trait RLTask: Send + Sync {
    /// Get task configuration
    fn config(&self) -> &TaskConfig;

    /// Reset the environment to initial state
    fn reset(&mut self, world: &mut World) -> Observation;

    /// Execute an action and return (observation, reward, done, info)
    fn step(&mut self, world: &mut World, action: &Action) -> StepResult;

    /// Get current observation
    fn get_observation(&self, world: &mut World) -> Observation;

    /// Compute reward for current state
    fn compute_reward(&self, world: &mut World) -> f32;

    /// Check if episode is done
    fn is_done(&self, world: &mut World) -> bool;

    /// Get episode information
    fn get_info(&self) -> EpisodeInfo;

    /// Render the task (for visualization)
    fn render(&self, _gizmos: &mut Gizmos, _world: &mut World) {}
}

/// Result of a step in the environment
#[derive(Clone, Debug)]
pub struct StepResult {
    pub observation: Observation,
    pub reward: f32,
    pub done: bool,
    pub truncated: bool,
    pub info: EpisodeInfo,
}

/// Resource to manage RL tasks
#[derive(Resource)]
pub struct RLTaskManager {
    pub current_task: Option<Box<dyn RLTask>>,
    pub episode_count: usize,
    pub total_steps: usize,
}

impl Default for RLTaskManager {
    fn default() -> Self {
        Self {
            current_task: None,
            episode_count: 0,
            total_steps: 0,
        }
    }
}

impl RLTaskManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_task(&mut self, task: Box<dyn RLTask>) {
        self.current_task = Some(task);
        self.episode_count = 0;
        self.total_steps = 0;
    }

    pub fn reset(&mut self, world: &mut World) -> Option<Observation> {
        if let Some(task) = &mut self.current_task {
            self.episode_count += 1;
            Some(task.reset(world))
        } else {
            None
        }
    }

    pub fn step(&mut self, world: &mut World, action: &Action) -> Option<StepResult> {
        if let Some(task) = &mut self.current_task {
            self.total_steps += 1;
            Some(task.step(world, action))
        } else {
            None
        }
    }

    pub fn render(&self, gizmos: &mut Gizmos, world: &mut World) {
        if let Some(task) = &self.current_task {
            task.render(gizmos, world);
        }
    }
}

/// System to render current RL task
pub fn rl_task_render_system(
    mut gizmos: Gizmos,
    world: &mut World,
    task_manager: Res<RLTaskManager>,
) {
    task_manager.render(&mut gizmos, world);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observation_creation() {
        let obs = Observation::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(obs.len(), 3);
        assert_eq!(obs.shape, vec![3]);
    }

    #[test]
    fn test_action_types() {
        let continuous = Action::continuous(vec![0.5, -0.5]);
        let discrete = Action::discrete(2);
        let multi_discrete = Action::multi_discrete(vec![1, 0, 2]);

        match continuous {
            Action::Continuous(data) => assert_eq!(data.len(), 2),
            _ => panic!("Wrong action type"),
        }

        match discrete {
            Action::Discrete(action) => assert_eq!(action, 2),
            _ => panic!("Wrong action type"),
        }

        match multi_discrete {
            Action::MultiDiscrete(actions) => assert_eq!(actions.len(), 3),
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_task_manager_default() {
        let manager = RLTaskManager::default();
        assert_eq!(manager.episode_count, 0);
        assert_eq!(manager.total_steps, 0);
        assert!(manager.current_task.is_none());
    }
}
