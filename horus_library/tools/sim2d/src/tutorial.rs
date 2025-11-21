//! Interactive tutorial system for learning HORUS and sim2d
//!
//! Provides step-by-step tutorials with automatic progress tracking.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// A complete tutorial with multiple steps
#[derive(Clone, Serialize, Deserialize)]
pub struct Tutorial {
    pub id: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<TutorialStep>,
    pub current_step: usize,
    pub completed: bool,
}

/// A single step in a tutorial
#[derive(Clone, Serialize, Deserialize)]
pub struct TutorialStep {
    pub title: String,
    pub instruction: String,
    pub hint: Option<String>,
    pub action_type: TutorialActionType,
    pub completed: bool,
}

/// Types of actions that can complete a tutorial step
#[derive(Clone, Serialize, Deserialize)]
pub enum TutorialActionType {
    /// Wait for simulation to start
    StartSimulation,
    /// Send a command to a topic
    SendCommand { topic: String, min_duration: f32 },
    /// Reach a position within threshold
    ReachPosition { x: f32, y: f32, threshold: f32 },
    /// Avoid collisions for a duration
    AvoidCollision { duration: f32 },
    /// Complete manual action
    ManualComplete,
}

/// Resource for tracking tutorial state
#[derive(Resource)]
pub struct TutorialState {
    pub active_tutorial: Option<Tutorial>,
    pub completed_tutorials: Vec<String>,
    pub show_tutorial_panel: bool,

    // Progress tracking
    pub simulation_running: bool,
    pub last_command_time: f32,
    pub command_start_time: Option<f32>,
    pub current_position: Vec2,
    pub collision_free_since: Option<f32>,
    pub elapsed_time: f32,
}

impl Default for TutorialState {
    fn default() -> Self {
        Self {
            active_tutorial: None,
            completed_tutorials: Vec::new(),
            show_tutorial_panel: false,
            simulation_running: false,
            last_command_time: 0.0,
            command_start_time: None,
            current_position: Vec2::ZERO,
            collision_free_since: None,
            elapsed_time: 0.0,
        }
    }
}

impl Tutorial {
    /// Check if a step is complete based on current state
    pub fn check_step_completion(&mut self, state: &TutorialState) -> bool {
        if self.completed || self.current_step >= self.steps.len() {
            return false;
        }

        let step = &mut self.steps[self.current_step];
        if step.completed {
            return false;
        }

        let complete = match &step.action_type {
            TutorialActionType::StartSimulation => state.simulation_running,
            TutorialActionType::SendCommand { min_duration, .. } => {
                if let Some(start) = state.command_start_time {
                    state.elapsed_time - start >= *min_duration
                } else {
                    false
                }
            }
            TutorialActionType::ReachPosition { x, y, threshold } => {
                let target = Vec2::new(*x, *y);
                state.current_position.distance(target) <= *threshold
            }
            TutorialActionType::AvoidCollision { duration } => {
                if let Some(since) = state.collision_free_since {
                    state.elapsed_time - since >= *duration
                } else {
                    false
                }
            }
            TutorialActionType::ManualComplete => {
                false // Must be completed manually via UI
            }
        };

        if complete {
            step.completed = true;
            self.current_step += 1;

            if self.current_step >= self.steps.len() {
                self.completed = true;
                return true;
            }
        }

        false
    }

    /// Get the current step, if any
    pub fn current_step(&self) -> Option<&TutorialStep> {
        if self.current_step < self.steps.len() {
            Some(&self.steps[self.current_step])
        } else {
            None
        }
    }

    /// Get progress as percentage
    pub fn progress(&self) -> f32 {
        if self.steps.is_empty() {
            return 1.0;
        }
        self.current_step as f32 / self.steps.len() as f32
    }
}

impl TutorialState {
    /// Start a tutorial
    pub fn start_tutorial(&mut self, tutorial: Tutorial) {
        self.active_tutorial = Some(tutorial);
        self.show_tutorial_panel = true;

        // Reset progress tracking
        self.command_start_time = None;
        self.collision_free_since = Some(self.elapsed_time);
    }

    /// Complete the current tutorial step manually
    pub fn complete_current_step(&mut self) {
        if let Some(tutorial) = &mut self.active_tutorial {
            if tutorial.current_step < tutorial.steps.len() {
                tutorial.steps[tutorial.current_step].completed = true;
                tutorial.current_step += 1;

                if tutorial.current_step >= tutorial.steps.len() {
                    tutorial.completed = true;
                    self.completed_tutorials.push(tutorial.id.clone());
                }
            }
        }
    }

    /// Stop the current tutorial
    pub fn stop_tutorial(&mut self) {
        if let Some(tutorial) = &self.active_tutorial {
            if tutorial.completed {
                self.completed_tutorials.push(tutorial.id.clone());
            }
        }
        self.active_tutorial = None;
    }

    /// Check if a tutorial has been completed
    pub fn is_completed(&self, id: &str) -> bool {
        self.completed_tutorials.contains(&id.to_string())
    }

