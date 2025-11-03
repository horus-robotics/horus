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

    /// World image file (PNG, JPG, PGM) - occupancy grid
    #[arg(long)]
    world_image: Option<String>,

    /// Resolution in meters per pixel (for world image)
    #[arg(long, default_value = "0.05")]
    resolution: f32,

    /// Obstacle threshold (0-255, pixels darker than this are obstacles)
    #[arg(long, default_value = "128")]
    threshold: u8,

    /// Control topic name
    #[arg(long, default_value = "/robot/cmd_vel")]
    topic: String,

    /// Robot name
    #[arg(long, default_value = "robot")]
    name: String,

    /// Run in headless mode (no GUI)
    #[arg(long)]
    headless: bool,
}

/// Robot configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RobotConfig {
    pub width: f32,
    pub length: f32,
    pub max_speed: f32,
    pub color: [f32; 3], // RGB
    #[serde(default)]
    pub visual: VisualComponents,
}

/// Optional visual components for robot appearance
#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct VisualComponents {
    /// Turret component (sits on top of hull)
    pub turret: Option<TurretConfig>,
    /// Cannon component (extends from turret)
    pub cannon: Option<CannonConfig>,
    /// Tread components (visual only, for tank-like appearance)
    pub treads: Option<TreadConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TurretConfig {
    pub width: f32,
    pub length: f32,
    pub offset_x: f32, // Offset from robot center
    pub offset_y: f32,
    #[serde(default = "default_turret_color")]
    pub color: [f32; 3],
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CannonConfig {
    pub length: f32,
    pub width: f32,
    pub offset_x: f32, // Offset from robot center (typically forward)
    #[serde(default = "default_cannon_color")]
    pub color: [f32; 3],
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TreadConfig {
    pub width: f32,  // Width of each tread
    pub offset: f32, // Distance from center to each tread
    #[serde(default = "default_tread_color")]
    pub color: [f32; 3],
}

fn default_turret_color() -> [f32; 3] {
    [0.25, 0.35, 0.18] // Darker olive
}

fn default_cannon_color() -> [f32; 3] {
    [0.2, 0.2, 0.2] // Dark gray
}

fn default_tread_color() -> [f32; 3] {
    [0.15, 0.15, 0.15] // Very dark gray
}

impl Default for RobotConfig {
    fn default() -> Self {
        Self {
            width: 0.5,             // 0.5m wide
            length: 0.8,            // 0.8m long
            max_speed: 2.0,         // 2 m/s max
            color: [0.2, 0.8, 0.2], // Green
            visual: VisualComponents::default(),
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

/// Obstacle marker
#[derive(Component)]
struct ObstacleElement;

/// Grid lines
#[derive(Component)]
struct GridLine;

/// Component to track physics rigid body handle for world elements
#[derive(Component)]
struct PhysicsHandle {
    rigid_body_handle: RigidBodyHandle,
}

/// Visual component markers - these follow the parent robot
#[derive(Component)]
struct RobotTurret {
    parent: Entity,
}

#[derive(Component)]
struct RobotCannon {
    parent: Entity,
}

#[derive(Component)]
struct RobotTread {
    parent: Entity,
    is_left: bool, // true for left tread, false for right
}

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

        // Load world config - prioritize image over config file
        let world_config = if let Some(image_file) = &args.world_image {
            // Load from image
            match Self::load_world_from_image(image_file, args.resolution, args.threshold) {
                Ok(config) => config,
                Err(e) => {
                    warn!("Failed to load world from image: {}", e);
                    warn!("Falling back to default world");
                    WorldConfig::default()
                }
            }
        } else if let Some(world_file) = &args.world {
            // Load from config file
            Self::load_world_config(world_file).unwrap_or_default()
        } else {
            // Use default
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

        // Auto-detect format from file extension
        if path.ends_with(".toml") {
            Ok(toml::from_str(&content)?)
        } else {
            // Default to YAML for .yaml, .yml, or no extension
            Ok(serde_yaml::from_str(&content)?)
        }
    }

    pub fn load_world_config(path: &str) -> Result<WorldConfig> {
        let content = std::fs::read_to_string(path)?;

        // Auto-detect format from file extension
        if path.ends_with(".toml") {
            Ok(toml::from_str(&content)?)
        } else {
            // Default to YAML for .yaml, .yml, or no extension
            Ok(serde_yaml::from_str(&content)?)
        }
    }

    pub fn load_world_from_image(image_path: &str, resolution: f32, threshold: u8) -> Result<WorldConfig> {
        use image::GenericImageView;

        info!("📷 Loading world from image: {}", image_path);
        info!("   Resolution: {} m/pixel", resolution);
        info!("   Threshold: {} (darker = obstacle)", threshold);

        // Load image
        let img = image::open(image_path)?;
        let (width_px, height_px) = img.dimensions();

        info!("   Image size: {}x{} pixels", width_px, height_px);

        // Convert to grayscale
        let gray_img = img.to_luma8();

        // Calculate world dimensions in meters
        let world_width = width_px as f32 * resolution;
        let world_height = height_px as f32 * resolution;

        info!("   World size: {:.2}m x {:.2}m", world_width, world_height);

        // Convert pixels to obstacles
        // Use a grid-based approach: group adjacent obstacle pixels into rectangles
        let mut obstacles = Vec::new();

        // Simple approach: create small square obstacles for each occupied pixel
        // This can be optimized later to merge adjacent pixels
        for y in 0..height_px {
            for x in 0..width_px {
                let pixel = gray_img.get_pixel(x, y)[0];

                // If pixel is darker than threshold, it's an obstacle
                if pixel < threshold {
                    // Convert pixel coordinates to world coordinates
                    // Image origin (0,0) is top-left, world origin is center
                    let world_x = (x as f32 - width_px as f32 / 2.0) * resolution;
                    let world_y = -(y as f32 - height_px as f32 / 2.0) * resolution; // Flip Y

                    obstacles.push(Obstacle {
                        pos: [world_x, world_y],
                        size: [resolution, resolution], // Square obstacle per pixel
                    });
                }
            }
        }

        info!("   Generated {} obstacle cells", obstacles.len());

        Ok(WorldConfig {
            width: world_width,
            height: world_height,
            obstacles,
        })
    }
}

/// Setup system - initializes everything
/// Helper function to spawn visual components for a robot
fn spawn_robot_visual_components(
    commands: &mut Commands,
    parent_entity: Entity,
    config: &RobotConfig,
    scale: f32,
) {
    info!("🎨 Spawning visual components for robot");

    // Spawn treads if configured
    if let Some(ref tread_config) = config.visual.treads {
        info!("  ✓ Spawning treads");

        let tread_color = Color::srgb(
            tread_config.color[0],
            tread_config.color[1],
            tread_config.color[2],
        );

        // Left tread
        commands.spawn((
            Sprite {
                color: tread_color,
                custom_size: Some(Vec2::new(
                    config.length * scale,
                    tread_config.width * scale,
                )),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.9)), // Slightly below robot
            RobotTread {
                parent: parent_entity,
                is_left: true,
            },
        ));

        // Right tread
        commands.spawn((
            Sprite {
                color: tread_color,
                custom_size: Some(Vec2::new(
                    config.length * scale,
                    tread_config.width * scale,
                )),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.9)), // Slightly below robot
            RobotTread {
                parent: parent_entity,
                is_left: false,
            },
        ));
    }

    // Spawn turret if configured
    if let Some(ref turret_config) = config.visual.turret {
        info!("  ✓ Spawning turret");
        let turret_color = Color::srgb(
            turret_config.color[0],
            turret_config.color[1],
            turret_config.color[2],
        );

        commands.spawn((
            Sprite {
                color: turret_color,
                custom_size: Some(Vec2::new(
                    turret_config.length * scale,
                    turret_config.width * scale,
                )),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 1.1)), // Above robot
            RobotTurret {
                parent: parent_entity,
            },
        ));
    }

    // Spawn cannon if configured
    if let Some(ref cannon_config) = config.visual.cannon {
        info!("  ✓ Spawning cannon");
        let cannon_color = Color::srgb(
            cannon_config.color[0],
            cannon_config.color[1],
            cannon_config.color[2],
        );

        commands.spawn((
            Sprite {
                color: cannon_color,
                custom_size: Some(Vec2::new(
                    cannon_config.length * scale,
                    cannon_config.width * scale,
                )),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 1.2)), // Above turret
            RobotCannon {
                parent: parent_entity,
            },
        ));
    }
}

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
        let handle = physics_world.rigid_body_set.insert(rigid_body);
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
            PhysicsHandle {
                rigid_body_handle: handle,
            },
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
        let handle = physics_world.rigid_body_set.insert(rigid_body);
        physics_world.collider_set.insert(collider);

        // Visual (scaled)
        commands.spawn((
            Sprite {
                color: Color::srgb(0.6, 0.4, 0.2), // Brown obstacles (will be updated by visual_color_system)
                custom_size: Some(Vec2::new(size_visual.x, size_visual.y)),
                ..default()
            },
            Transform::from_translation(Vec3::new(pos_visual.x, pos_visual.y, 0.5)),
            WorldElement,
            ObstacleElement, // Mark as obstacle for color updates
            PhysicsHandle {
                rigid_body_handle: handle,
            },
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

    let robot_entity = commands.spawn((
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
    )).id();

    // Spawn visual components if configured
    spawn_robot_visual_components(&mut commands, robot_entity, &app_config.robot_config, scale);

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
    mut robot_query: Query<(&Robot, &mut Transform, &mut Sprite)>,
    physics_world: Res<PhysicsWorld>,
    app_config: Res<AppConfig>,
) {
    let scale = 50.0; // Same scale used in setup
    for (robot, mut transform, mut sprite) in robot_query.iter_mut() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get(robot.rigid_body_handle) {
            // Update robot visual position from physics (scale up for visibility)
            let pos = rigid_body.translation();
            let rot = rigid_body.rotation();

            transform.translation.x = pos.x * scale;
            transform.translation.y = pos.y * scale;
            transform.rotation = Quat::from_rotation_z(rot.angle());

            // Update robot visual size and color from live config
            sprite.custom_size = Some(Vec2::new(
                app_config.robot_config.length * scale,
                app_config.robot_config.width * scale,
            ));
            sprite.color = Color::srgb(
                app_config.robot_config.color[0],
                app_config.robot_config.color[1],
                app_config.robot_config.color[2],
            );
        }
    }
}

