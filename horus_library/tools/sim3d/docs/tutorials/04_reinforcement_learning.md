# Tutorial 4: Reinforcement Learning

This tutorial covers setting up Sim3D for reinforcement learning, including environment design, observations, actions, rewards, domain randomization, and Python bindings.

## Prerequisites

- Completed [Tutorial 3: Sensors](03_sensors.md)
- Python 3.8+ with pip
- Basic understanding of RL concepts

## RL Architecture Overview

Sim3D supports two RL workflows:

1. **Native Rust**: High-performance training with direct integration
2. **Python Bindings**: Gymnasium-compatible interface for popular RL libraries

```
+-------------------+     +------------------+     +-------------------+
|   RL Algorithm    | <-> |  sim3d_rl (Py)   | <-> |   Sim3D Engine    |
| (PPO, SAC, etc.)  |     | (Gymnasium API)  |     |  (Rust/Rapier3D)  |
+-------------------+     +------------------+     +-------------------+
```

## Setting Up an RL Environment

### Defining the Environment in Rust

```rust
use bevy::prelude::*;
use sim3d::rl::{RLEnvironment, ResetInfo, StepResult};
use sim3d::physics::PhysicsWorld;

#[derive(Resource)]
pub struct NavigationEnv {
    // Environment configuration
    pub max_steps: usize,
    pub current_step: usize,

    // Goal
    pub target_position: Vec3,
    pub goal_radius: f32,

    // State
    pub robot_position: Vec3,
    pub robot_velocity: Vec3,
    pub robot_orientation: Quat,

    // Observation/action dimensions
    pub obs_dim: usize,
    pub action_dim: usize,
}

impl Default for NavigationEnv {
    fn default() -> Self {
        Self {
            max_steps: 1000,
            current_step: 0,
            target_position: Vec3::new(5.0, 0.0, 5.0),
            goal_radius: 0.5,
            robot_position: Vec3::ZERO,
            robot_velocity: Vec3::ZERO,
            robot_orientation: Quat::IDENTITY,
            obs_dim: 20,
            action_dim: 2,
        }
    }
}

impl RLEnvironment for NavigationEnv {
    fn reset(&mut self) -> (Vec<f32>, ResetInfo) {
        self.current_step = 0;

        // Randomize starting position
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.robot_position = Vec3::new(
            rng.gen_range(-2.0..2.0),
            0.0,
            rng.gen_range(-2.0..2.0),
        );
        self.robot_velocity = Vec3::ZERO;
        self.robot_orientation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));

        // Randomize goal
        self.target_position = Vec3::new(
            rng.gen_range(3.0..8.0),
            0.0,
            rng.gen_range(-5.0..5.0),
        );

        let obs = self.get_observation();
        let info = ResetInfo::default();

        (obs, info)
    }

    fn step(&mut self, action: &[f32]) -> StepResult {
        self.current_step += 1;

        // Apply action (linear velocity, angular velocity)
        let linear_vel = action[0].clamp(-1.0, 1.0);
        let angular_vel = action[1].clamp(-1.0, 1.0);

        // Update robot state (simplified kinematics)
        let dt = 0.02;  // 50 Hz
        let forward = self.robot_orientation * Vec3::Z;
        self.robot_velocity = forward * linear_vel * 2.0;  // Max 2 m/s
        self.robot_position += self.robot_velocity * dt;
        self.robot_orientation *= Quat::from_rotation_y(angular_vel * 2.0 * dt);

        // Compute reward
        let reward = self.compute_reward();

        // Check termination
        let (done, truncated) = self.check_termination();

        let obs = self.get_observation();
        let info = self.get_info();

        StepResult {
            observation: obs,
            reward,
            done,
            truncated,
            info,
        }
    }

    fn get_observation(&self) -> Vec<f32> {
        let mut obs = Vec::with_capacity(self.obs_dim);

        // Robot position (3)
        obs.push(self.robot_position.x / 10.0);
        obs.push(self.robot_position.y / 10.0);
        obs.push(self.robot_position.z / 10.0);

        // Robot velocity (3)
        obs.push(self.robot_velocity.x / 2.0);
        obs.push(self.robot_velocity.y / 2.0);
        obs.push(self.robot_velocity.z / 2.0);

        // Robot orientation as quaternion (4)
        obs.push(self.robot_orientation.x);
        obs.push(self.robot_orientation.y);
        obs.push(self.robot_orientation.z);
        obs.push(self.robot_orientation.w);

        // Target position (3)
        obs.push(self.target_position.x / 10.0);
        obs.push(self.target_position.y / 10.0);
        obs.push(self.target_position.z / 10.0);

        // Relative target position (3)
        let rel_target = self.target_position - self.robot_position;
        obs.push(rel_target.x / 10.0);
        obs.push(rel_target.y / 10.0);
        obs.push(rel_target.z / 10.0);

        // Distance to target (1)
        obs.push(rel_target.length() / 15.0);

        // Heading to target (1)
        let target_dir = rel_target.normalize_or_zero();
        let robot_forward = self.robot_orientation * Vec3::Z;
        let heading_error = robot_forward.dot(target_dir);
        obs.push(heading_error);

        // Pad to obs_dim
        while obs.len() < self.obs_dim {
            obs.push(0.0);
        }

        obs
    }
}

impl NavigationEnv {
    fn compute_reward(&self) -> f32 {
        let distance = (self.target_position - self.robot_position).length();

        // Dense reward: negative distance
        let distance_reward = -distance * 0.1;

        // Bonus for reaching goal
        let goal_bonus = if distance < self.goal_radius { 100.0 } else { 0.0 };

        // Penalty for going out of bounds
        let bounds_penalty = if self.robot_position.length() > 12.0 { -50.0 } else { 0.0 };

        // Small time penalty to encourage efficiency
        let time_penalty = -0.01;

        distance_reward + goal_bonus + bounds_penalty + time_penalty
    }

    fn check_termination(&self) -> (bool, bool) {
        let distance = (self.target_position - self.robot_position).length();

        // Success: reached goal
        let success = distance < self.goal_radius;

        // Failure: out of bounds
        let out_of_bounds = self.robot_position.length() > 12.0;

        // Truncation: max steps reached
        let truncated = self.current_step >= self.max_steps;

        let done = success || out_of_bounds;

        (done, truncated)
    }

    fn get_info(&self) -> std::collections::HashMap<String, f32> {
        let mut info = std::collections::HashMap::new();
        let distance = (self.target_position - self.robot_position).length();

        info.insert("distance_to_goal".to_string(), distance);
        info.insert("step".to_string(), self.current_step as f32);
        info.insert("success".to_string(), if distance < self.goal_radius { 1.0 } else { 0.0 });

        info
    }
}
```

