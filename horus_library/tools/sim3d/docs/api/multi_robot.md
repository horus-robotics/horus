# Multi-Robot API Reference

This document provides a comprehensive reference for Sim3D's multi-robot simulation capabilities, including robot management, communication, and swarm coordination.

## Robot Management

### Robot Component

```rust
#[derive(Component)]
pub struct Robot {
    /// Unique robot identifier
    pub id: RobotId,

    /// Robot name
    pub name: String,

    /// Robot type/model
    pub robot_type: String,

    /// Is robot active?
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RobotId(String);

impl RobotId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Robot {
    pub fn new(name: &str) -> Self {
        Self {
            id: RobotId::new(name),
            name: name.to_string(),
            robot_type: "generic".to_string(),
            active: true,
        }
    }

    pub fn with_type(mut self, robot_type: &str) -> Self {
        self.robot_type = robot_type.to_string();
        self
    }
}
```

### Spawning Multiple Robots

```rust
use sim3d::robot::urdf_loader::URDFLoader;
use sim3d::multi_robot::{Robot, RobotId};

fn spawn_robot_fleet(
    mut commands: Commands,
    mut physics_world: ResMut<PhysicsWorld>,
    mut tf_tree: ResMut<TFTree>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut loader = URDFLoader::new()
        .with_base_path("assets/robots/turtlebot3");

    // Spawn multiple robots in a grid
    let robot_positions = [
        Vec3::new(-3.0, 0.0, -3.0),
        Vec3::new(3.0, 0.0, -3.0),
        Vec3::new(-3.0, 0.0, 3.0),
        Vec3::new(3.0, 0.0, 3.0),
    ];

    for (i, position) in robot_positions.iter().enumerate() {
        let robot_name = format!("robot_{}", i);

        // Create robot entity manually or load URDF
        let robot_entity = commands.spawn((
            Robot::new(&robot_name).with_type("turtlebot3_burger"),
            Transform::from_translation(*position),
            Visibility::default(),
            Name::new(robot_name.clone()),
        )).id();

        // Add physics body
        let rb = RigidBodyBuilder::dynamic()
            .translation(vector![position.x, position.y + 0.1, position.z])
            .build();
        let rb_handle = physics_world.spawn_rigid_body(rb, robot_entity);

        // Add collider
        let collider = ColliderBuilder::new(ColliderShape::Cylinder {
            half_height: 0.07,
            radius: 0.1,
        }).build();
        physics_world.spawn_collider(collider, rb_handle);

        println!("Spawned robot '{}' at {:?}", robot_name, position);
    }
}

// Query robots
fn list_robots(robots: Query<(&Robot, &Transform)>) {
    for (robot, transform) in robots.iter() {
        println!(
            "Robot '{}' ({}): pos={:?}",
            robot.name,
            robot.robot_type,
            transform.translation
        );
    }
}
```

## Communication System

### RobotMessage

```rust
#[derive(Clone, Debug)]
pub struct RobotMessage {
    /// Sender robot ID
    pub from: RobotId,

    /// Receiver robot ID (None for broadcast)
    pub to: Option<RobotId>,

    /// Message payload (raw bytes)
    pub payload: Vec<u8>,

    /// Message timestamp
    pub timestamp: f64,

    /// Unique message ID
    pub id: u64,
}

impl RobotMessage {
    /// Create a new message
    pub fn new(
        from: RobotId,
        to: Option<RobotId>,
        payload: Vec<u8>,
        timestamp: f64,
    ) -> Self;

    /// Check if this is a broadcast message
    pub fn is_broadcast(&self) -> bool {
        self.to.is_none()
    }

    /// Get message size in bytes
    pub fn size_bytes(&self) -> usize {
        self.payload.len()
    }
}
```

### CommunicationManager

