# Getting Started with Sim3D

Sim3D is a production-grade 3D robotics simulator built with Bevy and Rapier3D, designed for both visual simulation and high-performance reinforcement learning training.

## Features

- **Dual-mode operation**: Visual 3D rendering + headless RL training
- **URDF support**: Load standard robot descriptions
- **RL-first design**: Vectorized environments, domain randomization
- **HORUS-native**: Direct Hub integration for sim-to-real transfer
- **Performance**: 60 FPS visual / 100K+ steps/sec headless
- **Pure Rust**: Memory-safe, cross-platform, single binary

## Installation

### System Dependencies

**Ubuntu/Debian:**
```bash
sudo apt update && sudo apt install -y \
    pkg-config \
    libx11-dev \
    libxi-dev \
    libxcursor-dev \
    libxrandr-dev \
    libasound2-dev \
    libudev-dev \
    libwayland-dev \
    libxkbcommon-dev \
    libssl-dev \
    libfontconfig1-dev
```

**Fedora/RHEL:**
```bash
sudo dnf install -y \
    pkg-config \
    libX11-devel \
    libXi-devel \
    libXcursor-devel \
    libXrandr-devel \
    alsa-lib-devel \
    systemd-devel \
    wayland-devel \
    libxkbcommon-devel \
    openssl-devel \
    fontconfig-devel
```

**macOS:**
```bash
xcode-select --install
```

### Environment Variables

Set these environment variables before building:

```bash
export PKG_CONFIG_ALLOW_SYSTEM_LIBS=1
export PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1
```

### Building from Source

Clone the HORUS repository and build Sim3D:

```bash
git clone https://github.com/horus-project/horus.git
cd horus/horus_library/tools/sim3d

# Visual mode (default) - includes 3D rendering
cargo build --release

# Headless mode - for RL training without rendering
cargo build --release --no-default-features --features headless

# With editor tools
cargo build --release --features editor

# Full build with all features including Python bindings
cargo build --release --features full
```

### Installing via Cargo

If published to crates.io:

```bash
cargo install sim3d
```

## First Simulation Example

Here is a complete example that creates a simple simulation world with a ground plane and a falling box:

```rust
use bevy::prelude::*;
use sim3d::physics::{PhysicsWorld, collider::*, rigid_body::*};

fn main() {
    App::new()
        // Add default Bevy plugins for rendering
        .add_plugins(DefaultPlugins)
        // Initialize our physics world
        .init_resource::<PhysicsWorld>()
        // Add startup system to create scene
        .add_systems(Startup, setup_scene)
        // Add physics step system
        .add_systems(Update, physics_step)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut physics_world: ResMut<PhysicsWorld>,
) {
    // Create camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Create directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Create ground plane (static body)
    let ground_entity = commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).id();

    // Add ground collider to physics
    let ground_rb = rapier3d::prelude::RigidBodyBuilder::fixed().build();
    let ground_handle = physics_world.spawn_rigid_body(ground_rb, ground_entity);
    let ground_collider = create_ground_collider(20.0, 20.0);
    physics_world.spawn_collider(ground_collider, ground_handle);

    // Create falling box (dynamic body)
    let box_entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.2, 0.2),
            ..default()
        })),
        Transform::from_xyz(0.0, 5.0, 0.0),
    )).id();

    // Add box rigid body and collider
    let box_rb = rapier3d::prelude::RigidBodyBuilder::dynamic()
        .translation(rapier3d::prelude::vector![0.0, 5.0, 0.0])
        .build();
    let box_handle = physics_world.spawn_rigid_body(box_rb, box_entity);
    let box_collider = create_box_collider(Vec3::new(0.5, 0.5, 0.5));
    physics_world.spawn_collider(box_collider, box_handle);
}

fn physics_step(
    mut physics_world: ResMut<PhysicsWorld>,
    mut transforms: Query<&mut Transform>,
) {
    // Step physics simulation
    physics_world.step();

    // Sync physics positions to Bevy transforms
    for (handle, rb) in physics_world.rigid_body_set.iter() {
        let entity = Entity::from_bits(rb.user_data as u64);
        if let Ok(mut transform) = transforms.get_mut(entity) {
            let pos = rb.position().translation;
            let rot = rb.position().rotation;
            transform.translation = Vec3::new(pos.x, pos.y, pos.z);
            transform.rotation = Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w);
        }
    }
}
```

