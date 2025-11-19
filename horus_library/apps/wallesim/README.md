# WALL-E Simulation (wallesim)

A 3D simulation demo featuring WALL-E, the lovable waste-collecting robot from Pixar's WALL-E, navigating the Axiom spaceship's cargo bay.

## Overview

This example demonstrates HORUS sim3d capabilities with:
- **WALL-E Robot Model**: Authentic URDF model with articulated neck, head, arms, and tank tracks
- **Axiom Cargo Bay**: Futuristic spaceship environment with storage containers, trash cubes, and obstacles
- **LiDAR Sensor**: 360-degree scanning for navigation and obstacle detection
- **Physics Simulation**: Realistic robot dynamics and collision handling

## Features

### WALL-E Robot
- **Tank Drive**: Differential drive using left/right tracks
- **Articulated Neck**: Revolute joint for head tilt (±45°)
- **Rotating Head**: Pan joint for looking around (±90°)
- **Movable Arms**: Simple arm joints for gestures (±90°)
- **Binocular Eyes**: Iconic dual-cylinder eye design
- **LiDAR Sensor**: Mounted on head for environmental scanning

### Axiom Cargo Bay World
- **30m × 25m** spaceship cargo hold
- **Storage containers** in various colors (blue, red, green, yellow)
- **Trash cubes** - WALL-E's signature compressed waste
- **Cylindrical storage tanks** for navigation obstacles
- **Clean futuristic lighting** with multiple light sources
- **Easter egg**: Plant-in-boot from the movie!

## Installation

After running `./install.sh`, the wallesim example will be available in:

```bash
~/.horus/cache/horus@VERSION/examples/wallesim/
```

### Copy to Your Workspace

```bash
# Copy the example to your workspace
cp -r ~/.horus/cache/horus@VERSION/examples/wallesim ~/my_wallesim

# Navigate to your copy
cd ~/my_wallesim
```

## Quick Start

### Option 1: Using Launch Script (Easiest)

The easiest way to launch the simulation:

```bash
cd ~/my_wallesim
./launch.sh
```

This script automatically handles all file paths and launches the simulation.

### Option 2: Using HORUS CLI with Absolute Path

If you want to run from anywhere:

```bash
cd ~/my_wallesim
horus sim3d --world "$(pwd)/world.yaml"
```

Or specify the full path directly:

```bash
horus sim3d --world /path/to/your/wallesim/world.yaml
```

This will:
1. Load the Axiom cargo bay world
2. Spawn WALL-E at the center
3. Initialize the LiDAR sensor
4. Open the 3D visualization window

### Option 2: Using Python

Create a Python script `run_wallesim.py`:

```python
#!/usr/bin/env python3
from horus.tools.sim3d import Sim3D

# Load the simulation
sim = Sim3D()
sim.load_world("world.yaml")

# Run the simulation
sim.run()
```

Then execute:
```bash
python3 run_wallesim.py
```

## Controlling WALL-E

### Keyboard Controls (in simulation window)

- **Arrow Keys**: Move WALL-E forward/backward, turn left/right
- **W/S**: Tilt head up/down
- **A/D**: Rotate head left/right
- **Q/E**: Move left/right arms
- **Space**: Stop all motion
- **R**: Reset WALL-E position
- **ESC**: Exit simulation

### Programmatic Control

Example Python script for autonomous navigation:

```python
from horus import Node, Publisher
from horus.messages import CmdVel
import time

class WallEController(Node):
    def __init__(self):
        super().__init__("walle_controller")
        self.cmd_pub = self.create_publisher("walle/cmd_vel", CmdVel)

    def move_forward(self, duration=2.0):
        """Move WALL-E forward"""
        cmd = CmdVel(linear_x=0.2, angular_z=0.0)
        self.cmd_pub.publish(cmd)
        time.sleep(duration)
        self.stop()

    def turn_left(self, duration=1.0):
        """Turn WALL-E left"""
        cmd = CmdVel(linear_x=0.0, angular_z=0.5)
        self.cmd_pub.publish(cmd)
        time.sleep(duration)
        self.stop()

    def stop(self):
        """Stop WALL-E"""
        cmd = CmdVel(linear_x=0.0, angular_z=0.0)
        self.cmd_pub.publish(cmd)

# Usage
if __name__ == "__main__":
    controller = WallEController()

    # Simple navigation routine
    controller.move_forward(2.0)
    controller.turn_left(1.5)
    controller.move_forward(3.0)
    controller.stop()
```