```rust
#[derive(Resource)]
pub struct CommunicationManager {
    queues: HashMap<RobotId, VecDeque<RobotMessage>>,
    config: ChannelConfig,
    sent_count: u64,
    received_count: u64,
    bytes_transmitted: usize,
}

#[derive(Clone)]
pub struct ChannelConfig {
    /// Maximum messages in queue per robot
    pub max_queue_size: usize,

    /// Maximum message size in bytes
    pub max_message_size: usize,

    /// Bandwidth limit (bytes per second, None = unlimited)
    pub bandwidth_limit: Option<usize>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_message_size: 1024 * 1024,  // 1MB
            bandwidth_limit: None,
        }
    }
}

impl CommunicationManager {
    /// Create new manager with config
    pub fn new(config: ChannelConfig) -> Self;

    /// Register a robot (creates message queue)
    pub fn register_robot(&mut self, robot_id: RobotId);

    /// Unregister a robot
    pub fn unregister_robot(&mut self, robot_id: &RobotId);

    /// Send a message
    pub fn send_message(
        &mut self,
        from: RobotId,
        to: Option<RobotId>,  // None for broadcast
        payload: Vec<u8>,
        timestamp: f64,
    ) -> Result<(), String>;

    /// Receive next message for a robot
    pub fn receive_message(&mut self, robot_id: &RobotId) -> Option<RobotMessage>;

    /// Peek at next message without removing
    pub fn peek_message(&self, robot_id: &RobotId) -> Option<&RobotMessage>;

    /// Get number of pending messages
    pub fn pending_count(&self, robot_id: &RobotId) -> usize;

    /// Clear all messages for a robot
    pub fn clear_queue(&mut self, robot_id: &RobotId);

    /// Get communication statistics
    pub fn get_stats(&self) -> CommunicationStats;
}

#[derive(Debug, Clone, Copy)]
pub struct CommunicationStats {
    pub sent_count: u64,
    pub received_count: u64,
    pub bytes_transmitted: usize,
    pub active_robots: usize,
}
```

### Usage Example

```rust
use sim3d::multi_robot::communication::{CommunicationManager, ChannelConfig};
use serde::{Serialize, Deserialize};

// Define a typed message
#[derive(Serialize, Deserialize)]
struct PositionMessage {
    x: f32,
    y: f32,
    heading: f32,
}

fn setup_communication(mut commands: Commands) {
    // Insert communication manager as resource
    commands.insert_resource(CommunicationManager::new(ChannelConfig {
        max_queue_size: 100,
        max_message_size: 4096,
        bandwidth_limit: Some(1_000_000),  // 1 MB/s
    }));
}

fn robot_broadcast_position(
    mut comm: ResMut<CommunicationManager>,
    robots: Query<(&Robot, &Transform)>,
    time: Res<Time>,
) {
    for (robot, transform) in robots.iter() {
        // Create position message
        let msg = PositionMessage {
            x: transform.translation.x,
            y: transform.translation.z,
            heading: transform.rotation.to_euler(EulerRot::YXZ).0,
        };

        // Serialize to bytes
        let payload = bincode::serialize(&msg).unwrap();

        // Broadcast to all robots
        if let Err(e) = comm.send_message(
            robot.id.clone(),
            None,  // Broadcast
            payload,
            time.elapsed_secs_f64(),
        ) {
            eprintln!("Failed to send message: {}", e);
        }
    }
}

fn robot_receive_messages(
    mut comm: ResMut<CommunicationManager>,
    robots: Query<&Robot>,
) {
    for robot in robots.iter() {
        while let Some(msg) = comm.receive_message(&robot.id) {
            // Deserialize message
            if let Ok(pos_msg) = bincode::deserialize::<PositionMessage>(&msg.payload) {
                println!(
                    "{} received position from {}: ({:.2}, {:.2})",
                    robot.name,
                    msg.from.as_str(),
                    pos_msg.x,
                    pos_msg.y
                );
            }
        }
    }
}
```

## Network Simulation

Realistic network conditions with latency, packet loss, and bandwidth limits.

### NetworkConfig

```rust
#[derive(Clone)]
pub struct NetworkConfig {
    /// Base latency in seconds
    pub base_latency: f64,

    /// Latency variance (std dev)
    pub latency_variance: f64,

    /// Packet loss probability (0.0 - 1.0)
    pub packet_loss_rate: f64,

    /// Bandwidth limit (bytes/second, None = unlimited)
    pub bandwidth_limit: Option<usize>,

    /// Maximum jitter in seconds
    pub jitter: f64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            base_latency: 0.01,       // 10ms
            latency_variance: 0.002,  // 2ms std dev
            packet_loss_rate: 0.0,
            bandwidth_limit: None,
            jitter: 0.005,            // 5ms
        }
    }
}

impl NetworkConfig {
    /// WiFi network preset
    pub fn wifi() -> Self {
        Self {
            base_latency: 0.005,
            latency_variance: 0.003,
            packet_loss_rate: 0.01,
            bandwidth_limit: Some(54_000_000 / 8),  // 54 Mbps
            jitter: 0.010,
        }
    }

    /// 4G/LTE network preset
    pub fn lte() -> Self {
        Self {
            base_latency: 0.050,
            latency_variance: 0.020,
            packet_loss_rate: 0.02,
            bandwidth_limit: Some(100_000_000 / 8),  // 100 Mbps
            jitter: 0.030,
        }
    }

    /// Local wired network preset
    pub fn wired() -> Self {
        Self {
            base_latency: 0.001,
            latency_variance: 0.0001,
            packet_loss_rate: 0.0001,
            bandwidth_limit: Some(1_000_000_000 / 8),  // 1 Gbps
            jitter: 0.001,
        }
    }

    /// Unreliable network for stress testing
    pub fn unreliable() -> Self {
        Self {
            base_latency: 0.100,
            latency_variance: 0.050,
            packet_loss_rate: 0.10,
            bandwidth_limit: Some(1_000_000 / 8),  // 1 Mbps
            jitter: 0.100,
        }
    }
}
```

