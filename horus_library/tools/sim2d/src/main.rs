//! # sim2d - Simple 2D Robotics Simulator
//!
//! One command, physics + visualization, simple control.
//!
//! Usage:
//!   sim2d                                    # Default robot + world
//!   sim2d --robot my_robot.yaml              # Custom robot
//!   sim2d --world my_world.yaml              # Custom world
//!   sim2d --topic /robot/cmd_vel             # Custom control topic
//!
//! Control from another terminal:
//!   cargo run -p simple_driver

mod ui;

use anyhow::Result;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use clap::Parser;
use horus_core::{communication::Hub, core::NodeInfo};
use horus_library::messages::CmdVel;
use nalgebra::Vector2;
use rapier2d::prelude::*;
use tracing::{info, warn};

/// CLI arguments
#[derive(Parser)]
#[command(name = "sim2d")]
#[command(about = "Simple 2D robotics simulator with physics")]
pub struct Args {
    /// Robot configuration file (YAML)
    #[arg(long)]
    robot: Option<String>,

    /// World configuration file (YAML)
    #[arg(long)]
    world: Option<String>,

    /// Control topic name
    #[arg(long, default_value = "/robot/cmd_vel")]
    topic: String,

    /// Robot name
    #[arg(long, default_value = "robot")]
    name: String,
}

/// Robot configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RobotConfig {
    pub width: f32,
    pub length: f32,
    pub max_speed: f32,
    pub color: [f32; 3], // RGB
}

impl Default for RobotConfig {
    fn default() -> Self {
        Self {
            width: 0.5,             // 0.5m wide
            length: 0.8,            // 0.8m long
            max_speed: 2.0,         // 2 m/s max
            color: [0.2, 0.8, 0.2], // Green
        }
    }
}

/// World configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WorldConfig {
    pub width: f32,
    pub height: f32,
    pub obstacles: Vec<Obstacle>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Obstacle {
    pub pos: [f32; 2],
    pub size: [f32; 2],
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            width: 20.0,
            height: 15.0,
            obstacles: vec![
                Obstacle {
                    pos: [5.0, 5.0],
                    size: [2.0, 1.0],
                },
                Obstacle {
                    pos: [-3.0, -2.0],
                    size: [1.5, 1.5],
                },
                Obstacle {
                    pos: [0.0, 7.0],
                    size: [3.0, 0.5],
                },
            ],
        }
    }
}

/// Robot entity in Bevy
#[derive(Component)]
struct Robot {
    #[allow(dead_code)]
    pub name: String,
    pub config: RobotConfig,
    pub rigid_body_handle: RigidBodyHandle,
}

/// World boundaries and obstacles
#[derive(Component)]
struct WorldElement;

/// HORUS communication system
#[derive(Resource)]
struct HorusComm {
    cmd_vel_sub: Hub<CmdVel>,
    node_info: NodeInfo,
}

/// Physics world
#[derive(Resource)]
struct PhysicsWorld {
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    gravity: Vector<f32>,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    physics_hooks: (),
    event_handler: (),
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            gravity: vector![0.0, 0.0], // No gravity for top-down view
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            physics_hooks: (),
            event_handler: (),
        }
    }
}

/// App configuration
#[derive(Resource)]
pub struct AppConfig {
    pub args: Args,
    pub robot_config: RobotConfig,
    pub world_config: WorldConfig,
}

impl AppConfig {
    fn new(args: Args) -> Self {
        // Load robot config
        let robot_config = if let Some(robot_file) = &args.robot {
            Self::load_robot_config(robot_file).unwrap_or_default()
        } else {
            RobotConfig::default()
        };

        // Load world config
        let world_config = if let Some(world_file) = &args.world {
            Self::load_world_config(world_file).unwrap_or_default()
        } else {
            WorldConfig::default()
        };

        info!(
            " Robot: {:.1}m x {:.1}m, max speed: {:.1} m/s",
            robot_config.length, robot_config.width, robot_config.max_speed
        );
        info!(
            "🌍 World: {:.1}m x {:.1}m with {} obstacles",
            world_config.width,
            world_config.height,
            world_config.obstacles.len()
        );
        info!(" Control topic: {}", args.topic);

        Self {
            args,
            robot_config,
            world_config,
        }
    }

    pub fn load_robot_config(path: &str) -> Result<RobotConfig> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn load_world_config(path: &str) -> Result<WorldConfig> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
}