/// Robot config change detection - respawns visual components when config changes
fn robot_visual_reload_system(
    mut commands: Commands,
    robot_query: Query<(Entity, &Robot)>,
    visual_components: Query<Entity, Or<(With<RobotTurret>, With<RobotCannon>, With<RobotTread>)>>,
    app_config: Res<AppConfig>,
) {
    if !app_config.is_changed() {
        return;
    }

    info!("🔄 Robot config changed - reloading visual components");

    // Despawn all existing visual components
    for entity in visual_components.iter() {
        commands.entity(entity).despawn();
    }

    // Respawn visual components for each robot with new config
    for (entity, _robot) in robot_query.iter() {
        spawn_robot_visual_components(&mut commands, entity, &app_config.robot_config, 50.0);
    }
}

/// Visual component sync - updates turret, cannon, treads to follow robot
fn visual_component_sync_system(
    robot_query: Query<(&Robot, &Transform)>,
    mut turret_query: Query<(&RobotTurret, &mut Transform), Without<Robot>>,
    mut cannon_query: Query<(&RobotCannon, &mut Transform), (Without<Robot>, Without<RobotTurret>)>,
    mut tread_query: Query<(&RobotTread, &mut Transform), (Without<Robot>, Without<RobotTurret>, Without<RobotCannon>)>,
) {
    let scale = 50.0;

    // Update turrets
    for (turret, mut transform) in turret_query.iter_mut() {
        if let Ok((robot, robot_transform)) = robot_query.get(turret.parent) {
            if let Some(ref turret_config) = robot.config.visual.turret {
                // Apply robot rotation to offset
                let angle = robot_transform.rotation.to_euler(EulerRot::ZYX).0;
                let rotated_offset = Vec2::new(
                    turret_config.offset_x * angle.cos() - turret_config.offset_y * angle.sin(),
                    turret_config.offset_x * angle.sin() + turret_config.offset_y * angle.cos(),
                ) * scale;

                transform.translation.x = robot_transform.translation.x + rotated_offset.x;
                transform.translation.y = robot_transform.translation.y + rotated_offset.y;
                transform.rotation = robot_transform.rotation;
            }
        }
    }

    // Update cannons
    for (cannon, mut transform) in cannon_query.iter_mut() {
        if let Ok((robot, robot_transform)) = robot_query.get(cannon.parent) {
            if let Some(ref cannon_config) = robot.config.visual.cannon {
                // Cannon extends forward from robot center
                let angle = robot_transform.rotation.to_euler(EulerRot::ZYX).0;
                let forward_offset = Vec2::new(
                    cannon_config.offset_x * angle.cos(),
                    cannon_config.offset_x * angle.sin(),
                ) * scale;

                transform.translation.x = robot_transform.translation.x + forward_offset.x;
                transform.translation.y = robot_transform.translation.y + forward_offset.y;
                transform.rotation = robot_transform.rotation;
            }
        }
    }

    // Update treads
    for (tread, mut transform) in tread_query.iter_mut() {
        if let Ok((robot, robot_transform)) = robot_query.get(tread.parent) {
            if let Some(ref tread_config) = robot.config.visual.treads {
                // Treads are offset perpendicular to forward direction
                let angle = robot_transform.rotation.to_euler(EulerRot::ZYX).0;
                let offset_multiplier = if tread.is_left { 1.0 } else { -1.0 };
                let lateral_offset = Vec2::new(
                    -angle.sin() * tread_config.offset * offset_multiplier,
                    angle.cos() * tread_config.offset * offset_multiplier,
                ) * scale;

                transform.translation.x = robot_transform.translation.x + lateral_offset.x;
                transform.translation.y = robot_transform.translation.y + lateral_offset.y;
                transform.rotation = robot_transform.rotation;
            }
        }
    }
}