### NetworkSimulator

```rust
#[derive(Resource)]
pub struct NetworkSimulator {
    packets: VecDeque<NetworkPacket>,
    config: NetworkConfig,
    current_time: f64,
    bytes_this_second: usize,
    last_bandwidth_reset: f64,
    packets_sent: u64,
    packets_dropped: u64,
    packets_delivered: u64,
}

impl NetworkSimulator {
    pub fn new(config: NetworkConfig) -> Self;

    /// Send message through network
    pub fn send(&mut self, message: RobotMessage) -> Result<(), String>;

    /// Get messages ready for delivery
    pub fn receive(&mut self) -> Vec<RobotMessage>;

    /// Update simulation time
    pub fn update_time(&mut self, delta: f64);

    /// Get pending packet count
    pub fn pending_count(&self) -> usize;

    /// Get network statistics
    pub fn get_stats(&self) -> NetworkStats;

    /// Update network configuration
    pub fn set_config(&mut self, config: NetworkConfig);
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkStats {
    pub packets_sent: u64,
    pub packets_dropped: u64,
    pub packets_delivered: u64,
    pub pending_packets: usize,
}

impl NetworkStats {
    pub fn packet_loss_rate(&self) -> f64;
    pub fn delivery_rate(&self) -> f64;
}
```

### Usage Example

```rust
fn setup_network(mut commands: Commands) {
    // Use WiFi network simulation
    commands.insert_resource(NetworkSimulator::new(NetworkConfig::wifi()));
}

fn network_step_system(
    mut network: ResMut<NetworkSimulator>,
    mut comm: ResMut<CommunicationManager>,
    time: Res<Time>,
) {
    // Update network time
    network.update_time(time.delta_secs_f64());

    // Deliver packets that have arrived
    let messages = network.receive();
    for message in messages {
        if let Some(recipient) = &message.to {
            let _ = comm.send_message(
                message.from,
                Some(recipient.clone()),
                message.payload,
                message.timestamp,
            );
        }
    }
}
```

## Swarm Coordination

### SwarmAgent

Implements Reynolds flocking rules (separation, alignment, cohesion).

```rust
#[derive(Component)]
pub struct SwarmAgent {
    /// Desired separation from neighbors
    pub separation_distance: f32,

    /// Alignment weight (match neighbor velocity)
    pub alignment_weight: f32,

    /// Cohesion weight (move toward center)
    pub cohesion_weight: f32,

    /// Separation weight (avoid neighbors)
    pub separation_weight: f32,

    /// Maximum speed
    pub max_speed: f32,

    /// Perception radius for neighbors
    pub perception_radius: f32,
}

impl Default for SwarmAgent {
    fn default() -> Self {
        Self {
            separation_distance: 2.0,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            separation_weight: 1.5,
            max_speed: 2.0,
            perception_radius: 5.0,
        }
    }
}
```

### Formation Control

```rust
#[derive(Component)]
pub struct FormationController {
    /// Formation type
    pub formation_type: FormationType,

    /// Position index in formation
    pub formation_index: usize,

    /// Formation scale
    pub scale: f32,

    /// Leader to follow
    pub leader: Option<RobotId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormationType {
    /// Line formation
    Line,
    /// Circle formation
    Circle,
    /// Grid formation
    Grid,
    /// V formation (wedge)
    Wedge,
    /// Custom formation
    Custom,
}

impl FormationController {
    pub fn new(formation_type: FormationType, index: usize) -> Self;

    /// Get target position in formation relative to leader
    pub fn get_formation_position(&self, leader_transform: &Transform) -> Vec3;
}
```

### Consensus Algorithm

```rust
#[derive(Resource, Default)]
pub struct ConsensusState {
    values: HashMap<RobotId, f32>,
    pub convergence_threshold: f32,
}

impl ConsensusState {
    pub fn new(threshold: f32) -> Self;

    /// Set robot's value
    pub fn set_value(&mut self, robot_id: RobotId, value: f32);

    /// Get robot's value
    pub fn get_value(&self, robot_id: &RobotId) -> Option<f32>;

    /// Get average value across all robots
    pub fn average(&self) -> f32;

    /// Check if consensus reached
    pub fn is_converged(&self) -> bool;

    /// Get variance of values
    pub fn variance(&self) -> f32;
}
```

### Task Allocation

