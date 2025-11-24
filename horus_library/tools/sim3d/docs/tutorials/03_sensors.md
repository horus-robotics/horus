# Tutorial 3: Sensors

This tutorial covers adding and using sensors in Sim3D, including LiDAR, Camera, IMU, and other sensor types.

## Prerequisites

- Completed [Tutorial 2: Robot Simulation](02_robot_simulation.md)
- Understanding of Bevy ECS components and systems

## Sensor Overview

Sim3D provides 16 sensor types for robot perception:

| Sensor | Description | Output |
|--------|-------------|--------|
| LiDAR2D | 2D laser scanner | Array of distances |
| LiDAR3D | 3D point cloud | Point cloud with intensity |
| Camera | RGB camera | Image buffer |
| DepthCamera | Depth sensor | Depth image |
| RGBDCamera | Combined RGB+Depth | RGB + Depth images |
| IMU | Inertial measurement | Orientation, angular velocity, acceleration |
| GPS | Global positioning | Position with noise |
| ForceTorque | Contact sensor | 6-DOF force/torque |
| ContactSensor | Collision detection | Contact points |
| Encoder | Wheel/joint encoder | Position, velocity |
| Magnetometer | Magnetic field | 3-axis field vector |
| Barometer | Altitude sensor | Pressure/altitude |
| Radar | Radio detection | Range + velocity |
| Sonar | Acoustic ranging | Distance |
| ThermalCamera | Infrared imaging | Temperature image |
| EventCamera | Neuromorphic vision | Event stream |

## Adding LiDAR to a Robot

### 2D LiDAR

```rust
use bevy::prelude::*;
use sim3d::sensors::lidar2d::{Lidar2D, Lidar2DData, lidar2d_update_system};

fn add_lidar2d(
    mut commands: Commands,
    robot_entity: Entity,
) {
    // Create LiDAR sensor as child of robot
    commands.entity(robot_entity).with_children(|parent| {
        parent.spawn((
            // LiDAR configuration
            Lidar2D {
                // Sensor parameters
                rate_hz: 10.0,
                range_min: 0.1,
                range_max: 12.0,

                // Angular configuration
                angle_min: -std::f32::consts::PI,      // -180 degrees
                angle_max: std::f32::consts::PI,       // +180 degrees
                angle_increment: 0.017,                 // ~1 degree resolution

                // Noise model
                range_noise_std: 0.01,  // 1cm std dev

                // State
                last_update: 0.0,
            },
            // Data storage
            Lidar2DData::default(),
            // Position relative to robot base
            Transform::from_xyz(0.0, 0.2, 0.0),
            Name::new("lidar2d"),
        ));
    });
}

// System to process LiDAR data
fn process_lidar2d(
    query: Query<(&Lidar2DData, &Name)>,
) {
    for (lidar_data, name) in query.iter() {
        if lidar_data.ranges.is_empty() {
            continue;
        }

        // Find closest obstacle
        let min_range = lidar_data.ranges.iter()
            .filter(|&&r| r > 0.0 && r < f32::INFINITY)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f32::INFINITY);

        if min_range < 0.5 {
            println!("{}: Obstacle at {:.2}m!", name, min_range);
        }
    }
}
```

### 3D LiDAR

```rust
use sim3d::sensors::lidar3d::{Lidar3D, Lidar3DData, lidar3d_update_system};

fn add_lidar3d(
    mut commands: Commands,
    robot_entity: Entity,
) {
    commands.entity(robot_entity).with_children(|parent| {
        parent.spawn((
            Lidar3D {
                rate_hz: 10.0,
                range_min: 0.5,
                range_max: 100.0,

                // Horizontal scan
                horizontal_fov: std::f32::consts::TAU,  // 360 degrees
                horizontal_resolution: 1024,            // Points per revolution

                // Vertical scan (channels)
                vertical_fov: 0.52,                     // 30 degrees total
                vertical_channels: 16,                  // 16-beam LiDAR

                // Noise
                range_noise_std: 0.02,
                intensity_noise_std: 0.1,

                // State
                last_update: 0.0,
            },
            Lidar3DData::default(),
            Transform::from_xyz(0.0, 0.5, 0.0),
            Name::new("velodyne_vlp16"),
        ));
    });
}

// Process 3D point cloud
fn process_lidar3d(
    query: Query<&Lidar3DData>,
) {
    for lidar_data in query.iter() {
        let num_points = lidar_data.points.len();
        if num_points == 0 {
            continue;
        }

        // Calculate average height of points
        let avg_height: f32 = lidar_data.points.iter()
            .map(|p| p.y)
            .sum::<f32>() / num_points as f32;

        println!("LiDAR3D: {} points, avg height: {:.2}m", num_points, avg_height);
    }
}
```

