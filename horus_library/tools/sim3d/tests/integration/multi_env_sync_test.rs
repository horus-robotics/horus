//! Integration test for multi-environment synchronization
//!
//! Tests that multiple environments can run in parallel without interference,
//! maintaining independent state and producing consistent results.

use bevy::prelude::*;
use sim3d::rl::{
    tasks::*, Action, RLTask, RLTaskManager,
};
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_parallel_environment_independence() {
    // Create multiple environments
    let num_envs = 4;
    let mut envs: Vec<(Box<dyn RLTask>, World)> = Vec::new();

    for _ in 0..num_envs {
        let task = Box::new(ReachingTask::new(10, 6)) as Box<dyn RLTask>;
        let world = World::new();
        envs.push((task, world));
    }

    // Reset all environments
    let mut initial_observations = Vec::new();
    for (task, world) in &mut envs {
        let obs = task.reset(world);
        initial_observations.push(obs.clone());
    }

    // Verify each environment has its own state
    for i in 0..num_envs {
        for j in (i + 1)..num_envs {
            // Observations might differ due to randomization
            let obs_i = &initial_observations[i];
            let obs_j = &initial_observations[j];
            assert_eq!(obs_i.len(), obs_j.len(), "Observation dimensions should match");
        }
    }

    // Step each environment independently
    for (idx, (task, world)) in envs.iter_mut().enumerate() {
        let action = Action::Continuous(vec![idx as f32 * 0.1; 6]);
        let result = task.step(world, &action);
        assert!(result.reward.is_finite());
    }
}

#[test]
fn test_sequential_multi_env_consistency() {
    let num_envs = 3;
    let num_steps = 20;

    let mut managers: Vec<RLTaskManager> = Vec::new();
    let mut worlds: Vec<World> = Vec::new();

    // Initialize environments
    for _ in 0..num_envs {
        let mut manager = RLTaskManager::new();
        manager.set_task(Box::new(BalancingTask::new(6, 1)));
        let world = World::new();

        managers.push(manager);
        worlds.push(world);
    }

    // Reset all
    for (manager, world) in managers.iter_mut().zip(worlds.iter_mut()) {
        manager.reset(world);
    }

    // Run all environments for same number of steps
    let mut all_rewards = vec![0.0; num_envs];

    for step in 0..num_steps {
        for (env_idx, (manager, world)) in managers.iter_mut().zip(worlds.iter_mut()).enumerate() {
            let action = Action::Continuous(vec![(step as f32 * 0.1).sin() * 0.3]);
            if let Some(result) = manager.step(world, &action) {
                all_rewards[env_idx] += result.reward;

                if result.done || result.truncated {
                    // Reset if done
                    manager.reset(world);
                }
            }
        }
    }

    // Verify all environments ran
    for (idx, reward) in all_rewards.iter().enumerate() {
        assert!(reward.is_finite(), "Environment {} produced invalid reward", idx);
        println!("Environment {} total reward: {}", idx, reward);
    }

    // Verify step counts
    for (idx, manager) in managers.iter().enumerate() {
        println!("Environment {} total steps: {}", idx, manager.total_steps);
        assert!(manager.total_steps > 0);
    }
}

#[test]
fn test_concurrent_environment_isolation() {
    // This test verifies environments maintain isolation when accessed from different threads
    let num_threads = 3;
    let steps_per_thread = 30;

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            thread::spawn(move || {
                let mut world = World::new();
                let mut task = LocomotionTask::new(22, 12);

                task.reset(&mut world);

                let mut total_reward = 0.0;
                for step in 0..steps_per_thread {
                    let phase = (thread_id * 100 + step) as f32 * 0.1;
                    let action = Action::Continuous(
                        (0..12).map(|i| (phase + i as f32).sin() * 0.3).collect()
                    );

                    let result = task.step(&mut world, &action);
                    total_reward += result.reward;

                    if result.done || result.truncated {
                        break;
                    }
                }

                (thread_id, total_reward, task.get_info().steps)
            })
        })
        .collect();

    // Collect results
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Verify all threads completed successfully
    for (thread_id, reward, steps) in results {
        println!("Thread {} - Reward: {}, Steps: {}", thread_id, reward, steps);
        assert!(reward.is_finite());
        assert!(steps > 0);
    }
}

#[test]
fn test_env_state_rollback() {
    let mut world = World::new();
    let mut task = NavigationTask::new(21, 2);

    // Initial reset
    let obs_initial = task.reset(&mut world);

    // Take some steps
    for _ in 0..10 {
        let action = Action::Continuous(vec![0.5, 0.0]);
        task.step(&mut world, &action);
    }

    // Reset again - should get fresh state
    let obs_after_reset = task.reset(&mut world);

    // Both should be valid observations of same dimension
    assert_eq!(obs_initial.len(), obs_after_reset.len());

    // State should be reset (observations may differ due to randomization)
    let info = task.get_info();
    assert_eq!(info.steps, 0, "Steps should be reset to 0");
    assert_eq!(info.total_reward, 0.0, "Reward should be reset to 0");
}

