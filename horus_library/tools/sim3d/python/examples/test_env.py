#!/usr/bin/env python3
"""
Example: Test environment basic functionality
"""

import numpy as np
import sim3d_rl


def test_single_env(task_type="reaching"):
    """Test a single environment"""
    print(f"\n{'='*60}")
    print(f"Testing {task_type} task (single environment)")
    print('='*60)

    # Create environment
    env = sim3d_rl.make_env(task_type)

    # Print environment info
    print(f"\nObservation space: {env.observation_space()}")
    print(f"Action space: {env.action_space()}")

    # Reset and run episode
    obs = env.reset()
    print(f"\nInitial observation shape: {obs.shape}")
    print(f"Initial observation: {obs[:5]}...")  # Show first 5 values

    total_reward = 0
    step_count = 0
    max_steps = 100

    for step in range(max_steps):
        # Random action
        action_dim = env.action_space()['shape'][0]
        action = np.random.uniform(-1, 1, size=action_dim).tolist()

        # Step environment
        obs, reward, done, truncated, info = env.step(action)
        total_reward += reward
        step_count += 1

        if step % 20 == 0:
            print(f"  Step {step}: reward={reward:.3f}, done={done}, truncated={truncated}")

        if done or truncated:
            print(f"\nEpisode finished!")
            print(f"  Total steps: {step_count}")
            print(f"  Total reward: {total_reward:.3f}")
            print(f"  Success: {info['success']}")
            print(f"  Termination reason: {info['termination_reason']}")
            break

    env.close()
    print(f"\nEnvironment stats:")
    print(f"  Episodes: {env.episode_count}")
    print(f"  Total steps: {env.total_steps}")


def test_vectorized_env(task_type="reaching", num_envs=4):
    """Test vectorized environment"""
    print(f"\n{'='*60}")
    print(f"Testing {task_type} task (vectorized, {num_envs} envs)")
    print('='*60)

    # Create vectorized environment
    vec_env = sim3d_rl.make_vec_env(task_type, num_envs=num_envs)

    print(f"\nNumber of environments: {vec_env.num_envs}")

    # Reset all environments
    obs = vec_env.reset()
    print(f"Observations shape: {obs.shape}")

    # Run a few steps
    total_rewards = np.zeros(num_envs)

    for step in range(50):
        # Random actions for all environments
        actions = [
            np.random.uniform(-1, 1, size=6).tolist()
            for _ in range(num_envs)
        ]

        # Step all environments
        obs, rewards, dones, truncateds, infos = vec_env.step(actions)

        total_rewards += np.array(rewards)

        if step % 10 == 0:
            print(f"\nStep {step}:")
            print(f"  Observations shape: {obs.shape}")
            print(f"  Rewards: {[f'{r:.3f}' for r in rewards]}")
            print(f"  Dones: {dones}")

    vec_env.close()

    print(f"\nFinal total rewards per environment:")
    for i, r in enumerate(total_rewards):
        print(f"  Env {i}: {r:.3f}")


def test_all_tasks():
    """Test all available tasks"""
    tasks = ["reaching", "balancing", "locomotion", "navigation", "manipulation", "push"]

    for task in tasks:
        try:
            test_single_env(task)
        except Exception as e:
            print(f"Error testing {task}: {e}")
            import traceback
            traceback.print_exc()


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Test Sim3D RL environments")
    parser.add_argument("--task", type=str, default="reaching",
                        choices=["reaching", "balancing", "locomotion", "navigation",
                                 "manipulation", "push", "all"],
                        help="Task to test (or 'all' for all tasks)")
    parser.add_argument("--vectorized", action="store_true",
                        help="Test vectorized environment")
    parser.add_argument("--num-envs", type=int, default=4,
                        help="Number of parallel environments for vectorized test")

    args = parser.parse_args()

    if args.task == "all":
        test_all_tasks()
    else:
        if args.vectorized:
            test_vectorized_env(args.task, args.num_envs)
        else:
            test_single_env(args.task)


if __name__ == "__main__":
    main()