## Adding a Camera

### RGB Camera

```rust
use sim3d::sensors::camera::{Camera, CameraData, RGBCameraData, camera_update_system};

fn add_rgb_camera(
    mut commands: Commands,
    robot_entity: Entity,
) {
    commands.entity(robot_entity).with_children(|parent| {
        parent.spawn((
            Camera {
                rate_hz: 30.0,
                width: 640,
                height: 480,

                // Camera intrinsics
                fov_horizontal: 1.22,  // 70 degrees
                fov_vertical: 0.92,    // ~53 degrees

                // Clipping planes
                near_clip: 0.1,
                far_clip: 100.0,

                // Noise (for RGB)
                noise_std: 0.01,

                last_update: 0.0,
            },
            RGBCameraData::default(),
            Transform::from_xyz(0.1, 0.2, 0.0)
                .looking_to(Vec3::X, Vec3::Y),
            Name::new("front_camera"),
        ));
    });
}

// Process camera image
fn process_camera(
    query: Query<(&RGBCameraData, &Name)>,
) {
    for (camera_data, name) in query.iter() {
        if camera_data.data.is_empty() {
            continue;
        }

        // Calculate average brightness
        let avg_brightness: f32 = camera_data.data.iter()
            .map(|&v| v as f32 / 255.0)
            .sum::<f32>() / camera_data.data.len() as f32;

        println!("{}: {}x{}, brightness: {:.2}",
            name,
            camera_data.width,
            camera_data.height,
            avg_brightness
        );
    }
}
```

### Depth Camera

```rust
use sim3d::sensors::camera::{DepthCamera, DepthCameraData};

fn add_depth_camera(
    mut commands: Commands,
    robot_entity: Entity,
) {
    commands.entity(robot_entity).with_children(|parent| {
        parent.spawn((
            DepthCamera {
                rate_hz: 30.0,
                width: 640,
                height: 480,
                fov_horizontal: 1.22,
                near_clip: 0.1,
                far_clip: 10.0,  // Depth cameras typically have shorter range
                depth_noise_std: 0.005,  // 5mm noise
                last_update: 0.0,
            },
            DepthCameraData::default(),
            Transform::from_xyz(0.1, 0.2, 0.0)
                .looking_to(Vec3::X, Vec3::Y),
            Name::new("depth_camera"),
        ));
    });
}

// Process depth data
fn process_depth_camera(
    query: Query<(&DepthCameraData, &Name)>,
) {
    for (depth_data, name) in query.iter() {
        if depth_data.depth.is_empty() {
            continue;
        }

        // Find minimum depth (closest object)
        let min_depth = depth_data.depth.iter()
            .filter(|&&d| d > 0.0 && !d.is_nan())
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f32::INFINITY);

        println!("{}: Closest object at {:.2}m", name, min_depth);
    }
}
```

## Adding an IMU

