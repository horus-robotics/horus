//! Integration test for end-to-end RL training
//!
//! Tests that the RL environment can be used for actual training,
//! including observation generation, action application, and episode rollouts.

use bevy::prelude::*;
use sim3d::rl::{
    tasks::*, Action, Observation, RLTask, RLTaskManager, StepResult,
};

#[test]
fn test_reaching_task_episode() {
    let mut world = World::new();
    let mut task = ReachingTask::new(10, 6);

    // Reset environment
    let initial_obs = task.reset(&mut world);
    assert_eq!(initial_obs.len(), 10);

    // Run a full episode
    let mut total_reward = 0.0;
    let max_steps = 100;

    for step in 0..max_steps {
        // Random action
        let action = Action::Continuous(vec![
            (step as f32 * 0.1).sin() * 0.5,
            (step as f32 * 0.1).cos() * 0.5,
            0.0,
            0.0,
            0.0,
            0.0,
        ]);

        let result = task.step(&mut world, &action);
        total_reward += result.reward;

        // Verify result structure
        assert_eq!(result.observation.len(), 10);
        assert!(result.reward.is_finite());

        if result.done || result.truncated {
            println!("Episode finished at step {} with reward {}", step, total_reward);
            break;
        }
    }

    // Verify episode info
    let info = task.get_info();
    assert_eq!(info.total_reward, total_reward);
    assert!(info.steps > 0 && info.steps <= max_steps);
}

#[test]
fn test_balancing_task_episode() {
    let mut world = World::new();
    let mut task = BalancingTask::new(6, 1);

    let initial_obs = task.reset(&mut world);
    assert!(initial_obs.len() >= 6 && initial_obs.len() <= 8);

    let mut total_reward = 0.0;
    let max_steps = 200;

    for step in 0..max_steps {
        // PD-like control action
        let action = Action::Continuous(vec![
            (step as f32 * 0.05).sin() * 0.3,
        ]);

        let result = task.step(&mut world, &action);
        total_reward += result.reward;

        assert_eq!(result.observation.len(), initial_obs.len());

        if result.done || result.truncated {
            println!("Balancing episode finished at step {}", step);
            break;
        }
    }

    let info = task.get_info();
    assert!(info.steps > 0);
}

#[test]
fn test_locomotion_task_episode() {
    let mut world = World::new();
    let mut task = LocomotionTask::new(22, 12);

    let initial_obs = task.reset(&mut world);
    assert!(initial_obs.len() >= 19 && initial_obs.len() <= 22);

    let max_steps = 150;
    let mut step_count = 0;

    for step in 0..max_steps {
        // Sinusoidal gait pattern
        let phase = step as f32 * 0.1;
        let action = Action::Continuous(
            (0..12)
                .map(|i| (phase + i as f32).sin() * 0.4)
                .collect()
        );

        let result = task.step(&mut world, &action);
        step_count += 1;

        if result.done || result.truncated {
            break;
        }
    }

    assert!(step_count > 0);
}

#[test]
fn test_navigation_task_episode() {
    let mut world = World::new();
    let mut task = NavigationTask::new(21, 2);

    let initial_obs = task.reset(&mut world);
    assert!(initial_obs.len() >= 19 && initial_obs.len() <= 21);

    let max_steps = 100;
    let mut reached_goal = false;

    for _ in 0..max_steps {
        // Simple navigation toward goal
        let action = Action::Continuous(vec![0.5, 0.0]);

        let result = task.step(&mut world, &action);

        if result.info.success {
            reached_goal = true;
            println!("Goal reached!");
            break;
        }

        if result.done || result.truncated {
            break;
        }
    }

    // May or may not reach goal with simple control
    println!("Navigation task completed. Goal reached: {}", reached_goal);
}