/// Camera control system - applies zoom and pan
fn camera_control_system(
    mut camera_query: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
    camera_controller: Res<ui::CameraController>,
) {
    for (mut transform, mut projection) in camera_query.iter_mut() {
        // Apply pan
        transform.translation.x = camera_controller.pan_x;
        transform.translation.y = camera_controller.pan_y;

        // Apply zoom by adjusting orthographic scale
        // Lower scale = more zoomed in, higher scale = more zoomed out
        projection.scale = 1.0 / camera_controller.zoom;
    }
}

/// Visual color update system - updates colors based on preferences
fn visual_color_system(
    mut obstacle_query: Query<&mut Sprite, (With<ObstacleElement>, Without<GridLine>)>,
    mut wall_query: Query<&mut Sprite, (With<WorldElement>, Without<ObstacleElement>, Without<GridLine>)>,
    visual_prefs: Res<ui::VisualPreferences>,
) {
    // Update obstacle colors
    for mut sprite in obstacle_query.iter_mut() {
        sprite.color = Color::srgb(
            visual_prefs.obstacle_color[0],
            visual_prefs.obstacle_color[1],
            visual_prefs.obstacle_color[2],
        );
    }

    // Update wall colors
    for mut sprite in wall_query.iter_mut() {
        sprite.color = Color::srgb(
            visual_prefs.wall_color[0],
            visual_prefs.wall_color[1],
            visual_prefs.wall_color[2],
        );
    }
}

