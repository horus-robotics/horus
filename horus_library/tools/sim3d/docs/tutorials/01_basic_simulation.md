# Tutorial 1: Basic Simulation

This tutorial covers the fundamentals of creating simulations in Sim3D, including creating an empty world, adding a ground plane, spawning rigid bodies, and running the simulation.

## Prerequisites

- Sim3D built with visual features: `cargo build --release`
- Basic understanding of Rust and the Bevy ECS framework

## Creating an Empty World

Let's start with the minimal setup for a Sim3D application:

```rust
use bevy::prelude::*;
use sim3d::physics::PhysicsWorld;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sim3D - Basic Simulation".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<PhysicsWorld>()
        .add_systems(Startup, setup_world)
        .add_systems(Update, (physics_step_system, sync_transforms_system))
        .run();
}

fn setup_world(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
    });

    // Directional light (sun)
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));

    println!("Empty world created!");
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

## Adding a Ground Plane

Every simulation needs a ground plane for objects to rest on:

```rust
use sim3d::physics::collider::{ColliderBuilder, ColliderShape, create_ground_collider};
use rapier3d::prelude::*;

fn setup_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut physics_world: ResMut<PhysicsWorld>,
) {
    // Visual ground plane
    let ground_entity = commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.6, 0.4),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("Ground"),
    )).id();

    // Physics ground - static rigid body
    let ground_rb = RigidBodyBuilder::fixed()
        .translation(vector![0.0, -0.1, 0.0])
        .build();
    let ground_handle = physics_world.spawn_rigid_body(ground_rb, ground_entity);

    // Ground collider - thin box
    let ground_collider = ColliderBuilder::new(ColliderShape::Box {
        half_extents: Vec3::new(25.0, 0.1, 25.0),
    })
    .friction(0.8)
    .restitution(0.1)
    .build();
    physics_world.spawn_collider(ground_collider, ground_handle);

    println!("Ground plane created: 50x50 meters");
}
```

## Spawning Rigid Bodies

### Spawning a Box

```rust
fn spawn_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
    position: Vec3,
    half_extents: Vec3,
    color: Color,
) -> Entity {
    // Create visual mesh
    let entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(
            half_extents.x * 2.0,
            half_extents.y * 2.0,
            half_extents.z * 2.0,
        ))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            metallic: 0.3,
            perceptual_roughness: 0.6,
            ..default()
        })),
        Transform::from_translation(position),
        Name::new("Box"),
    )).id();

    // Create dynamic rigid body
    let rb = RigidBodyBuilder::dynamic()
        .translation(vector![position.x, position.y, position.z])
        .build();
    let rb_handle = physics_world.spawn_rigid_body(rb, entity);

    // Create box collider
    let collider = ColliderBuilder::new(ColliderShape::Box { half_extents })
        .friction(0.5)
        .restitution(0.3)
        .density(1.0)
        .build();
    physics_world.spawn_collider(collider, rb_handle);

    entity
}
```

### Spawning a Sphere

```rust
fn spawn_sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
    position: Vec3,
    radius: f32,
    color: Color,
) -> Entity {
    // Create visual mesh
    let entity = commands.spawn((
        Mesh3d(meshes.add(Sphere::new(radius).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            metallic: 0.5,
            perceptual_roughness: 0.3,
            ..default()
        })),
        Transform::from_translation(position),
        Name::new("Sphere"),
    )).id();

    // Create dynamic rigid body
    let rb = RigidBodyBuilder::dynamic()
        .translation(vector![position.x, position.y, position.z])
        .build();
    let rb_handle = physics_world.spawn_rigid_body(rb, entity);

    // Create sphere collider
    let collider = ColliderBuilder::new(ColliderShape::Sphere { radius })
        .friction(0.3)
        .restitution(0.7)  // Bouncy!
        .density(1.0)
        .build();
    physics_world.spawn_collider(collider, rb_handle);

    entity
}
```

### Spawning a Cylinder

```rust
fn spawn_cylinder(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
    position: Vec3,
    radius: f32,
    half_height: f32,
    color: Color,
) -> Entity {
    // Create visual mesh
    let entity = commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(radius, half_height * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            metallic: 0.4,
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_translation(position),
        Name::new("Cylinder"),
    )).id();

    // Create dynamic rigid body
    let rb = RigidBodyBuilder::dynamic()
        .translation(vector![position.x, position.y, position.z])
        .build();
    let rb_handle = physics_world.spawn_rigid_body(rb, entity);

    // Create cylinder collider
    let collider = ColliderBuilder::new(ColliderShape::Cylinder {
        half_height,
        radius,
    })
    .friction(0.6)
    .restitution(0.2)
    .density(1.0)
    .build();
    physics_world.spawn_collider(collider, rb_handle);

    entity
}
```

### Spawning a Capsule

```rust
fn spawn_capsule(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
    position: Vec3,
    radius: f32,
    half_length: f32,
    color: Color,
) -> Entity {
    // Create visual mesh
    let entity = commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(radius, half_length * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            metallic: 0.2,
            perceptual_roughness: 0.7,
            ..default()
        })),
        Transform::from_translation(position),
        Name::new("Capsule"),
    )).id();

    // Create dynamic rigid body
    let rb = RigidBodyBuilder::dynamic()
        .translation(vector![position.x, position.y, position.z])
        .build();
    let rb_handle = physics_world.spawn_rigid_body(rb, entity);

    // Create capsule collider
    let collider = ColliderBuilder::new(ColliderShape::Capsule {
        half_height: half_length,
        radius,
    })
    .friction(0.5)
    .restitution(0.4)
    .density(1.0)
    .build();
    physics_world.spawn_collider(collider, rb_handle);

    entity
}
```

## Complete Runnable Example

Here's a complete example that puts everything together:

```rust
use bevy::prelude::*;
use sim3d::physics::{PhysicsWorld, collider::{ColliderBuilder, ColliderShape}};
use rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sim3D - Basic Simulation Tutorial".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<PhysicsWorld>()
        .add_systems(Startup, setup_simulation)
        .add_systems(Update, (
            physics_step_system,
            sync_transforms_system,
            camera_control_system,
        ))
        .run();
}