#[test]
fn test_manipulation_task_episode() {
    let mut world = World::new();
    let mut task = ManipulationTask::new(25, 4);

    let initial_obs = task.reset(&mut world);
    assert!(initial_obs.len() >= 24 && initial_obs.len() <= 25);

    let max_steps = 200;

    for step in 0..max_steps {
        // Three-phase control: approach, grasp, lift
        let action = if step < 50 {
            // Approach phase
            Action::Continuous(vec![0.1, 0.1, -0.1, 0.0])
        } else if step < 100 {
            // Grasp phase
            Action::Continuous(vec![0.0, 0.0, 0.0, 1.0])
        } else {
            // Lift phase
            Action::Continuous(vec![0.0, 0.0, 0.2, 1.0])
        };

        let result = task.step(&mut world, &action);

        if result.done || result.truncated {
            println!("Manipulation episode finished at step {}", step);
            break;
        }
    }
}

#[test]
fn test_push_task_episode() {
    let mut world = World::new();
    let mut task = PushTask::new(30, 2);

    let initial_obs = task.reset(&mut world);
    assert!(initial_obs.len() >= 28 && initial_obs.len() <= 30);

    let max_steps = 150;

    for step in 0..max_steps {
        // Push in a consistent direction
        let action = Action::Continuous(vec![0.3, 0.0]);

        let result = task.step(&mut world, &action);

        if result.done || result.truncated {
            println!("Push task finished at step {}", step);
            break;
        }
    }
}

#[test]
fn test_task_manager_integration() {
    let mut world = World::new();
    let mut task_manager = RLTaskManager::new();

    // Set reaching task
    task_manager.set_task(Box::new(ReachingTask::new(10, 6)));

    // Test reset
    let obs = task_manager.reset(&mut world);
    assert!(obs.is_some());
    assert_eq!(task_manager.episode_count, 1);

    // Test step
    let action = Action::Continuous(vec![0.1, 0.1, 0.1, 0.0, 0.0, 0.0]);
    let result = task_manager.step(&mut world, &action);
    assert!(result.is_some());
    assert_eq!(task_manager.total_steps, 1);

    // Test multiple episodes
    for _ in 0..5 {
        task_manager.reset(&mut world);
    }
    assert_eq!(task_manager.episode_count, 6);
}

#[test]
fn test_episode_reset_consistency() {
    let mut world = World::new();
    let mut task = ReachingTask::new(10, 6);

    // Run multiple episodes and ensure reset works
    for episode in 0..3 {
        let obs = task.reset(&mut world);
        assert_eq!(obs.len(), 10);

        // Run partial episode
        for _ in 0..10 {
            let action = Action::Continuous(vec![0.1; 6]);
            let result = task.step(&mut world, &action);

            if result.done || result.truncated {
                break;
            }
        }

        println!("Episode {} completed", episode);
    }
}

#[test]
fn test_observation_consistency() {
    let mut world = World::new();
    let mut task = BalancingTask::new(6, 1);

    let obs1 = task.get_observation(&mut world);
    let obs2 = task.get_observation(&mut world);

    // Observations should be identical when world state unchanged
    assert_eq!(obs1.len(), obs2.len());
    for (a, b) in obs1.data.iter().zip(obs2.data.iter()) {
        assert!((a - b).abs() < 1e-6, "Observation changed without step");
    }
}

#[test]
fn test_reward_computation_consistency() {
    let mut world = World::new();
    let mut task = NavigationTask::new(21, 2);

    task.reset(&mut world);

    let reward1 = task.compute_reward(&mut world);
    let reward2 = task.compute_reward(&mut world);

    // Rewards should be identical when state unchanged
    assert!((reward1 - reward2).abs() < 1e-6);
    assert!(reward1.is_finite());
}

#[test]
fn test_action_validation() {
    let mut world = World::new();
    let mut task = ReachingTask::new(10, 6);

    task.reset(&mut world);

    // Test with correct action dimension
    let action = Action::Continuous(vec![0.5; 6]);
    let result = task.step(&mut world, &action);
    assert!(result.reward.is_finite());

    // Test with too few actions (should handle gracefully)
    let action_short = Action::Continuous(vec![0.5; 3]);
    let result = task.step(&mut world, &action_short);
    assert!(result.reward.is_finite());

    // Test with too many actions (extra should be ignored)
    let action_long = Action::Continuous(vec![0.5; 10]);
    let result = task.step(&mut world, &action_long);
    assert!(result.reward.is_finite());
}
