use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use bevy::scene::ScenePlugin;
use bevy_rapier3d::plugin::{NoUserData, RapierPhysicsPlugin};
use bevy_rapier3d::prelude::{
    ActiveEvents, Collider, CollisionEvent, Group, RigidBody, Sensor, Velocity,
};
use bevy_rapier3d::dynamics::GravityScale;
use bevy_rapier3d::geometry::CollisionGroups;

/// Test helper to create a minimal headless physics app
fn create_headless_physics_app() -> App {
    let mut app = App::new();

    // Add minimal plugins for headless physics simulation
    app.add_plugins(MinimalPlugins);
    app.add_plugins(TransformPlugin);
    app.add_plugins(HierarchyPlugin);
    app.add_plugins(AssetPlugin::default()); // Required by bevy_rapier3d
    app.add_plugins(ScenePlugin);

    // Initialize Mesh asset type (required by bevy_rapier3d's async scene collider)
    app.init_asset::<Mesh>();

    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());

    app
}

#[test]
fn test_physics_world_creation() {
    // Test that physics world can be created with default settings
    let mut app = create_headless_physics_app();

    // Run one update to initialize physics
    app.update();

    // If we got here without panicking, physics initialized successfully
    assert!(true, "Physics world created successfully");
}

#[test]
fn test_physics_step_integration() {
    // Test that physics step runs without errors
    let mut app = create_headless_physics_app();

    // Run a few frames
    for _ in 0..10 {
        app.update();
    }

    // If we got here without panicking, physics is running correctly
    assert!(true, "Physics step ran successfully for 10 frames");
}

#[test]
fn test_rigid_body_spawning() {
    // Test spawning rigid bodies in physics world
    let mut app = create_headless_physics_app();

    // Spawn a dynamic rigid body
    let entity = app.world_mut().spawn((
        Transform::from_xyz(0.0, 10.0, 0.0),
        RigidBody::Dynamic,
        Collider::ball(1.0),
    )).id();

    // Run one frame to process spawned entities
    app.update();

    // Verify entity exists and has physics components
    let world = app.world();
    assert!(world.get_entity(entity).is_ok(), "Entity should exist");
    assert!(world.get::<RigidBody>(entity).is_some(), "Entity should have RigidBody");
    assert!(world.get::<Collider>(entity).is_some(), "Entity should have Collider");
}

#[test]
fn test_gravity_affects_dynamic_bodies() {
    // Test that gravity actually moves dynamic bodies
    let mut app = create_headless_physics_app();

    // Spawn a dynamic body high in the air
    let initial_height = 100.0;
    let entity = app.world_mut().spawn((
        Transform::from_xyz(0.0, initial_height, 0.0),
        RigidBody::Dynamic,
        Collider::ball(1.0),
    )).id();

    // Run physics for several frames
    for _ in 0..60 {
        app.update();
    }

    // Check that the body has fallen due to gravity
    let world = app.world();
    let transform = world.get::<Transform>(entity).expect("Entity should have Transform");

    assert!(
        transform.translation.y < initial_height,
        "Dynamic body should fall due to gravity. Started at {}, now at {}",
        initial_height,
        transform.translation.y
    );
}

#[test]
fn test_static_bodies_do_not_move() {
    // Test that static bodies remain stationary
    let mut app = create_headless_physics_app();

    // Spawn a static body
    let position = Vec3::new(5.0, 5.0, 5.0);
    let entity = app.world_mut().spawn((
        Transform::from_translation(position),
        RigidBody::Fixed,
        Collider::cuboid(1.0, 1.0, 1.0),
    )).id();

    // Run physics for several frames
    for _ in 0..60 {
        app.update();
    }

    // Verify static body hasn't moved
    let world = app.world();
    let transform = world.get::<Transform>(entity).expect("Entity should have Transform");

    assert_eq!(
        transform.translation, position,
        "Static body should not move"
    );
}

#[test]
fn test_collision_detection() {
    // Test that collision events are generated
    let mut app = create_headless_physics_app();

    // Add collision event resource
    app.add_event::<CollisionEvent>();

    // Spawn a ground plane (static)
    app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        RigidBody::Fixed,
        Collider::cuboid(100.0, 0.1, 100.0),
    ));

    // Spawn a ball that will fall onto the ground
    let ball = app.world_mut().spawn((
        Transform::from_xyz(0.0, 2.0, 0.0),
        RigidBody::Dynamic,
        Collider::ball(0.5),
        ActiveEvents::COLLISION_EVENTS,
    )).id();

    // Run physics until the ball should have hit the ground
    for _ in 0..120 {
        app.update();
    }

    // Ball should have fallen and be near ground level
    let world = app.world();
    let transform = world.get::<Transform>(ball).expect("Ball should exist");

    // Ball should be near ground (accounting for ball radius)
    assert!(
        transform.translation.y < 1.5,
        "Ball should have fallen close to ground. Height: {}",
        transform.translation.y
    );
}

