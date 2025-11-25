"""
sim2d - Simple 2D robotics simulator

A lightweight 2D physics-based robotics simulator integrated with HORUS.

## Usage

```python
from horus.library.sim2d import Sim2D

# Create simulator
sim = Sim2D(robot_name="my_robot", headless=False)

# Add obstacles
sim.add_obstacle(pos=(5.0, 5.0), size=(1.0, 1.0), shape="rectangle")
sim.add_obstacle(pos=(10.0, 8.0), size=(0.5, 0.5), shape="circle")

# Run simulation
sim.run(duration=10.0)
```
"""

try:
    from horus.library._library.sim2d import Sim2D, RobotConfigPy, WorldConfigPy

    __all__ = ["Sim2D", "RobotConfigPy", "WorldConfigPy"]
except ImportError:
    # Fallback if native module not available
    __all__ = []
