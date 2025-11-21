# Sim3D Asset Library

This directory contains robot models, objects, and other assets for the Sim3D physics simulator.

## Directory Structure

```
assets/
├── robots/           # Robot URDF models and meshes
│   ├── turtlebot3/  # TurtleBot3 mobile robot
│   ├── ur5e/        # Universal Robots UR5e arm
│   ├── panda/       # Franka Panda arm with gripper
│   ├── fetch/       # Fetch mobile manipulator
│   ├── hsr/         # Toyota HSR service robot
│   └── quadcopter/  # Generic quadcopter drone
├── objects/         # Graspable objects and furniture
│   ├── ycb_objects.yaml      # YCB dataset configuration
│   ├── furniture.yaml        # Furniture and obstacles
│   └── meshes/               # Object mesh files
├── materials/       # Material presets and textures
├── worlds/          # SDF/Gazebo world files
├── scenes/          # Scene description YAML files
└── meshes/          # Shared mesh assets
```

## Available Assets

### Robots

#### TurtleBot3 Burger (`robots/turtlebot3/burger.urdf`)
- **Description:** Compact mobile robot for navigation
- **Components:**
  - Differential drive base (0.138m width)
  - Two motorized wheels
  - Passive caster wheel
  - 360° LiDAR sensor
  - 9-axis IMU sensor
- **Mass:** 1.0 kg
- **Use case:** Mobile navigation, SLAM, obstacle avoidance

#### UR5e Robotic Arm (`robots/ur5e/ur5e.urdf`)
- **Description:** 6-DOF collaborative robotic arm
- **Reach:** 850mm
- **Payload:** 5kg
- **Components:**
  - 6 revolute joints
  - Base link + 5 arm links + end-effector
  - Tool mounting flange
- **Use case:** Manipulation, pick-and-place, assembly

#### Franka Panda (`robots/panda/panda.urdf`)
- **Description:** 7-DOF arm with parallel jaw gripper
- **Reach:** 855mm
- **Payload:** 3kg
- **Components:**
  - 7 revolute joints (redundant kinematics)
  - Parallel gripper with 2 prismatic fingers
  - Hand + gripper assembly
- **Max opening:** 80mm
- **Use case:** Precise manipulation, research, grasping

### Objects

#### YCB Object Dataset (`objects/ycb_objects.yaml`)
Standard manipulation research objects:
- **Food Items:** Master chef can, cracker box, sugar box, tomato soup, mustard bottle
- **Kitchen:** Pitcher, bowl, mug, bleach cleanser
- **Tools:** Power drill, scissors, markers, clamps
- **Shapes:** Wood blocks, foam bricks
- **Primitives:** Cubes, spheres, cylinders, capsules

#### Furniture (`objects/furniture.yaml`)
Indoor environment objects:
- **Tables:** Dining table (1.5m × 0.8m), coffee table (1.0m × 0.6m)
- **Chairs:** Basic chair with metal legs
- **Shelving:** 4-shelf bookcase (1.8m height)
- **Cabinets:** Kitchen cabinet (static)
- **Beds:** Single bed with mattress
- **Obstacles:** Traffic cones, barrels, wall sections, door frames

### Materials

Available material presets (via `MaterialPreset`):
- **Metals:** `metal()`, `steel()`, `aluminum()`
- **Polymers:** `plastic()`, `rubber()`
- **Natural:** `wood()`, `glass()`, `concrete()`
- **Textiles:** `cloth()`, `carpet()`, `leather()`
- **Others:** `foam()`, `paper()`

## Asset Creation Workflow

### 1. Creating a Robot Model

#### URDF Structure

Create a URDF file following this structure:

