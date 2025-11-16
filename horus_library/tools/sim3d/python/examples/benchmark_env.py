#!/usr/bin/env python3
"""
Benchmark sim3d environment performance
Tests throughput and latency of vectorized environments
"""

import argparse
import time
import numpy as np
import sim3d_rl


def benchmark_single_env(task_name, num_steps=10000):
    """Benchmark single environment performance."""
    print(f"\n{'='*60}")
    print(f"Benchmarking Single Environment: {task_name}")
    print(f"{'='*60}")

    obs_dims = {"reaching": 10, "balancing": 8, "locomotion": 12, "navigation": 20, "manipulation": 15, "push": 18}
    action_dims = {"reaching": 6, "balancing": 1, "locomotion": 2, "navigation": 2, "manipulation": 7, "push": 2}

    env = sim3d_rl.make_env(task_name, obs_dim=obs_dims[task_name], action_dim=action_dims[task_name])
    action_dim = action_dims[task_name]

    # Warmup
    print("Warming up...")
    obs = env.reset()
    for _ in range(100):
        action = np.random.randn(action_dim).astype(np.float32)
        obs, reward, done, truncated, info = env.step(action)
        if done or truncated:
            obs = env.reset()

    # Benchmark
    print(f"Running {num_steps:,} steps...")
    obs = env.reset()
    start_total = time.perf_counter()

    for _ in range(num_steps):
        action = np.random.randn(action_dim).astype(np.float32)
        obs, reward, done, truncated, info = env.step(action)
        if done or truncated:
            env.reset()

    total_time = time.perf_counter() - start_total
    env.close()

    print(f"\nThroughput: {num_steps / total_time:,.0f} steps/sec")
    print(f"Time per step: {(total_time / num_steps) * 1000:.3f} ms")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Benchmark sim3d environment")
    parser.add_argument("--task", type=str, default="navigation", help="Task to benchmark")
    parser.add_argument("--steps", type=int, default=10000, help="Number of steps")
    args = parser.parse_args()
    
    benchmark_single_env(args.task, num_steps=args.steps)
