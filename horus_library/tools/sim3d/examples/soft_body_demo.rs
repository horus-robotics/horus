//! Soft body physics demonstration
//!
//! This example demonstrates:
//! - Cloth simulation with hanging fabric
//! - Rope/cable physics with catenary curves
//! - Deformable objects (rubber ball)
//! - Different soft body materials

use bevy::prelude::*;
use sim3d::physics::{
    soft_body::{
        cloth::Cloth, material::SoftBodyMaterial, particle::ParticleSystem, rope::Rope,
        spring::SpringSystem, SoftBodyPlugin,
    },
    PhysicsPlugin,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugin::default(), SoftBodyPlugin))
        .add_systems(Startup, setup_soft_body_scene)
        .run();
}

fn setup_soft_body_scene(mut commands: Commands) {
    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 3.0, 8.0).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
        ..default()
    });

    // Lighting
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
        ..default()
    });

    // Ground plane
    commands.spawn(PbrBundle {
        mesh: commands.spawn_empty().id().into(), // Placeholder
        ..default()
    });

    // Example 1: Hanging cloth (flag)
    let cloth = Cloth::new(20, 15, 2.0, 1.5)
        .with_material(SoftBodyMaterial::cloth())
        .with_thickness(0.001);

    let (particle_system, spring_system) = cloth.create_systems(Vec3::new(-3.0, 4.0, 0.0), Vec3::Z);

    commands.spawn((
        cloth,
        particle_system,
        spring_system,
        Transform::default(),
        GlobalTransform::default(),
    ));

    // Example 2: Rope bridge
    let rope1 = Rope::new(20, 3.0, 0.02).with_material(SoftBodyMaterial::rope());

    let (particle_system1, spring_system1) =
        rope1.create_systems(Vec3::new(-1.5, 3.0, -2.0), Vec3::new(1.5, 3.0, -2.0));

    commands.spawn((
        rope1,
        particle_system1,
        spring_system1,
        Transform::default(),
        GlobalTransform::default(),
    ));

    // Example 3: Soft rubber ball (simplified as hanging mass-spring system)
    let mut ball_particle_system = ParticleSystem::new(Vec3::new(0.0, -9.81, 0.0));
    let mut ball_spring_system = SpringSystem::new();

    let ball_material = SoftBodyMaterial::rubber();
    let radius = 0.3;
    let segments = 8;

    // Create a spherical structure with particles and springs
    for i in 0..segments {
        let theta = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let pos = Vec3::new(1.0 + radius * theta.cos(), 2.0 + radius * theta.sin(), -1.0);
        ball_particle_system.add_particle(
            sim3d::physics::soft_body::particle::Particle::new(pos, 0.1)
                .with_damping(ball_material.damping),
        );
    }

    // Connect particles in a circle
    for i in 0..segments {
        let next = (i + 1) % segments;
        ball_spring_system.connect(i, next, 0.2, 100.0);
    }

    commands.spawn((
        ball_particle_system,
        ball_spring_system,
        Transform::default(),
        GlobalTransform::default(),
    ));

    // Example 4: Elastic foam cube (simplified 2D grid)
    let mut foam_particle_system = ParticleSystem::new(Vec3::new(0.0, -9.81, 0.0));
    let mut foam_spring_system = SpringSystem::new();

    let foam_material = SoftBodyMaterial::foam();
    let grid_size = 4;
    let spacing = 0.2;

    // Create 2D grid of particles
    for y in 0..grid_size {
        for x in 0..grid_size {
            let pos = Vec3::new(3.0 + x as f32 * spacing, 2.0 + y as f32 * spacing, 0.0);
            foam_particle_system.add_particle(
                sim3d::physics::soft_body::particle::Particle::new(pos, 0.05)
                    .with_damping(foam_material.damping)
                    .with_fixed(y == grid_size - 1), // Fix top row
            );
        }
    }

    // Connect neighboring particles
    for y in 0..grid_size {
        for x in 0..grid_size {
            let idx = y * grid_size + x;

            // Horizontal springs
            if x < grid_size - 1 {
                foam_spring_system.connect(idx, idx + 1, spacing, 50.0);
            }

            // Vertical springs
            if y < grid_size - 1 {
                foam_spring_system.connect(idx, idx + grid_size, spacing, 50.0);
            }

            // Diagonal springs for shear resistance
            if x < grid_size - 1 && y < grid_size - 1 {
                let diag_length = (spacing * spacing * 2.0).sqrt();
                foam_spring_system.connect(idx, idx + grid_size + 1, diag_length, 30.0);
            }
        }
    }

    commands.spawn((
        foam_particle_system,
        foam_spring_system,
        Transform::default(),
        GlobalTransform::default(),
    ));

    info!("Soft body physics demo loaded!");
    info!("- Cloth simulation (left)");
    info!("- Rope bridge (center)");
    info!("- Rubber ball (center-right)");
    info!("- Foam cube (right)");
}
