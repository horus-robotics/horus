# Tutorial 2: Robot Simulation

This tutorial covers loading URDF robot models, controlling joints, and reading robot state in Sim3D.

## Prerequisites

- Completed [Tutorial 1: Basic Simulation](01_basic_simulation.md)
- A URDF robot file (we'll use TurtleBot3 as an example)

## URDF Overview

URDF (Unified Robot Description Format) is the standard XML format for describing robot models. A URDF file contains:

- **Links**: Rigid bodies of the robot (wheels, base, arms)
- **Joints**: Connections between links (revolute, prismatic, fixed)
- **Visuals**: 3D meshes for rendering
- **Collisions**: Simplified collision shapes
- **Inertials**: Mass and inertia properties

## Loading a URDF Robot

Sim3D provides the `URDFLoader` for loading robot models:

```rust
use bevy::prelude::*;
use sim3d::physics::PhysicsWorld;
use sim3d::robot::urdf_loader::URDFLoader;
use sim3d::tf::tree::TFTree;

fn load_robot(
    mut commands: Commands,
    mut physics_world: ResMut<PhysicsWorld>,
    mut tf_tree: ResMut<TFTree>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create URDF loader with custom search paths
    let mut loader = URDFLoader::new()
        .with_base_path("assets/robots/turtlebot3");

    // Load the robot
    match loader.load(
        "assets/robots/turtlebot3/burger.urdf",
        &mut commands,
        &mut physics_world,
        &mut tf_tree,
        &mut meshes,
        &mut materials,
    ) {
        Ok(robot_entity) => {
            println!("Robot loaded successfully: {:?}", robot_entity);
        }
        Err(e) => {
            eprintln!("Failed to load robot: {}", e);
        }
    }
}
```

### Example TurtleBot3 URDF Structure

Here's a simplified URDF for TurtleBot3:

```xml
<?xml version="1.0"?>
<robot name="turtlebot3_burger">
  <!-- Base Link -->
  <link name="base_link">
    <visual>
      <geometry>
        <mesh filename="package://turtlebot3/meshes/base.stl"/>
      </geometry>
      <material name="grey">
        <color rgba="0.5 0.5 0.5 1"/>
      </material>
    </visual>
    <collision>
      <geometry>
        <cylinder length="0.14" radius="0.10"/>
      </geometry>
    </collision>
    <inertial>
      <mass value="1.0"/>
      <inertia ixx="0.01" ixy="0" ixz="0" iyy="0.01" iyz="0" izz="0.01"/>
    </inertial>
  </link>

  <!-- Left Wheel -->
  <link name="wheel_left_link">
    <visual>
      <geometry>
        <cylinder length="0.018" radius="0.033"/>
      </geometry>
    </visual>
    <collision>
      <geometry>
        <cylinder length="0.018" radius="0.033"/>
      </geometry>
    </collision>
    <inertial>
      <mass value="0.1"/>
      <inertia ixx="0.0001" ixy="0" ixz="0" iyy="0.0001" iyz="0" izz="0.0001"/>
    </inertial>
  </link>

  <joint name="wheel_left_joint" type="continuous">
    <parent link="base_link"/>
    <child link="wheel_left_link"/>
    <origin xyz="0 0.08 0.033" rpy="-1.5708 0 0"/>
    <axis xyz="0 0 1"/>
  </joint>

  <!-- Right Wheel (similar structure) -->
  <link name="wheel_right_link">
    <!-- ... -->
  </link>

  <joint name="wheel_right_joint" type="continuous">
    <parent link="base_link"/>
    <child link="wheel_right_link"/>
    <origin xyz="0 -0.08 0.033" rpy="-1.5708 0 0"/>
    <axis xyz="0 0 1"/>
  </joint>
</robot>
```

## Controlling Joints

### Joint Types

Sim3D supports these joint types from URDF:

| Type | Description | Control |
|------|-------------|---------|
| `revolute` | Rotation with limits | Position/velocity |
| `continuous` | Unlimited rotation | Velocity |
| `prismatic` | Linear motion | Position/velocity |
| `fixed` | No motion | N/A |
| `spherical` | Ball joint | Quaternion |

### Position Control

Control joint position using motors:

```rust
use sim3d::physics::joints::{PhysicsJoint, JointType, add_joint_motor};
use rapier3d::prelude::*;

fn control_joint_position(
    mut physics_world: ResMut<PhysicsWorld>,
    joints: Query<&PhysicsJoint>,
    target_position: f32,  // In radians for revolute joints
) {
    for joint in joints.iter() {
        if let JointType::Revolute = joint.joint_type {
            // Get the joint from physics world
            if let Some(impulse_joint) = physics_world.impulse_joint_set.get_mut(joint.handle) {
                // Set position motor
                // Parameters: target_pos, target_vel, stiffness, damping
                impulse_joint.data.set_motor_position(
                    JointAxis::AngX,
                    target_position,
                    100.0,  // Stiffness (Nm/rad)
                    10.0,   // Damping (Nm*s/rad)
                );
            }
        }
    }
}
```

### Velocity Control

Control joint velocity for continuous motion:

```rust
fn control_joint_velocity(
    mut physics_world: ResMut<PhysicsWorld>,
    joints: Query<(&PhysicsJoint, &Name)>,
    wheel_velocity: f32,  // In rad/s
) {
    for (joint, name) in joints.iter() {
        // Control wheel joints by name
        if name.as_str().contains("wheel") {
            if let Some(impulse_joint) = physics_world.impulse_joint_set.get_mut(joint.handle) {
                // Set velocity motor
                impulse_joint.data.set_motor_velocity(
                    JointAxis::AngX,
                    wheel_velocity,
                    50.0,  // Max impulse (Nm)
                );
            }
        }
    }
}
```

### Differential Drive Control

For wheeled robots like TurtleBot3:

```rust
#[derive(Component)]
struct DifferentialDrive {
    wheel_separation: f32,
    wheel_radius: f32,
    max_velocity: f32,
}

fn differential_drive_system(
    mut physics_world: ResMut<PhysicsWorld>,
    robots: Query<&DifferentialDrive>,
    joints: Query<(&PhysicsJoint, &Name)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for drive in robots.iter() {
        // Get velocity commands from input
        let mut linear_vel = 0.0_f32;
        let mut angular_vel = 0.0_f32;

        if input.pressed(KeyCode::ArrowUp) {
            linear_vel = 0.5;
        }
        if input.pressed(KeyCode::ArrowDown) {
            linear_vel = -0.5;
        }
        if input.pressed(KeyCode::ArrowLeft) {
            angular_vel = 1.0;
        }
        if input.pressed(KeyCode::ArrowRight) {
            angular_vel = -1.0;
        }

        // Convert to wheel velocities
        let left_vel = (linear_vel - angular_vel * drive.wheel_separation / 2.0)
            / drive.wheel_radius;
        let right_vel = (linear_vel + angular_vel * drive.wheel_separation / 2.0)
            / drive.wheel_radius;

        // Apply to wheel joints
        for (joint, name) in joints.iter() {
            if let Some(impulse_joint) = physics_world.impulse_joint_set.get_mut(joint.handle) {
                if name.as_str().contains("left") {
                    impulse_joint.data.set_motor_velocity(
                        JointAxis::AngX,
                        left_vel,
                        10.0,
                    );
                } else if name.as_str().contains("right") {
                    impulse_joint.data.set_motor_velocity(
                        JointAxis::AngX,
                        right_vel,
                        10.0,
                    );
                }
            }
        }
    }
}
```

## Reading Joint State

### Getting Joint Positions

```rust
#[derive(Debug)]
struct JointState {
    name: String,
    position: f32,
    velocity: f32,
}

fn read_joint_states(
    physics_world: Res<PhysicsWorld>,
    joints: Query<(&PhysicsJoint, &Name)>,
) -> Vec<JointState> {
    let mut states = Vec::new();

    for (joint, name) in joints.iter() {
        if let Some(impulse_joint) = physics_world.impulse_joint_set.get(joint.handle) {
            // Get rigid bodies connected by this joint
            let (body1_handle, body2_handle) = (
                impulse_joint.body1,
                impulse_joint.body2,
            );

            if let (Some(body1), Some(body2)) = (
                physics_world.rigid_body_set.get(body1_handle),
                physics_world.rigid_body_set.get(body2_handle),
            ) {
                // Calculate relative position between bodies
                let anchor1 = impulse_joint.data.local_anchor1();
                let anchor2 = impulse_joint.data.local_anchor2();

                // For revolute joints, extract angle
                let position = match joint.joint_type {
                    JointType::Revolute => {
                        // Calculate relative rotation around joint axis
                        let q1 = body1.position().rotation;
                        let q2 = body2.position().rotation;
                        let rel_rot = q1.inverse() * q2;
                        // Extract angle (simplified - assumes X axis rotation)
                        2.0 * rel_rot.i.atan2(rel_rot.w)
                    }
                    JointType::Prismatic => {
                        // Calculate relative translation along axis
                        let p1 = body1.position().translation;
                        let p2 = body2.position().translation;
                        (p2 - p1).norm()
                    }
                    _ => 0.0,
                };

                // Get velocity from motor if available
                let velocity = impulse_joint.data.motor(JointAxis::AngX)
                    .map(|m| m.target_vel)
                    .unwrap_or(0.0);

                states.push(JointState {
                    name: name.to_string(),
                    position,
                    velocity,
                });
            }
        }
    }

    states
}
```

### Monitoring Joint Limits

```rust
fn check_joint_limits(
    physics_world: Res<PhysicsWorld>,
    joints: Query<(&PhysicsJoint, &Name)>,
) {
    for (joint, name) in joints.iter() {
        if let Some(impulse_joint) = physics_world.impulse_joint_set.get(joint.handle) {
            // Check if joint has limits
            if let Some(limits) = impulse_joint.data.limits(JointAxis::AngX) {
                let current_pos = 0.0; // Calculate actual position
                let margin = 0.1; // 0.1 rad margin

                if current_pos < limits.min + margin {
                    println!("Warning: {} near lower limit", name);
                }
                if current_pos > limits.max - margin {
                    println!("Warning: {} near upper limit", name);
                }
            }
        }
    }
}
```

## Complete Example: TurtleBot3 Simulation

```rust
use bevy::prelude::*;
use sim3d::physics::{PhysicsWorld, collider::ColliderShape, collider::ColliderBuilder};
use sim3d::physics::joints::{PhysicsJoint, JointType};
use sim3d::robot::urdf_loader::URDFLoader;
use sim3d::tf::tree::TFTree;
use rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sim3D - TurtleBot3 Simulation".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<PhysicsWorld>()
        .init_resource::<TFTree>()
        .add_systems(Startup, setup_simulation)
        .add_systems(Update, (
            physics_step_system,
            sync_transforms_system,
            robot_control_system,
            display_robot_state_system,
        ))
        .run();
}

#[derive(Component)]
struct TurtleBot3 {
    wheel_separation: f32,
    wheel_radius: f32,
}

fn setup_simulation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut physics_world: ResMut<PhysicsWorld>,
    mut tf_tree: ResMut<TFTree>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Lighting
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -0.8,
            0.5,
            0.0,
        )),
    ));

    // Ground
    let ground_entity = commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.3, 0.3),
            ..default()
        })),
        Transform::default(),
    )).id();

    let ground_rb = RigidBodyBuilder::fixed().build();
    let ground_handle = physics_world.spawn_rigid_body(ground_rb, ground_entity);
    let ground_collider = ColliderBuilder::new(ColliderShape::Box {
        half_extents: Vec3::new(10.0, 0.1, 10.0),
    })
    .friction(0.8)
    .build();
    physics_world.spawn_collider(ground_collider, ground_handle);

    // Create a simplified TurtleBot3 robot programmatically
    create_turtlebot3(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut physics_world,
        Vec3::new(0.0, 0.1, 0.0),
    );

    println!("Controls:");
    println!("  Arrow Up/Down: Move forward/backward");
    println!("  Arrow Left/Right: Turn left/right");
    println!("  Space: Stop");
}

fn create_turtlebot3(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
    position: Vec3,
) {
    let wheel_separation = 0.16;
    let wheel_radius = 0.033;
    let base_radius = 0.10;
    let base_height = 0.14;

    // Base body
    let base_entity = commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(base_radius, base_height))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.2),
            ..default()
        })),
        Transform::from_translation(position + Vec3::Y * (base_height / 2.0)),
        Name::new("base_link"),
        TurtleBot3 {
            wheel_separation,
            wheel_radius,
        },
    )).id();

    let base_rb = RigidBodyBuilder::dynamic()
        .translation(vector![
            position.x,
            position.y + base_height / 2.0,
            position.z
        ])
        .build();
    let base_handle = physics_world.spawn_rigid_body(base_rb, base_entity);

    let base_collider = ColliderBuilder::new(ColliderShape::Cylinder {
        half_height: base_height / 2.0,
        radius: base_radius,
    })
    .friction(0.5)
    .density(10.0)
    .build();
    physics_world.spawn_collider(base_collider, base_handle);

    // Left wheel
    let left_wheel_pos = position + Vec3::new(0.0, wheel_radius, wheel_separation / 2.0);
    let left_wheel_entity = commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(wheel_radius, 0.018))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 0.1),
            ..default()
        })),
        Transform::from_translation(left_wheel_pos)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        Name::new("wheel_left"),
    )).id();

    let left_rb = RigidBodyBuilder::dynamic()
        .translation(vector![left_wheel_pos.x, left_wheel_pos.y, left_wheel_pos.z])
        .build();
    let left_handle = physics_world.spawn_rigid_body(left_rb, left_wheel_entity);

    let wheel_collider = ColliderBuilder::new(ColliderShape::Cylinder {
        half_height: 0.009,
        radius: wheel_radius,
    })
    .friction(1.0)
    .density(5.0)
    .build();
    physics_world.spawn_collider(wheel_collider.clone(), left_handle);

    // Create revolute joint for left wheel
    let left_joint = GenericJointBuilder::new(JointAxesMask::LOCKED_REVOLUTE_AXES)
        .local_anchor1(nalgebra::Point3::new(0.0, -base_height / 2.0 + wheel_radius, wheel_separation / 2.0))
        .local_anchor2(nalgebra::Point3::new(0.0, 0.0, 0.0))
        .local_axis1(nalgebra::Unit::new_normalize(nalgebra::Vector3::new(0.0, 0.0, 1.0)))
        .local_axis2(nalgebra::Unit::new_normalize(nalgebra::Vector3::new(0.0, 1.0, 0.0)))
        .build();
    let left_joint_handle = physics_world.impulse_joint_set.insert(
        base_handle,
        left_handle,
        left_joint,
        true,
    );

    commands.entity(left_wheel_entity).insert(PhysicsJoint {
        handle: left_joint_handle,
        joint_type: JointType::Revolute,
    });

    // Right wheel
    let right_wheel_pos = position + Vec3::new(0.0, wheel_radius, -wheel_separation / 2.0);
    let right_wheel_entity = commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(wheel_radius, 0.018))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 0.1),
            ..default()
        })),
        Transform::from_translation(right_wheel_pos)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        Name::new("wheel_right"),
    )).id();

    let right_rb = RigidBodyBuilder::dynamic()
        .translation(vector![right_wheel_pos.x, right_wheel_pos.y, right_wheel_pos.z])
        .build();
    let right_handle = physics_world.spawn_rigid_body(right_rb, right_wheel_entity);
    physics_world.spawn_collider(wheel_collider, right_handle);

    // Create revolute joint for right wheel
    let right_joint = GenericJointBuilder::new(JointAxesMask::LOCKED_REVOLUTE_AXES)
        .local_anchor1(nalgebra::Point3::new(0.0, -base_height / 2.0 + wheel_radius, -wheel_separation / 2.0))
        .local_anchor2(nalgebra::Point3::new(0.0, 0.0, 0.0))
        .local_axis1(nalgebra::Unit::new_normalize(nalgebra::Vector3::new(0.0, 0.0, 1.0)))
        .local_axis2(nalgebra::Unit::new_normalize(nalgebra::Vector3::new(0.0, 1.0, 0.0)))
        .build();
    let right_joint_handle = physics_world.impulse_joint_set.insert(
        base_handle,
        right_handle,
        right_joint,
        true,
    );

    commands.entity(right_wheel_entity).insert(PhysicsJoint {
        handle: right_joint_handle,
        joint_type: JointType::Revolute,
    });

    // Caster wheel (simplified as sphere)
    let caster_pos = position + Vec3::new(-base_radius * 0.7, wheel_radius * 0.5, 0.0);
    let caster_entity = commands.spawn((
        Mesh3d(meshes.add(Sphere::new(wheel_radius * 0.5).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.05, 0.05),
            ..default()
        })),
        Transform::from_translation(caster_pos),
        Name::new("caster"),
    )).id();

    let caster_rb = RigidBodyBuilder::dynamic()
        .translation(vector![caster_pos.x, caster_pos.y, caster_pos.z])
        .build();
    let caster_handle = physics_world.spawn_rigid_body(caster_rb, caster_entity);

    let caster_collider = ColliderBuilder::new(ColliderShape::Sphere {
        radius: wheel_radius * 0.5,
    })
    .friction(0.1)
    .density(2.0)
    .build();
    physics_world.spawn_collider(caster_collider, caster_handle);

    // Fixed joint for caster to base
    let caster_joint = GenericJointBuilder::new(JointAxesMask::LOCKED_SPHERICAL_AXES)
        .local_anchor1(nalgebra::Point3::new(-base_radius * 0.7, -base_height / 2.0 + wheel_radius * 0.5, 0.0))
        .local_anchor2(nalgebra::Point3::new(0.0, 0.0, 0.0))
        .build();
    physics_world.impulse_joint_set.insert(base_handle, caster_handle, caster_joint, true);

    println!("TurtleBot3 created at {:?}", position);
}

fn robot_control_system(
    mut physics_world: ResMut<PhysicsWorld>,
    robots: Query<&TurtleBot3>,
    joints: Query<(&PhysicsJoint, &Name)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for robot in robots.iter() {
        let mut linear_vel = 0.0_f32;
        let mut angular_vel = 0.0_f32;

        if input.pressed(KeyCode::ArrowUp) {
            linear_vel = 0.3;
        }
        if input.pressed(KeyCode::ArrowDown) {
            linear_vel = -0.3;
        }
        if input.pressed(KeyCode::ArrowLeft) {
            angular_vel = 2.0;
        }
        if input.pressed(KeyCode::ArrowRight) {
            angular_vel = -2.0;
        }

        // Convert twist to wheel velocities
        let left_vel = (linear_vel - angular_vel * robot.wheel_separation / 2.0)
            / robot.wheel_radius;
        let right_vel = (linear_vel + angular_vel * robot.wheel_separation / 2.0)
            / robot.wheel_radius;

        // Apply to wheel joints
        for (joint, name) in joints.iter() {
            if let Some(impulse_joint) = physics_world.impulse_joint_set.get_mut(joint.handle) {
                if name.as_str().contains("left") {
                    impulse_joint.data.set_motor_velocity(
                        JointAxis::AngX,
                        left_vel,
                        5.0,
                    );
                } else if name.as_str().contains("right") {
                    impulse_joint.data.set_motor_velocity(
                        JointAxis::AngX,
                        right_vel,
                        5.0,
                    );
                }
            }
        }
    }
}

fn display_robot_state_system(
    robots: Query<&Transform, With<TurtleBot3>>,
    time: Res<Time>,
    mut last_print: Local<f32>,
) {
    // Print state every second
    if time.elapsed_secs() - *last_print < 1.0 {
        return;
    }
    *last_print = time.elapsed_secs();

    for transform in robots.iter() {
        let pos = transform.translation;
        let euler = transform.rotation.to_euler(EulerRot::YXZ);
        println!(
            "Robot: pos=({:.2}, {:.2}, {:.2}), yaw={:.2}rad",
            pos.x, pos.y, pos.z, euler.0
        );
    }
}

fn physics_step_system(mut physics_world: ResMut<PhysicsWorld>) {
    physics_world.step();
}

fn sync_transforms_system(
    physics_world: Res<PhysicsWorld>,
    mut transforms: Query<&mut Transform>,
) {
    for (_handle, rb) in physics_world.rigid_body_set.iter() {
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

## Loading from URDF File

For loading actual URDF files:

```rust
fn load_urdf_robot(
    mut commands: Commands,
    mut physics_world: ResMut<PhysicsWorld>,
    mut tf_tree: ResMut<TFTree>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut loader = URDFLoader::new()
        .with_base_path("assets/robots");

    // Load TurtleBot3 from URDF
    match loader.load(
        "assets/robots/turtlebot3_burger/model.urdf",
        &mut commands,
        &mut physics_world,
        &mut tf_tree,
        &mut meshes,
        &mut materials,
    ) {
        Ok(entity) => {
            // Add the TurtleBot3 component for control
            commands.entity(entity).insert(TurtleBot3 {
                wheel_separation: 0.16,
                wheel_radius: 0.033,
            });
            println!("Robot loaded: {:?}", entity);
        }
        Err(e) => {
            eprintln!("Failed to load robot: {}", e);
        }
    }
}
```

## Next Steps

- [Tutorial 3: Sensors](03_sensors.md) - Add sensors to your robots
- [API Reference: Physics](../api/physics.md) - Advanced physics configuration