```rust
#[derive(Resource, Default)]
pub struct TaskAllocation {
    assignments: HashMap<RobotId, String>,
    costs: HashMap<(RobotId, String), f32>,
}

impl TaskAllocation {
    pub fn new() -> Self;

    /// Set cost for robot to complete task
    pub fn set_cost(&mut self, robot_id: RobotId, task_id: String, cost: f32);

    /// Assign task to robot
    pub fn assign(&mut self, robot_id: RobotId, task_id: String);

    /// Get assigned task
    pub fn get_assignment(&self, robot_id: &RobotId) -> Option<&String>;

    /// Greedy task allocation
    pub fn allocate_greedy(&mut self, robots: &[RobotId], tasks: &[String]);

    /// Clear all assignments
    pub fn clear(&mut self);
}
```

### Usage Example: Swarm Formation

```rust
fn setup_swarm(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut physics_world: ResMut<PhysicsWorld>,
) {
    // Spawn leader
    let leader_entity = commands.spawn((
        Robot::new("leader"),
        Mesh3d(meshes.add(Sphere::new(0.3).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.3, 0.0),
    )).id();

    // Spawn followers in formation
    for i in 0..8 {
        let follower_entity = commands.spawn((
            Robot::new(&format!("follower_{}", i)),
            SwarmAgent {
                separation_distance: 1.5,
                alignment_weight: 0.5,
                cohesion_weight: 1.0,
                separation_weight: 2.0,
                max_speed: 3.0,
                perception_radius: 8.0,
            },
            FormationController {
                formation_type: FormationType::Circle,
                formation_index: i,
                scale: 1.0,
                leader: Some(RobotId::new("leader")),
            },
            Mesh3d(meshes.add(Sphere::new(0.2).mesh().ico(3).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.5, 1.0),
                ..default()
            })),
            Transform::from_xyz(
                (i as f32 * 0.8).cos() * 3.0,
                0.2,
                (i as f32 * 0.8).sin() * 3.0,
            ),
        )).id();
    }
}

fn swarm_system(
    mut followers: Query<
        (&mut Transform, &SwarmAgent, &FormationController, &Robot),
        Without<Camera3d>,
    >,
    leaders: Query<(&Transform, &Robot), Without<SwarmAgent>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut transform, agent, formation, robot) in followers.iter_mut() {
        // Find leader
        if let Some(leader_id) = &formation.leader {
            if let Some((leader_transform, _)) = leaders.iter()
                .find(|(_, r)| &r.id == leader_id)
            {
                // Get target formation position
                let target = formation.get_formation_position(leader_transform);

                // Move toward target
                let direction = target - transform.translation;
                let distance = direction.length();

                if distance > 0.1 {
                    let velocity = direction.normalize() * agent.max_speed.min(distance * 2.0);
                    transform.translation += velocity * dt;

                    // Face movement direction
                    if velocity.length() > 0.01 {
                        let forward = velocity.normalize();
                        transform.rotation = Quat::from_rotation_arc(Vec3::Z, forward);
                    }
                }
            }
        }
    }
}
```

### Consensus Example

```rust
fn consensus_system(
    mut consensus: ResMut<ConsensusState>,
    robots: Query<(&Robot, &Transform)>,
    mut comm: ResMut<CommunicationManager>,
    time: Res<Time>,
) {
    // Update each robot's value (e.g., estimated target position)
    for (robot, transform) in robots.iter() {
        let estimated_value = transform.translation.x;  // Simple example
        consensus.set_value(robot.id.clone(), estimated_value);
    }

    // Check convergence
    if consensus.is_converged() {
        println!(
            "Consensus reached! Average value: {:.3}, Variance: {:.6}",
            consensus.average(),
            consensus.variance()
        );
    } else {
        // Exchange values with neighbors (via communication)
        let avg = consensus.average();

        // Simple consensus update: move toward average
        for (robot, _) in robots.iter() {
            if let Some(current) = consensus.get_value(&robot.id) {
                let updated = current * 0.9 + avg * 0.1;
                consensus.set_value(robot.id.clone(), updated);
            }
        }
    }
}
```

## Complete Multi-Robot Example

```rust
use bevy::prelude::*;
use sim3d::physics::PhysicsWorld;
use sim3d::multi_robot::{
    Robot, RobotId,
    communication::{CommunicationManager, ChannelConfig},
    coordination::{SwarmAgent, FormationType, FormationController},
    network::{NetworkSimulator, NetworkConfig},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<PhysicsWorld>()
        .insert_resource(CommunicationManager::new(ChannelConfig::default()))
        .insert_resource(NetworkSimulator::new(NetworkConfig::wifi()))
        .add_systems(Startup, setup_multi_robot)
        .add_systems(Update, (
            physics_step_system,
            network_step_system,
            swarm_coordination_system,
            formation_control_system,
            robot_communication_system,
            sync_transforms_system,
        ))
        .run();
}
```
