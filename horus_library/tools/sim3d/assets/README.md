# sim3d Assets

This directory contains robot models, scenes, and other assets for sim3d.

## Directory Structure

```
assets/
├── models/          # Robot URDF files and meshes
│   └── simple_robot/
│       └── simple_robot.urdf
├── scenes/          # Scene description YAML files
│   └── simple_navigation.yaml
└── textures/        # Texture files (future)
```

## Available Assets

### Robots

#### Simple Robot (`models/simple_robot/`)
- **Description:** Basic differential drive robot with 2 wheels and LiDAR
- **Components:**
  - Base link (0.3m x 0.3m x 0.1m box)
  - Left/right wheels (0.05m radius cylinders)
  - LiDAR sensor mount
- **Use case:** Testing, basic navigation experiments

**Load in sim3d:**
```bash
sim3d --robot assets/models/simple_robot/simple_robot.urdf
```

**Load in Python:**
```python
# Scene setup would load this URDF
# (Scene loading API pending)
```

### Scenes

#### Simple Navigation (`scenes/simple_navigation.yaml`)
- **Description:** Navigation task with obstacles
- **Contents:**
  - 20m x 20m arena with walls
  - 4 box obstacles
  - 2 cylinder obstacles
  - Goal marker
  - Simple robot with LiDAR
- **Use case:** RL navigation training, obstacle avoidance

**Load scene:**
```bash
sim3d --world assets/scenes/simple_navigation.yaml
```

## Adding Your Own Assets

### Robot Models (URDF)

1. Create directory: `mkdir -p assets/models/my_robot`
2. Add URDF file: `assets/models/my_robot/my_robot.urdf`
3. Add mesh files (if any): `assets/models/my_robot/meshes/*.stl`
4. Test loading: `sim3d --robot assets/models/my_robot/my_robot.urdf`

**URDF Requirements:**
- All links must have `<inertial>` tags
- Mass and inertia values must be physical
- Meshes referenced with relative paths
- Use standard URDF 1.0 format

### Scene Files (YAML)

Create a new scene file `assets/scenes/my_scene.yaml`:

```yaml
world:
  name: "My Scene"
  gravity: [0.0, -9.81, 0.0]

robots:
  - name: "robot1"
    urdf: "models/simple_robot/simple_robot.urdf"
    position: [0.0, 0.1, 0.0]
    orientation: [0.0, 0.0, 0.0]

objects:
  - type: box
    name: "ground"
    position: [0.0, 0.0, 0.0]
    size: [10.0, 0.1, 10.0]
    static: true
    material:
      color: [0.5, 0.5, 0.5, 1.0]
      friction: 0.7
```

**Supported object types:**
- `box` - Cuboid (requires `size: [x, y, z]`)
- `sphere` - Sphere (requires `radius`)
- `cylinder` - Cylinder (requires `radius` and `height`)
- `mesh` - Custom mesh (requires `mesh_file`)

## Downloading More Robots

### TurtleBot3 (Popular ROS robot)

```bash
# Download TurtleBot3 URDF
cd assets/models
git clone https://github.com/ROBOTIS-GIT/turtlebot3
mv turtlebot3/turtlebot3_description/urdf turtlebot3_urdf
mv turtlebot3/turtlebot3_description/meshes turtlebot3_urdf/
rm -rf turtlebot3

# Test
sim3d --robot assets/models/turtlebot3_urdf/turtlebot3_burger.urdf
```

### UR5e Robotic Arm

```bash
# Download UR5e
cd assets/models
git clone https://github.com/ros-industrial/universal_robot
# Extract URDF and meshes (similar process)
```

## Common Issues

### "Failed to load URDF"
- Check file path is correct
- Verify URDF is valid XML
- Ensure all mesh files exist

### "Missing inertial properties"
- All links need `<inertial>` tags
- Add mass and inertia values

### "Mesh not found"
- Check mesh paths in URDF
- Use relative paths from URDF location
- Supported formats: .stl, .dae, .obj

## Performance Tips

- Use low-poly meshes for better performance
- Combine small collision geometries
- Use primitive shapes (box, sphere, cylinder) when possible
- Keep triangle count < 10K per mesh for real-time sim

## Future Assets

Planning to add:
- TurtleBot3 (Burger, Waffle, WafflePi)
- UR5e robotic arm
- Quadrotor drone
- More complex scenes (warehouse, maze, manipulation table)
- Texture files for materials

## Contributing

To contribute assets:
1. Ensure URDF follows standard format
2. Include both visual and collision geometry
3. Test loading in sim3d
4. Add description to this README
5. Submit PR or open issue with download link
