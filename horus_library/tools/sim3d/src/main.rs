use bevy::prelude::*;

mod cli;
mod config;
mod horus_bridge;
mod physics;
mod rendering;
mod robot;
mod scene;
mod sensors;
mod systems;
mod tf;
mod ui;
mod utils;
mod view_modes;

#[cfg(feature = "python")]
mod rl;

use cli::{Cli, Mode};
use physics::PhysicsWorld;
use scene::spawner::SpawnedObjects;
use systems::sensor_update::{SensorUpdatePlugin, SensorSystemSet};
use tf::TFTree;

/// System sets for organizing update order
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimSystemSet {
    /// Physics simulation and force application
    Physics,
    /// Transform frame updates
    TF,
}

fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting sim3d");
    info!("Mode: {:?}", cli.mode);

    match cli.mode {
        Mode::Visual => run_visual_mode(cli),
        Mode::Headless => run_headless_mode(cli),
    }
}

fn run_visual_mode(cli: Cli) {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "sim3d - HORUS 3D Simulator".into(),
                    resolution: (1920.0, 1080.0).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                ..default()
            })
            .disable::<bevy::log::LogPlugin>(), // Disable since we init tracing manually
    );

    #[cfg(feature = "visual")]
    {
        use bevy_egui::EguiPlugin;
        app.add_plugins(EguiPlugin);
    }

    #[cfg(feature = "editor")]
    {
        use bevy_editor_pls::EditorPlugin;
        app.add_plugins(EditorPlugin::default());
    }

    // Add sensor update plugin
    app.add_plugins(SensorUpdatePlugin);

    app.insert_resource(PhysicsWorld::default())
        .insert_resource(TFTree::new("world"))
        .insert_resource(SpawnedObjects::default())
        .insert_resource(cli);

    // Configure system set ordering: Physics -> Sensors -> TF
    app.configure_sets(
        Update,
        (
            SimSystemSet::Physics,
            SensorSystemSet::Update,
            SensorSystemSet::Visualization,
            SimSystemSet::TF,
        )
            .chain(),
    );

    app.add_systems(Startup, (rendering::setup::setup_scene,));

    // Physics systems
    app.add_systems(
        Update,
        (
            systems::physics_step::physics_step_system,
            systems::sync_visual::apply_external_forces_system,
            systems::sync_visual::apply_external_impulses_system,
            systems::sync_visual::apply_differential_drive_system,
            systems::sync_visual::sync_physics_to_visual_system,
            systems::sync_visual::sync_velocities_from_physics_system,
        )
            .chain()
            .in_set(SimSystemSet::Physics),
    );

    // TF update systems
    app.add_systems(
        Update,
        systems::tf_update::tf_update_system.in_set(SimSystemSet::TF),
    );

    #[cfg(feature = "visual")]
    {
        app.add_systems(Update, (ui::debug_panel::debug_panel_system,));

        app.add_systems(
            Update,
            (
                rendering::camera_controller::camera_controller_system,
                tf::visualizer::render_tf_frames,
            ),
        );
    }

    info!("Starting visual simulation");
    app.run();
}

fn run_headless_mode(cli: Cli) {
    info!("Starting headless mode for RL training");

    let mut app = App::new();

    // Use minimal plugins (no rendering, no input, no audio)
    app.add_plugins(MinimalPlugins);

    // Add essential resources
    app.insert_resource(PhysicsWorld::default())
        .insert_resource(TFTree::new("world"))
        .insert_resource(SpawnedObjects::default())
        .insert_resource(cli);

    // Configure system set ordering: Physics -> Sensors -> TF
    app.configure_sets(
        Update,
        (
            SimSystemSet::Physics,
            SensorSystemSet::Update,
            SimSystemSet::TF,
        )
            .chain(),
    );

    // Add sensor update plugin (without visualization)
    app.add_plugins(SensorUpdatePlugin);

    // Physics systems (same as visual mode)
    app.add_systems(
        Update,
        (
            systems::physics_step::physics_step_system,
            systems::sync_visual::apply_external_forces_system,
            systems::sync_visual::apply_external_impulses_system,
            systems::sync_visual::apply_differential_drive_system,
            systems::sync_visual::sync_physics_to_visual_system,
            systems::sync_visual::sync_velocities_from_physics_system,
        )
            .chain()
            .in_set(SimSystemSet::Physics),
    );

    // TF update systems
    app.add_systems(
        Update,
        systems::tf_update::tf_update_system.in_set(SimSystemSet::TF),
    );

    #[cfg(feature = "python")]
    {
        use rl::RLTaskManager;

        // Add RL task manager
        app.init_resource::<RLTaskManager>();

        // Add RL rendering system (for debug gizmos if needed)
        app.add_systems(Update, rl::rl_task_render_system);
    }

    // Setup initial scene (without rendering)
    app.add_systems(Startup, setup_headless_scene);

    info!("Running headless simulation at maximum speed");
    info!("Press Ctrl+C to stop");

    app.run();
}

/// Setup scene for headless mode (no rendering components)
fn setup_headless_scene(
    commands: Commands,
    mut physics_world: ResMut<PhysicsWorld>,
) {
    info!("Setting up headless scene");

    // Create a simple ground plane for physics
    use physics::collider::{ColliderBuilder, ColliderShape};
    use rapier3d::prelude::*;

    // Ground plane
    let ground_rb = RigidBodyBuilder::fixed()
        .translation(vector![0.0, 0.0, 0.0])
        .build();

    let ground_handle = physics_world.rigid_body_set.insert(ground_rb);

    let ground_collider = ColliderBuilder::new(ColliderShape::Box {
        half_extents: Vec3::new(50.0, 0.1, 50.0),
    })
    .friction(0.7)
    .build();

    // Use reborrow pattern to split mutable borrows
    let PhysicsWorld {
        collider_set,
        rigid_body_set,
        ..
    } = &mut *physics_world;

    collider_set.insert_with_parent(
        ground_collider,
        ground_handle,
        rigid_body_set,
    );

    info!("Headless scene setup complete");
}
