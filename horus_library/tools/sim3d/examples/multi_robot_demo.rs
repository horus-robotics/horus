// Multi-robot simulation example
//
// This example demonstrates:
// - Managing multiple robots
// - Inter-robot communication
// - Swarm coordination
// - Formation control
// - Network simulation
// - Lock-step synchronization

use bevy::prelude::*;
use sim3d::multi_robot::{
    communication::CommunicationManager,
    coordination::{FormationController, FormationType, SwarmAgent},
    network::{NetworkConfig, NetworkSimulator},
    registry::{RobotCapability, RobotMetadata, RobotRegistry},
    sync::{SyncMode, SynchronizationManager},
    MultiRobotPlugin, Robot, RobotId,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MultiRobotPlugin { max_robots: 20 },
        ))
        .add_systems(Startup, setup_multi_robot_scene)
        .add_systems(Update, (robot_communication_example, print_stats))
        .run();
}

fn setup_multi_robot_scene(
    mut commands: Commands,
    mut registry: ResMut<RobotRegistry>,
    mut sync_manager: ResMut<SynchronizationManager>,
    mut network: ResMut<NetworkSimulator>,
) {
    // Configure network for WiFi-like conditions
    network.set_config(NetworkConfig::wifi());

    // Set lock-step synchronization at 30 Hz
    sync_manager.set_mode(SyncMode::LockStep);
    sync_manager.fixed_timestep = 1.0 / 30.0;

    // Create a leader robot
    let leader_id = RobotId::new("leader");
    commands.spawn((
        Robot::new(leader_id.clone(), "turtlebot3"),
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
    ));

    // Register leader metadata
    let leader_metadata = RobotMetadata::new(leader_id.clone(), "turtlebot3")
        .with_capability(RobotCapability::Mobile)
        .with_capability(RobotCapability::Vision)
        .with_speed(0.5)
        .with_custom("role", "leader");
    registry.register(leader_metadata);

    // Create follower robots in a swarm
    for i in 0..5 {
        let follower_id = RobotId::new(format!("follower_{}", i));

        // Followers with swarm behavior
        commands.spawn((
            Robot::new(follower_id.clone(), "turtlebot3"),
            Transform::from_xyz(i as f32 * 2.0, 0.0, 2.0),
            GlobalTransform::default(),
            SwarmAgent {
                separation_distance: 1.5,
                alignment_weight: 1.0,
                cohesion_weight: 1.0,
                separation_weight: 1.5,
                max_speed: 1.0,
                perception_radius: 3.0,
            },
        ));

        let follower_metadata = RobotMetadata::new(follower_id.clone(), "turtlebot3")
            .with_capability(RobotCapability::Mobile)
            .with_speed(0.5)
            .with_custom("role", "follower");
        registry.register(follower_metadata);

        sync_manager.register_robot(follower_id);
    }

    // Create robots in formation
    for i in 0..4 {
        let formation_id = RobotId::new(format!("formation_{}", i));

        commands.spawn((
            Robot::new(formation_id.clone(), "turtlebot3"),
            Transform::from_xyz(i as f32, 0.0, -3.0),
            GlobalTransform::default(),
            FormationController {
                formation_type: FormationType::Wedge,
                formation_index: i,
                scale: 1.0,
                leader: Some(leader_id.clone()),
            },
        ));

        let formation_metadata = RobotMetadata::new(formation_id.clone(), "turtlebot3")
            .with_capability(RobotCapability::Mobile)
            .with_speed(0.5)
            .with_custom("role", "formation");
        registry.register(formation_metadata);
    }

    // Add camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 15.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add lighting
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
    ));
}

fn robot_communication_example(
    robots: Query<&Robot>,
    mut comm_manager: ResMut<CommunicationManager>,
    time: Res<Time>,
) {
    // Send periodic broadcast messages
    if time.elapsed_secs() % 2.0 < 0.01 {
        for robot in robots.iter() {
            if robot.id.as_str() == "leader" {
                let message = format!("Status update at {:.2}s", time.elapsed_secs());
                let _ = comm_manager.send_message(
                    robot.id.clone(),
                    None, // Broadcast
                    message.as_bytes().to_vec(),
                    time.elapsed_secs_f64(),
                );
            }
        }
    }

    // Process received messages
    for robot in robots.iter() {
        while let Some(message) = comm_manager.receive_message(&robot.id) {
            if let Ok(text) = String::from_utf8(message.payload.clone()) {
                info!(
                    "Robot {} received from {}: {}",
                    robot.id.as_str(),
                    message.from.as_str(),
                    text
                );
            }
        }
    }
}

fn print_stats(
    comm_manager: Res<CommunicationManager>,
    network: Res<NetworkSimulator>,
    sync_manager: Res<SynchronizationManager>,
    registry: Res<RobotRegistry>,
    time: Res<Time>,
) {
    // Print stats every 5 seconds
    if time.elapsed_secs() % 5.0 < 0.01 {
        let comm_stats = comm_manager.get_stats();
        let net_stats = network.get_stats();

        info!("=== Multi-Robot Statistics ===");
        info!("Registered robots: {}", registry.count());
        info!("Sync step: {}", sync_manager.current_step());
        info!(
            "Communication: {} sent, {} received",
            comm_stats.sent_count, comm_stats.received_count
        );
        info!(
            "Network: {:.1}% packet loss",
            net_stats.packet_loss_rate() * 100.0
        );

        // Query robots by capability
        let mobile_robots = registry.find_by_capability(&RobotCapability::Mobile);
        info!("Mobile robots: {}", mobile_robots.len());
    }
}
