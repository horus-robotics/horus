"""
sim3d - 3D robotics simulator with RL support

A high-performance 3D physics simulator with Gymnasium-compatible RL environments.

## Usage

```python
from horus.library.sim3d import make_env, make_vec_env, Sim3DEnv

# Create single environment
env = make_env("locomotion")
obs = env.reset()
obs, reward, done, truncated, info = env.step([0.1] * 12)

# Create vectorized environment for parallel training
vec_env = make_vec_env("reaching", num_envs=8)
obs = vec_env.reset()

# Available tasks: reaching, balancing, locomotion, navigation, manipulation, push
```

## Available RL Tasks

| Task | Obs Dim | Action Dim | Description |
|------|---------|------------|-------------|
| reaching | 10 | 6 | End-effector reaching targets |
| balancing | 6 | 1 | Pole/pendulum balancing |
| locomotion | 22 | 12 | Robot walking/running |
| navigation | 21 | 2 | Goal-based navigation |
| manipulation | 25 | 4 | Object manipulation/grasping |
| push | 30 | 2 | Object pushing tasks |
"""

try:
    from horus.library._library.sim3d import (
        Sim3DEnv,
        VecSim3DEnv,
        make_env,
        make_vec_env,
    )

    __all__ = ["Sim3DEnv", "VecSim3DEnv", "make_env", "make_vec_env"]
except ImportError:
    # sim3d feature not enabled or native module not available
    __all__ = []
