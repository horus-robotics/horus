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
use horus_library::messages::{CmdVel, Imu, LaserScan, Odometry, Pose2D, Twist};
use nalgebra::Vector2;
use rapier2d::prelude::*;
use horus_core::core::LogSummary;
use serde::{Deserialize, Serialize};
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
    #[serde(default = "default_robot_name")]
    pub name: String,
    #[serde(default = "default_robot_topic_prefix")]
    pub topic_prefix: String, // Topic prefix (e.g., "/robot0", "/robot1")
    #[serde(default = "default_robot_position")]
    pub position: [f32; 2], // Initial position [x, y]
    pub width: f32,
    pub length: f32,
    pub max_speed: f32,
    pub color: [f32; 3], // RGB
    #[serde(default)]
    pub visual: VisualComponents,
    #[serde(default)]
    pub lidar: LidarConfig,
}

fn default_robot_name() -> String {
    "robot".to_string()
}

fn default_robot_topic_prefix() -> String {
    "/robot".to_string()
}

fn default_robot_position() -> [f32; 2] {
    [0.0, 0.0]
}

/// LIDAR sensor configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LidarConfig {
    pub enabled: bool,
    pub range_max: f32,
    pub range_min: f32,
    pub angle_min: f32, // radians
    pub angle_max: f32, // radians
    pub num_rays: usize,
}

impl Default for LidarConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            range_max: 10.0,       // 10 meters max range
            range_min: 0.1,        // 10 cm min range
            angle_min: -std::f32::consts::PI, // -180 degrees
            angle_max: std::f32::consts::PI,  // +180 degrees
            num_rays: 360,         // 1 degree resolution
        }
    }
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
            name: "robot".to_string(),
            topic_prefix: "/robot".to_string(),
            position: [0.0, 0.0],
            width: 0.5,             // 0.5m wide
            length: 0.8,            // 0.8m long
            max_speed: 2.0,         // 2 m/s max
            color: [0.2, 0.8, 0.2], // Green
            visual: VisualComponents::default(),
            lidar: LidarConfig::default(),
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

/// Obstacle shape type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ObstacleShape {
    Rectangle,
    Circle,
}