    /// Update tutorial progress
    pub fn update(&mut self, dt: f32) {
        self.elapsed_time += dt;

        // Take tutorial out temporarily to avoid borrow checker issues
        if let Some(mut tutorial) = self.active_tutorial.take() {
            tutorial.check_step_completion(self);
            self.active_tutorial = Some(tutorial);
        }
    }

    /// Record that a command was sent
    pub fn record_command(&mut self) {
        if self.command_start_time.is_none() {
            self.command_start_time = Some(self.elapsed_time);
        }
        self.last_command_time = self.elapsed_time;
    }

    /// Record a collision (resets collision-free timer)
    pub fn record_collision(&mut self) {
        self.collision_free_since = None;
    }

    /// Start tracking collision-free time
    pub fn start_collision_free_tracking(&mut self) {
        if self.collision_free_since.is_none() {
            self.collision_free_since = Some(self.elapsed_time);
        }
    }
}

/// Built-in tutorial: Basics
pub fn tutorial_basics() -> Tutorial {
    Tutorial {
        id: "basics".to_string(),
        title: "sim2d Basics".to_string(),
        description: "Learn the fundamentals of sim2d and robot control".to_string(),
        steps: vec![
            TutorialStep {
                title: "Welcome to sim2d!".to_string(),
                instruction: "This tutorial will teach you the basics of controlling a robot in sim2d. Click 'Next Step' or press the Play button to start the simulation.".to_string(),
                hint: Some("Look for the Play button in the Simulation Control section".to_string()),
                action_type: TutorialActionType::ManualComplete,
                completed: false,
            },
            TutorialStep {
                title: "Start the Simulation".to_string(),
                instruction: "Press the Play button (or SPACE) to start the simulation running.".to_string(),
                hint: None,
                action_type: TutorialActionType::StartSimulation,
                completed: false,
            },
            TutorialStep {
                title: "Control Your Robot".to_string(),
                instruction: "Send a velocity command to your robot using HORUS. The robot should move for at least 1 second.".to_string(),
                hint: Some("Use: horus run \"echo 'CmdVel(1.0, 0.0)'\" /robot/cmd_vel".to_string()),
                action_type: TutorialActionType::SendCommand {
                    topic: "/robot/cmd_vel".to_string(),
                    min_duration: 1.0,
                },
                completed: false,
            },
            TutorialStep {
                title: "Navigate to Goal".to_string(),
                instruction: "Move your robot to position (5.0, 5.0). You're currently being tracked!".to_string(),
                hint: Some("Combine linear velocity (forward/backward) with angular velocity (rotation) to reach the target.".to_string()),
                action_type: TutorialActionType::ReachPosition {
                    x: 5.0,
                    y: 5.0,
                    threshold: 1.0,
                },
                completed: false,
            },
            TutorialStep {
                title: "Tutorial Complete!".to_string(),
                instruction: "Congratulations! You've completed the basics tutorial. Try the other tutorials to learn more advanced features.".to_string(),
                hint: None,
                action_type: TutorialActionType::ManualComplete,
                completed: false,
            },
        ],
        current_step: 0,
        completed: false,
    }
}

/// Built-in tutorial: Obstacle Avoidance
pub fn tutorial_obstacle_avoidance() -> Tutorial {
    Tutorial {
        id: "obstacles".to_string(),
        title: "Obstacle Avoidance".to_string(),
        description: "Learn to navigate around obstacles safely".to_string(),
        steps: vec![
            TutorialStep {
                title: "Understanding Obstacles".to_string(),
                instruction: "Obstacles are shown as brown rectangles and circles. Your robot will collide with them if you're not careful.".to_string(),
                hint: None,
                action_type: TutorialActionType::ManualComplete,
                completed: false,
            },
            TutorialStep {
                title: "Navigate Without Colliding".to_string(),
                instruction: "Drive your robot for 5 seconds without hitting any obstacles. Stay collision-free!".to_string(),
                hint: Some("Use slower speeds and wider turns to avoid obstacles".to_string()),
                action_type: TutorialActionType::AvoidCollision {
                    duration: 5.0,
                },
                completed: false,
            },
            TutorialStep {
                title: "Reach the Goal".to_string(),
                instruction: "Navigate to position (10.0, 10.0) while avoiding all obstacles.".to_string(),
                hint: Some("Plan your path before moving. Look for gaps between obstacles.".to_string()),
                action_type: TutorialActionType::ReachPosition {
                    x: 10.0,
                    y: 10.0,
                    threshold: 1.0,
                },
                completed: false,
            },
            TutorialStep {
                title: "Well Done!".to_string(),
                instruction: "You've mastered basic obstacle avoidance. Try implementing an autonomous navigation algorithm!".to_string(),
                hint: None,
                action_type: TutorialActionType::ManualComplete,
                completed: false,
            },
        ],
        current_step: 0,
        completed: false,
    }
}