## File Structure

```
wallesim/
├── horus.yaml              # Project configuration
├── world.yaml              # Axiom cargo bay scene definition
├── README.md               # This file
└── models/
    └── walle/
        └── walle.urdf      # WALL-E robot model
```

## Customization

### Modify the World

Edit `world.yaml` to:
- Add more obstacles or containers
- Change lighting conditions
- Adjust world size
- Add new trash piles
- Modify material colors/properties

Example - adding a new box:
```yaml
objects:
  - type: box
    name: "my_custom_box"
    position: [1.0, 0.5, 2.0]
    size: [1.0, 1.0, 1.0]
    static: true
    material:
      color: [0.8, 0.2, 0.8, 1.0]  # Purple
      friction: 0.6
```

### Modify WALL-E

Edit `models/walle/walle.urdf` to:
- Adjust joint limits
- Change link dimensions
- Modify mass/inertia properties
- Add new sensors
- Change visual appearance

### Sensor Configuration

The LiDAR sensor can be configured in `world.yaml`:
```yaml
sensors:
  - type: lidar2d
    name: "main_lidar"
    link: "lidar_link"
    config:
      num_rays: 720        # Increase resolution
      fov: 6.2832          # Full 360°
      max_range: 20.0      # Extend range
      rate_hz: 20.0        # Faster scanning
```

## Advanced Usage

### Record Sensor Data

```python
from horus import Node, Subscriber
from horus.messages import LaserScan
import numpy as np

class DataRecorder(Node):
    def __init__(self):
        super().__init__("data_recorder")
        self.lidar_sub = self.create_subscriber(
            "walle/lidar/scan",
            LaserScan,
            self.lidar_callback
        )
        self.scans = []

    def lidar_callback(self, msg):
        # Save scan data
        self.scans.append(msg.ranges)

        # Process data
        min_dist = np.min(msg.ranges)
        if min_dist < 0.5:
            print(f"⚠️  Obstacle detected at {min_dist:.2f}m!")

# Run recorder
recorder = DataRecorder()
```

### Create Autonomous Navigation

Combine with path planning nodes:

```bash
# Terminal 1: Start simulation
horus sim3d --world world.yaml

# Terminal 2: Run path planner
horus run --node path_planner --config nav_config.yaml

# Terminal 3: Run WALL-E controller
python3 autonomous_walle.py
```

## Troubleshooting

### WALL-E falls through the floor
- Ensure physics is enabled in simulation
- Check URDF inertial properties
- Verify collision geometry is defined

### LiDAR not working
- Confirm sensor link name matches URDF
- Check LiDAR is mounted on a valid link
- Verify sensor topic name in subscriber

### Poor visualization performance
- Reduce LiDAR rays: `num_rays: 180`
- Lower update rate: `rate_hz: 5.0`
- Decrease world size or object count

### Robot moves unexpectedly
- Check joint limits in URDF
- Verify cmd_vel topic name
- Ensure proper coordinate frames

## Technical Specifications

### WALL-E Dimensions
- **Body**: 0.4m × 0.5m × 0.3m (W×D×H)
- **Track Width**: 0.54m (between tracks)
- **Total Mass**: ~21.5 kg
- **Track Radius**: 0.08m

### Cargo Bay Dimensions
- **Floor**: 30m × 25m
- **Height**: 5m
- **Walls**: 0.3m thick

### Performance
- **Physics Step**: 60 Hz (configurable)
- **Rendering**: 30-60 FPS (depends on hardware)
- **LiDAR Update**: 10 Hz (configurable)

## Credits

- **Robot Design**: Inspired by WALL-E from Pixar Animation Studios
- **HORUS Framework**: HORUS Team
- **sim3d Tool**: HORUS 3D simulation engine

## License

This example is licensed under Apache-2.0, same as HORUS framework.

**Note**: WALL-E character and design are trademarks of Pixar Animation Studios / The Walt Disney Company. This simulation is a fan-made educational example and is not affiliated with or endorsed by Pixar or Disney.

## Next Steps

- Explore other HORUS examples: `snakesim`, `tanksim`
- Read sim3d documentation: `horus help sim3d`
- Join the community: [HORUS GitHub](https://github.com/horus-robotics/horus)
- Build your own robot models!

---

**Have fun exploring the Axiom cargo bay with WALL-E!** 🤖🌱
