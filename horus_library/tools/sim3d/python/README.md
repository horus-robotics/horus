# Sim3D RL - Python Bindings

Python bindings for the Sim3D robotics simulator, providing Gymnasium-compatible RL environments.

## Features

- 🤖 6 robot learning tasks (reaching, balancing, locomotion, navigation, manipulation, push)
- [LAUNCH] Vectorized environments for parallel training
- [CONFIG] Gymnasium-compatible API
- [FAST] High-performance physics simulation (Rapier3D)
- [DESIGN] Optional 3D visualization (Bevy)

## Installation

### From source

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Python package
cd python
pip install -e .
```

### With RL dependencies

```bash
pip install -e ".[rl]"
```

## Quick Start

```python
import sim3d_rl
import numpy as np

# Create a reaching task environment
env = sim3d_rl.make_env("reaching")

# Reset environment
obs = env.reset()

# Run episode
done = False
total_reward = 0

while not done:
    # Random action
    action = np.random.uniform(-1, 1, size=6)

    # Step environment
    obs, reward, done, truncated, info = env.step(action)
    total_reward += reward

print(f"Episode reward: {total_reward}")
print(f"Success: {info['success']}")
```

## Available Tasks

### 1. Reaching
- **Goal**: Move end-effector to target 3D position
- **Obs dim**: 10 (ee_pos, target_pos, distance, direction)
- **Action dim**: 6 (joint velocities/torques)

```python
env = sim3d_rl.make_env("reaching")
```

### 2. Balancing
- **Goal**: Balance inverted pendulum (cart-pole)
- **Obs dim**: 6 (cart_pos, cart_vel, angle, ang_vel, sin, cos)
- **Action dim**: 1 (cart force)

```python
env = sim3d_rl.make_env("balancing")
```

### 3. Locomotion
- **Goal**: Walk at target velocity while staying upright
- **Obs dim**: 22 (pos, quat, vel, ang_vel, target_vel, vel_error)
- **Action dim**: 12 (joint torques)

```python
env = sim3d_rl.make_env("locomotion")
```

### 4. Navigation
- **Goal**: Navigate to goal position avoiding obstacles
- **Obs dim**: 21 (pos, quat, vel, ang_vel, goal, distance, direction, progress)
- **Action dim**: 2 (linear_vel, angular_vel)

```python
env = sim3d_rl.make_env("navigation")
```

### 5. Manipulation
- **Goal**: Grasp and move object to target
- **Obs dim**: 25 (gripper_pos, gripper_vel, obj_pos, obj_vel, target, distances, grasp_state, directions)
- **Action dim**: 4 (dx, dy, dz, grasp_cmd)

```python
env = sim3d_rl.make_env("manipulation")
```

### 6. Push
- **Goal**: Push object to target location
- **Obs dim**: 30 (pusher_pos, pusher_quat, pusher_vel, obj_pos, obj_vel, target, distances, directions, optimal_dir)
- **Action dim**: 2 (vx, vz)

```python
env = sim3d_rl.make_env("push")
```

## Vectorized Environments

Train multiple environments in parallel:

```python
# Create 8 parallel environments
vec_env = sim3d_rl.make_vec_env("reaching", num_envs=8)

# Reset all
obs = vec_env.reset()  # Shape: (8, obs_dim)

# Step all with vectorized actions
actions = np.random.uniform(-1, 1, size=(8, 6))
obs, rewards, dones, truncateds, infos = vec_env.step(actions)
```

## Training with Stable-Baselines3

```python
from stable_baselines3 import PPO
from stable_baselines3.common.vec_env import DummyVecEnv
import sim3d_rl

# Create environment
env = sim3d_rl.make_env("reaching")

# Wrap in SB3 compatible wrapper
env = DummyVecEnv([lambda: env])

# Create and train agent
model = PPO("MlpPolicy", env, verbose=1)
model.learn(total_timesteps=100000)

# Save model
model.save("reaching_agent")

# Test trained agent
obs = env.reset()
for _ in range(1000):
    action, _states = model.predict(obs, deterministic=True)
    obs, reward, done, info = env.step(action)
    if done:
        obs = env.reset()
```

## Advanced Usage

### Custom Observation/Action Dimensions

```python
# Create environment with custom dimensions
env = sim3d_rl.make_env("reaching", obs_dim=15, action_dim=8)
```

### Environment Properties

```python
# Get environment info
print(f"Observation space: {env.observation_space()}")
print(f"Action space: {env.action_space()}")
print(f"Episode count: {env.episode_count}")
print(f"Total steps: {env.total_steps}")
```

## Performance Tips

1. **Use vectorized environments** for parallel training
2. **Compile with release mode** for maximum performance:
   ```bash
   RUSTFLAGS="-C target-cpu=native" pip install -e .
   ```
3. **Disable rendering** for headless training (automatic in Python bindings)

## API Reference

### Sim3DEnv

Gymnasium-compatible single environment.

**Methods:**
- `reset()` → `observation`
- `step(action)` → `(observation, reward, done, truncated, info)`
- `observation_space()` → `dict`
- `action_space()` → `dict`
- `render(mode)` → `None`
- `close()` → `None`

**Properties:**
- `episode_count` (int): Number of episodes completed
- `total_steps` (int): Total steps across all episodes

### VecSim3DEnv

Vectorized environment for parallel training.

**Methods:**
- `reset()` → `observations` (shape: num_envs × obs_dim)
- `step(actions)` → `(observations, rewards, dones, truncateds, infos)`
- `close()` → `None`

**Properties:**
- `num_envs` (int): Number of parallel environments

## License

MIT OR Apache-2.0

## Contributing

Contributions welcome! Please see the main HORUS repository for guidelines.
