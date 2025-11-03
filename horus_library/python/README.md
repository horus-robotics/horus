# HORUS Library for Python

Standard robotics messages, nodes, and algorithms for HORUS.

## Installation

```bash
cd horus_library/python
maturin develop --release
```

Or install from PyPI (when published):
```bash
pip install horus-library
```

## Quick Start

```python
from horus.library import Pose2D, Twist, Transform

# Create a 2D pose
pose = Pose2D(x=1.0, y=2.0, theta=0.5)
print(pose)  # Pose2D(x=1.000, y=2.000, theta=0.500)

# Create a twist command (2D robot)
cmd = Twist.new_2d(linear_x=0.5, angular_z=0.1)
print(cmd)  # Twist(linear=[0.50, 0.00, 0.00], angular=[0.00, 0.00, 0.10])

# Create a transform
tf = Transform.from_pose_2d(pose)
print(tf)
```

## Available Message Types

### Geometry Messages
- **Pose2D** - 2D position and orientation (x, y, theta)
- **Twist** - Linear and angular velocity (3D)
- **Transform** - 3D transformation with translation and quaternion rotation
- **Point3** - 3D point
- **Vector3** - 3D vector with operations (dot, cross, normalize)
- **Quaternion** - 3D rotation representation

## Usage with HORUS Nodes

```python
import horus
from horus.library import Pose2D, Twist

def robot_tick(node):
    # Get current pose
    if node.has_msg("pose"):
        pose = node.get("pose")  # Returns Pose2D
        print(f"Robot at: {pose}")

    # Send velocity command
    cmd = Twist.new_2d(linear_x=0.5, angular_z=0.0)
    node.send("cmd_vel", cmd)

robot = horus.Node(
    name="robot_controller",
    subs="pose",
    pubs="cmd_vel",
    tick=robot_tick
)

horus.run(robot)
```

## Features

- ✅ Type-safe message classes
- ✅ Zero-copy where possible (same memory layout as Rust)
- ✅ Familiar Pythonic API
- ✅ All methods documented
- ✅ Compatible with horus-robotics core package

## Coming Soon

- Sensor messages (LaserScan, IMU, Odometry)
- Control messages (MotorCommand, ServoCommand)
- Standard nodes (PIDController, Logger, etc.)
- Algorithms (filters, transforms, etc.)

## License

Apache-2.0