## Domain Randomization

Domain randomization improves sim-to-real transfer by training on varied environments:

```rust
use sim3d::rl::domain_randomization::{
    DomainRandomizationConfig,
    DomainRandomizer,
    PhysicsRandomization,
    VisualRandomization,
    EnvironmentRandomization,
};

fn setup_domain_randomization() -> DomainRandomizer {
    let config = DomainRandomizationConfig {
        physics: PhysicsRandomization {
            // Mass variations (multiplier)
            mass_range: Some((0.8, 1.2)),
            // Friction coefficient range
            friction_range: Some((0.3, 0.9)),
            // Bounciness
            restitution_range: Some((0.0, 0.3)),
            // Gravity variation
            gravity_range: Some((
                Vec3::new(0.0, -8.0, 0.0),
                Vec3::new(0.0, -12.0, 0.0),
            )),
            // Joint parameters
            joint_damping_range: Some((0.1, 1.0)),
            joint_stiffness_range: Some((50.0, 200.0)),
            // Center of mass offset
            com_offset_range: Some((
                Vec3::new(-0.05, -0.05, -0.05),
                Vec3::new(0.05, 0.05, 0.05),
            )),
        },
        visual: VisualRandomization {
            // Lighting variations
            light_intensity_range: Some((500.0, 2000.0)),
            light_color_temp_range: Some((3000.0, 7000.0)),
            light_direction_range: Some((
                Vec3::new(-1.0, 0.5, -1.0),
                Vec3::new(1.0, 1.0, 1.0),
            )),
            // Material colors
            material_color_range: Some((
                Color::srgb(0.5, 0.5, 0.5),
                Color::srgb(1.0, 1.0, 1.0),
            )),
            randomize_textures: false,
            // Camera noise
            camera_position_noise: Some((
                Vec3::new(-0.1, -0.1, -0.1),
                Vec3::new(0.1, 0.1, 0.1),
            )),
            camera_rotation_noise: Some((-0.1, 0.1)),
        },
        environment: EnvironmentRandomization {
            // Initial state variations
            object_position_noise: Some((
                Vec3::new(-0.2, 0.0, -0.2),
                Vec3::new(0.2, 0.1, 0.2),
            )),
            object_rotation_noise: Some((
                -std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            )),
            target_position_noise: Some((
                Vec3::new(-0.1, -0.1, -0.1),
                Vec3::new(0.1, 0.1, 0.1),
            )),
            // Scale variations
            object_scale_range: Some((
                Vec3::new(0.9, 0.9, 0.9),
                Vec3::new(1.1, 1.1, 1.1),
            )),
            // Distractor objects
            num_distractors_range: Some((0, 5)),
        },
        seed: None,  // Random seed
    };

    DomainRandomizer::new(config)
}

// Apply randomization on reset
fn apply_randomization(
    randomizer: &DomainRandomizer,
    physics_world: &mut PhysicsWorld,
) {
    // Sample random physics parameters
    let friction = randomizer.sample_friction();
    let restitution = randomizer.sample_restitution();
    let mass_multiplier = randomizer.sample_mass();

    // Apply to all colliders
    for (_handle, collider) in physics_world.collider_set.iter_mut() {
        collider.set_friction(friction);
        collider.set_restitution(restitution);
    }

    // Apply to rigid bodies
    for (_handle, rb) in physics_world.rigid_body_set.iter_mut() {
        let current_mass = rb.mass();
        rb.set_additional_mass(current_mass * mass_multiplier, true);
    }

    // Sample and apply gravity
    let gravity = randomizer.sample_gravity();
    physics_world.set_gravity(rapier3d::prelude::vector![gravity.x, gravity.y, gravity.z]);
}
```