```rust
use sim3d::sensors::imu::{IMU, IMUData, imu_update_system};

fn add_imu(
    mut commands: Commands,
    robot_entity: Entity,
) {
    commands.entity(robot_entity).with_children(|parent| {
        parent.spawn((
            IMU {
                rate_hz: 100.0,

                // Accelerometer configuration
                accel_noise_density: 0.0004,     // m/s^2/sqrt(Hz)
                accel_random_walk: 0.006,        // m/s^3/sqrt(Hz)
                accel_bias_stability: 0.00004,   // m/s^2

                // Gyroscope configuration
                gyro_noise_density: 0.0002,      // rad/s/sqrt(Hz)
                gyro_random_walk: 0.00004,       // rad/s^2/sqrt(Hz)
                gyro_bias_stability: 0.00001,    // rad/s

                // Current bias values (evolve over time)
                accel_bias: Vec3::ZERO,
                gyro_bias: Vec3::ZERO,

                last_update: 0.0,
            },
            IMUData::default(),
            Transform::from_xyz(0.0, 0.05, 0.0),
            Name::new("imu"),
        ));
    });
}

// Process IMU data
fn process_imu(
    query: Query<(&IMUData, &Name)>,
) {
    for (imu_data, name) in query.iter() {
        // Convert orientation to Euler angles for display
        let euler = imu_data.orientation.to_euler(EulerRot::XYZ);

        println!("{}: roll={:.1}, pitch={:.1}, yaw={:.1}",
            name,
            euler.0.to_degrees(),
            euler.1.to_degrees(),
            euler.2.to_degrees()
        );

        // Check for high angular velocity (potential fall)
        if imu_data.angular_velocity.length() > 2.0 {
            println!("  WARNING: High angular velocity detected!");
        }
    }
}
```

## Adding GPS

```rust
use sim3d::sensors::gps::{GPS, GPSData, gps_update_system, VelocitySmoothingMethod};

fn add_gps(
    mut commands: Commands,
    robot_entity: Entity,
) {
    commands.entity(robot_entity).with_children(|parent| {
        // Standard consumer-grade GPS
        parent.spawn((
            GPS {
                rate_hz: 10.0,

                // Noise parameters (in meters)
                horizontal_noise_std: 2.5,  // Typical consumer GPS
                vertical_noise_std: 5.0,

                // Bias/drift
                horizontal_bias: Vec2::ZERO,
                vertical_bias: 0.0,
                bias_drift_rate: 0.01,

                // Satellite configuration
                min_satellites: 4,
                current_satellites: 8,
                hdop: 1.2,
                vdop: 1.8,

                // Velocity computation
                position_history: Default::default(),
                history_size: 10,
                velocity_smoothing: VelocitySmoothingMethod::WeightedAverage,
                min_samples_for_velocity: 2,

                last_update: 0.0,
            },
            GPSData::default(),
            Transform::from_xyz(0.0, 0.3, 0.0),
            Name::new("gps"),
        ));
    });

    // For RTK-GPS (high accuracy)
    commands.entity(robot_entity).with_children(|parent| {
        parent.spawn((
            GPS::high_accuracy(),  // 2cm accuracy
            GPSData::default(),
            Transform::from_xyz(0.0, 0.3, 0.0),
            Name::new("rtk_gps"),
        ));
    });
}

// Process GPS data
fn process_gps(
    query: Query<(&GPSData, &Name)>,
) {
    for (gps_data, name) in query.iter() {
        if !gps_data.has_valid_fix() {
            println!("{}: No GPS fix", name);
            continue;
        }

        println!("{}: pos=({:.2}, {:.2}, {:.2}), sats={}, hdop={:.1}",
            name,
            gps_data.position.x,
            gps_data.position.y,
            gps_data.position.z,
            gps_data.satellites_visible,
            gps_data.hdop
        );

        // Velocity if available
        if let Some(velocity) = gps_data.velocity {
            let speed = velocity.length();
            println!("  velocity: {:.2} m/s", speed);
        }
    }
}
```

## Complete Example: Multi-Sensor Robot