fn setup_simulation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut physics_world: ResMut<PhysicsWorld>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(15.0, 10.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Lighting
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
    });

    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));

    // Ground plane
    let ground_entity = commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(30.0, 30.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.55, 0.35),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).id();

    let ground_rb = RigidBodyBuilder::fixed()
        .translation(vector![0.0, -0.1, 0.0])
        .build();
    let ground_handle = physics_world.spawn_rigid_body(ground_rb, ground_entity);
    let ground_collider = ColliderBuilder::new(ColliderShape::Box {
        half_extents: Vec3::new(15.0, 0.1, 15.0),
    })
    .friction(0.8)
    .build();
    physics_world.spawn_collider(ground_collider, ground_handle);

    // Spawn various objects
    let colors = [
        Color::srgb(0.9, 0.2, 0.2),  // Red
        Color::srgb(0.2, 0.9, 0.2),  // Green
        Color::srgb(0.2, 0.2, 0.9),  // Blue
        Color::srgb(0.9, 0.9, 0.2),  // Yellow
        Color::srgb(0.9, 0.2, 0.9),  // Magenta
    ];

    // Spawn falling boxes in a line
    for i in 0..5 {
        let x = (i as f32 - 2.0) * 2.0;
        let y = 5.0 + i as f32 * 1.5;
        spawn_physics_box(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut physics_world,
            Vec3::new(x, y, 0.0),
            Vec3::new(0.4, 0.4, 0.4),
            colors[i % colors.len()],
        );
    }

    // Spawn bouncing spheres
    for i in 0..3 {
        let x = (i as f32 - 1.0) * 3.0;
        spawn_physics_sphere(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut physics_world,
            Vec3::new(x, 8.0, 3.0),
            0.5,
            colors[(i + 2) % colors.len()],
        );
    }

    // Spawn a stack of cylinders
    for i in 0..4 {
        spawn_physics_cylinder(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut physics_world,
            Vec3::new(-4.0, 0.5 + i as f32 * 1.2, -3.0),
            0.4,
            0.5,
            colors[i % colors.len()],
        );
    }

    // Spawn capsules
    for i in 0..2 {
        spawn_physics_capsule(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut physics_world,
            Vec3::new(4.0, 3.0 + i as f32 * 2.0, -3.0),
            0.3,
            0.5,
            colors[(i + 3) % colors.len()],
        );
    }

    println!("Simulation created with multiple physics objects!");
}

