#!/usr/bin/env python3
"""
Train a PPO agent on sim3d navigation task
Requires: stable-baselines3, gymnasium
"""

import argparse
import numpy as np
from stable_baselines3 import PPO
from stable_baselines3.common.env_util import make_vec_env
from stable_baselines3.common.callbacks import EvalCallback, CheckpointCallback
import sim3d_rl


def make_sim3d_env(task_name, rank, seed=0):
    """
    Utility function for multiprocessed env.

    Args:
        task_name: Name of the RL task
        rank: Index of subprocess
        seed: Random seed
    """
    def _init():
        # Observation dimensions per task
        obs_dims = {
            "reaching": 10,
            "balancing": 8,
            "locomotion": 12,
            "navigation": 20,
            "manipulation": 15,
            "push": 18,
        }

        # Action dimensions per task
        action_dims = {
            "reaching": 6,
            "balancing": 1,
            "locomotion": 2,
            "navigation": 2,
            "manipulation": 7,
            "push": 2,
        }

        env = sim3d_rl.make_env(
            task_name,
            obs_dim=obs_dims[task_name],
            action_dim=action_dims[task_name]
        )
        env.reset(seed=seed + rank)
        return env
    return _init


def train_ppo(task_name, total_timesteps=1_000_000, n_envs=4, seed=42):
    """
    Train PPO agent on sim3d environment.

    Args:
        task_name: RL task to train on
        total_timesteps: Total training timesteps
        n_envs: Number of parallel environments
        seed: Random seed
    """
    print(f"Training PPO on {task_name} task")
    print(f"Total timesteps: {total_timesteps:,}")
    print(f"Parallel envs: {n_envs}")
    print(f"Random seed: {seed}")
    print("-" * 50)

    # Create vectorized environment
    env = make_vec_env(
        make_sim3d_env(task_name, 0, seed),
        n_envs=n_envs,
        seed=seed
    )

    # Create evaluation environment
    eval_env = make_vec_env(
        make_sim3d_env(task_name, 100, seed + 100),
        n_envs=1,
        seed=seed + 100
    )

    # Callbacks
    eval_callback = EvalCallback(
        eval_env,
        best_model_save_path=f"./logs/ppo_{task_name}/best/",
        log_path=f"./logs/ppo_{task_name}/eval/",
        eval_freq=10000,
        n_eval_episodes=10,
        deterministic=True,
        render=False,
    )

    checkpoint_callback = CheckpointCallback(
        save_freq=50000,
        save_path=f"./logs/ppo_{task_name}/checkpoints/",
        name_prefix=f"ppo_{task_name}",
    )

    # Create PPO agent
    model = PPO(
        "MlpPolicy",
        env,
        learning_rate=3e-4,
        n_steps=2048,
        batch_size=64,
        n_epochs=10,
        gamma=0.99,
        gae_lambda=0.95,
        clip_range=0.2,
        clip_range_vf=None,
        normalize_advantage=True,
        ent_coef=0.0,
        vf_coef=0.5,
        max_grad_norm=0.5,
        use_sde=False,
        sde_sample_freq=-1,
        target_kl=None,
        tensorboard_log=f"./logs/ppo_{task_name}/tensorboard/",
        policy_kwargs=dict(
            net_arch=dict(pi=[256, 256], vf=[256, 256]),
            activation_fn=torch.nn.ReLU,
        ),
        verbose=1,
        seed=seed,
    )

    print("\nStarting training...")
    model.learn(
        total_timesteps=total_timesteps,
        callback=[eval_callback, checkpoint_callback],
        progress_bar=True,
    )

    # Save final model
    model.save(f"./logs/ppo_{task_name}/final_model")
    print(f"\nTraining complete! Model saved to ./logs/ppo_{task_name}/final_model")

    # Cleanup
    env.close()
    eval_env.close()


def test_environment(task_name, num_episodes=5):
    """
    Test environment without training.

    Args:
        task_name: RL task to test
        num_episodes: Number of test episodes
    """
    print(f"Testing {task_name} environment...")

    obs_dims = {
        "reaching": 10, "balancing": 8, "locomotion": 12,
        "navigation": 20, "manipulation": 15, "push": 18,
    }
    action_dims = {
        "reaching": 6, "balancing": 1, "locomotion": 2,
        "navigation": 2, "manipulation": 7, "push": 2,
    }

    env = sim3d_rl.make_env(
        task_name,
        obs_dim=obs_dims[task_name],
        action_dim=action_dims[task_name]
    )

    for episode in range(num_episodes):
        obs = env.reset()
        total_reward = 0
        steps = 0
        done = False

        print(f"\nEpisode {episode + 1}/{num_episodes}")
        print(f"Initial observation shape: {obs.shape}")

        while not done:
            # Random action
            action = env.action_space.sample()
            obs, reward, done, truncated, info = env.step(action)
            total_reward += reward
            steps += 1

            if done or truncated:
                break

        print(f"  Steps: {steps}")
        print(f"  Total reward: {total_reward:.2f}")
        print(f"  Success: {info.get('success', False)}")
        print(f"  Termination: {info.get('termination_reason', 'unknown')}")

    env.close()
    print("\nEnvironment test complete!")


if __name__ == "__main__":
    import torch  # Import here to avoid issues if not using GPU

    parser = argparse.ArgumentParser(description="Train PPO on sim3d tasks")
    parser.add_argument(
        "--task",
        type=str,
        default="navigation",
        choices=["reaching", "balancing", "locomotion", "navigation", "manipulation", "push"],
        help="RL task to train on"
    )
    parser.add_argument(
        "--timesteps",
        type=int,
        default=1_000_000,
        help="Total training timesteps"
    )
    parser.add_argument(
        "--n-envs",
        type=int,
        default=4,
        help="Number of parallel environments"
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=42,
        help="Random seed"
    )
    parser.add_argument(
        "--test-only",
        action="store_true",
        help="Only test environment without training"
    )

    args = parser.parse_args()

    if args.test_only:
        test_environment(args.task)
    else:
        train_ppo(
            args.task,
            total_timesteps=args.timesteps,
            n_envs=args.n_envs,
            seed=args.seed
        )