/// Grid rendering system - shows/hides and updates grid
fn grid_system(
    mut commands: Commands,
    visual_prefs: Res<ui::VisualPreferences>,
    existing_grid: Query<Entity, With<GridLine>>,
    app_config: Res<AppConfig>,
) {
    // Clear existing grid if preferences changed or grid is disabled
    if visual_prefs.is_changed() || !visual_prefs.show_grid {
        for entity in existing_grid.iter() {
            commands.entity(entity).despawn();
        }
    }

    // Re-create grid if enabled
    if visual_prefs.show_grid && visual_prefs.is_changed() {
        let scale = 50.0;
        let world_half_width = app_config.world_config.width / 2.0;
        let world_half_height = app_config.world_config.height / 2.0;
        let spacing = visual_prefs.grid_spacing;

        let grid_color = Color::srgba(
            visual_prefs.grid_color[0],
            visual_prefs.grid_color[1],
            visual_prefs.grid_color[2],
            0.3, // Semi-transparent
        );

        // Vertical lines
        let mut x = -world_half_width;
        while x <= world_half_width {
            commands.spawn((
                Sprite {
                    color: grid_color,
                    custom_size: Some(Vec2::new(
                        0.05 * scale,
                        app_config.world_config.height * scale,
                    )),
                    ..default()
                },
                Transform::from_translation(Vec3::new(x * scale, 0.0, -1.0)),
                GridLine,
            ));
            x += spacing;
        }

        // Horizontal lines
        let mut y = -world_half_height;
        while y <= world_half_height {
            commands.spawn((
                Sprite {
                    color: grid_color,
                    custom_size: Some(Vec2::new(
                        app_config.world_config.width * scale,
                        0.05 * scale,
                    )),
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, y * scale, -1.0)),
                GridLine,
            ));
            y += spacing;
        }
    }
}