impl Default for ObstacleShape {
    fn default() -> Self {
        Self::Rectangle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obstacle {
    pub pos: [f32; 2],
    #[serde(default)]
    pub shape: ObstacleShape,
    /// For rectangles: [width, height], for circles: [radius, radius]
    pub size: [f32; 2],
    #[serde(default)]
    pub color: Option<[f32; 3]>, // RGB color (0.0-1.0)
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            width: 20.0,
            height: 15.0,
            obstacles: vec![
                Obstacle {
                    pos: [5.0, 5.0],
                    shape: ObstacleShape::Rectangle,
                    size: [2.0, 1.0],
                    color: None,
                },
                Obstacle {
                    pos: [-3.0, -2.0],
                    shape: ObstacleShape::Rectangle,
                    size: [1.5, 1.5],
                    color: None,
                },
                Obstacle {
                    pos: [0.0, 7.0],
                    shape: ObstacleShape::Rectangle,
                    size: [3.0, 0.5],
                    color: None,
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

/// Velocity arrow marker
#[derive(Component)]
struct VelocityArrow;

/// LIDAR ray marker
#[derive(Component)]
struct LidarRay;

/// Trajectory trail marker
#[derive(Component)]
struct TrajectoryPoint;

/// Trajectory trail history
#[derive(Resource)]
struct TrajectoryHistory {
    points: Vec<(f32, f32)>, // (x, y) positions
}

impl Default for TrajectoryHistory {
    fn default() -> Self {
        Self { points: Vec::new() }
    }
}

/// Last LIDAR scan for visualization
#[derive(Resource, Default)]
struct LastLidarScan {
    ranges: Vec<f32>,
    angles: Vec<f32>,
    robot_pos: (f32, f32),
    robot_angle: f32,
}

/// Previous velocity for IMU acceleration calculation
#[derive(Resource)]
struct PreviousVelocity {
    linear: (f32, f32),
    angular: f32,
    timestamp: std::time::Instant,
}

impl Default for PreviousVelocity {
    fn default() -> Self {
        Self {
            linear: (0.0, 0.0),
            angular: 0.0,
            timestamp: std::time::Instant::now(),
        }
    }
}

/// Collision tracking resource
#[derive(Resource, Default)]
struct CollisionState {
    is_colliding: bool,
    collision_count: usize,
    last_collision_time: Option<std::time::Instant>,
}

/// Collision marker component
#[derive(Component)]
struct CollisionIndicator;

/// Obstacle command message for dynamic spawning/removal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleCommand {
    pub action: ObstacleAction,
    pub obstacle: Obstacle,
}

impl LogSummary for ObstacleCommand {
    fn log_summary(&self) -> String {
        format!(
            "ObstacleCmd({:?}, pos=[{:.1}, {:.1}])",
            self.action, self.obstacle.pos[0], self.obstacle.pos[1]
        )
    }
}

/// Action to perform on an obstacle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ObstacleAction {
    Add,
    Remove,
}

/// Unique ID for dynamically spawned obstacles
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DynamicObstacleId(u64);

/// Counter for generating unique obstacle IDs
#[derive(Resource)]
struct ObstacleIdCounter(u64);

impl Default for ObstacleIdCounter {
    fn default() -> Self {
        Self(0)
    }
}

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

/// Per-robot HORUS communication hubs
struct RobotHubs {
    cmd_vel_sub: Hub<CmdVel>,
    odom_pub: Hub<Odometry>,
    lidar_pub: Hub<LaserScan>,
    imu_pub: Hub<Imu>,
}

/// HORUS communication system
#[derive(Resource)]
struct HorusComm {
    robot_hubs: std::collections::HashMap<String, RobotHubs>, // Per-robot hubs indexed by robot name
    obstacle_cmd_sub: Hub<ObstacleCommand>, // Shared obstacle command topic
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
    pub robots: Vec<RobotConfig>, // Support multiple robots
    pub world_config: WorldConfig,
}

impl AppConfig {
    fn new(args: Args) -> Self {
        // Load robot config(s)
        let robots = if let Some(robot_file) = &args.robot {
            Self::load_robots_config(robot_file).unwrap_or_else(|_| vec![RobotConfig::default()])
        } else {
            // Create single default robot using CLI args
            let mut robot = RobotConfig::default();
            robot.name = args.name.clone();
            robot.topic_prefix = args.topic.strip_suffix("/cmd_vel").unwrap_or(&args.topic).to_string();
            vec![robot]
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

        // Log robot info
        if robots.len() == 1 {
            info!(
                " Robot: {:.1}m x {:.1}m, max speed: {:.1} m/s",
                robots[0].length, robots[0].width, robots[0].max_speed
            );
            info!(" Control topic: {}/cmd_vel", robots[0].topic_prefix);
        } else {
            info!(" {} robots configured", robots.len());
            for robot in &robots {
                info!(
                    "   - {}: {:.1}m x {:.1}m at {:?}",
                    robot.name, robot.length, robot.width, robot.position
                );
            }
        }

        info!(
            "World: {:.1}m x {:.1}m with {} obstacles",
            world_config.width,
            world_config.height,
            world_config.obstacles.len()
        );

        Self {
            args,
            robots,
            world_config,
        }
    }

    pub fn load_robots_config(path: &str) -> Result<Vec<RobotConfig>> {
        let content = std::fs::read_to_string(path)?;

        // Try to parse as multi-robot config first
        #[derive(serde::Deserialize)]
        struct MultiRobotConfig {
            robots: Vec<RobotConfig>,
        }

        // Auto-detect format from file extension
        if path.ends_with(".toml") {
            // Try multi-robot format first
            if let Ok(config) = toml::from_str::<MultiRobotConfig>(&content) {
                Ok(config.robots)
            } else {
                // Fall back to single robot
                Ok(vec![toml::from_str(&content)?])
            }
        } else {
            // YAML format
            if let Ok(config) = serde_yaml::from_str::<MultiRobotConfig>(&content) {
                Ok(config.robots)
            } else {
                // Fall back to single robot
                Ok(vec![serde_yaml::from_str(&content)?])
            }
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

    pub fn load_world_from_image(
        image_path: &str,
        resolution: f32,
        threshold: u8,
    ) -> Result<WorldConfig> {
        use image::GenericImageView;

        info!("Loading world from image: {}", image_path);
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
                        shape: ObstacleShape::Rectangle,
                        size: [resolution, resolution], // Square obstacle per pixel
                        color: None,
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
    info!(" Spawning visual components for robot");

    // Spawn treads if configured
    if let Some(ref tread_config) = config.visual.treads {
        info!("   Spawning treads");

        let tread_color = Color::srgb(
            tread_config.color[0],
            tread_config.color[1],
            tread_config.color[2],
        );

        // Left tread
        commands.spawn((
            Sprite {
                color: tread_color,
                custom_size: Some(Vec2::new(config.length * scale, tread_config.width * scale)),
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
                custom_size: Some(Vec2::new(config.length * scale, tread_config.width * scale)),
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
        info!("   Spawning turret");
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
        info!("   Spawning cannon");
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

    // Create HORUS communication - per-robot hubs
    let mut robot_hubs = std::collections::HashMap::new();
    let mut horus_connected = true;

    for robot_config in &app_config.robots {
        let cmd_vel_topic = format!("{}/cmd_vel", robot_config.topic_prefix);
        let odom_topic = format!("{}/odom", robot_config.topic_prefix);
        let scan_topic = format!("{}/scan", robot_config.topic_prefix);
        let imu_topic = format!("{}/imu", robot_config.topic_prefix);

        match (
            Hub::new(&cmd_vel_topic),
            Hub::new(&odom_topic),
            Hub::new(&scan_topic),
            Hub::new(&imu_topic),
        ) {
            (Ok(cmd_vel_sub), Ok(odom_pub), Ok(lidar_pub), Ok(imu_pub)) => {
                robot_hubs.insert(robot_config.name.clone(), RobotHubs {
                    cmd_vel_sub,
                    odom_pub,
                    lidar_pub,
                    imu_pub,
                });
                info!(" Connected HORUS for robot '{}':", robot_config.name);
                info!("    cmd_vel: {}", cmd_vel_topic);
                info!("    odom: {}", odom_topic);
                info!("    scan: {}", scan_topic);
                info!("    imu: {}", imu_topic);
            }
            _ => {
                warn!(" Failed to connect HORUS for robot '{}'", robot_config.name);
                horus_connected = false;
            }
        }
    }

    // Check if we have any robot connections before moving robot_hubs
    let has_robot_hubs = !robot_hubs.is_empty();

    // Create shared obstacle command hub
    match Hub::new("/sim2d/obstacle_cmd") {
        Ok(obstacle_cmd_sub) => {
            let node_info = NodeInfo::new("sim2d".to_string(), true);
            commands.insert_resource(HorusComm {
                robot_hubs,
                obstacle_cmd_sub,
                node_info,
            });
            info!(" Connected to obstacle command topic: /sim2d/obstacle_cmd");
        }
        _ if !has_robot_hubs => {
            warn!(" Failed to connect to HORUS");
            warn!("   Robots will not respond to external commands or publish sensor data");
        }
        _ => {
            warn!(" Failed to connect to obstacle command topic, but robot topics are OK");
        }
    }

    if !horus_connected && !has_robot_hubs {
        warn!(" No HORUS connections established");
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
        let pos_visual = vector![obstacle.pos[0] * scale, obstacle.pos[1] * scale];

        // Determine obstacle color (custom or default brown)
        let obstacle_color = obstacle.color
            .map(|c| Color::srgb(c[0], c[1], c[2]))
            .unwrap_or(Color::srgb(0.6, 0.4, 0.2)); // Default brown

        // Create physics body and visual representation based on shape
        let (rigid_body, collider, sprite) = match obstacle.shape {
            ObstacleShape::Rectangle => {
                let size_physics = vector![obstacle.size[0], obstacle.size[1]];
                let size_visual = vector![obstacle.size[0] * scale, obstacle.size[1] * scale];

                let rigid_body = RigidBodyBuilder::fixed().translation(pos_physics).build();
                let collider = ColliderBuilder::cuboid(size_physics.x / 2.0, size_physics.y / 2.0).build();
                let sprite = Sprite {
                    color: obstacle_color,
                    custom_size: Some(Vec2::new(size_visual.x, size_visual.y)),
                    ..default()
                };

                (rigid_body, collider, sprite)
            }
            ObstacleShape::Circle => {
                let radius_physics = obstacle.size[0]; // Use first element as radius
                let radius_visual = radius_physics * scale;

                let rigid_body = RigidBodyBuilder::fixed().translation(pos_physics).build();
                let collider = ColliderBuilder::ball(radius_physics).build();

                // For circles, use a circular sprite (Bevy will render it as a circle if we use a circle mesh)
                // For now, we'll use a square sprite as a placeholder
                // TODO: Use proper circle mesh in the future
                let sprite = Sprite {
                    color: obstacle_color,
                    custom_size: Some(Vec2::new(radius_visual * 2.0, radius_visual * 2.0)),
                    ..default()
                };

                (rigid_body, collider, sprite)
            }
        };

        let handle = physics_world.rigid_body_set.insert(rigid_body);
        physics_world.collider_set.insert(collider);

        // Visual (scaled)
        commands.spawn((
            sprite,
            Transform::from_translation(Vec3::new(pos_visual.x, pos_visual.y, 0.5)),
            WorldElement,
            ObstacleElement, // Mark as obstacle for color updates
            PhysicsHandle {
                rigid_body_handle: handle,
            },
        ));
    }

    // Create robots (support multiple robots)
    for robot_config in &app_config.robots {
        let robot_pos = vector![robot_config.position[0], robot_config.position[1]]; // Use configured position
        let robot_size = vector![
            robot_config.length * scale,
            robot_config.width * scale
        ];

        // Physics (use original scale)
        let robot_size_physics = vector![
            robot_config.length,
            robot_config.width
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
            robot_config.color[0],
            robot_config.color[1],
            robot_config.color[2],
        );

        let robot_entity = commands
            .spawn((
                Sprite {
                    color: robot_color,
                    custom_size: Some(Vec2::new(robot_size.x, robot_size.y)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(robot_pos.x * scale, robot_pos.y * scale, 1.0)),
                Robot {
                    name: robot_config.name.clone(),
                    config: robot_config.clone(),
                    rigid_body_handle: robot_handle,
                },
            ))
            .id();

        // Spawn visual components if configured
        spawn_robot_visual_components(&mut commands, robot_entity, robot_config, scale);
    }

    info!(" sim2d setup complete!");
    info!(
        "   [#] World: {}x{} meters",
        app_config.world_config.width, app_config.world_config.height
    );
    info!(
        "    Robots: {} spawned",
        app_config.robots.len()
    );
    info!(
        "   Obstacles: {} created",
        app_config.world_config.obstacles.len()
    );
}

/// HORUS communication system
fn horus_system(
    mut horus_comm: Option<ResMut<HorusComm>>,
    robots: Query<&Robot>,
    mut physics_world: ResMut<PhysicsWorld>,
    ui_state: Res<ui::UiState>,
) {
    // Don't process commands if paused
    if ui_state.paused {
        return;
    }

    if let Some(ref mut comm) = horus_comm {
        let HorusComm {
            ref mut robot_hubs,
            ref mut node_info,
            ..
        } = **comm;

        // Process cmd_vel for each robot independently
        for robot in robots.iter() {
            if let Some(hubs) = robot_hubs.get_mut(&robot.name) {
                if let Some(cmd_vel) = hubs.cmd_vel_sub.recv(&mut Some(node_info)) {
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
}

/// Physics update system
fn physics_system(
    mut physics_world: ResMut<PhysicsWorld>,
    ui_state: Res<ui::UiState>,
) {
    // Don't run physics if paused
    if ui_state.paused {
        return;
    }

    let PhysicsWorld {
        ref mut physics_pipeline,
        ref gravity,
        ref mut integration_parameters,
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

    // Apply simulation speed
    let mut params = *integration_parameters;
    params.dt *= ui_state.simulation_speed;

    physics_pipeline.step(
        gravity,
        &params,
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
                robot.config.length * scale,
                robot.config.width * scale,
            ));
            sprite.color = Color::srgb(
                robot.config.color[0],
                robot.config.color[1],
                robot.config.color[2],
            );
        }
    }
}

/// Telemetry update system - updates robot telemetry data
fn telemetry_system(
    robot_query: Query<&Robot>,
    physics_world: Res<PhysicsWorld>,
    mut telemetry: ResMut<ui::RobotTelemetry>,
) {
    for robot in robot_query.iter() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get(robot.rigid_body_handle) {
            let pos = rigid_body.translation();
            let vel = rigid_body.linvel();
            let rot = rigid_body.rotation();
            let angvel = rigid_body.angvel();

            telemetry.position = (pos.x, pos.y);
            telemetry.velocity = (vel.x, vel.y);
            telemetry.heading = rot.angle();
            telemetry.angular_velocity = angvel;
        }
    }
}

/// Odometry publishing system - publishes robot state to HORUS
fn odometry_publish_system(
    robot_query: Query<&Robot>,
    physics_world: Res<PhysicsWorld>,
    mut horus_comm: Option<ResMut<HorusComm>>,
) {
    if let Some(ref mut comm) = horus_comm {
        let HorusComm { robot_hubs, node_info, .. } = &mut **comm;

        for robot in robot_query.iter() {
            // Get this robot's hubs
            if let Some(hubs) = robot_hubs.get_mut(&robot.name) {
                if let Some(rigid_body) = physics_world.rigid_body_set.get(robot.rigid_body_handle) {
                    let pos = rigid_body.translation();
                    let vel = rigid_body.linvel();
                    let rot = rigid_body.rotation();
                    let angvel = rigid_body.angvel();

                    // Get current timestamp
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;

                    // Create odometry message
                    let mut odom = Odometry::new();

                    // Set pose
                    odom.pose = Pose2D::new(pos.x as f64, pos.y as f64, rot.angle() as f64);

                    // Set twist (velocity)
                    odom.twist = Twist {
                        linear: [vel.x as f64, vel.y as f64, 0.0],
                        angular: [0.0, 0.0, angvel as f64],
                        timestamp,
                    };

                    // Set frame IDs
                    let frame_id = b"odom\0";
                    let child_frame_id = b"base_link\0";
                    odom.frame_id[..frame_id.len()].copy_from_slice(frame_id);
                    odom.child_frame_id[..child_frame_id.len()].copy_from_slice(child_frame_id);

                    // Set timestamp
                    odom.timestamp = timestamp;

                    // Publish to this robot's odom topic
                    let _ = hubs.odom_pub.send(odom, &mut Some(node_info));
                }
            }
        }
    }
}

/// IMU simulation system - publishes IMU data
fn imu_system(
    robot_query: Query<&Robot>,
    physics_world: Res<PhysicsWorld>,
    mut horus_comm: Option<ResMut<HorusComm>>,
    mut prev_vel: ResMut<PreviousVelocity>,
) {
    if let Some(ref mut comm) = horus_comm {
        let HorusComm { robot_hubs, node_info, .. } = &mut **comm;

        for robot in robot_query.iter() {
            // Get this robot's hubs
            if let Some(hubs) = robot_hubs.get_mut(&robot.name) {
                if let Some(rigid_body) = physics_world.rigid_body_set.get(robot.rigid_body_handle) {
                    let vel = rigid_body.linvel();
                    let rot = rigid_body.rotation();
                    let angvel = rigid_body.angvel();

                    // Calculate time delta
                    let now = std::time::Instant::now();
                    let dt = now.duration_since(prev_vel.timestamp).as_secs_f32();

                    // Calculate linear acceleration (change in velocity / time)
                    let accel_x = if dt > 0.0 {
                        (vel.x - prev_vel.linear.0) / dt
                    } else {
                        0.0
                    };
                    let accel_y = if dt > 0.0 {
                        (vel.y - prev_vel.linear.1) / dt
                    } else {
                        0.0
                    };

                    // Update previous velocity
                    prev_vel.linear = (vel.x, vel.y);
                    prev_vel.angular = angvel;
                    prev_vel.timestamp = now;

                    // Create IMU message
                    let mut imu = Imu::new();

                    // Set orientation as quaternion (rotation around Z-axis only for 2D)
                    let angle = rot.angle() as f64;
                    let half_angle = angle / 2.0;
                    imu.orientation = [
                        0.0,                      // x
                        0.0,                      // y
                        half_angle.sin(),         // z
                        half_angle.cos(),         // w
                    ];

                    // Set angular velocity (only Z-axis for 2D)
                    imu.angular_velocity = [0.0, 0.0, angvel as f64];

                    // Set linear acceleration
                    imu.linear_acceleration = [accel_x as f64, accel_y as f64, 0.0];

                    // Set timestamp
                    imu.timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;

                    // Publish to this robot's IMU topic
                    let _ = hubs.imu_pub.send(imu, &mut Some(node_info));
                }
            }
        }
    }
}

/// LIDAR simulation system - performs raycasting and publishes LaserScan
fn lidar_system(
    robot_query: Query<&Robot>,
    physics_world: Res<PhysicsWorld>,
    mut horus_comm: Option<ResMut<HorusComm>>,
    app_config: Res<AppConfig>,
    mut last_scan: ResMut<LastLidarScan>,
) {
    if let Some(ref mut comm) = horus_comm {
        let HorusComm { robot_hubs, node_info, .. } = &mut **comm;

        for robot in robot_query.iter() {
            // Get this robot's hubs
            if let Some(hubs) = robot_hubs.get_mut(&robot.name) {
                if !robot.config.lidar.enabled {
                    continue;
                }

                if let Some(rigid_body) = physics_world.rigid_body_set.get(robot.rigid_body_handle) {
                    let pos = rigid_body.translation();
                    let rot = rigid_body.rotation();
                    let robot_angle = rot.angle();

                    let lidar_cfg = &robot.config.lidar;

                    // Create laser scan message
                    let mut scan = LaserScan::default();
                    scan.angle_min = lidar_cfg.angle_min;
                    scan.angle_max = lidar_cfg.angle_max;
                    scan.range_min = lidar_cfg.range_min;
                    scan.range_max = lidar_cfg.range_max;
                    scan.angle_increment = (lidar_cfg.angle_max - lidar_cfg.angle_min) / lidar_cfg.num_rays as f32;
                    scan.scan_time = 0.1; // 10 Hz scan rate
                    scan.time_increment = scan.scan_time / lidar_cfg.num_rays as f32;

                    // Perform raycasting for each beam
                    let query_pipeline = QueryPipeline::new();
                    let step = (lidar_cfg.angle_max - lidar_cfg.angle_min) / (lidar_cfg.num_rays as f32 - 1.0);

                    // Store for visualization
                    last_scan.ranges.clear();
                    last_scan.angles.clear();
                    last_scan.robot_pos = (pos.x, pos.y);
                    last_scan.robot_angle = robot_angle;

                    for i in 0..lidar_cfg.num_rays.min(360) {
                        let angle = lidar_cfg.angle_min + i as f32 * step + robot_angle;
                        let ray_dir = vector![angle.cos(), angle.sin()];
                        let ray = Ray::new(point![pos.x, pos.y], ray_dir);

                        // Cast ray and find nearest intersection
                        let hit = query_pipeline.cast_ray(
                            &physics_world.rigid_body_set,
                            &physics_world.collider_set,
                            &ray,
                            lidar_cfg.range_max,
                            true, // solid objects only
                            QueryFilter::default(),
                        );

                        let range = hit.map(|(_, toi)| toi).unwrap_or(0.0);
                        scan.ranges[i] = range;

                        // Store for visualization
                        last_scan.ranges.push(range);
                        last_scan.angles.push(angle);
                    }

                    // Set timestamp
                    scan.timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;

                    // Publish to this robot's LiDAR topic
                    let _ = hubs.lidar_pub.send(scan, &mut Some(node_info));
                }
            }
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

    info!("Robot config changed - reloading visual components");

    // Despawn all existing visual components
    for entity in visual_components.iter() {
        commands.entity(entity).despawn();
    }

    // Respawn visual components for each robot with new config
    for (entity, robot) in robot_query.iter() {
        spawn_robot_visual_components(&mut commands, entity, &robot.config, 50.0);
    }
}

/// Visual component sync - updates turret, cannon, treads to follow robot
fn visual_component_sync_system(
    robot_query: Query<(&Robot, &Transform)>,
    mut turret_query: Query<(&RobotTurret, &mut Transform), Without<Robot>>,
    mut cannon_query: Query<(&RobotCannon, &mut Transform), (Without<Robot>, Without<RobotTurret>)>,
    mut tread_query: Query<
        (&RobotTread, &mut Transform),
        (Without<Robot>, Without<RobotTurret>, Without<RobotCannon>),
    >,
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

/// Collision detection system - detects collisions using the physics engine
fn collision_detection_system(
    robot_query: Query<&Robot>,
    physics_world: Res<PhysicsWorld>,
    mut collision_state: ResMut<CollisionState>,
) {
    collision_state.is_colliding = false;

    for robot in robot_query.iter() {
        // Get all colliders attached to the robot's rigid body
        if let Some(rigid_body) = physics_world.rigid_body_set.get(robot.rigid_body_handle) {
            // Check for contacts by iterating through all contact pairs in narrow phase
            for contact_pair in physics_world.narrow_phase.contact_pairs() {
                // Check if this contact involves any of the robot's colliders
                // We need to check the collider handles, not rigid body handles
                let robot_colliders: Vec<_> = rigid_body.colliders().iter().collect();

                let involves_robot = robot_colliders.contains(&&contact_pair.collider1)
                    || robot_colliders.contains(&&contact_pair.collider2);

                if involves_robot && contact_pair.has_any_active_contact {
                    collision_state.is_colliding = true;
                    collision_state.collision_count += 1;
                    collision_state.last_collision_time = Some(std::time::Instant::now());
                    break;
                }
            }
        }
    }
}

/// Collision visual indicator system - shows visual feedback for collisions
fn collision_indicator_system(
    mut commands: Commands,
    robot_query: Query<&Transform, With<Robot>>,
    collision_state: Res<CollisionState>,
    existing_indicators: Query<Entity, With<CollisionIndicator>>,
) {
    // Clear existing indicators
    for entity in existing_indicators.iter() {
        commands.entity(entity).despawn();
    }

    // Show indicator if colliding
    if collision_state.is_colliding {
        for transform in robot_query.iter() {
            // Red circle around robot to indicate collision
            commands.spawn((
                Sprite {
                    color: Color::srgba(1.0, 0.0, 0.0, 0.5), // Semi-transparent red
                    custom_size: Some(Vec2::new(60.0, 60.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(
                    transform.translation.x,
                    transform.translation.y,
                    0.9, // Just below robot
                )),
                CollisionIndicator,
            ));
        }
    }
}

/// Mouse camera control system - handles mouse input for camera movement
fn mouse_camera_system(
    mut camera_controller: ResMut<ui::CameraController>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut mouse_wheel: EventReader<bevy::input::mouse::MouseWheel>,
) {
    // Handle mouse wheel zoom
    for ev in mouse_wheel.read() {
        let zoom_delta = match ev.unit {
            bevy::input::mouse::MouseScrollUnit::Line => ev.y * 0.1,
            bevy::input::mouse::MouseScrollUnit::Pixel => ev.y * 0.01,
        };

        camera_controller.zoom = (camera_controller.zoom + zoom_delta).clamp(0.1, 5.0);
    }

    // Handle middle mouse button drag for pan
    if mouse_button_input.pressed(MouseButton::Middle) {
        for ev in mouse_motion.read() {
            // Invert the delta for natural panning
            camera_controller.pan_x -= ev.delta.x;
            camera_controller.pan_y += ev.delta.y; // Y is inverted in screen space
        }
    } else {
        // Clear motion events when not panning
        mouse_motion.clear();
    }
}

/// Visual color update system - updates colors based on preferences
fn visual_color_system(
    mut obstacle_query: Query<&mut Sprite, (With<ObstacleElement>, Without<GridLine>)>,
    mut wall_query: Query<
        &mut Sprite,
        (
            With<WorldElement>,
            Without<ObstacleElement>,
            Without<GridLine>,
        ),
    >,
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

/// Velocity arrow visualization system
fn velocity_arrow_system(
    mut commands: Commands,
    robot_query: Query<(&Robot, &Transform)>,
    physics_world: Res<PhysicsWorld>,
    visual_prefs: Res<ui::VisualPreferences>,
    existing_arrows: Query<Entity, With<VelocityArrow>>,
) {
    // Clear existing arrows
    for entity in existing_arrows.iter() {
        commands.entity(entity).despawn();
    }

    if !visual_prefs.show_velocity_arrows {
        return;
    }

    let scale = 50.0;

    for (robot, robot_transform) in robot_query.iter() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get(robot.rigid_body_handle) {
            let vel = rigid_body.linvel();
            let speed = (vel.x * vel.x + vel.y * vel.y).sqrt();

            if speed < 0.01 {
                continue; // Don't show arrow for stationary robot
            }

            // Arrow length proportional to speed
            let arrow_length = speed * scale * 2.0;
            let vel_angle = vel.y.atan2(vel.x);

            // Draw arrow as a rectangle
            commands.spawn((
                Sprite {
                    color: Color::srgb(1.0, 1.0, 0.0), // Yellow
                    custom_size: Some(Vec2::new(arrow_length, 5.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(
                    robot_transform.translation.x + (arrow_length / 2.0) * vel_angle.cos(),
                    robot_transform.translation.y + (arrow_length / 2.0) * vel_angle.sin(),
                    2.0, // Above robot
                ))
                .with_rotation(Quat::from_rotation_z(vel_angle)),
                VelocityArrow,
            ));
        }
    }
}

/// LIDAR rays visualization system
fn lidar_rays_system(
    mut commands: Commands,
    visual_prefs: Res<ui::VisualPreferences>,
    last_scan: Res<LastLidarScan>,
    existing_rays: Query<Entity, With<LidarRay>>,
) {
    // Clear existing rays
    for entity in existing_rays.iter() {
        commands.entity(entity).despawn();
    }

    if !visual_prefs.show_lidar_rays || last_scan.ranges.is_empty() {
        return;
    }

    let scale = 50.0;
    let (robot_x, robot_y) = last_scan.robot_pos;

    // Only draw every Nth ray to avoid clutter
    let ray_step = (last_scan.ranges.len() / 60).max(1);

    for i in (0..last_scan.ranges.len()).step_by(ray_step) {
        let range = last_scan.ranges[i];
        if range < 0.01 {
            continue; // Invalid reading
        }

        let angle = last_scan.angles[i];
        let end_x = robot_x + range * angle.cos();
        let end_y = robot_y + range * angle.sin();

        // Calculate midpoint and length
        let mid_x = (robot_x + end_x) / 2.0 * scale;
        let mid_y = (robot_y + end_y) / 2.0 * scale;
        let length = range * scale;

        // Draw ray as thin line
        commands.spawn((
            Sprite {
                color: Color::srgba(0.0, 1.0, 1.0, 0.3), // Cyan, semi-transparent
                custom_size: Some(Vec2::new(length, 1.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(mid_x, mid_y, 1.5))
                .with_rotation(Quat::from_rotation_z(angle)),
            LidarRay,
        ));
    }
}

/// Trajectory trail system
fn trajectory_system(
    mut commands: Commands,
    robot_query: Query<&Transform, With<Robot>>,
    visual_prefs: Res<ui::VisualPreferences>,
    mut trajectory: ResMut<TrajectoryHistory>,
    existing_points: Query<Entity, With<TrajectoryPoint>>,
    time: Res<Time>,
) {
    // Update trajectory history
    if visual_prefs.show_trajectory {
        for transform in robot_query.iter() {
            // Sample every 0.1 seconds
            if time.elapsed_secs() as usize % 10 == 0 {
                let pos = (transform.translation.x, transform.translation.y);
                trajectory.points.push(pos);

                // Limit trail length
                if trajectory.points.len() > visual_prefs.trajectory_length {
                    trajectory.points.remove(0);
                }
            }
        }
    } else {
        trajectory.points.clear();
    }

    // Clear existing visualization
    for entity in existing_points.iter() {
        commands.entity(entity).despawn();
    }

    if !visual_prefs.show_trajectory || trajectory.points.is_empty() {
        return;
    }

    // Draw trail points
    for (i, &(x, y)) in trajectory.points.iter().enumerate() {
        let alpha = (i as f32 / trajectory.points.len() as f32) * 0.8 + 0.2;

        commands.spawn((
            Sprite {
                color: Color::srgba(0.2, 0.8, 0.2, alpha), // Green with fading
                custom_size: Some(Vec2::new(4.0, 4.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(x, y, 0.8)),
            TrajectoryPoint,
        ));
    }
}

/// Dynamic obstacle spawning/removal system
fn dynamic_obstacle_system(
    mut commands: Commands,
    mut physics_world: ResMut<PhysicsWorld>,
    mut id_counter: ResMut<ObstacleIdCounter>,
    obstacle_entities: Query<(Entity, &DynamicObstacleId, &PhysicsHandle), With<ObstacleElement>>,
    mut horus_comm: Option<ResMut<HorusComm>>,
    visual_prefs: Res<ui::VisualPreferences>,
) {
    // Only process if we have HORUS connection
    let Some(comm) = horus_comm.as_deref_mut() else {
        return;
    };

    // Try to receive obstacle command (non-blocking)
    let HorusComm {
        ref mut obstacle_cmd_sub,
        ref mut node_info,
        ..
    } = *comm;
    if let Some(cmd) = obstacle_cmd_sub.recv(&mut Some(node_info)) {
        match cmd.action {
            ObstacleAction::Add => {
                info!("  Spawning dynamic obstacle at {:?}", cmd.obstacle.pos);

                let scale = 50.0; // Same scale as setup
                let pos_physics = vector![cmd.obstacle.pos[0], cmd.obstacle.pos[1]];
                let pos_visual = vector![cmd.obstacle.pos[0] * scale, cmd.obstacle.pos[1] * scale];

                // Determine obstacle color
                let obstacle_color = cmd.obstacle.color
                    .map(|c| Color::srgb(c[0], c[1], c[2]))
                    .unwrap_or(Color::srgb(
                        visual_prefs.obstacle_color[0],
                        visual_prefs.obstacle_color[1],
                        visual_prefs.obstacle_color[2],
                    ));

                // Create physics body and visual based on shape
                let (rigid_body, collider, sprite) = match cmd.obstacle.shape {
                    ObstacleShape::Rectangle => {
                        let size_physics = vector![cmd.obstacle.size[0], cmd.obstacle.size[1]];
                        let size_visual = vector![cmd.obstacle.size[0] * scale, cmd.obstacle.size[1] * scale];

                        let rigid_body = RigidBodyBuilder::fixed().translation(pos_physics).build();
                        let collider = ColliderBuilder::cuboid(size_physics.x / 2.0, size_physics.y / 2.0).build();
                        let sprite = Sprite {
                            color: obstacle_color,
                            custom_size: Some(Vec2::new(size_visual.x, size_visual.y)),
                            ..default()
                        };

                        (rigid_body, collider, sprite)
                    }
                    ObstacleShape::Circle => {
                        let radius_physics = cmd.obstacle.size[0];
                        let radius_visual = radius_physics * scale;

                        let rigid_body = RigidBodyBuilder::fixed().translation(pos_physics).build();
                        let collider = ColliderBuilder::ball(radius_physics).build();
                        let sprite = Sprite {
                            color: obstacle_color,
                            custom_size: Some(Vec2::new(radius_visual * 2.0, radius_visual * 2.0)),
                            ..default()
                        };

                        (rigid_body, collider, sprite)
                    }
                };

                // Insert into physics world
                let PhysicsWorld {
                    ref mut rigid_body_set,
                    ref mut collider_set,
                    ..
                } = *physics_world;

                let handle = rigid_body_set.insert(rigid_body);
                collider_set.insert_with_parent(collider, handle, rigid_body_set);

                // Generate unique ID for this obstacle
                let obstacle_id = DynamicObstacleId(id_counter.0);
                id_counter.0 += 1;

                // Spawn visual entity
                commands.spawn((
                    sprite,
                    Transform::from_translation(Vec3::new(pos_visual.x, pos_visual.y, 0.5)),
                    WorldElement,
                    ObstacleElement,
                    obstacle_id,
                    PhysicsHandle {
                        rigid_body_handle: handle,
                    },
                ));

                info!("  Spawned dynamic obstacle with ID: {:?}", obstacle_id);
            }
            ObstacleAction::Remove => {
                // Find and remove obstacle at position
                let target_pos = cmd.obstacle.pos;
                let tolerance = 0.1; // 10cm tolerance

                // Destructure physics_world to avoid borrow checker issues
                let PhysicsWorld {
                    ref mut rigid_body_set,
                    ref mut collider_set,
                    ref mut island_manager,
                    ref mut impulse_joint_set,
                    ref mut multibody_joint_set,
                    ..
                } = *physics_world;

                for (entity, _id, physics_handle) in obstacle_entities.iter() {
                    if let Some(rb) = rigid_body_set.get(physics_handle.rigid_body_handle) {
                        let rb_pos = rb.translation();
                        let distance = ((rb_pos.x - target_pos[0]).powi(2) + (rb_pos.y - target_pos[1]).powi(2)).sqrt();

                        if distance < tolerance {
                            info!("  Removing dynamic obstacle at {:?}", target_pos);

                            // Remove from physics world
                            rigid_body_set.remove(
                                physics_handle.rigid_body_handle,
                                island_manager,
                                collider_set,
                                impulse_joint_set,
                                multibody_joint_set,
                                true,
                            );

                            // Despawn visual entity
                            commands.entity(entity).despawn();
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// World reload system - detects config changes and reloads world
fn world_reload_system(
    mut commands: Commands,
    app_config: Res<AppConfig>,
    mut physics_world: ResMut<PhysicsWorld>,
    world_entities: Query<
        (Entity, &PhysicsHandle),
        Or<(With<WorldElement>, With<ObstacleElement>)>,
    >,
    visual_prefs: Res<ui::VisualPreferences>,
) {
    // Only reload if world config changed
    if !app_config.is_changed() {
        return;
    }

    info!("Reloading world...");

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
        collider_set.insert_with_parent(collider, handle, rigid_body_set);

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
        let pos_visual = vector![obstacle.pos[0] * scale, obstacle.pos[1] * scale];

        // Determine obstacle color (custom or visual preferences default)
        let obstacle_color = obstacle.color
            .map(|c| Color::srgb(c[0], c[1], c[2]))
            .unwrap_or(Color::srgb(
                visual_prefs.obstacle_color[0],
                visual_prefs.obstacle_color[1],
                visual_prefs.obstacle_color[2],
            ));

        // Create physics body and visual representation based on shape
        let (rigid_body, collider, sprite) = match obstacle.shape {
            ObstacleShape::Rectangle => {
                let size_physics = vector![obstacle.size[0], obstacle.size[1]];
                let size_visual = vector![obstacle.size[0] * scale, obstacle.size[1] * scale];

                let rigid_body = RigidBodyBuilder::fixed().translation(pos_physics).build();
                let collider = ColliderBuilder::cuboid(size_physics.x / 2.0, size_physics.y / 2.0).build();
                let sprite = Sprite {
                    color: obstacle_color,
                    custom_size: Some(Vec2::new(size_visual.x, size_visual.y)),
                    ..default()
                };

                (rigid_body, collider, sprite)
            }
            ObstacleShape::Circle => {
                let radius_physics = obstacle.size[0]; // Use first element as radius
                let radius_visual = radius_physics * scale;

                let rigid_body = RigidBodyBuilder::fixed().translation(pos_physics).build();
                let collider = ColliderBuilder::ball(radius_physics).build();
                let sprite = Sprite {
                    color: obstacle_color,
                    custom_size: Some(Vec2::new(radius_visual * 2.0, radius_visual * 2.0)),
                    ..default()
                };

                (rigid_body, collider, sprite)
            }
        };

        let handle = rigid_body_set.insert(rigid_body);
        collider_set.insert_with_parent(collider, handle, rigid_body_set);

        // Visual (scaled)
        commands.spawn((
            sprite,
            Transform::from_translation(Vec3::new(pos_visual.x, pos_visual.y, 0.5)),
            WorldElement,
            ObstacleElement,
            PhysicsHandle {
                rigid_body_handle: handle,
            },
        ));
    }

    info!(
        " World reloaded: {}x{}m with {} obstacles",
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
            .insert_resource(ui::RobotTelemetry::default()) // Telemetry for sensor systems
            .insert_resource(LastLidarScan::default()) // LIDAR scan storage
            .insert_resource(PreviousVelocity::default()) // Previous velocity for IMU
            .insert_resource(ObstacleIdCounter::default()) // Dynamic obstacle ID counter
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    horus_system,
                    physics_system,
                    telemetry_system,           // Update robot telemetry
                    odometry_publish_system,    // Publish odometry to HORUS
                    imu_system,                 // IMU simulation and publishing
                    lidar_system,               // LIDAR simulation and publishing
                    dynamic_obstacle_system,    // Dynamic obstacle spawning/removal
                ),
            );
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
        .insert_resource(ui::RobotTelemetry::default()) // Add robot telemetry
        .insert_resource(ui::PerformanceMetrics::default()) // Add performance metrics
        .insert_resource(TrajectoryHistory::default()) // Add trajectory history
        .insert_resource(LastLidarScan::default()) // Add LIDAR scan storage
        .insert_resource(PreviousVelocity::default()) // Add previous velocity for IMU
        .insert_resource(CollisionState::default()) // Add collision state tracking
        .insert_resource(ObstacleIdCounter::default()) // Dynamic obstacle ID counter
        .add_systems(Startup, setup)
        // Core systems
        .add_systems(
            Update,
            (
                horus_system,
                physics_system,
                telemetry_system,
                visual_sync_system,
                visual_component_sync_system,
            ),
        )
        // Sensor systems
        .add_systems(
            Update,
            (
                odometry_publish_system,
                imu_system,
                lidar_system,
            ),
        )
        // Visual systems
        .add_systems(
            Update,
            (
                velocity_arrow_system,
                lidar_rays_system,
                trajectory_system,
                grid_system,
                visual_color_system,
            ),
        )
        // Interaction systems
        .add_systems(
            Update,
            (
                collision_detection_system,
                collision_indicator_system,
                mouse_camera_system,
                camera_control_system,
            ),
        )
        // UI and reload systems
        .add_systems(
            Update,
            (
                ui::ui_system,
                ui::file_dialog_system,
                world_reload_system,
                robot_visual_reload_system,
                dynamic_obstacle_system,  // Dynamic obstacle spawning/removal
            ),
        );
    }

    // Run the app
    app.run();

    Ok(())
}
