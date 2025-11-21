# Robot Models Library

This directory contains URDF models and meshes for common robots used in robotics research and development.

## Available Robots

### Mobile Robots

#### TurtleBot3 (`turtlebot3/`)
- **Variants**: Burger, Waffle, Waffle Pi
- **Type**: Differential drive mobile robot
- **Sensors**: LiDAR, Camera, IMU
- **Use Cases**: Navigation, SLAM, mobile manipulation

#### Fetch (`fetch/`)
- **Type**: Mobile manipulator
- **Sensors**: LiDAR, RGBD camera, IMU
- **Manipulator**: 7-DOF arm + 2-finger gripper
- **Use Cases**: Mobile manipulation, grasping, navigation

#### HSR - Human Support Robot (`hsr/`)
- **Type**: Service robot
- **Sensors**: Multiple cameras, LiDAR, force/torque sensors
- **Manipulator**: Single arm with gripper
- **Use Cases**: Service robotics, elderly care, assistive tasks

### Robotic Arms

#### UR5e (`ur5e/`)
- **Type**: 6-DOF collaborative arm
- **Reach**: 850mm
- **Payload**: 5kg
- **Use Cases**: Pick-and-place, assembly, research

#### Panda (Franka Emika) (`panda/`)
- **Type**: 7-DOF collaborative arm
- **Reach**: 855mm
- **Payload**: 3kg
- **Sensors**: Joint torque sensors, force/torque sensor
- **Use Cases**: Research, precise manipulation, human-robot interaction

### Aerial Vehicles

#### Quadcopter (`quadcopter/`)
- **Type**: Generic quadrotor UAV
- **Variants**: X-configuration, +-configuration
- **Use Cases**: Aerial robotics, control research, vision

## Directory Structure

Each robot directory contains:
```
robot_name/
├── urdf/
│   ├── robot.urdf          # Main URDF file
│   ├── robot.gazebo        # Gazebo-specific properties
│   └── robot.xacro         # Parameterized URDF (if applicable)
├── meshes/
│   ├── visual/             # Visual meshes (high detail)
│   └── collision/          # Collision meshes (simplified)
├── config/
│   └── joint_limits.yaml   # Joint limits and properties
└── README.md               # Robot-specific documentation
```

## Usage

### Load in sim3d

```rust
use sim3d::robot::RobotLoader;

// Load TurtleBot3 Burger
let robot = RobotLoader::from_urdf("assets/robots/turtlebot3/burger/urdf/turtlebot3_burger.urdf")?;

// Load UR5e
let ur5e = RobotLoader::from_urdf("assets/robots/ur5e/urdf/ur5e.urdf")?;
```

### Load with Python bindings

```python
import sim3d

# Load robot
robot = sim3d.Robot.from_urdf("assets/robots/panda/urdf/panda.urdf")

# Spawn in simulation
sim.add_robot(robot, position=[0, 0, 0.5])
```

## Model Sources

- **TurtleBot3**: https://github.com/ROBOTIS-GIT/turtlebot3
- **UR5e**: https://github.com/ros-industrial/universal_robot
- **Panda**: https://github.com/frankaemika/franka_ros
- **Fetch**: https://github.com/fetchrobotics/fetch_ros
- **HSR**: https://github.com/ToyotaResearchInstitute/hsr_description

## Adding New Robots

To add a new robot model:

1. Create directory: `assets/robots/robot_name/`
2. Add URDF and meshes following the structure above
3. Create `README.md` with robot specifications
4. Add entry to this file
5. Create example scene in `assets/worlds/`

## License Notes

Each robot model may have different license requirements. Check individual robot directories for license information. Most models are under BSD or Apache 2.0 licenses.