/// Setup system - initializes everything
fn setup(
    mut commands: Commands,
    app_config: Res<AppConfig>,
    mut physics_world: ResMut<PhysicsWorld>,
) {
    info!(" Setting up sim2d");

    // Setup camera with better positioning
    commands.spawn((
        Camera2d,
        Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
    ));

    // Create HORUS communication
    match Hub::new(&app_config.args.topic) {
        Ok(cmd_vel_sub) => {
            let node_info = NodeInfo::new("sim2d".to_string(), true);
            commands.insert_resource(HorusComm {
                cmd_vel_sub,
                node_info,
            });
            info!(" Connected to HORUS topic: {}", app_config.args.topic);
        }
        Err(e) => {
            warn!(" Failed to connect to HORUS: {}", e);
            warn!("   Robot will not respond to external commands");
        }
    }

    // Create world boundaries (scale up by 50 for visibility in pixels)
    let scale = 50.0;
    let world_half_width = app_config.world_config.width / 2.0 * scale;
    let world_half_height = app_config.world_config.height / 2.0 * scale;

    let boundaries = [
        // Bottom, Top, Left, Right walls
        (
            vector![0.0, -world_half_height],
            vector![app_config.world_config.width * scale, 0.2 * scale],
        ),
        (
            vector![0.0, world_half_height],
            vector![app_config.world_config.width * scale, 0.2 * scale],
        ),
        (
            vector![-world_half_width, 0.0],
            vector![0.2 * scale, app_config.world_config.height * scale],
        ),
        (
            vector![world_half_width, 0.0],
            vector![0.2 * scale, app_config.world_config.height * scale],
        ),
    ];

    let boundaries_physics = [
        // Bottom, Top, Left, Right walls (original scale for physics)
        (
            vector![0.0, -app_config.world_config.height / 2.0],
            vector![app_config.world_config.width, 0.2],
        ),
        (
            vector![0.0, app_config.world_config.height / 2.0],
            vector![app_config.world_config.width, 0.2],
        ),
        (
            vector![-app_config.world_config.width / 2.0, 0.0],
            vector![0.2, app_config.world_config.height],
        ),
        (
            vector![app_config.world_config.width / 2.0, 0.0],
            vector![0.2, app_config.world_config.height],
        ),
    ];

    for ((pos, size), (pos_scaled, size_scaled)) in boundaries_physics.iter().zip(boundaries.iter())
    {
        // Physics (original scale)
        let rigid_body = RigidBodyBuilder::fixed().translation(*pos).build();
        let collider = ColliderBuilder::cuboid(size.x / 2.0, size.y / 2.0).build();
        let _handle = physics_world.rigid_body_set.insert(rigid_body);
        physics_world.collider_set.insert(collider);

        // Visual (scaled for visibility)
        commands.spawn((
            Sprite {
                color: Color::srgb(0.3, 0.3, 0.3), // Gray walls
                custom_size: Some(Vec2::new(size_scaled.x, size_scaled.y)),
                ..default()
            },
            Transform::from_translation(Vec3::new(pos_scaled.x, pos_scaled.y, 0.0)),
            WorldElement,
        ));
    }

    // Create obstacles
    for obstacle in &app_config.world_config.obstacles {
        let pos_physics = vector![obstacle.pos[0], obstacle.pos[1]];
        let size_physics = vector![obstacle.size[0], obstacle.size[1]];
        let pos_visual = vector![obstacle.pos[0] * scale, obstacle.pos[1] * scale];
        let size_visual = vector![obstacle.size[0] * scale, obstacle.size[1] * scale];

        // Physics (original scale)
        let rigid_body = RigidBodyBuilder::fixed().translation(pos_physics).build();
        let collider = ColliderBuilder::cuboid(size_physics.x / 2.0, size_physics.y / 2.0).build();
        let _handle = physics_world.rigid_body_set.insert(rigid_body);
        physics_world.collider_set.insert(collider);

        // Visual (scaled)
        commands.spawn((
            Sprite {
                color: Color::srgb(0.6, 0.4, 0.2), // Brown obstacles
                custom_size: Some(Vec2::new(size_visual.x, size_visual.y)),
                ..default()
            },
            Transform::from_translation(Vec3::new(pos_visual.x, pos_visual.y, 0.5)),
            WorldElement,
        ));
    }

    // Create robot
    let robot_pos = vector![0.0, 0.0]; // Start at center (physics uses original scale)
    let robot_size = vector![
        app_config.robot_config.length * scale,
        app_config.robot_config.width * scale
    ];

    // Physics (use original scale)
    let robot_size_physics = vector![
        app_config.robot_config.length,
        app_config.robot_config.width
    ];
    let rigid_body = RigidBodyBuilder::dynamic()
        .translation(robot_pos)
        .linear_damping(2.0) // Natural deceleration
        .angular_damping(1.0)
        .build();
    let collider =
        ColliderBuilder::cuboid(robot_size_physics.x / 2.0, robot_size_physics.y / 2.0).build();

    let robot_handle = physics_world.rigid_body_set.insert(rigid_body);
    physics_world.collider_set.insert(collider);

    // Visual
    let robot_color = Color::srgb(
        app_config.robot_config.color[0],
        app_config.robot_config.color[1],
        app_config.robot_config.color[2],
    );

    commands.spawn((
        Sprite {
            color: robot_color,
            custom_size: Some(Vec2::new(robot_size.x, robot_size.y)),
            ..default()
        },
        Transform::from_translation(Vec3::new(robot_pos.x, robot_pos.y, 1.0)),
        Robot {
            name: app_config.args.name.clone(),
            config: app_config.robot_config.clone(),
            rigid_body_handle: robot_handle,
        },
    ));

    info!(" sim2d setup complete!");
    info!(
        "   [#] World: {}x{} meters",
        app_config.world_config.width, app_config.world_config.height
    );
    info!(
        "    Robot: {}x{} meters at ({}, {})",
        robot_size.x, robot_size.y, robot_pos.x, robot_pos.y
    );
    info!(
        "   🏢 Obstacles: {} created",
        app_config.world_config.obstacles.len()
    );
}

