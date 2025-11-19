use bevy::prelude::*;
use crate::cli::Cli;
use crate::physics::PhysicsWorld;
use crate::rendering::camera_controller::OrbitCamera;
use crate::scene::loader::SceneLoader;
use crate::scene::spawner::SpawnedObjects;
use crate::tf::TFTree;

pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut physics_world: ResMut<PhysicsWorld>,
    mut spawned_objects: ResMut<SpawnedObjects>,
    mut tf_tree: ResMut<TFTree>,
    cli: Res<Cli>,
) {
    // Always spawn camera with orbit controller
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCamera::default(),
    ));

    // Always spawn directional light (may be overridden by scene)
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
    ));

    // Load world file if provided, otherwise create default ground plane
    if let Some(world_path) = &cli.world {
        info!("Loading world from: {:?}", world_path);
        match SceneLoader::load_scene(
            world_path,
            &mut commands,
            &mut physics_world,
            &mut meshes,
            &mut materials,
            &mut spawned_objects,
            &mut tf_tree,
        ) {
            Ok(loaded_scene) => {
                info!("Successfully loaded scene: {}", loaded_scene.definition.name);
                commands.insert_resource(loaded_scene);
            }
            Err(e) => {
                error!("Failed to load world file: {}", e);
                warn!("Falling back to default empty scene");
                spawn_default_ground(&mut commands, &mut meshes, &mut materials);
            }
        }
    } else {
        info!("No world file specified, creating default ground plane");
        spawn_default_ground(&mut commands, &mut meshes, &mut materials);
    }

    info!("Scene setup complete");
}

fn spawn_default_ground(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
    ));
}
