//! Performance benchmarks for RL environments
//!
//! Measures physics step timing, observation generation overhead,
//! and vectorized environment scaling.

use std::time::{Duration, Instant};
use bevy::prelude::*;
use sim3d::rl::{
    tasks::*,
    Action, RLTask, RLTaskManager,
};

/// Benchmark result structure
#[derive(Debug)]
struct BenchmarkResult {
    name: String,
    total_time: Duration,
    iterations: usize,
    avg_time_us: f64,
    min_time_us: f64,
    max_time_us: f64,
    std_dev_us: f64,
}

impl BenchmarkResult {
    fn new(name: String, times: Vec<Duration>) -> Self {
        let total_time: Duration = times.iter().sum();
        let iterations = times.len();

        let times_us: Vec<f64> = times.iter().map(|d| d.as_secs_f64() * 1_000_000.0).collect();

        let avg_time_us = times_us.iter().sum::<f64>() / iterations as f64;
        let min_time_us = times_us.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_time_us = times_us.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Calculate standard deviation
        let variance = times_us.iter()
            .map(|t| (t - avg_time_us).powi(2))
            .sum::<f64>() / iterations as f64;
        let std_dev_us = variance.sqrt();

        Self {
            name,
            total_time,
            iterations,
            avg_time_us,
            min_time_us,
            max_time_us,
            std_dev_us,
        }
    }

    fn print(&self) {
        println!("\n=== {} ===", self.name);
        println!("Iterations: {}", self.iterations);
        println!("Total time: {:.2}ms", self.total_time.as_secs_f64() * 1000.0);
        println!("Average: {:.2}µs ({:.2}ms)", self.avg_time_us, self.avg_time_us / 1000.0);
        println!("Min: {:.2}µs", self.min_time_us);
        println!("Max: {:.2}µs", self.max_time_us);
        println!("StdDev: {:.2}µs", self.std_dev_us);
        println!("Throughput: {:.0} steps/sec", 1_000_000.0 / self.avg_time_us);
    }
}

/// Benchmark physics step timing
fn benchmark_physics_step<T: RLTask>(task_name: &str, mut task: T, iterations: usize) -> BenchmarkResult {
    let mut world = World::new();
    task.reset(&mut world);

    let action = Action::Continuous(vec![0.1; task.config().action_dim]);
    let mut times = Vec::with_capacity(iterations);

    // Warm-up
    for _ in 0..10 {
        task.step(&mut world, &action);
    }

    // Benchmark
    for _ in 0..iterations {
        let start = Instant::now();
        task.step(&mut world, &action);
        times.push(start.elapsed());
    }

    BenchmarkResult::new(format!("Physics Step - {}", task_name), times)
}

/// Benchmark observation generation
fn benchmark_observation_generation<T: RLTask>(task_name: &str, mut task: T, iterations: usize) -> BenchmarkResult {
    let mut world = World::new();
    task.reset(&mut world);

    let mut times = Vec::with_capacity(iterations);

    // Warm-up
    for _ in 0..10 {
        task.get_observation(&mut world);
    }

    // Benchmark
    for _ in 0..iterations {
        let start = Instant::now();
        task.get_observation(&mut world);
        times.push(start.elapsed());
    }

    BenchmarkResult::new(format!("Observation Generation - {}", task_name), times)
}

/// Benchmark reward computation
fn benchmark_reward_computation<T: RLTask>(task_name: &str, mut task: T, iterations: usize) -> BenchmarkResult {
    let mut world = World::new();
    task.reset(&mut world);

    let mut times = Vec::with_capacity(iterations);

    // Warm-up
    for _ in 0..10 {
        task.compute_reward(&mut world);
    }

    // Benchmark
    for _ in 0..iterations {
        let start = Instant::now();
        task.compute_reward(&mut world);
        times.push(start.elapsed());
    }

    BenchmarkResult::new(format!("Reward Computation - {}", task_name), times)
}

