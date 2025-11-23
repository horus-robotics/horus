//! # sim2d - Simple 2D Robotics Simulator Library
//!
//! This crate provides both a binary executable and a library interface
//! for the sim2d 2D robotics simulator.
//!
//! ## Binary Usage
//! ```bash
//! sim2d --robot my_robot.yaml --world my_world.yaml
//! ```
//!
//! ## Library Usage
//! ```rust,no_run
//! use sim2d::{Sim2DBuilder, RobotConfig, WorldConfig};
//!
//! let sim = Sim2DBuilder::new()
//!     .with_robot(RobotConfig::default())
//!     .with_world(WorldConfig::default())
//!     .headless(true)
//!     .build()
//!     .unwrap();
//! ```

// Re-export main module types
mod main_impl;
pub use main_impl::*;

// UI module
pub mod ui;

// Scenario save/load system
pub mod scenario;

// Recording/playback system
pub mod recorder;

// Camera sensor
pub mod camera;

// World editor
pub mod editor;

// Performance metrics
pub mod metrics;

// Robot kinematics
pub mod kinematics;

// Advanced sensors
pub mod sensors;

// Python API module (optional, enabled with "python" feature)
#[cfg(feature = "python")]
pub mod python_api;

use anyhow::Result;
use bevy::prelude::*;
use bevy::render::batching::gpu_preprocessing::GpuPreprocessingSupport;
use bevy::render::view::window::screenshot::CapturedScreenshots;
use bevy::render::RenderApp;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

/// Builder for creating a Sim2D simulation instance
pub struct Sim2DBuilder {
    robot_config: Option<RobotConfig>,
    world_config: Option<WorldConfig>,
    robot_name: String,
    topic_prefix: String,
    headless: bool,
}

impl Sim2DBuilder {
    /// Create a new Sim2D builder with default settings
    pub fn new() -> Self {
        Self {
            robot_config: None,
            world_config: None,
            robot_name: "robot".to_string(),
            topic_prefix: "/robot".to_string(),
            headless: false,
        }
    }

    /// Set the robot configuration
    pub fn with_robot(mut self, config: RobotConfig) -> Self {
        self.robot_config = Some(config);
        self
    }

    /// Set the world configuration
    pub fn with_world(mut self, config: WorldConfig) -> Self {
        self.world_config = Some(config);
        self
    }

    /// Set the robot name
    pub fn robot_name(mut self, name: impl Into<String>) -> Self {
        self.robot_name = name.into();
        self
    }

    /// Set the topic prefix for HORUS communication
    pub fn topic_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.topic_prefix = prefix.into();
        self
    }

    /// Enable or disable headless mode (no GUI)
    pub fn headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    /// Build and initialize the simulation
    pub fn build(self) -> Result<Sim2DApp> {
        let robot_config = self.robot_config.unwrap_or_default();
        let world_config = self.world_config.unwrap_or_default();

        Sim2DApp::new(
            robot_config,
            world_config,
            self.robot_name,
            self.topic_prefix,
            self.headless,
        )
    }
}

impl Default for Sim2DBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// State of the simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimState {
    /// Simulation is running
    Running,
    /// Simulation is paused
    Paused,
    /// Simulation has stopped
    Stopped,
}

/// Main simulation application wrapper
pub struct Sim2DApp {
    app: App,
    state: Arc<Mutex<SimState>>,
    robot_config: RobotConfig,
    world_config: WorldConfig,
}

impl Sim2DApp {
    /// Create a new Sim2D application
    pub fn new(
        robot_config: RobotConfig,
        world_config: WorldConfig,
        robot_name: String,
        topic_prefix: String,
        headless: bool,
    ) -> Result<Self> {
        let app = create_app(
            robot_config.clone(),
            world_config.clone(),
            robot_name,
            topic_prefix,
            headless,
        )?;

        let state = Arc::new(Mutex::new(SimState::Paused));

        Ok(Self {
            app,
            state,
            robot_config,
            world_config,
        })
    }

    /// Get the current simulation state
    pub fn state(&self) -> SimState {
        *self.state.lock().unwrap()
    }

    /// Start or resume the simulation
    pub fn run(&mut self) {
        *self.state.lock().unwrap() = SimState::Running;
    }

    /// Pause the simulation
    pub fn pause(&mut self) {
        *self.state.lock().unwrap() = SimState::Paused;
    }

    /// Stop the simulation completely
    pub fn stop(&mut self) {
        *self.state.lock().unwrap() = SimState::Stopped;
    }

    /// Reset the simulation to initial state
    pub fn reset(&mut self) -> Result<()> {
        // Recreate the app with the same configuration
        let robot_name = self.robot_config.name.clone();
        let topic_prefix = self.robot_config.topic_prefix.clone();
        let is_headless = self.app.is_plugin_added::<bevy::window::WindowPlugin>();

        self.app = create_app(
            self.robot_config.clone(),
            self.world_config.clone(),
            robot_name,
            topic_prefix,
            !is_headless,
        )?;

        self.app.update();
        *self.state.lock().unwrap() = SimState::Paused;

        Ok(())
    }

    /// Step the simulation forward by one frame
    pub fn step(&mut self) {
        if *self.state.lock().unwrap() != SimState::Stopped {
            self.app.update();
        }
    }

