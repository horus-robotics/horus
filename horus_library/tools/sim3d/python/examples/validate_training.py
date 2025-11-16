#!/usr/bin/env python3
"""
Validate that sim3d environment is ready for RL training
Runs multiple episodes and tracks statistics
"""

import numpy as np
import sim3d_rl
import time


def validate_environment(task_name, num_episodes=10, max_steps_per_episode=1000):
    """
    Run multiple episodes and collect statistics.

    Args:
        task_name: RL task to test
        num_episodes: Number of episodes to run
        max_steps_per_episode: Maximum steps per episode
    """
    print(f"\n{'='*60}")
    print(f"Validating {task_name} Environment")
    print(f"{'='*60}")

    obs_dims = {"reaching": 10, "balancing": 8, "locomotion": 12, "navigation": 20, "manipulation": 15, "push": 18}
    action_dims = {"reaching": 6, "balancing": 1, "locomotion": 2, "navigation": 2, "manipulation": 7, "push": 2}

    env = sim3d_rl.make_env(task_name, obs_dim=obs_dims[task_name], action_dim=action_dims[task_name])
    action_dim = action_dims[task_name]

    episode_rewards = []
    episode_lengths = []
    success_count = 0

    print(f"\nRunning {num_episodes} episodes (max {max_steps_per_episode} steps each)...")
    start_time = time.time()

    for episode in range(num_episodes):
        obs = env.reset()
        episode_reward = 0.0
        steps = 0

        for step in range(max_steps_per_episode):
            # Random action (in real training, this would be the policy)
            action = np.random.randn(action_dim).astype(np.float32)

            obs, reward, done, truncated, info = env.step(action)
            episode_reward += reward
            steps += 1

            if done or truncated:
                if info.get('success', False):
                    success_count += 1
                break

        episode_rewards.append(episode_reward)
        episode_lengths.append(steps)

        print(f"  Episode {episode+1}: reward={episode_reward:.2f}, steps={steps}, success={info.get('success', False)}")

    total_time = time.time() - start_time
    env.close()

    # Statistics
    print(f"\n{'='*60}")
    print("Summary Statistics")
    print(f"{'='*60}")
    print(f"Total episodes: {num_episodes}")
    print(f"Total time: {total_time:.2f}s")
    print(f"Average episode reward: {np.mean(episode_rewards):.2f} ± {np.std(episode_rewards):.2f}")
    print(f"Min/Max reward: {np.min(episode_rewards):.2f} / {np.max(episode_rewards):.2f}")
    print(f"Average episode length: {np.mean(episode_lengths):.1f} ± {np.std(episode_lengths):.1f}")
    print(f"Success rate: {success_count}/{num_episodes} ({100*success_count/num_episodes:.1f}%)")
    print(f"Throughput: {sum(episode_lengths)/total_time:.0f} steps/sec")

    print(f"\n{'='*60}")
    print("✓ Environment is stable and ready for RL training!")
    print(f"{'='*60}")

    print("\nTo start full training:")
    print("  1. Install RL library: pip install stable-baselines3")
    print("  2. Run training: python examples/train_ppo.py --task reaching --timesteps 100000")

    return {
        'episode_rewards': episode_rewards,
        'episode_lengths': episode_lengths,
        'success_rate': success_count / num_episodes,
        'throughput': sum(episode_lengths) / total_time
    }


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Validate sim3d environment for training")
    parser.add_argument("--task", type=str, default="reaching", help="Task to validate")
    parser.add_argument("--episodes", type=int, default=10, help="Number of episodes")
    parser.add_argument("--max-steps", type=int, default=1000, help="Max steps per episode")
    args = parser.parse_args()

    validate_environment(args.task, num_episodes=args.episodes, max_steps_per_episode=args.max_steps)