/// HORUS communication system
fn horus_system(
    mut horus_comm: Option<ResMut<HorusComm>>,
    robots: Query<&Robot>,
    mut physics_world: ResMut<PhysicsWorld>,
) {
    if let Some(comm) = horus_comm.as_mut() {
        let HorusComm {
            ref mut cmd_vel_sub,
            ref mut node_info,
        } = **comm;
        if let Some(cmd_vel) = cmd_vel_sub.recv(Some(node_info)) {
            for robot in robots.iter() {
                if let Some(rigid_body) = physics_world
                    .rigid_body_set
                    .get_mut(robot.rigid_body_handle)
                {
                    // Convert differential drive to physics forces
                    let robot_angle = rigid_body.rotation().angle();
                    let forward_dir = Vector2::new(robot_angle.cos(), robot_angle.sin());

                    // Apply linear velocity in robot's forward direction
                    let linear_vel = forward_dir
                        * cmd_vel
                            .linear
                            .clamp(-robot.config.max_speed, robot.config.max_speed);
                    let angular_vel = cmd_vel.angular.clamp(-3.0, 3.0); // Max angular speed

                    rigid_body.set_linvel(vector![linear_vel.x, linear_vel.y], true);
                    rigid_body.set_angvel(angular_vel, true);
                }
            }
        }
    }
}

/// Physics update system
fn physics_system(mut physics_world: ResMut<PhysicsWorld>) {
    let PhysicsWorld {
        ref mut physics_pipeline,
        ref gravity,
        ref integration_parameters,
        ref mut island_manager,
        ref mut broad_phase,
        ref mut narrow_phase,
        ref mut rigid_body_set,
        ref mut collider_set,
        ref mut impulse_joint_set,
        ref mut multibody_joint_set,
        ref mut ccd_solver,
        ref physics_hooks,
        ref event_handler,
    } = *physics_world;

    physics_pipeline.step(
        gravity,
        integration_parameters,
        island_manager,
        broad_phase,
        narrow_phase,
        rigid_body_set,
        collider_set,
        impulse_joint_set,
        multibody_joint_set,
        ccd_solver,
        None,
        physics_hooks,
        event_handler,
    );
}

/// Visual sync system - updates Bevy transforms from physics
fn visual_sync_system(
    mut robot_query: Query<(&Robot, &mut Transform)>,
    physics_world: Res<PhysicsWorld>,
) {
    let scale = 50.0; // Same scale used in setup
    for (robot, mut transform) in robot_query.iter_mut() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get(robot.rigid_body_handle) {
            // Update robot visual position from physics (scale up for visibility)
            let pos = rigid_body.translation();
            let rot = rigid_body.rotation();

            transform.translation.x = pos.x * scale;
            transform.translation.y = pos.y * scale;
            transform.rotation = Quat::from_rotation_z(rot.angle());
        }
    }
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    info!(" Starting sim2d - Simple 2D Robotics Simulator");
    info!("   One command, physics + visualization!");

    // Create app configuration
    let app_config = AppConfig::new(args);

    info!("[>] Control the robot from another terminal:");
    info!("   cargo run -p simple_driver");
    info!("   (publishes to: {})", app_config.args.topic);

    // Run Bevy app
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "sim2d - 2D Robotics Simulator".to_string(),
                        resolution: (1600.0, 900.0).into(), // Wider for panel + viewport
                        ..default()
                    }),
                    ..default()
                })
                .disable::<bevy::log::LogPlugin>(), // Disable Bevy logging to avoid conflicts
        )
        .add_plugins(EguiPlugin) // Add egui plugin for UI
        .insert_resource(app_config)
        .insert_resource(PhysicsWorld::default())
        .insert_resource(ui::UiState::default()) // Add UI state
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                ui::ui_system,          // UI panel rendering
                ui::file_dialog_system, // File picker handling
                horus_system,
                physics_system,
                visual_sync_system,
            ),
        )
        .run();

    Ok(())
}