    /// Run the simulation for a specified duration (in seconds)
    pub fn run_for(&mut self, duration: f32) {
        let target_frames = (duration * 60.0) as u32; // Assuming 60 FPS
        *self.state.lock().unwrap() = SimState::Running;

        for _ in 0..target_frames {
            if *self.state.lock().unwrap() == SimState::Stopped {
                break;
            }
            self.app.update();
        }
    }

    /// Run the simulation continuously until stopped
    /// This is a blocking call that will run the Bevy app loop
    pub fn run_blocking(mut self) {
        *self.state.lock().unwrap() = SimState::Running;
        // Ensure app is fully initialized before running
        self.app.finish();
        self.app.cleanup();
        self.app.run();
    }

    /// Get a reference to the robot configuration
    pub fn robot_config(&self) -> &RobotConfig {
        &self.robot_config
    }

    /// Get a reference to the world configuration
    pub fn world_config(&self) -> &WorldConfig {
        &self.world_config
    }

    /// Get the Bevy app (for advanced usage)
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }
}

/// Create a Bevy app with the given configuration
fn create_app(
    robot_config: RobotConfig,
    world_config: WorldConfig,
    robot_name: String,
    topic_prefix: String,
    headless: bool,
) -> Result<App> {
    use main_impl::*;

    let mut robot_cfg = robot_config;
    robot_cfg.name = robot_name.clone();
    robot_cfg.topic_prefix = topic_prefix.clone();

    let args = Args {
        robot: None,
        world: None,
        world_image: None,
        resolution: 0.05,
        threshold: 128,
        topic: format!("{}/cmd_vel", topic_prefix),
        name: robot_name,
        headless,
    };

    let app_config = AppConfig {
        args,
        robots: vec![robot_cfg],
        world_config,
    };

    let mut app = App::new();

    if headless {
        // Headless mode - minimal plugins, no rendering
        app.add_plugins(MinimalPlugins)
            .insert_resource(app_config)
            .insert_resource(PhysicsWorld::default())
            .insert_resource(ui::UiState::default())
            .insert_resource(ui::VisualPreferences::default())
            .insert_resource(ui::RobotTelemetry::default())
            .insert_resource(editor::WorldEditor::new())
            .insert_resource(metrics::PerformanceMetrics::default())
            .insert_resource(LastLidarScan::default())
            .insert_resource(PreviousVelocity::default())
            .insert_resource(ObstacleIdCounter::default())
            .add_systems(Startup, setup)
            // Tick start - runs first to mark beginning of frame
            .add_systems(Update, tick_start_system)
            .add_systems(
                Update,
                (
                    horus_system,
                    physics_system,
                    telemetry_system,
                    odometry_publish_system,
                    imu_system,
                    lidar_system,
                    dynamic_obstacle_system,
                )
                    .after(tick_start_system),
            );
    } else {
        // GUI mode - full visualization
        // Disable pipelined rendering to avoid RenderAppChannels issues with bevy_egui
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "sim2d - 2D Robotics Simulator".to_string(),
                        resolution: (1600.0, 900.0).into(),
                        ..default()
                    }),
                    ..default()
                })
                .disable::<bevy::log::LogPlugin>()
                .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
        )
        .add_plugins(bevy_egui::EguiPlugin)
        .insert_resource({
            let (_, rx) = channel();
            CapturedScreenshots(Arc::new(Mutex::new(rx)))
        })
        .insert_resource(app_config)
        .insert_resource(PhysicsWorld::default())
        .insert_resource(ui::UiState::default())
        .insert_resource(ui::VisualPreferences::default())
        .insert_resource(ui::CameraController::default())
        .insert_resource(ui::RobotTelemetry::default())
        .insert_resource(ui::PerformanceMetrics::default())
        .insert_resource(recorder::Recorder::default())
        .insert_resource(editor::WorldEditor::new())
        .insert_resource(metrics::PerformanceMetrics::default())
        .insert_resource(TrajectoryHistory::default())
        .insert_resource(LastLidarScan::default())
        .insert_resource(PreviousVelocity::default())
        .insert_resource(CollisionState::default())
        .insert_resource(ObstacleIdCounter::default())
        .add_systems(Startup, setup)
        // Tick start - runs first to mark beginning of frame
        .add_systems(Update, tick_start_system)
        .add_systems(
            Update,
            (
                horus_system,
                physics_system,
                telemetry_system,
                visual_sync_system,
                visual_component_sync_system,
            )
                .after(tick_start_system),
        )
        .add_systems(
            Update,
            (odometry_publish_system, imu_system, lidar_system).after(tick_start_system),
        )
        .add_systems(
            Update,
            (
                trajectory_system,
                velocity_arrow_system,
                collision_detection_system,
                collision_indicator_system,
                dynamic_obstacle_system,
            ),
        )
        .add_systems(
            Update,
            (
                camera_control_system,
                mouse_camera_system,
                keyboard_input_system,
                reset_system,
                robot_visual_reload_system,
                world_reload_system,
            ),
        )
        .add_systems(
            Update,
            (visual_color_system, grid_system, lidar_rays_system),
        )
        .add_systems(Update, ui::ui_system)
        .add_systems(Update, ui::file_dialog_system)
        // Tick end - runs last to record frame completion
        .add_systems(Update, tick_end_system.after(ui::file_dialog_system));

        // Initialize GpuPreprocessingSupport in RenderApp to avoid panic
        // This is needed because we disabled PipelinedRenderingPlugin
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(GpuPreprocessingSupport::None);
        }
    }

    Ok(app)
}