```xml
<?xml version="1.0"?>
<robot name="your_robot">
  <!-- Base Link -->
  <link name="base_link">
    <visual>
      <geometry>
        <mesh filename="meshes/base.obj"/>
        <!-- OR use primitives: -->
        <!-- <cylinder radius="0.1" length="0.2"/> -->
      </geometry>
      <material name="blue">
        <color rgba="0.0 0.0 1.0 1.0"/>
      </material>
    </visual>
    <collision>
      <geometry>
        <!-- Simplified geometry for collision -->
        <cylinder radius="0.1" length="0.2"/>
      </geometry>
    </collision>
    <inertial>
      <mass value="5.0"/>
      <inertia ixx="0.01" ixy="0" ixz="0"
               iyy="0.01" iyz="0" izz="0.01"/>
    </inertial>
  </link>

  <!-- Joint -->
  <joint name="joint1" type="revolute">
    <parent link="base_link"/>
    <child link="link1"/>
    <origin xyz="0 0 0.1" rpy="0 0 0"/>
    <axis xyz="0 0 1"/>
    <limit lower="-3.14" upper="3.14" effort="100" velocity="2.0"/>
  </joint>

  <!-- Additional links and joints... -->
</robot>
```

#### Important Guidelines

1. **Link Names**: Must be unique within the robot
2. **Joint Types**:
   - `revolute`: Rotating joint with limits
   - `continuous`: Rotating joint without limits
   - `prismatic`: Sliding joint
   - `fixed`: Rigid connection
   - `floating`: 6-DOF free movement
   - `planar`: 2D planar movement

3. **Inertial Properties**: Required for physics simulation
   - Calculate realistic mass and inertia tensors
   - Use CAD software or approximations for complex shapes

4. **Collision Geometry**:
   - Keep simpler than visual geometry for performance
   - Use primitive shapes when possible
   - Avoid high-poly meshes

### 2. Preparing Mesh Assets

#### Mesh Optimization

Use the built-in mesh optimization tools:

```rust
use sim3d::assets::{decimate_mesh, generate_lods, DecimationOptions, LODConfig};

// Load and decimate a high-poly mesh
let mut mesh = load_mesh("high_poly_model.obj")?;

// Option 1: Target triangle count
let options = DecimationOptions::default()
    .with_target_triangles(5000)
    .preserve_boundaries(true);

decimate_mesh(&mut mesh, options)?;

// Option 2: Generate LOD levels
let lod_config = LODConfig {
    num_levels: 3,           // Create 3 LOD levels
    reduction_per_level: 0.5, // 50% reduction each level
    preserve_boundaries: true,
};

let lods = generate_lods(&mesh, lod_config)?;
```

#### Supported Formats

- **OBJ**: Wavefront OBJ (widely supported, simple)
- **STL**: STereoLithography (binary or ASCII)
- **COLLADA (.dae)**: Complex scenes with materials
- **glTF/GLB**: Modern format with PBR materials

#### Mesh Requirements

- Manifold geometry (watertight, no holes)
- Correct normals (outward-facing)
- Reasonable triangle count (< 50k triangles for real-time)
- Proper scale (use meters as base unit)

### 3. Asset Validation

Validate your assets before use:

```rust
use sim3d::assets::{validate_urdf, validate_robot_package};

// Validate a single URDF file
let report = validate_urdf("robots/my_robot/robot.urdf")?;
report.print_report();

// Validate entire robot package
let report = validate_robot_package("robots/my_robot")?;
if !report.is_valid() {
    eprintln!("Validation failed with {} errors", report.errors.len());
}
```

The validator checks for:
- Well-formed XML
- Unique link/joint names
- Valid joint types and references
- Existence of referenced mesh files
- Proper inertial properties
- Collision geometry

### 4. Using Material Presets

Apply realistic materials to objects:

```rust
use sim3d::physics::MaterialPreset;

// Use built-in materials
let steel = MaterialPreset::metal();
let rubber = MaterialPreset::rubber();
let wood = MaterialPreset::wood();

// New materials in v0.2.0
let cloth = MaterialPreset::cloth();
let foam = MaterialPreset::foam();
let carpet = MaterialPreset::carpet();
let leather = MaterialPreset::leather();
let paper = MaterialPreset::paper();

// Custom material
let custom = MaterialPreset {
    friction: 0.8,
    restitution: 0.3,
    density: 1200.0,
    damping: 0.05,
    color: Color::srgb(0.5, 0.3, 0.2),
};
```

### 5. Loading Assets in Code

#### Loading Robot Models