/// World reload system - detects config changes and reloads world
fn world_reload_system(
    mut commands: Commands,
    app_config: Res<AppConfig>,
    mut physics_world: ResMut<PhysicsWorld>,
    world_entities: Query<(Entity, &PhysicsHandle), Or<(With<WorldElement>, With<ObstacleElement>)>>,
    visual_prefs: Res<ui::VisualPreferences>,
) {
    // Only reload if world config changed
    if !app_config.is_changed() {
        return;
    }

    info!("🔄 Reloading world...");

    // Destructure physics_world to avoid borrow checker issues
    let PhysicsWorld {
        ref mut rigid_body_set,
        ref mut collider_set,
        ref mut island_manager,
        ref mut impulse_joint_set,
        ref mut multibody_joint_set,
        ..
    } = *physics_world;

    // 1. Despawn all existing world entities (visual)
    for (entity, physics_handle) in world_entities.iter() {
        // Remove physics body
        rigid_body_set.remove(
            physics_handle.rigid_body_handle,
            island_manager,
            collider_set,
            impulse_joint_set,
            multibody_joint_set,
            true,
        );
        // Remove Bevy entity
        commands.entity(entity).despawn();
    }

    // 2. Recreate world with new config
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
        let handle = rigid_body_set.insert(rigid_body);
        collider_set.insert_with_parent(
            collider,
            handle,
            rigid_body_set,
        );

        // Visual (scaled for visibility)
        commands.spawn((
            Sprite {
                color: Color::srgb(
                    visual_prefs.wall_color[0],
                    visual_prefs.wall_color[1],
                    visual_prefs.wall_color[2],
                ),
                custom_size: Some(Vec2::new(size_scaled.x, size_scaled.y)),
                ..default()
            },
            Transform::from_translation(Vec3::new(pos_scaled.x, pos_scaled.y, 0.0)),
            WorldElement,
            PhysicsHandle {
                rigid_body_handle: handle,
            },
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
        let handle = rigid_body_set.insert(rigid_body);
        collider_set.insert_with_parent(
            collider,
            handle,
            rigid_body_set,
        );

        // Visual (scaled)
        commands.spawn((
            Sprite {
                color: Color::srgb(
                    visual_prefs.obstacle_color[0],
                    visual_prefs.obstacle_color[1],
                    visual_prefs.obstacle_color[2],
                ),
                custom_size: Some(Vec2::new(size_visual.x, size_visual.y)),
                ..default()
            },
            Transform::from_translation(Vec3::new(pos_visual.x, pos_visual.y, 0.5)),
            WorldElement,
            ObstacleElement,
            PhysicsHandle {
                rigid_body_handle: handle,
            },
        ));
    }

    info!("✓ World reloaded: {}x{}m with {} obstacles",
        app_config.world_config.width,
        app_config.world_config.height,
        app_config.world_config.obstacles.len()
    );
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    let headless = args.headless;

    if headless {
        info!(" Starting sim2d - Headless Mode");
        info!("   Physics only, no visualization");
    } else {
        info!(" Starting sim2d - Simple 2D Robotics Simulator");
        info!("   One command, physics + visualization!");
    }

    // Create app configuration
    let app_config = AppConfig::new(args);

    info!("[>] Control the robot from another terminal:");
    info!("   cargo run -p simple_driver");
    info!("   (publishes to: {})", app_config.args.topic);

    // Build app based on mode
    let mut app = App::new();

    if headless {
        // Headless mode - minimal plugins, no rendering
        app.add_plugins(MinimalPlugins)
            .insert_resource(app_config)
            .insert_resource(PhysicsWorld::default())
            .add_systems(Startup, setup)
            .add_systems(Update, (horus_system, physics_system));
    } else {
        // GUI mode - full visualization
        app.add_plugins(
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
        .insert_resource(ui::VisualPreferences::default()) // Add visual preferences
        .insert_resource(ui::CameraController::default()) // Add camera controller
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                ui::ui_system,          // UI panel rendering
                ui::file_dialog_system, // File picker handling
                world_reload_system,    // Live world reloading
                robot_visual_reload_system, // Robot visual component reloading
                camera_control_system,  // Camera zoom/pan
                grid_system,            // Grid overlay
                visual_color_system,    // Dynamic color updates
                horus_system,
                physics_system,
                visual_sync_system,
                visual_component_sync_system, // Tank visual components
            ),
        );
    }

    // Run the app
    app.run();

    Ok(())
}