## Python Bindings

### Installation

```bash
cd horus_library/tools/sim3d/python
pip install -e .
```

### Basic Usage

```python
import sim3d_rl
import numpy as np

# Create environment
env = sim3d_rl.make_env(
    task="navigation",
    obs_dim=20,
    action_dim=2
)

# Reset environment
obs, info = env.reset()
print(f"Observation shape: {obs.shape}")
print(f"Action space: {env.action_space}")

# Run episode
total_reward = 0
done = False

while not done:
    # Random action
    action = env.action_space.sample()

    # Step environment
    obs, reward, done, truncated, info = env.step(action)
    total_reward += reward

    if done or truncated:
        print(f"Episode finished! Total reward: {total_reward:.2f}")
        print(f"Info: {info}")
        break

env.close()
```

### Vectorized Environments

For parallel training:

```python
import sim3d_rl

# Create vectorized environment (4 parallel envs)
vec_env = sim3d_rl.make_vec_env(
    task="navigation",
    obs_dim=20,
    action_dim=2,
    num_envs=4
)

# Reset all environments
obs = vec_env.reset()
print(f"Observations shape: {obs.shape}")  # (4, 20)

# Step all environments
actions = np.random.uniform(-1, 1, size=(4, 2))
obs, rewards, dones, infos = vec_env.step(actions)

vec_env.close()
```