fn spawn_physics_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
    position: Vec3,
    half_extents: Vec3,
    color: Color,
) {
    let entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(
            half_extents.x * 2.0,
            half_extents.y * 2.0,
            half_extents.z * 2.0,
        ))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            ..default()
        })),
        Transform::from_translation(position),
    )).id();

    let rb = RigidBodyBuilder::dynamic()
        .translation(vector![position.x, position.y, position.z])
        .build();
    let rb_handle = physics_world.spawn_rigid_body(rb, entity);

    let collider = ColliderBuilder::new(ColliderShape::Box { half_extents })
        .friction(0.5)
        .restitution(0.3)
        .build();
    physics_world.spawn_collider(collider, rb_handle);
}

fn spawn_physics_sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
    position: Vec3,
    radius: f32,
    color: Color,
) {
    let entity = commands.spawn((
        Mesh3d(meshes.add(Sphere::new(radius).mesh().ico(4).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            ..default()
        })),
        Transform::from_translation(position),
    )).id();

    let rb = RigidBodyBuilder::dynamic()
        .translation(vector![position.x, position.y, position.z])
        .build();
    let rb_handle = physics_world.spawn_rigid_body(rb, entity);

    let collider = ColliderBuilder::new(ColliderShape::Sphere { radius })
        .friction(0.3)
        .restitution(0.8)
        .build();
    physics_world.spawn_collider(collider, rb_handle);
}

fn spawn_physics_cylinder(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
    position: Vec3,
    radius: f32,
    half_height: f32,
    color: Color,
) {
    let entity = commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(radius, half_height * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            ..default()
        })),
        Transform::from_translation(position),
    )).id();

    let rb = RigidBodyBuilder::dynamic()
        .translation(vector![position.x, position.y, position.z])
        .build();
    let rb_handle = physics_world.spawn_rigid_body(rb, entity);

    let collider = ColliderBuilder::new(ColliderShape::Cylinder { half_height, radius })
        .friction(0.6)
        .restitution(0.2)
        .build();
    physics_world.spawn_collider(collider, rb_handle);
}

fn spawn_physics_capsule(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
    position: Vec3,
    radius: f32,
    half_height: f32,
    color: Color,
) {
    let entity = commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(radius, half_height * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            ..default()
        })),
        Transform::from_translation(position),
    )).id();

    let rb = RigidBodyBuilder::dynamic()
        .translation(vector![position.x, position.y, position.z])
        .build();
    let rb_handle = physics_world.spawn_rigid_body(rb, entity);

    let collider = ColliderBuilder::new(ColliderShape::Capsule { half_height, radius })
        .friction(0.5)
        .restitution(0.4)
        .build();
    physics_world.spawn_collider(collider, rb_handle);
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

fn camera_control_system(
    mut camera: Query<&mut Transform, With<Camera3d>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let mut camera_transform = camera.single_mut();
    let speed = 5.0 * time.delta_secs();

    if keys.pressed(KeyCode::KeyW) {
        let forward = camera_transform.forward();
        camera_transform.translation += forward * speed;
    }
    if keys.pressed(KeyCode::KeyS) {
        let forward = camera_transform.forward();
        camera_transform.translation -= forward * speed;
    }
    if keys.pressed(KeyCode::KeyA) {
        let left = camera_transform.left();
        camera_transform.translation += left * speed;
    }
    if keys.pressed(KeyCode::KeyD) {
        let right = camera_transform.right();
        camera_transform.translation += right * speed;
    }
    if keys.pressed(KeyCode::Space) {
        camera_transform.translation.y += speed;
    }
    if keys.pressed(KeyCode::ShiftLeft) {
        camera_transform.translation.y -= speed;
    }
}
```

## Running the Example

Save the code above to `examples/basic_simulation.rs` and run:

```bash
cargo run --example basic_simulation
```

Use WASD keys to move the camera, Space/Shift for up/down movement.

## Next Steps

- [Tutorial 2: Robot Simulation](02_robot_simulation.md) - Learn to load and control URDF robots
- [API Reference: Physics](../api/physics.md) - Deep dive into physics configuration
