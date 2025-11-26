# sim2d Exercise Scenarios

This directory contains pre-built exercise scenarios for learning robotics with sim2d and HORUS.

## Available Scenarios

### 01. Basic Navigation
**File:** `01_basic_navigation.yaml`
**Difficulty:** Beginner
**Topics:** Basic movement, velocity control
**Goal:** Navigate robot from start to goal position avoiding a single obstacle
**Commands:**
```bash
sim2d --scenario scenarios/01_basic_navigation.yaml
horus run "echo 'CmdVel(1.0, 0.0)' > robot1.cmd_vel"
```

### 02. Obstacle Avoidance Maze
**File:** `02_obstacle_maze.yaml`
**Difficulty:** Intermediate
**Topics:** LIDAR sensor, reactive navigation, obstacle avoidance
**Goal:** Navigate through maze using LIDAR data
**Commands:**
```bash
sim2d --scenario scenarios/02_obstacle_maze.yaml
horus monitor robot1.lidar  # View LIDAR data
# Implement reactive obstacle avoidance algorithm
```

### 03. Ackermann Parking
**File:** `03_ackermann_parking.yaml`
**Difficulty:** Intermediate
**Topics:** Ackermann steering, car-like kinematics, parking
**Goal:** Park car-like robot in designated space
**Commands:**
```bash
sim2d --scenario scenarios/03_ackermann_parking.yaml
horus run "echo 'CmdVel(1.0, 0.3)' > car.cmd_vel"  # Forward with right turn
# Note: angular velocity is steering angle for Ackermann robots!
```

### 04. Omnidirectional Warehouse
**File:** `04_omnidirectional_warehouse.yaml`
**Difficulty:** Intermediate
**Topics:** Holonomic drive, omnidirectional movement, waypoint navigation
**Goal:** Navigate warehouse using omnidirectional capabilities
**Commands:**
```bash
sim2d --scenario scenarios/04_omnidirectional_warehouse.yaml
horus run "echo 'CmdVel(0.7, 0.7)' > holonomic.cmd_vel"  # Diagonal movement
# linear = forward/back, angular = left/right strafe
```

### 05. Multi-Robot Coordination
**File:** `05_multi_robot_coordination.yaml`
**Difficulty:** Advanced
**Topics:** Multi-robot systems, coordination, collision avoidance
**Goal:** Coordinate three robots to swap positions
**Commands:**
```bash
sim2d --scenario scenarios/05_multi_robot_coordination.yaml
# Control each robot independently:
horus run "echo 'CmdVel(1.0, 0.0)' > robot1.cmd_vel"
horus run "echo 'CmdVel(1.0, 0.0)' > robot2.cmd_vel"
horus run "echo 'CmdVel(1.0, 0.0)' > robot3.cmd_vel"
```

## Loading Scenarios

There are three ways to load scenarios:

### 1. Command Line
```bash
sim2d --scenario path/to/scenario.yaml
```

### 2. Python API
```python
from sim2d import Scenario

scenario = Scenario.load_from_file("scenarios/01_basic_navigation.yaml")
world_config = scenario.to_world_config()
robot_configs = scenario.to_robot_configs()
# Use configs to create simulation
```

### 3. GUI File Dialog
1. Launch sim2d GUI
2. Click "Load Scenario" button
3. Select scenario file
4. Simulation will reset to scenario state

## Creating Custom Scenarios

### Scenario File Format

```yaml
version: "1.0"
name: "My Custom Scenario"
description: "Description of the scenario"

world:
  width: 20.0
  height: 15.0
  obstacles:
    - pos: [x, y]
      size: [width, height]
      shape: rectangle  # or circle
      color: [r, g, b]  # Optional, RGB 0-1

robots:
  - name: "robot_name"
    position: [x, y]
    heading: 0.0  # radians
    velocity: [linear, angular]
    config:
      name: "robot_name"
      topic_prefix: "robot_name"
      position: [x, y]
      width: 0.6
      length: 0.9
      max_speed: 2.0
      color: [r, g, b]
      kinematics:
        kinematic_type: differential  # or ackermann, omnidirectional
        ackermann:  # Only for ackermann type
          wheelbase: 0.5
          max_steering_angle: 0.7
      lidar:
        enabled: true
        range_max: 10.0
        range_min: 0.1
        num_rays: 360
        angle_min: -3.14159
        angle_max: 3.14159
      camera:
        enabled: false

simulation:
  time: 0.0
  timestep: 0.016
  paused: false
```

## Robot Kinematics Types

### Differential Drive
- **Type:** `differential`
- **Control:** `CmdVel(linear, angular)`
  - `linear`: Forward/backward velocity (m/s)
  - `angular`: Rotational velocity (rad/s)
- **Characteristics:** Two independent wheels, can rotate in place
- **Use cases:** Most mobile robots, vacuum cleaners

### Ackermann Steering
- **Type:** `ackermann`
- **Control:** `CmdVel(linear, angular)`
  - `linear`: Forward/backward velocity (m/s)
  - `angular`: **Steering angle** (rad, NOT angular velocity!)
- **Characteristics:** Car-like, cannot rotate in place
- **Use cases:** Autonomous cars, delivery vehicles
- **Parameters:**
  - `wheelbase`: Distance between front and rear axles
  - `max_steering_angle`: Maximum steering angle in radians

### Omnidirectional
- **Type:** `omnidirectional`
- **Control:** `CmdVel(linear, angular)`
  - `linear`: X velocity in robot frame (m/s)
  - `angular`: Y velocity in robot frame (m/s)
- **Characteristics:** Holonomic, can move in any direction without rotating
- **Use cases:** Warehouse robots, research platforms
- **Note:** Robot does NOT rotate from velocity commands

## Tips for Creating Scenarios

1. **Start Simple:** Begin with basic navigation before adding complexity
2. **Test Incrementally:** Add obstacles one at a time
3. **Use Appropriate Sizes:**
   - Typical robot: 0.5-1.0m width/length
   - Obstacles: 1.0-5.0m for walls
   - World: 15-30m for most exercises
4. **Color Coding:**
   - Use distinct colors for different obstacle types
   - Mark goals/waypoints with bright colors (yellow, green)
5. **LIDAR Configuration:**
   - 360 rays for full coverage
   - 180 rays for forward-only sensing
   - 10-15m range for most environments

## Performance Metrics

All scenarios automatically track:
- Path length
- Time to goal
- Collision count
- Average speed
- Energy consumption

Access metrics via:
```bash
horus monitor robot_name.metrics
```

Or view in GUI metrics dashboard.

## Exporting Scenarios

Save current simulation state as scenario:
```python
from sim2d import Scenario

scenario = Scenario.from_current_state(
    name="My Scenario",
    description="Saved state",
    world_config=world_config,
    robot_configs=robot_configs,
    time=current_time
)
scenario.save_to_file("my_scenario.yaml")
```

Or use GUI "Save Scenario" button.

## Additional Resources

- [USAGE.md](../USAGE.md) - Complete sim2d usage guide
- [README.md](../README.md) - Feature overview
- [examples/](../examples/) - Python code examples
- [configs/](../configs/) - Robot and world configuration templates