## Key Concepts

### PhysicsWorld

The `PhysicsWorld` resource manages all physics simulation state using Rapier3D:

- **RigidBodySet**: Collection of all rigid bodies (dynamic, static, kinematic)
- **ColliderSet**: Collision shapes attached to rigid bodies
- **ImpulseJointSet**: Joint constraints between bodies
- **QueryPipeline**: Spatial queries like ray casting

### Sensors

Sim3D provides 16 sensor types for robot perception:

- **LiDAR (2D/3D)**: Ray-based distance sensing
- **Camera (RGB/Depth/RGBD)**: Visual perception
- **IMU**: Orientation and acceleration
- **GPS**: Global positioning with configurable accuracy
- **Force/Torque**: Contact force measurement
- **Encoders**: Wheel odometry
- And more (Radar, Sonar, Thermal, Event Camera, etc.)

### Robot Loading

Robots are loaded via URDF (Unified Robot Description Format):

```rust
use sim3d::robot::urdf_loader::URDFLoader;

let mut loader = URDFLoader::new()
    .with_base_path("assets/robots");

let robot_entity = loader.load(
    "turtlebot3/burger.urdf",
    &mut commands,
    &mut physics_world,
    &mut tf_tree,
    &mut meshes,
    &mut materials,
)?;
```

### Reinforcement Learning

For RL training, use headless mode with Python bindings:

```python
import sim3d_rl

# Create environment
env = sim3d_rl.make_env("navigation", obs_dim=20, action_dim=2)

# Standard Gymnasium interface
obs = env.reset()
for _ in range(1000):
    action = env.action_space.sample()
    obs, reward, done, truncated, info = env.step(action)
    if done:
        obs = env.reset()
```

## Project Structure

```
sim3d/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── main.rs             # CLI application
│   ├── physics/            # Rapier3D physics integration
│   ├── robot/              # Robot loading and control
│   ├── sensors/            # All 16 sensor implementations
│   ├── rl/                 # Reinforcement learning support
│   ├── multi_robot/        # Multi-robot coordination
│   └── rendering/          # Visual rendering systems
├── assets/
│   ├── robots/             # URDF robot models
│   ├── scenes/             # Predefined scenes
│   └── objects/            # Object definitions
├── configs/                # YAML configuration files
├── python/                 # Python RL bindings
└── docs/                   # Documentation
```

## Running the Simulator

### Visual Mode

```bash
# Default visual mode
./target/release/sim3d --mode visual

# With custom robot
./target/release/sim3d --robot assets/robots/turtlebot3/burger.urdf

# With custom scene
./target/release/sim3d --scene assets/scenes/warehouse.yaml
```

### Headless Mode

```bash
# Headless mode for training
./target/release/sim3d --mode headless

# With specific task
./target/release/sim3d --mode headless --task navigation
```

### Controls (Visual Mode)

- **Right Mouse Button**: Rotate camera
- **Mouse Wheel**: Zoom in/out
- **WASD**: Move camera (if enabled)
- **ESC**: Exit

## Next Steps

- [Tutorial 1: Basic Simulation](tutorials/01_basic_simulation.md) - Create worlds and spawn objects
- [Tutorial 2: Robot Simulation](tutorials/02_robot_simulation.md) - Load and control robots
- [Tutorial 3: Sensors](tutorials/03_sensors.md) - Add sensors to your robots
- [Tutorial 4: Reinforcement Learning](tutorials/04_reinforcement_learning.md) - Train RL agents
- [API Reference: Physics](api/physics.md) - Detailed physics API
- [API Reference: Sensors](api/sensors.md) - All sensor configurations