## Training with Stable-Baselines3

Complete PPO training example:

```python
#!/usr/bin/env python3
"""Train PPO agent on Sim3D navigation task."""

import numpy as np
import torch
from stable_baselines3 import PPO
from stable_baselines3.common.env_util import make_vec_env
from stable_baselines3.common.callbacks import EvalCallback, CheckpointCallback
from stable_baselines3.common.vec_env import SubprocVecEnv
import sim3d_rl


class Sim3DEnvWrapper:
    """Wrapper to make sim3d_rl compatible with SB3."""

    def __init__(self, task: str, obs_dim: int, action_dim: int):
        self.env = sim3d_rl.make_env(task, obs_dim=obs_dim, action_dim=action_dim)
        self.observation_space = self.env.observation_space
        self.action_space = self.env.action_space

    def reset(self, seed=None):
        if seed is not None:
            np.random.seed(seed)
        return self.env.reset()

    def step(self, action):
        return self.env.step(action)

    def close(self):
        self.env.close()

    def render(self, mode='human'):
        pass  # Handled internally


def make_env(task: str, rank: int, seed: int = 0):
    """Create environment factory."""
    def _init():
        task_configs = {
            "reaching": {"obs_dim": 10, "action_dim": 6},
            "balancing": {"obs_dim": 8, "action_dim": 1},
            "locomotion": {"obs_dim": 12, "action_dim": 2},
            "navigation": {"obs_dim": 20, "action_dim": 2},
            "manipulation": {"obs_dim": 15, "action_dim": 7},
            "push": {"obs_dim": 18, "action_dim": 2},
        }

        config = task_configs[task]
        env = Sim3DEnvWrapper(task, config["obs_dim"], config["action_dim"])
        env.reset(seed=seed + rank)
        return env

    return _init


def train(
    task: str = "navigation",
    total_timesteps: int = 1_000_000,
    n_envs: int = 8,
    seed: int = 42,
):
    """Train PPO on Sim3D environment."""
    print(f"Training PPO on {task}")
    print(f"Total timesteps: {total_timesteps:,}")
    print(f"Parallel environments: {n_envs}")

    # Create vectorized training environment
    train_env = SubprocVecEnv([make_env(task, i, seed) for i in range(n_envs)])

    # Create evaluation environment
    eval_env = SubprocVecEnv([make_env(task, 100, seed + 1000)])

    # Callbacks
    eval_callback = EvalCallback(
        eval_env,
        best_model_save_path=f"./models/{task}/best/",
        log_path=f"./logs/{task}/eval/",
        eval_freq=10000 // n_envs,
        n_eval_episodes=10,
        deterministic=True,
    )

    checkpoint_callback = CheckpointCallback(
        save_freq=50000 // n_envs,
        save_path=f"./models/{task}/checkpoints/",
        name_prefix=f"ppo_{task}",
    )

    # Create PPO model
    model = PPO(
        "MlpPolicy",
        train_env,
        learning_rate=3e-4,
        n_steps=2048,
        batch_size=64,
        n_epochs=10,
        gamma=0.99,
        gae_lambda=0.95,
        clip_range=0.2,
        ent_coef=0.01,
        vf_coef=0.5,
        max_grad_norm=0.5,
        tensorboard_log=f"./logs/{task}/tensorboard/",
        policy_kwargs={
            "net_arch": {"pi": [256, 256], "vf": [256, 256]},
            "activation_fn": torch.nn.ReLU,
        },
        verbose=1,
        seed=seed,
    )

    # Train
    print("\nStarting training...")
    model.learn(
        total_timesteps=total_timesteps,
        callback=[eval_callback, checkpoint_callback],
        progress_bar=True,
    )

    # Save final model
    model.save(f"./models/{task}/final_model")
    print(f"\nTraining complete! Model saved to ./models/{task}/final_model")

    # Cleanup
    train_env.close()
    eval_env.close()

    return model


def evaluate(model_path: str, task: str, n_episodes: int = 10):
    """Evaluate trained model."""
    task_configs = {
        "reaching": {"obs_dim": 10, "action_dim": 6},
        "balancing": {"obs_dim": 8, "action_dim": 1},
        "locomotion": {"obs_dim": 12, "action_dim": 2},
        "navigation": {"obs_dim": 20, "action_dim": 2},
        "manipulation": {"obs_dim": 15, "action_dim": 7},
        "push": {"obs_dim": 18, "action_dim": 2},
    }

    config = task_configs[task]
    env = Sim3DEnvWrapper(task, config["obs_dim"], config["action_dim"])
    model = PPO.load(model_path)

    rewards = []
    successes = []

    for episode in range(n_episodes):
        obs, _ = env.reset()
        total_reward = 0
        done = False

        while not done:
            action, _ = model.predict(obs, deterministic=True)
            obs, reward, done, truncated, info = env.step(action)
            total_reward += reward

            if done or truncated:
                break

        rewards.append(total_reward)
        successes.append(info.get("success", 0))

        print(f"Episode {episode + 1}: reward={total_reward:.2f}, success={info.get('success', False)}")

    print(f"\nEvaluation Results ({n_episodes} episodes):")
    print(f"  Mean reward: {np.mean(rewards):.2f} +/- {np.std(rewards):.2f}")
    print(f"  Success rate: {np.mean(successes) * 100:.1f}%")

    env.close()


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--task", default="navigation", choices=[
        "reaching", "balancing", "locomotion", "navigation", "manipulation", "push"
    ])
    parser.add_argument("--timesteps", type=int, default=1_000_000)
    parser.add_argument("--n-envs", type=int, default=8)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--eval", type=str, help="Path to model for evaluation")

    args = parser.parse_args()

    if args.eval:
        evaluate(args.eval, args.task)
    else:
        train(args.task, args.timesteps, args.n_envs, args.seed)
```