#[test]
fn test_sync_vs_async_consistency() {
    // Test that environments produce consistent results regardless of execution order
    let num_envs = 4;
    let num_steps = 15;

    // Synchronous execution
    let mut sync_results = Vec::new();
    for env_id in 0..num_envs {
        let mut world = World::new();
        let mut task = ReachingTask::new(10, 6);
        task.reset(&mut world);

        let mut total_reward = 0.0;
        for step in 0..num_steps {
            let action = Action::Continuous(vec![(env_id + step) as f32 * 0.1; 6]);
            let result = task.step(&mut world, &action);
            total_reward += result.reward;
        }
        sync_results.push(total_reward);
    }

    // Interleaved execution (simulating async)
    let mut async_envs: Vec<(Box<dyn RLTask>, World)> = Vec::new();
    for _ in 0..num_envs {
        let task = Box::new(ReachingTask::new(10, 6)) as Box<dyn RLTask>;
        let mut world = World::new();
        async_envs.push((task, world));
    }

    // Reset all
    for (task, world) in &mut async_envs {
        task.reset(world);
    }

    let mut async_results = vec![0.0; num_envs];

    // Interleave steps
    for step in 0..num_steps {
        for (env_id, (task, world)) in async_envs.iter_mut().enumerate() {
            let action = Action::Continuous(vec![(env_id + step) as f32 * 0.1; 6]);
            let result = task.step(world, &action);
            async_results[env_id] += result.reward;
        }
    }

    // Results should be consistent
    for (sync, async_r) in sync_results.iter().zip(async_results.iter()) {
        assert!(
            (sync - async_r).abs() < 1e-4,
            "Sync ({}) and async ({}) results should match",
            sync,
            async_r
        );
    }
}

#[test]
fn test_env_cloning_independence() {
    // Test that environments can be "cloned" (reset with same config) without interference
    let base_task = ReachingTask::new(10, 6);
    let config = base_task.config().clone();

    let mut envs = Vec::new();
    for _ in 0..3 {
        let mut world = World::new();
        let mut task = ReachingTask::new(config.obs_dim, config.action_dim);
        task.reset(&mut world);
        envs.push((task, world));
    }

    // Each environment should operate independently
    for (idx, (task, world)) in envs.iter_mut().enumerate() {
        let action = Action::Continuous(vec![idx as f32 * 0.2; 6]);
        let result = task.step(world, &action);
        println!("Env {} reward: {}", idx, result.reward);
        assert!(result.reward.is_finite());
    }
}

#[test]
fn test_vectorized_batch_operations() {
    // Simulate vectorized environment operations
    let batch_size = 8;
    let mut envs: Vec<(ManipulationTask, World)> = Vec::new();

    // Initialize batch
    for _ in 0..batch_size {
        let task = ManipulationTask::new(25, 4);
        let world = World::new();
        envs.push((task, world));
    }

    // Batch reset
    let mut observations = Vec::new();
    for (task, world) in &mut envs {
        let obs = task.reset(world);
        observations.push(obs);
    }

    assert_eq!(observations.len(), batch_size);

    // Batch step with same action
    let batch_action = Action::Continuous(vec![0.1, 0.1, 0.0, 0.5]);
    let mut rewards = Vec::new();

    for (task, world) in &mut envs {
        let result = task.step(world, &batch_action);
        rewards.push(result.reward);
    }

    assert_eq!(rewards.len(), batch_size);
    for reward in rewards {
        assert!(reward.is_finite());
    }
}

#[test]
fn test_environment_lifecycle() {
    // Test complete lifecycle: create, reset, run, reset, run again
    let mut world = World::new();
    let mut task_manager = RLTaskManager::new();

    task_manager.set_task(Box::new(PushTask::new(30, 2)));

    // First episode
    task_manager.reset(&mut world);
    for _ in 0..20 {
        let action = Action::Continuous(vec![0.3, 0.0]);
        if let Some(result) = task_manager.step(&mut world, &action) {
            if result.done || result.truncated {
                break;
            }
        }
    }

    let episode1_steps = task_manager.total_steps;

    // Second episode
    task_manager.reset(&mut world);
    for _ in 0..20 {
        let action = Action::Continuous(vec![0.3, 0.1]);
        if let Some(result) = task_manager.step(&mut world, &action) {
            if result.done || result.truncated {
                break;
            }
        }
    }

    let episode2_steps = task_manager.total_steps;

    assert!(episode2_steps > episode1_steps);
    assert_eq!(task_manager.episode_count, 2);
}