```rust
use bevy::prelude::*;
use sim3d::physics::{PhysicsWorld, collider::{ColliderBuilder, ColliderShape}};
use sim3d::sensors::{
    lidar2d::{Lidar2D, Lidar2DData},
    imu::{IMU, IMUData},
    camera::{Camera, RGBCameraData},
    gps::{GPS, GPSData, VelocitySmoothingMethod},
};
use rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sim3D - Multi-Sensor Robot".into(),
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
            sensor_update_system,
            display_sensor_data_system,
        ))
        .run();
}

#[derive(Component)]
struct SensorRobot;

fn setup_simulation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut physics_world: ResMut<PhysicsWorld>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.5, 0.0)),
    ));

    // Ground
    let ground_entity = commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.4, 0.3),
            ..default()
        })),
        Transform::default(),
    )).id();

    let ground_rb = RigidBodyBuilder::fixed().build();
    let ground_handle = physics_world.spawn_rigid_body(ground_rb, ground_entity);
    let ground_collider = ColliderBuilder::new(ColliderShape::Box {
        half_extents: Vec3::new(25.0, 0.1, 25.0),
    })
    .friction(0.8)
    .build();
    physics_world.spawn_collider(ground_collider, ground_handle);

    // Add some obstacles for sensors to detect
    for i in 0..5 {
        let angle = (i as f32 / 5.0) * std::f32::consts::TAU;
        let dist = 3.0;
        let pos = Vec3::new(angle.cos() * dist, 0.5, angle.sin() * dist);

        let obstacle_entity = commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.5, 1.0, 0.5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.2, 0.2),
                ..default()
            })),
            Transform::from_translation(pos),
        )).id();

        let rb = RigidBodyBuilder::fixed()
            .translation(vector![pos.x, pos.y, pos.z])
            .build();
        let rb_handle = physics_world.spawn_rigid_body(rb, obstacle_entity);
        let collider = ColliderBuilder::new(ColliderShape::Box {
            half_extents: Vec3::new(0.25, 0.5, 0.25),
        }).build();
        physics_world.spawn_collider(collider, rb_handle);
    }

    // Create robot with sensors
    create_sensor_robot(&mut commands, &mut meshes, &mut materials, &mut physics_world);
}

fn create_sensor_robot(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    physics_world: &mut PhysicsWorld,
) {
    // Robot base
    let robot_entity = commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.3, 0.2))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.3, 0.6),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.15, 0.0),
        SensorRobot,
        Name::new("sensor_robot"),
    )).id();

    let robot_rb = RigidBodyBuilder::dynamic()
        .translation(vector![0.0, 0.15, 0.0])
        .build();
    let robot_handle = physics_world.spawn_rigid_body(robot_rb, robot_entity);
    let robot_collider = ColliderBuilder::new(ColliderShape::Cylinder {
        half_height: 0.1,
        radius: 0.3,
    })
    .friction(0.5)
    .density(5.0)
    .build();
    physics_world.spawn_collider(robot_collider, robot_handle);

    // Add sensors as children
    commands.entity(robot_entity).with_children(|parent| {
        // LiDAR on top
        parent.spawn((
            Lidar2D {
                rate_hz: 10.0,
                range_min: 0.1,
                range_max: 10.0,
                angle_min: -std::f32::consts::PI,
                angle_max: std::f32::consts::PI,
                angle_increment: 0.0175,  // ~1 degree
                range_noise_std: 0.01,
                last_update: 0.0,
            },
            Lidar2DData::default(),
            Transform::from_xyz(0.0, 0.15, 0.0),
            Name::new("lidar"),
        ));

        // IMU in center
        parent.spawn((
            IMU {
                rate_hz: 100.0,
                accel_noise_density: 0.0004,
                accel_random_walk: 0.006,
                accel_bias_stability: 0.00004,
                gyro_noise_density: 0.0002,
                gyro_random_walk: 0.00004,
                gyro_bias_stability: 0.00001,
                accel_bias: Vec3::ZERO,
                gyro_bias: Vec3::ZERO,
                last_update: 0.0,
            },
            IMUData::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Name::new("imu"),
        ));

        // Front camera
        parent.spawn((
            Camera {
                rate_hz: 30.0,
                width: 320,
                height: 240,
                fov_horizontal: 1.22,
                fov_vertical: 0.92,
                near_clip: 0.1,
                far_clip: 50.0,
                noise_std: 0.01,
                last_update: 0.0,
            },
            RGBCameraData::default(),
            Transform::from_xyz(0.25, 0.1, 0.0)
                .looking_to(Vec3::X, Vec3::Y),
            Name::new("front_camera"),
        ));

        // GPS antenna
        parent.spawn((
            GPS {
                rate_hz: 10.0,
                horizontal_noise_std: 2.5,
                vertical_noise_std: 5.0,
                horizontal_bias: Vec2::ZERO,
                vertical_bias: 0.0,
                bias_drift_rate: 0.01,
                min_satellites: 4,
                current_satellites: 8,
                hdop: 1.2,
                vdop: 1.8,
                position_history: Default::default(),
                history_size: 10,
                velocity_smoothing: VelocitySmoothingMethod::LinearRegression,
                min_samples_for_velocity: 3,
                last_update: 0.0,
            },
            GPSData::default(),
            Transform::from_xyz(0.0, 0.2, 0.0),
            Name::new("gps"),
        ));
    });
}

fn sensor_update_system(
    time: Res<Time>,
    physics_world: Res<PhysicsWorld>,
    mut lidar_query: Query<(&mut Lidar2D, &mut Lidar2DData, &GlobalTransform)>,
    mut imu_query: Query<(&mut IMU, &mut IMUData, &GlobalTransform)>,
    mut gps_query: Query<(&mut GPS, &mut GPSData, &GlobalTransform)>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    // Update LiDAR sensors
    for (mut lidar, mut data, transform) in lidar_query.iter_mut() {
        if !lidar.should_update(current_time) {
            continue;
        }
        lidar.last_update = current_time;

        // Calculate number of rays
        let num_rays = ((lidar.angle_max - lidar.angle_min) / lidar.angle_increment) as usize + 1;
        data.ranges.clear();
        data.angles.clear();

        let origin = transform.translation();

        // Cast rays
        for i in 0..num_rays {
            let angle = lidar.angle_min + i as f32 * lidar.angle_increment;
            data.angles.push(angle);

            // Calculate ray direction in world space
            let local_dir = Vec3::new(angle.cos(), 0.0, angle.sin());
            let world_dir = transform.rotation() * local_dir;

            // Cast ray using physics world
            let ray = rapier3d::prelude::Ray::new(
                nalgebra::Point3::new(origin.x, origin.y, origin.z),
                nalgebra::Vector3::new(world_dir.x, world_dir.y, world_dir.z),
            );

            if let Some((_handle, toi)) = physics_world.query_pipeline.cast_ray(
                &physics_world.rigid_body_set,
                &physics_world.collider_set,
                &ray,
                lidar.range_max,
                true,
                rapier3d::prelude::QueryFilter::default(),
            ) {
                // Add noise
                let mut rng = rand::thread_rng();
                use rand::Rng;
                let noise = rng.gen_range(-lidar.range_noise_std..lidar.range_noise_std);
                let range = (toi + noise).max(lidar.range_min);
                data.ranges.push(range);
            } else {
                data.ranges.push(f32::INFINITY);
            }
        }
    }

    // Update IMU sensors
    for (mut imu, mut data, transform) in imu_query.iter_mut() {
        if current_time - imu.last_update < 1.0 / imu.rate_hz {
            continue;
        }
        imu.last_update = current_time;

        // Get orientation
        data.orientation = transform.rotation();

        // Simulate angular velocity (would need velocity tracking in real impl)
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let gyro_noise = Vec3::new(
            rng.gen_range(-imu.gyro_noise_density..imu.gyro_noise_density),
            rng.gen_range(-imu.gyro_noise_density..imu.gyro_noise_density),
            rng.gen_range(-imu.gyro_noise_density..imu.gyro_noise_density),
        );
        data.angular_velocity = imu.gyro_bias + gyro_noise;

        // Simulate linear acceleration (gravity + noise)
        let gravity_world = Vec3::new(0.0, -9.81, 0.0);
        let gravity_body = transform.rotation().inverse() * gravity_world;
        let accel_noise = Vec3::new(
            rng.gen_range(-imu.accel_noise_density..imu.accel_noise_density),
            rng.gen_range(-imu.accel_noise_density..imu.accel_noise_density),
            rng.gen_range(-imu.accel_noise_density..imu.accel_noise_density),
        );
        data.linear_acceleration = -gravity_body + imu.accel_bias + accel_noise;
    }

    // Update GPS sensors
    for (mut gps, mut data, transform) in gps_query.iter_mut() {
        if current_time - gps.last_update < 1.0 / gps.rate_hz {
            continue;
        }
        gps.last_update = current_time;
        gps.update_bias(dt);
        gps.update_satellite_count();

        if !gps.has_fix() {
            data.fix_quality = 0;
            continue;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let true_pos = transform.translation();

        data.position = Vec3::new(
            true_pos.x + rng.gen_range(-gps.horizontal_noise_std..gps.horizontal_noise_std) + gps.horizontal_bias.x,
            true_pos.y + rng.gen_range(-gps.vertical_noise_std..gps.vertical_noise_std) + gps.vertical_bias,
            true_pos.z + rng.gen_range(-gps.horizontal_noise_std..gps.horizontal_noise_std) + gps.horizontal_bias.y,
        );

        data.satellites_visible = gps.current_satellites;
        data.hdop = gps.hdop;
        data.vdop = gps.vdop;
        data.fix_quality = 1;
        data.timestamp = current_time;

        gps.add_position_sample(current_time, data.position);
        data.velocity = gps.compute_velocity();
    }
}

fn display_sensor_data_system(
    lidar_query: Query<(&Lidar2DData, &Name)>,
    imu_query: Query<(&IMUData, &Name)>,
    gps_query: Query<(&GPSData, &Name)>,
    time: Res<Time>,
    mut last_display: Local<f32>,
) {
    // Only display every second
    if time.elapsed_secs() - *last_display < 1.0 {
        return;
    }
    *last_display = time.elapsed_secs();

    println!("\n=== Sensor Data ===");

    // LiDAR
    for (data, name) in lidar_query.iter() {
        let valid_ranges: Vec<_> = data.ranges.iter()
            .filter(|&&r| r > 0.0 && r < f32::INFINITY)
            .collect();

        if let Some(&&min_range) = valid_ranges.iter().min_by(|a, b| a.partial_cmp(b).unwrap()) {
            println!("{}: {} rays, closest={:.2}m", name, data.ranges.len(), min_range);
        } else {
            println!("{}: {} rays, no obstacles", name, data.ranges.len());
        }
    }

    // IMU
    for (data, name) in imu_query.iter() {
        let euler = data.orientation.to_euler(EulerRot::XYZ);
        println!("{}: roll={:.1}, pitch={:.1}, yaw={:.1}",
            name,
            euler.0.to_degrees(),
            euler.1.to_degrees(),
            euler.2.to_degrees()
        );
    }

    // GPS
    for (data, name) in gps_query.iter() {
        if data.fix_quality > 0 {
            println!("{}: ({:.2}, {:.2}, {:.2}), sats={}",
                name,
                data.position.x,
                data.position.y,
                data.position.z,
                data.satellites_visible
            );
        } else {
            println!("{}: No fix", name);
        }
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

## Reading Sensor Data Summary

| Sensor | Data Type | Key Fields |
|--------|-----------|------------|
| Lidar2D | `Lidar2DData` | `ranges: Vec<f32>`, `angles: Vec<f32>` |
| Lidar3D | `Lidar3DData` | `points: Vec<Vec3>`, `intensities: Vec<f32>` |
| Camera | `RGBCameraData` | `data: Vec<u8>`, `width`, `height` |
| DepthCamera | `DepthCameraData` | `depth: Vec<f32>`, `width`, `height` |
| IMU | `IMUData` | `orientation`, `angular_velocity`, `linear_acceleration` |
| GPS | `GPSData` | `position`, `velocity`, `satellites_visible`, `fix_quality` |

## Next Steps

- [Tutorial 4: Reinforcement Learning](04_reinforcement_learning.md) - Use sensors for RL training
- [API Reference: Sensors](../api/sensors.md) - Complete sensor configuration reference