## Implementing Custom Reward Functions

```python
import numpy as np


class RewardFunction:
    """Base class for reward functions."""

    def __call__(self, state: dict, action: np.ndarray, next_state: dict) -> float:
        raise NotImplementedError


class NavigationReward(RewardFunction):
    """Reward for navigation task."""

    def __init__(
        self,
        goal_bonus: float = 100.0,
        distance_scale: float = 0.1,
        collision_penalty: float = -10.0,
        time_penalty: float = -0.01,
        velocity_bonus_scale: float = 0.5,
    ):
        self.goal_bonus = goal_bonus
        self.distance_scale = distance_scale
        self.collision_penalty = collision_penalty
        self.time_penalty = time_penalty
        self.velocity_bonus_scale = velocity_bonus_scale

    def __call__(self, state: dict, action: np.ndarray, next_state: dict) -> float:
        reward = 0.0

        # Distance reward (dense)
        prev_dist = state["distance_to_goal"]
        curr_dist = next_state["distance_to_goal"]
        reward += (prev_dist - curr_dist) * self.distance_scale

        # Velocity toward goal bonus
        if "velocity_toward_goal" in next_state:
            reward += next_state["velocity_toward_goal"] * self.velocity_bonus_scale

        # Goal reached bonus
        if curr_dist < state.get("goal_radius", 0.5):
            reward += self.goal_bonus

        # Collision penalty
        if next_state.get("collision", False):
            reward += self.collision_penalty

        # Time penalty
        reward += self.time_penalty

        return reward


class ManipulationReward(RewardFunction):
    """Reward for manipulation task."""

    def __init__(
        self,
        grasp_bonus: float = 50.0,
        place_bonus: float = 100.0,
        approach_scale: float = 1.0,
        drop_penalty: float = -20.0,
    ):
        self.grasp_bonus = grasp_bonus
        self.place_bonus = place_bonus
        self.approach_scale = approach_scale
        self.drop_penalty = drop_penalty

    def __call__(self, state: dict, action: np.ndarray, next_state: dict) -> float:
        reward = 0.0

        if not state["object_grasped"]:
            # Approach reward
            prev_dist = state["gripper_to_object_dist"]
            curr_dist = next_state["gripper_to_object_dist"]
            reward += (prev_dist - curr_dist) * self.approach_scale

            # Grasp bonus
            if next_state["object_grasped"]:
                reward += self.grasp_bonus
        else:
            # Move to target
            prev_dist = state["object_to_target_dist"]
            curr_dist = next_state["object_to_target_dist"]
            reward += (prev_dist - curr_dist) * self.approach_scale

            # Place bonus
            if curr_dist < 0.05:
                reward += self.place_bonus

            # Drop penalty
            if not next_state["object_grasped"]:
                reward += self.drop_penalty

        return reward
```

