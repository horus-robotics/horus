#!/usr/bin/env python3
"""
Example: Train a reaching task using Stable-Baselines3 PPO
"""

import argparse
import numpy as np
from stable_baselines3 import PPO
from stable_baselines3.common.callbacks import CheckpointCallback, EvalCallback
from stable_baselines3.common.vec_env import DummyVecEnv, VecNormalize
import sim3d_rl


def make_env_fn(task_type="reaching"):
    """Create and return a single environment"""
    def _init():
        return sim3d_rl.make_env(task_type)
    return _init


def train(args):
    """Train PPO agent on reaching task"""
    print(f"Training {args.task} task for {args.timesteps} timesteps")

    # Create vectorized environment
    if args.use_vec:
        print(f"Using {args.num_envs} parallel environments")
        env = sim3d_rl.make_vec_env(args.task, num_envs=args.num_envs)
    else:
        env = DummyVecEnv([make_env_fn(args.task) for _ in range(args.num_envs)])

    # Wrap in normalization
    if args.normalize:
        env = VecNormalize(env, norm_obs=True, norm_reward=True, clip_reward=10.0)

    # Create evaluation environment
    eval_env = DummyVecEnv([make_env_fn(args.task)])
    if args.normalize:
        eval_env = VecNormalize(eval_env, training=False, norm_obs=True, norm_reward=False)

    # Create callbacks
    checkpoint_callback = CheckpointCallback(
        save_freq=args.save_freq,
        save_path=f"./checkpoints/{args.task}/",
        name_prefix="rl_model"
    )

    eval_callback = EvalCallback(
        eval_env,
        best_model_save_path=f"./models/{args.task}/",
        log_path=f"./logs/{args.task}/",
        eval_freq=args.eval_freq,
        deterministic=True,
        render=False
    )

    # Create PPO agent
    model = PPO(
        "MlpPolicy",
        env,
        learning_rate=args.lr,
        n_steps=args.n_steps,
        batch_size=args.batch_size,
        n_epochs=args.n_epochs,
        gamma=args.gamma,
        gae_lambda=args.gae_lambda,
        clip_range=args.clip_range,
        ent_coef=args.ent_coef,
        vf_coef=args.vf_coef,
        max_grad_norm=args.max_grad_norm,
        verbose=1,
        tensorboard_log=f"./tensorboard/{args.task}/"
    )

    # Train agent
    model.learn(
        total_timesteps=args.timesteps,
        callback=[checkpoint_callback, eval_callback],
        log_interval=10
    )

    # Save final model
    model.save(f"./models/{args.task}/final_model")
    if args.normalize:
        env.save(f"./models/{args.task}/vec_normalize.pkl")

    print(f"Training complete! Model saved to ./models/{args.task}/")


def evaluate(args):
    """Evaluate trained agent"""
    print(f"Evaluating {args.task} task for {args.episodes} episodes")

    # Load environment
    env = sim3d_rl.make_env(args.task)

    # Load model
    model = PPO.load(f"./models/{args.task}/final_model")

    # Run evaluation
    episode_rewards = []
    episode_successes = []

    for ep in range(args.episodes):
        obs = env.reset()
        done = False
        total_reward = 0

        while not done:
            action, _states = model.predict(obs, deterministic=True)
            obs, reward, done, truncated, info = env.step(action)
            total_reward += reward

            if done or truncated:
                break

        episode_rewards.append(total_reward)
        episode_successes.append(info['success'])
        print(f"Episode {ep + 1}: Reward = {total_reward:.2f}, Success = {info['success']}")

    # Print statistics
    print("\n" + "="*50)
    print(f"Evaluation Results ({args.episodes} episodes):")
    print(f"  Mean Reward: {np.mean(episode_rewards):.2f} ± {np.std(episode_rewards):.2f}")
    print(f"  Success Rate: {np.mean(episode_successes) * 100:.1f}%")
    print("="*50)


def main():
    parser = argparse.ArgumentParser(description="Train or evaluate Sim3D RL agents")
    parser.add_argument("--mode", type=str, default="train", choices=["train", "eval"],
                        help="Mode: train or eval")
    parser.add_argument("--task", type=str, default="reaching",
                        choices=["reaching", "balancing", "locomotion", "navigation",
                                 "manipulation", "push"],
                        help="Task type")

    # Training arguments
    parser.add_argument("--timesteps", type=int, default=1000000,
                        help="Total training timesteps")
    parser.add_argument("--num-envs", type=int, default=8,
                        help="Number of parallel environments")
    parser.add_argument("--use-vec", action="store_true",
                        help="Use Rust vectorized environments")
    parser.add_argument("--normalize", action="store_true",
                        help="Normalize observations and rewards")

    # PPO hyperparameters
    parser.add_argument("--lr", type=float, default=3e-4, help="Learning rate")
    parser.add_argument("--n-steps", type=int, default=2048, help="Steps per update")
    parser.add_argument("--batch-size", type=int, default=64, help="Batch size")
    parser.add_argument("--n-epochs", type=int, default=10, help="Epochs per update")
    parser.add_argument("--gamma", type=float, default=0.99, help="Discount factor")
    parser.add_argument("--gae-lambda", type=float, default=0.95, help="GAE lambda")
    parser.add_argument("--clip-range", type=float, default=0.2, help="PPO clip range")
    parser.add_argument("--ent-coef", type=float, default=0.0, help="Entropy coefficient")
    parser.add_argument("--vf-coef", type=float, default=0.5, help="Value function coefficient")
    parser.add_argument("--max-grad-norm", type=float, default=0.5, help="Max gradient norm")

    # Callback arguments
    parser.add_argument("--save-freq", type=int, default=50000,
                        help="Save checkpoint frequency")
    parser.add_argument("--eval-freq", type=int, default=10000,
                        help="Evaluation frequency")

    # Evaluation arguments
    parser.add_argument("--episodes", type=int, default=100,
                        help="Number of evaluation episodes")

    args = parser.parse_args()

    if args.mode == "train":
        train(args)
    elif args.mode == "eval":
        evaluate(args)


if __name__ == "__main__":
    main()