/// Benchmark reset
fn benchmark_reset<T: RLTask>(task_name: &str, mut task: T, iterations: usize) -> BenchmarkResult {
    let mut world = World::new();
    let mut times = Vec::with_capacity(iterations);

    // Warm-up
    for _ in 0..5 {
        task.reset(&mut world);
    }

    // Benchmark
    for _ in 0..iterations {
        let start = Instant::now();
        task.reset(&mut world);
        times.push(start.elapsed());
    }

    BenchmarkResult::new(format!("Reset - {}", task_name), times)
}

/// Benchmark vectorized environment scaling
fn benchmark_vectorized_scaling(num_envs: usize, steps: usize) -> BenchmarkResult {
    let mut envs: Vec<(ReachingTask, World)> = Vec::with_capacity(num_envs);

    // Initialize environments
    for _ in 0..num_envs {
        let task = ReachingTask::new(10, 6);
        let world = World::new();
        envs.push((task, world));
    }

    // Reset all
    for (task, world) in &mut envs {
        task.reset(world);
    }

    let action = Action::Continuous(vec![0.1; 6]);
    let mut step_times = Vec::with_capacity(steps);

    // Benchmark
    for _ in 0..steps {
        let start = Instant::now();

        // Step all environments
        for (task, world) in &mut envs {
            task.step(world, &action);
        }

        step_times.push(start.elapsed());
    }

    BenchmarkResult::new(
        format!("Vectorized Scaling - {} envs", num_envs),
        step_times
    )
}

/// Benchmark full episode
fn benchmark_full_episode(task_name: &str, max_steps: usize) -> BenchmarkResult {
    let mut world = World::new();
    let mut task = match task_name {
        "reaching" => Box::new(ReachingTask::new(10, 6)) as Box<dyn RLTask>,
        "balancing" => Box::new(BalancingTask::new(6, 1)) as Box<dyn RLTask>,
        "locomotion" => Box::new(LocomotionTask::new(22, 12)) as Box<dyn RLTask>,
        "navigation" => Box::new(NavigationTask::new(21, 2)) as Box<dyn RLTask>,
        "manipulation" => Box::new(ManipulationTask::new(25, 4)) as Box<dyn RLTask>,
        "push" => Box::new(PushTask::new(30, 2)) as Box<dyn RLTask>,
        _ => Box::new(ReachingTask::new(10, 6)) as Box<dyn RLTask>,
    };

    let mut episode_times = Vec::with_capacity(10);

    for _ in 0..10 {
        let start = Instant::now();

        task.reset(&mut world);
        let action = Action::Continuous(vec![0.1; task.config().action_dim]);

        for _ in 0..max_steps {
            let result = task.step(&mut world, &action);
            if result.done || result.truncated {
                break;
            }
        }

        episode_times.push(start.elapsed());
    }

    BenchmarkResult::new(
        format!("Full Episode - {}", task_name),
        episode_times
    )
}