```rust
use sim3d::RobotLoader;

let mut loader = RobotLoader::new();

// Load from URDF
let robot = loader.load_urdf("robots/turtlebot3/burger.urdf")?;

// Spawn in simulation
let robot_entity = commands.spawn(RobotBundle {
    robot,
    transform: Transform::from_xyz(0.0, 0.0, 0.5),
    ..default()
});
```

#### Loading Objects

```rust
use sim3d::assets::MeshLoader;

let mut loader = MeshLoader::new();

// Load with options
let options = MeshLoadOptions::default()
    .with_scale(Vec3::splat(0.001))  // Convert mm to m
    .max_triangles(10000)             // Auto-decimate if needed
    .generate_normals(true);

let mesh_data = loader.load("objects/meshes/can.obj", options)?;
```

### 6. Creating Object Configurations

Define object sets in YAML:

```yaml
objects:
  - name: "soda_can"
    category: "food"
    mesh: "meshes/objects/can.obj"
    mass: 0.015  # kg
    dimensions: [0.066, 0.066, 0.122]  # x, y, z in meters
    friction: 0.5
    restitution: 0.1
    color: [1.0, 0.0, 0.0, 1.0]  # RGBA

  - name: "box"
    type: "primitive"
    geometry:
      type: "box"
      size: [0.1, 0.1, 0.1]
    mass: 0.5
    material: "cardboard"
```

## Best Practices

### Performance

1. **Mesh Complexity**
   - Visual meshes: < 10k triangles
   - Collision meshes: < 1k triangles
   - Use LODs for distant objects

2. **Instancing**
   - Reuse meshes for multiple objects
   - Share materials when possible

3. **Material Library**
   - Use predefined materials when possible
   - Minimize unique material count

### Realism

1. **Physical Properties**
   - Use realistic masses and inertias
   - Reference real-world data when available
   - Test stability in simulation

2. **Scale**
   - Maintain consistent units (meters)
   - Typical robot sizes: 0.3-2.0m
   - Object sizes: 0.05-0.5m

3. **Friction & Restitution**
   - Metal-metal: friction 0.15-0.25, restitution 0.3-0.4
   - Rubber-floor: friction 0.7-1.0, restitution 0.5-0.7
   - Wood-wood: friction 0.4-0.6, restitution 0.2-0.3

### Organization

1. **Directory Structure**
   ```
   robots/my_robot/
   ├── robot.urdf
   ├── meshes/
   │   ├── base.obj
   │   ├── link1.obj
   │   └── ...
   └── README.md
   ```

2. **Naming Conventions**
   - Links: `base_link`, `link1`, `gripper_left`, etc.
   - Joints: `shoulder_pan`, `elbow_flex`, etc.
   - Meshes: lowercase with underscores

3. **Documentation**
   - Include README with robot specs
   - Document coordinate frames
   - List dependencies and references

## Troubleshooting

### Common Issues

1. **"Mesh file not found"**
   - Check relative paths in URDF
   - Ensure meshes directory exists
   - Verify file extensions (.obj, .stl, etc.)

2. **"Joint references non-existent link"**
   - Ensure parent/child links are defined
   - Check for typos in link names
   - Verify link definition order

3. **"Invalid inertial properties"**
   - Ensure mass > 0
   - Check inertia tensor is positive-definite
   - Use realistic values (not arbitrary)

4. **Robot falls through floor**
   - Add collision geometry to all links
   - Check mesh normals (should point outward)
   - Verify collision geometry placement

5. **Joints don't move**
   - Check joint limits (lower < upper)
   - Ensure effort > 0
   - Verify joint type is not "fixed"

### Validation Tools

Run validation before loading:

```bash
# Test asset loading
cargo test --lib asset_validation

# Check mesh quality
cargo test --lib mesh_optimization
```

## References

- [URDF Specification](http://wiki.ros.org/urdf/XML)
- [SDF Format](http://sdformat.org/)
- [YCB Object Set](https://www.ycbbenchmarks.com/)
- [Bevy Rendering](https://bevyengine.org/)

## License

Assets may have different licenses. Check individual robot/object documentation.

Default assets are provided for research and educational use.