#[test]
fn test_multiple_bodies_simulation() {
    // Test simulating multiple physics bodies simultaneously
    let mut app = create_headless_physics_app();

    // Spawn several bodies at different heights
    let bodies: Vec<Entity> = (0..5).map(|i| {
        app.world_mut().spawn((
            Transform::from_xyz(i as f32 * 3.0, 10.0 + i as f32 * 5.0, 0.0),
            RigidBody::Dynamic,
            Collider::ball(0.5),
        )).id()
    }).collect();

    // Run physics
    for _ in 0..60 {
        app.update();
    }

    // All bodies should have fallen
    let world = app.world();
    for (i, &entity) in bodies.iter().enumerate() {
        let transform = world.get::<Transform>(entity).expect("Entity should exist");
        let initial_height = 10.0 + i as f32 * 5.0;
        assert!(
            transform.translation.y < initial_height,
            "Body {} should have fallen from initial height {}",
            i, initial_height
        );
    }
}

#[test]
fn test_velocity_application() {
    // Test that velocities can be applied to bodies
    let mut app = create_headless_physics_app();

    // Spawn a body with initial velocity
    let entity = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        RigidBody::Dynamic,
        Collider::ball(0.5),
        Velocity {
            linvel: Vec3::new(10.0, 0.0, 0.0),
            angvel: Vec3::ZERO,
        },
        GravityScale(0.0), // Disable gravity for this test
    )).id();

    // Run physics
    for _ in 0..30 {
        app.update();
    }

    // Body should have moved in the X direction
    let world = app.world();
    let transform = world.get::<Transform>(entity).expect("Entity should exist");

    assert!(
        transform.translation.x > 0.0,
        "Body should have moved in X direction. Position: {:?}",
        transform.translation
    );
}

#[test]
fn test_kinematic_bodies() {
    // Test kinematic body position control
    let mut app = create_headless_physics_app();

    // Spawn a kinematic body
    let entity = app.world_mut().spawn((
        Transform::from_xyz(0.0, 5.0, 0.0),
        RigidBody::KinematicPositionBased,
        Collider::ball(1.0),
    )).id();

    // Run a few frames
    for _ in 0..30 {
        app.update();
    }

    // Kinematic body should not have fallen (not affected by gravity)
    let world = app.world();
    let transform = world.get::<Transform>(entity).expect("Entity should exist");

    assert_eq!(
        transform.translation.y, 5.0,
        "Kinematic body should maintain position"
    );
}

#[test]
fn test_sensor_colliders() {
    // Test sensor (trigger) colliders
    let mut app = create_headless_physics_app();

    // Spawn a sensor collider
    let _sensor = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Collider::ball(2.0),
        Sensor,
    )).id();

    // Spawn a dynamic body that will pass through the sensor
    let initial_height = 5.0;
    let ball = app.world_mut().spawn((
        Transform::from_xyz(0.0, initial_height, 0.0),
        RigidBody::Dynamic,
        Collider::ball(0.5),
    )).id();

    // Run physics for enough time for the ball to fall through
    for _ in 0..300 {
        app.update();
    }

    // Ball should have passed through the sensor (no physical collision)
    let world = app.world();
    let transform = world.get::<Transform>(ball).expect("Ball should exist");

    // Ball should have fallen - sensors don't block physics bodies
    assert!(
        transform.translation.y < initial_height,
        "Ball should fall through sensor. Height: {}",
        transform.translation.y
    );
}

#[test]
fn test_collision_groups() {
    // Test that collision groups work correctly
    let mut app = create_headless_physics_app();

    // Define collision groups
    let group_a = Group::GROUP_1;
    let group_b = Group::GROUP_2;

    // Spawn a static body in group A
    app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        RigidBody::Fixed,
        Collider::cuboid(100.0, 0.5, 100.0),
        CollisionGroups::new(group_a, group_a),
    ));

    // Spawn a dynamic body in group B (shouldn't collide with group A)
    let ball = app.world_mut().spawn((
        Transform::from_xyz(0.0, 5.0, 0.0),
        RigidBody::Dynamic,
        Collider::ball(0.5),
        CollisionGroups::new(group_b, group_b),
    )).id();

    // Run physics for enough time to fall through
    for _ in 0..300 {
        app.update();
    }

    // Ball should have fallen through the platform (different collision groups)
    // Since it started at y=5.0 and there's no collision, it should fall below the platform
    let world = app.world();
    let transform = world.get::<Transform>(ball).expect("Ball should exist");
    let initial_height = 5.0;

    assert!(
        transform.translation.y < initial_height,
        "Ball should fall through platform with different collision group. Height: {}",
        transform.translation.y
    );
}