#[test]
fn run_all_benchmarks() {
    println!("\n╔══════════════════════════════════════════════╗");
    println!("║   RL PERFORMANCE BENCHMARKS                  ║");
    println!("╚══════════════════════════════════════════════╝");

    let iterations = 1000;

    // Physics step benchmarks
    println!("\n━━━ PHYSICS STEP TIMING ━━━");
    benchmark_physics_step("Reaching", ReachingTask::new(10, 6), iterations).print();
    benchmark_physics_step("Balancing", BalancingTask::new(6, 1), iterations).print();
    benchmark_physics_step("Locomotion", LocomotionTask::new(22, 12), iterations / 2).print();
    benchmark_physics_step("Navigation", NavigationTask::new(21, 2), iterations).print();
    benchmark_physics_step("Manipulation", ManipulationTask::new(25, 4), iterations / 2).print();
    benchmark_physics_step("Push", PushTask::new(30, 2), iterations).print();

    // Observation generation benchmarks
    println!("\n━━━ OBSERVATION GENERATION OVERHEAD ━━━");
    benchmark_observation_generation("Reaching", ReachingTask::new(10, 6), iterations).print();
    benchmark_observation_generation("Balancing", BalancingTask::new(6, 1), iterations).print();
    benchmark_observation_generation("Locomotion", LocomotionTask::new(22, 12), iterations).print();
    benchmark_observation_generation("Navigation", NavigationTask::new(21, 2), iterations).print();
    benchmark_observation_generation("Manipulation", ManipulationTask::new(25, 4), iterations).print();
    benchmark_observation_generation("Push", PushTask::new(30, 2), iterations).print();

    // Reward computation benchmarks
    println!("\n━━━ REWARD COMPUTATION OVERHEAD ━━━");
    benchmark_reward_computation("Reaching", ReachingTask::new(10, 6), iterations).print();
    benchmark_reward_computation("Balancing", BalancingTask::new(6, 1), iterations).print();

    // Reset benchmarks
    println!("\n━━━ RESET OVERHEAD ━━━");
    benchmark_reset("Reaching", ReachingTask::new(10, 6), 100).print();
    benchmark_reset("Balancing", BalancingTask::new(6, 1), 100).print();

    // Vectorized scaling benchmarks
    println!("\n━━━ VECTORIZED ENVIRONMENT SCALING ━━━");
    let steps = 100;
    benchmark_vectorized_scaling(1, steps).print();
    benchmark_vectorized_scaling(2, steps).print();
    benchmark_vectorized_scaling(4, steps).print();
    benchmark_vectorized_scaling(8, steps).print();
    benchmark_vectorized_scaling(16, steps).print();

    // Full episode benchmarks
    println!("\n━━━ FULL EPISODE PERFORMANCE ━━━");
    benchmark_full_episode("reaching", 100).print();
    benchmark_full_episode("balancing", 200).print();
    benchmark_full_episode("navigation", 100).print();

    println!("\n╔══════════════════════════════════════════════╗");
    println!("║   BENCHMARK COMPLETE                         ║");
    println!("╚══════════════════════════════════════════════╝\n");
}

#[test]
fn run_quick_benchmark() {
    println!("\n=== QUICK BENCHMARK ===");

    // Just test one task for CI
    let result = benchmark_physics_step("Reaching", ReachingTask::new(10, 6), 100);
    result.print();

    assert!(result.avg_time_us < 100_000.0, "Step time too slow: {:.2}µs", result.avg_time_us);
    println!("\n[OK] Performance within acceptable range");
}

#[test]
fn benchmark_task_manager() {
    println!("\n=== TASK MANAGER BENCHMARK ===");

    let mut world = World::new();
    let mut task_manager = RLTaskManager::new();
    task_manager.set_task(Box::new(ReachingTask::new(10, 6)));

    let iterations = 500;
    let mut times = Vec::with_capacity(iterations);

    task_manager.reset(&mut world);
    let action = Action::Continuous(vec![0.1; 6]);

    for _ in 0..iterations {
        let start = Instant::now();
        task_manager.step(&mut world, &action);
        times.push(start.elapsed());
    }

    let result = BenchmarkResult::new("Task Manager Step".to_string(), times);
    result.print();
}

#[test]
fn benchmark_memory_overhead() {
    println!("\n=== MEMORY OVERHEAD BENCHMARK ===");

    let num_envs = 100;
    let start_time = Instant::now();

    let envs: Vec<(ReachingTask, World)> = (0..num_envs)
        .map(|_| (ReachingTask::new(10, 6), World::new()))
        .collect();

    let creation_time = start_time.elapsed();

    println!("Created {} environments in {:.2}ms", num_envs, creation_time.as_secs_f64() * 1000.0);
    println!("Per-env creation time: {:.2}µs", creation_time.as_secs_f64() * 1_000_000.0 / num_envs as f64);

    // Estimate memory (rough approximation)
    let estimated_mem_mb = (std::mem::size_of::<ReachingTask>() + std::mem::size_of::<World>()) * num_envs / 1_000_000;
    println!("Estimated memory (base): ~{}MB", estimated_mem_mb);
}
