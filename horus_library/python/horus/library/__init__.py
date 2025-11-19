"""
HORUS Library - Standard robotics messages, nodes, and algorithms

The official standard library for the HORUS robotics framework (Python bindings).

## Structure

```
horus_library/
├── messages/      # Shared memory-safe messages
├── nodes/         # Reusable nodes (future)
└── algorithms/    # Common algorithms (future)
```

## Usage

```python
# Option 1: Import from submodules (organized)
from horus_library.messages import Pose2D, CmdVel, LaserScan
from horus_library.nodes import ...  # Future

# Option 2: Import from root (convenient)
from horus_library import Pose2D, CmdVel, LaserScan

# Option 3: Via main horus package (recommended)
from horus import Pose2D, CmdVel, LaserScan
```

## Examples

```python
>>> from horus import Pose2D, Twist, CmdVel, LaserScan
>>> pose = Pose2D(x=1.0, y=2.0, theta=0.5)
>>> cmd = Twist.new_2d(linear_x=0.5, angular_z=0.1)
>>> vel = CmdVel(linear=1.0, angular=0.5)
>>> scan = LaserScan()
>>> imu = Imu()
>>> battery = BatteryState(voltage=12.6, percentage=85.0)
>>> gps = NavSatFix(latitude=37.7749, longitude=-122.4194, altitude=10.0)
```
"""

__version__ = "0.1.5"

# Submodules
from . import messages
from . import nodes
from . import algorithms

# Re-export all messages at the root for convenience (matches Rust API)
from .messages import *

__all__ = [
    # Submodules
    "messages",
    "nodes",
    "algorithms",
]

# Add all message types to __all__
__all__.extend(messages.__all__)

# Future: Add node types when implemented
# __all__.extend(nodes.__all__)

# Future: Add algorithm types when implemented
# __all__.extend(algorithms.__all__)