## Curriculum Learning

Gradually increase task difficulty:

```python
class CurriculumManager:
    """Manage curriculum learning progression."""

    def __init__(self, stages: list, success_threshold: float = 0.8):
        self.stages = stages
        self.current_stage = 0
        self.success_threshold = success_threshold
        self.success_history = []
        self.history_size = 100

    def get_current_config(self) -> dict:
        """Get current stage configuration."""
        return self.stages[min(self.current_stage, len(self.stages) - 1)]

    def update(self, success: bool):
        """Update curriculum based on episode result."""
        self.success_history.append(1.0 if success else 0.0)

        # Keep only recent history
        if len(self.success_history) > self.history_size:
            self.success_history = self.success_history[-self.history_size:]

        # Check for stage advancement
        if len(self.success_history) >= self.history_size:
            success_rate = np.mean(self.success_history)
            if success_rate >= self.success_threshold:
                if self.current_stage < len(self.stages) - 1:
                    self.current_stage += 1
                    self.success_history = []
                    print(f"Curriculum: Advanced to stage {self.current_stage}")


# Example curriculum for navigation
navigation_curriculum = [
    # Stage 1: Short distances, no obstacles
    {"goal_distance_range": (2.0, 4.0), "num_obstacles": 0},
    # Stage 2: Medium distances, few obstacles
    {"goal_distance_range": (3.0, 6.0), "num_obstacles": 3},
    # Stage 3: Long distances, more obstacles
    {"goal_distance_range": (4.0, 8.0), "num_obstacles": 5},
    # Stage 4: Full difficulty
    {"goal_distance_range": (5.0, 10.0), "num_obstacles": 8},
]

curriculum = CurriculumManager(navigation_curriculum)
```

## Running Training

### Command Line

```bash
# Train navigation task
python train_ppo.py --task navigation --timesteps 2000000 --n-envs 16

# Train with specific seed
python train_ppo.py --task manipulation --seed 123

# Evaluate trained model
python train_ppo.py --task navigation --eval ./models/navigation/best/best_model.zip
```

### Monitor Training

```bash
# Start TensorBoard
tensorboard --logdir ./logs/

# View at http://localhost:6006
```

## Performance Tips

1. **Use vectorized environments**: More parallel envs = faster training
2. **Enable headless mode**: No rendering overhead during training
3. **Batch observations**: Minimize Python-Rust boundary crossings
4. **Use GPU**: For neural network computations
5. **Profile bottlenecks**: Use `py-spy` or similar tools

```python
# Optimal configuration for fast training
train(
    task="navigation",
    total_timesteps=10_000_000,
    n_envs=32,  # More parallel envs
    seed=42,
)
```

## Next Steps

- [API Reference: Sensors](../api/sensors.md) - Sensor configurations for RL
- [API Reference: Multi-Robot](../api/multi_robot.md) - Multi-agent RL
- [Deployment: Cloud](../deployment/cloud.md) - Scale training on cloud