/// Built-in tutorial: LIDAR Sensor
pub fn tutorial_lidar() -> Tutorial {
    Tutorial {
        id: "lidar".to_string(),
        title: "LIDAR Sensor".to_string(),
        description: "Learn to use the LIDAR sensor for environment perception".to_string(),
        steps: vec![
            TutorialStep {
                title: "LIDAR Basics".to_string(),
                instruction: "Your robot has a LIDAR sensor that publishes distance measurements to /robot/scan. Enable 'Show LIDAR Rays' in the Visuals section to see them.".to_string(),
                hint: Some("Find the Visuals section and check 'Show LIDAR Rays'".to_string()),
                action_type: TutorialActionType::ManualComplete,
                completed: false,
            },
            TutorialStep {
                title: "Reading LIDAR Data".to_string(),
                instruction: "Subscribe to the /robot/scan topic to receive LIDAR data. Drive around to see the measurements change.".to_string(),
                hint: Some("Use: horus sub /robot/scan to see real-time LIDAR data".to_string()),
                action_type: TutorialActionType::SendCommand {
                    topic: "/robot/cmd_vel".to_string(),
                    min_duration: 2.0,
                },
                completed: false,
            },
            TutorialStep {
                title: "Using LIDAR for Navigation".to_string(),
                instruction: "Use LIDAR data to navigate to (8.0, 8.0) while detecting and avoiding obstacles.".to_string(),
                hint: Some("Process LIDAR data to find the closest obstacles and steer away from them".to_string()),
                action_type: TutorialActionType::ReachPosition {
                    x: 8.0,
                    y: 8.0,
                    threshold: 1.0,
                },
                completed: false,
            },
            TutorialStep {
                title: "LIDAR Master!".to_string(),
                instruction: "Excellent work! You can now use LIDAR for autonomous navigation. Explore advanced topics like SLAM and mapping.".to_string(),
                hint: None,
                action_type: TutorialActionType::ManualComplete,
                completed: false,
            },
        ],
        current_step: 0,
        completed: false,
    }
}

/// Get all available tutorials
pub fn get_available_tutorials() -> Vec<Tutorial> {
    vec![
        tutorial_basics(),
        tutorial_obstacle_avoidance(),
        tutorial_lidar(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tutorial_creation() {
        let tutorial = tutorial_basics();
        assert_eq!(tutorial.id, "basics");
        assert_eq!(tutorial.current_step, 0);
        assert!(!tutorial.completed);
        assert!(!tutorial.steps.is_empty());
    }

    #[test]
    fn test_tutorial_progress() {
        let tutorial = tutorial_basics();
        let progress = tutorial.progress();
        assert_eq!(progress, 0.0);

        let mut tutorial = tutorial_basics();
        tutorial.current_step = tutorial.steps.len();
        let progress = tutorial.progress();
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_tutorial_state() {
        let mut state = TutorialState::default();
        assert!(state.active_tutorial.is_none());
        assert_eq!(state.completed_tutorials.len(), 0);

        let tutorial = tutorial_basics();
        state.start_tutorial(tutorial);
        assert!(state.active_tutorial.is_some());
        assert!(state.show_tutorial_panel);
    }

    #[test]
    fn test_manual_complete() {
        let mut state = TutorialState::default();
        let tutorial = tutorial_basics();
        state.start_tutorial(tutorial);

        state.complete_current_step();
        assert_eq!(state.active_tutorial.as_ref().unwrap().current_step, 1);
    }

    #[test]
    fn test_simulation_start_check() {
        let mut state = TutorialState::default();
        let mut tutorial = tutorial_basics();

        // Move to step that requires simulation start
        tutorial.current_step = 1;
        tutorial.steps[0].completed = true;

        state.active_tutorial = Some(tutorial);

        // Not started yet
        state.simulation_running = false;
        state.update(0.1);
        assert_eq!(state.active_tutorial.as_ref().unwrap().current_step, 1);

        // Started
        state.simulation_running = true;
        state.update(0.1);
        assert_eq!(state.active_tutorial.as_ref().unwrap().current_step, 2);
    }

    #[test]
    fn test_position_check() {
        let mut state = TutorialState::default();
        state.current_position = Vec2::new(4.9, 5.1);

        let tutorial = Tutorial {
            id: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            steps: vec![TutorialStep {
                title: "Go to position".to_string(),
                instruction: "Move to 5,5".to_string(),
                hint: None,
                action_type: TutorialActionType::ReachPosition {
                    x: 5.0,
                    y: 5.0,
                    threshold: 0.5,
                },
                completed: false,
            }],
            current_step: 0,
            completed: false,
        };

        state.start_tutorial(tutorial);
        state.update(0.1);

        // Should be complete
        assert!(state.active_tutorial.as_ref().unwrap().steps[0].completed);
    }

    #[test]
    fn test_available_tutorials() {
        let tutorials = get_available_tutorials();
        assert_eq!(tutorials.len(), 3);
        assert_eq!(tutorials[0].id, "basics");
        assert_eq!(tutorials[1].id, "obstacles");
        assert_eq!(tutorials[2].id, "lidar");
    }
}
