//! # 2D Joint System for Articulated Robots
//!
//! This module provides support for articulated robots with joints in sim2d,
//! enabling simulation of robotic arms, humanoid robots, and multi-link mechanisms.
//!
//! ## Supported Joint Types
//! - **Revolute**: Rotation around a point (elbows, knees, shoulders)
//! - **Prismatic**: Linear sliding motion (telescoping, linear actuators)
//! - **Fixed**: Rigid connection between links
//!
//! ## Example Configuration (YAML)
//! ```yaml
//! name: "arm_2dof"
//! topic_prefix: "/arm"
//! position: [0.0, 0.0]
//! fixed_base: true
//!
//! links:
//!   - name: "base"
//!     size: [0.2, 0.2]
//!     mass: 5.0
//!   - name: "upper_arm"
//!     size: [0.4, 0.08]
//!     mass: 1.0
//!
//! joints:
//!   - name: "shoulder"
//!     parent: "base"
//!     child: "upper_arm"
//!     joint_type:
//!       Revolute:
//!         limits: [-1.57, 1.57]
//! ```

use bevy::prelude::*;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Joint Motor Configuration
// ============================================================================

/// Motor configuration for actuated joints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JointMotor {
    /// Target position (radians for revolute, meters for prismatic)
    #[serde(default)]
    pub target_position: f32,
    /// Target velocity (rad/s or m/s)
    #[serde(default)]
    pub target_velocity: f32,
    /// Motor stiffness (PD controller proportional gain)
    #[serde(default = "default_motor_stiffness")]
    pub stiffness: f32,
    /// Motor damping (PD controller derivative gain)
    #[serde(default = "default_motor_damping")]
    pub damping: f32,
    /// Maximum force/torque the motor can apply
    #[serde(default = "default_motor_max_force")]
    pub max_force: f32,
}

fn default_motor_stiffness() -> f32 {
    100.0
}
fn default_motor_damping() -> f32 {
    10.0
}
fn default_motor_max_force() -> f32 {
    50.0
}

impl Default for JointMotor {
    fn default() -> Self {
        Self {
            target_position: 0.0,
            target_velocity: 0.0,
            stiffness: default_motor_stiffness(),
            damping: default_motor_damping(),
            max_force: default_motor_max_force(),
        }
    }
}

// ============================================================================
// Joint Types
// ============================================================================

/// Types of joints supported in 2D
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Joint2DType {
    /// Revolute joint - allows rotation around a point
    /// Most common joint type for arms, legs, etc.
    Revolute {
        /// Joint angle limits in radians [min, max]
        #[serde(default)]
        limits: Option<[f32; 2]>,
        /// Motor configuration for position/velocity control
        #[serde(default)]
        motor: Option<JointMotor>,
    },
    /// Prismatic joint - allows linear sliding motion
    /// Used for telescoping arms, linear actuators
    Prismatic {
        /// Axis of motion (will be normalized)
        #[serde(default = "default_prismatic_axis")]
        axis: [f32; 2],
        /// Travel limits in meters [min, max]
        #[serde(default)]
        limits: Option<[f32; 2]>,
        /// Motor configuration
        #[serde(default)]
        motor: Option<JointMotor>,
    },
    /// Fixed joint - rigidly connects two links
    Fixed,
}

fn default_prismatic_axis() -> [f32; 2] {
    [1.0, 0.0]
}

impl Default for Joint2DType {
    fn default() -> Self {
        Joint2DType::Revolute {
            limits: None,
            motor: None,
        }
    }
}

// ============================================================================
// Link Configuration
// ============================================================================

/// Visual shape for a link
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum LinkShape {
    /// Rectangular link (default)
    #[default]
    Rectangle,
    /// Capsule shape (rounded ends)
    Capsule,
    /// Circular shape
    Circle {
        /// Radius in meters
        radius: f32,
    },
}

/// Configuration for a single link (rigid body segment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link2D {
    /// Unique name for this link
    pub name: String,
    /// Link dimensions [length, width] in meters
    #[serde(default = "default_link_size")]
    pub size: [f32; 2],
    /// Mass in kg
    #[serde(default = "default_link_mass")]
    pub mass: f32,
    /// Color [r, g, b] (0.0-1.0)
    #[serde(default = "default_link_color")]
    pub color: [f32; 3],
    /// Visual/collision shape
    #[serde(default)]
    pub shape: LinkShape,
    /// Local center of mass offset from geometric center
    #[serde(default)]
    pub com_offset: [f32; 2],
    /// Collision group (links in same group don't collide with each other)
    #[serde(default)]
    pub collision_group: Option<u32>,
}

fn default_link_size() -> [f32; 2] {
    [0.3, 0.1]
}
fn default_link_mass() -> f32 {
    1.0
}
fn default_link_color() -> [f32; 3] {
    [0.4, 0.4, 0.8]
}

impl Default for Link2D {
    fn default() -> Self {
        Self {
            name: "link".to_string(),
            size: default_link_size(),
            mass: default_link_mass(),
            color: default_link_color(),
            shape: LinkShape::Rectangle,
            com_offset: [0.0, 0.0],
            collision_group: None,
        }
    }
}

// ============================================================================
// Joint Configuration
// ============================================================================

/// Configuration for a joint connecting two links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Joint2D {
    /// Unique name for this joint
    pub name: String,
    /// Parent link name (or "world" for base joints fixed to ground)
    pub parent: String,
    /// Child link name
    pub child: String,
    /// Joint type and parameters
    #[serde(default)]
    pub joint_type: Joint2DType,
    /// Anchor point on parent link (local coordinates)
    #[serde(default)]
    pub parent_anchor: [f32; 2],
    /// Anchor point on child link (local coordinates)
    #[serde(default)]
    pub child_anchor: [f32; 2],
    /// Initial joint position (angle for revolute, distance for prismatic)
    #[serde(default)]
    pub initial_position: f32,
}

impl Default for Joint2D {
    fn default() -> Self {
        Self {
            name: "joint".to_string(),
            parent: "world".to_string(),
            child: "link".to_string(),
            joint_type: Joint2DType::default(),
            parent_anchor: [0.0, 0.0],
            child_anchor: [0.0, 0.0],
            initial_position: 0.0,
        }
    }
}

// ============================================================================
// Articulated Robot Configuration
// ============================================================================

/// Type hint for the articulated robot (affects UI and defaults)
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ArticulatedRobotType {
    /// Generic multi-link robot
    #[default]
    Generic,
    /// Robotic arm
    Arm,
    /// Humanoid robot (side-view)
    Humanoid,
    /// Spider/multi-legged robot
    Spider,
    /// Snake robot
    Snake,
}

/// Complete configuration for an articulated (multi-link) robot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticulatedRobotConfig {
    /// Robot name
    pub name: String,
    /// Topic prefix for HORUS communication
    #[serde(default = "default_articulated_topic")]
    pub topic_prefix: String,
    /// Base position in world [x, y] meters
    #[serde(default)]
    pub position: [f32; 2],
    /// Base orientation in radians
    #[serde(default)]
    pub orientation: f32,
    /// All links in this robot
    pub links: Vec<Link2D>,
    /// All joints connecting links
    pub joints: Vec<Joint2D>,
    /// Whether the base link is fixed to the world
    #[serde(default)]
    pub fixed_base: bool,
    /// Robot type hint for UI
    #[serde(default)]
    pub robot_type: ArticulatedRobotType,
    /// Enable gravity for this robot (for side-view humanoid simulation)
    #[serde(default)]
    pub enable_gravity: bool,
    /// Self-collision enabled between links
    #[serde(default = "default_self_collision")]
    pub self_collision: bool,
}

fn default_articulated_topic() -> String {
    "/articulated".to_string()
}

fn default_self_collision() -> bool {
    false
}

impl Default for ArticulatedRobotConfig {
    fn default() -> Self {
        Self {
            name: "articulated_robot".to_string(),
            topic_prefix: default_articulated_topic(),
            position: [0.0, 0.0],
            orientation: 0.0,
            links: vec![Link2D::default()],
            joints: vec![],
            fixed_base: true,
            robot_type: ArticulatedRobotType::Generic,
            enable_gravity: false,
            self_collision: false,
        }
    }
}

impl ArticulatedRobotConfig {
    /// Load articulated robot configuration from a YAML file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Check that all links have unique names
        let mut link_names: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for link in &self.links {
            if !link_names.insert(&link.name) {
                return Err(format!("Duplicate link name: {}", link.name));
            }
        }

        // Check that all joints reference valid links
        for joint in &self.joints {
            if joint.parent != "world" && !link_names.contains(joint.parent.as_str()) {
                return Err(format!(
                    "Joint '{}' references unknown parent link '{}'",
                    joint.name, joint.parent
                ));
            }
            if !link_names.contains(joint.child.as_str()) {
                return Err(format!(
                    "Joint '{}' references unknown child link '{}'",
                    joint.name, joint.child
                ));
            }
        }

        // Check for cycles in the kinematic chain (simplified check)
        // A proper implementation would use topological sort

        Ok(())
    }

    /// Get the base link (link with no parent joint)
    pub fn get_base_link(&self) -> Option<&Link2D> {
        let child_links: std::collections::HashSet<&str> =
            self.joints.iter().map(|j| j.child.as_str()).collect();

        self.links
            .iter()
            .find(|link| !child_links.contains(link.name.as_str()))
    }

    /// Get ordered list of links (topological order from base to end effectors)
    pub fn get_links_ordered(&self) -> Vec<&Link2D> {
        // Build parent-child map
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        for joint in &self.joints {
            children
                .entry(joint.parent.as_str())
                .or_default()
                .push(&joint.child);
        }

        // Find root links (those not appearing as children)
        let child_set: std::collections::HashSet<&str> =
            self.joints.iter().map(|j| j.child.as_str()).collect();
        let roots: Vec<&str> = self
            .links
            .iter()
            .filter(|l| !child_set.contains(l.name.as_str()))
            .map(|l| l.name.as_str())
            .collect();

        // BFS from roots
        let mut ordered = Vec::new();
        let mut queue: std::collections::VecDeque<&str> = roots.into_iter().collect();
        let link_map: HashMap<&str, &Link2D> =
            self.links.iter().map(|l| (l.name.as_str(), l)).collect();

        while let Some(name) = queue.pop_front() {
            if let Some(link) = link_map.get(name) {
                ordered.push(*link);
            }
            if let Some(kids) = children.get(name) {
                for child in kids {
                    queue.push_back(child);
                }
            }
        }

        ordered
    }
}

// ============================================================================
// Bevy ECS Components
// ============================================================================

/// Component marking an articulated robot entity
#[derive(Component)]
pub struct ArticulatedRobot {
    /// Robot name
    pub name: String,
    /// Full configuration
    pub config: ArticulatedRobotConfig,
    /// Map from link name to Rapier rigid body handle
    pub link_handles: HashMap<String, RigidBodyHandle>,
    /// Map from joint name to Rapier impulse joint handle
    pub joint_handles: HashMap<String, ImpulseJointHandle>,
    /// Current joint positions (radians or meters)
    pub joint_positions: HashMap<String, f32>,
    /// Current joint velocities
    pub joint_velocities: HashMap<String, f32>,
}

/// Component for individual link entities (for visual rendering)
#[derive(Component)]
pub struct RobotLink {
    /// Parent robot name
    pub robot_name: String,
    /// This link's name
    pub link_name: String,
    /// Rapier rigid body handle
    pub rigid_body_handle: RigidBodyHandle,
}

/// Component for joint visualization markers
#[derive(Component)]
pub struct JointMarker {
    /// Parent robot name
    pub robot_name: String,
    /// Joint name
    pub joint_name: String,
}

/// Component for end effector tracking
#[derive(Component)]
pub struct EndEffector {
    /// Parent robot name
    pub robot_name: String,
    /// Link name this end effector is attached to
    pub link_name: String,
}

// ============================================================================
// Joint State Messages (for HORUS communication)
// ============================================================================

/// Joint state message - published by sim2d
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointState {
    /// Joint names
    pub name: Vec<String>,
    /// Current positions (radians for revolute, meters for prismatic)
    pub position: Vec<f32>,
    /// Current velocities
    pub velocity: Vec<f32>,
    /// Current efforts (torques/forces)
    pub effort: Vec<f32>,
}

/// Control mode for joint commands
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum JointControlMode {
    /// Position control (PD controller to target position)
    #[default]
    Position,
    /// Velocity control
    Velocity,
    /// Direct effort (torque/force) control
    Effort,
}

/// Joint command message - received by sim2d
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointCommand {
    /// Target joint names (empty = all joints)
    pub name: Vec<String>,
    /// Target positions (for Position mode)
    pub position: Vec<f32>,
    /// Target velocities (for Velocity mode)
    pub velocity: Vec<f32>,
    /// Direct efforts (for Effort mode)
    pub effort: Vec<f32>,
    /// Control mode
    #[serde(default)]
    pub mode: JointControlMode,
}

// ============================================================================
// Spawning Functions
// ============================================================================

/// Spawn an articulated robot into the physics world
pub fn spawn_articulated_robot(
    commands: &mut Commands,
    physics_world: &mut crate::PhysicsWorld,
    config: &ArticulatedRobotConfig,
    scale: f32,
) -> Entity {
    let mut link_handles: HashMap<String, RigidBodyHandle> = HashMap::new();
    let mut joint_handles: HashMap<String, ImpulseJointHandle> = HashMap::new();
    let mut joint_positions: HashMap<String, f32> = HashMap::new();
    let mut joint_velocities: HashMap<String, f32> = HashMap::new();

    // Determine collision groups for self-collision avoidance
    let robot_collision_group = if config.self_collision {
        // Each link gets its own group - they can collide
        InteractionGroups::all()
    } else {
        // All links in same group - they don't collide with each other
        // Group 0 is used for the robot, filter excludes group 0
        InteractionGroups::new(Group::GROUP_1, Group::ALL - Group::GROUP_1)
    };

    // Build link name -> position map for initial placement
    let mut link_positions: HashMap<String, (f32, f32, f32)> = HashMap::new();

    // Calculate initial positions using forward kinematics
    let ordered_links = config.get_links_ordered();
    for link in &ordered_links {
        let pos = calculate_link_initial_position(config, &link.name, &link_positions);
        link_positions.insert(link.name.clone(), pos);
    }

    // 1. Spawn all links as rigid bodies
    for link in &config.links {
        let (x, y, angle) = link_positions
            .get(&link.name)
            .copied()
            .unwrap_or((config.position[0], config.position[1], config.orientation));

        // Determine if this is the base link
        let is_base = config
            .joints
            .iter()
            .all(|j| j.child != link.name || j.parent == "world");
        let is_fixed_to_world = is_base && config.fixed_base;

        // Create rigid body
        let rigid_body = if is_fixed_to_world {
            RigidBodyBuilder::fixed()
                .translation(vector![x, y])
                .rotation(angle)
                .build()
        } else {
            RigidBodyBuilder::dynamic()
                .translation(vector![x, y])
                .rotation(angle)
                .linear_damping(0.5)
                .angular_damping(0.5)
                .build()
        };

        // Create collider based on shape
        let collider = match &link.shape {
            LinkShape::Rectangle => {
                ColliderBuilder::cuboid(link.size[0] / 2.0, link.size[1] / 2.0)
                    .density(link.mass / (link.size[0] * link.size[1]))
                    .collision_groups(robot_collision_group)
                    .build()
            }
            LinkShape::Capsule => {
                let half_length = (link.size[0] - link.size[1]) / 2.0;
                let radius = link.size[1] / 2.0;
                ColliderBuilder::capsule_x(half_length.max(0.01), radius)
                    .density(link.mass / (std::f32::consts::PI * radius * radius + link.size[0] * link.size[1]))
                    .collision_groups(robot_collision_group)
                    .build()
            }
            LinkShape::Circle { radius } => ColliderBuilder::ball(*radius)
                .density(link.mass / (std::f32::consts::PI * radius * radius))
                .collision_groups(robot_collision_group)
                .build(),
        };

        let handle = physics_world.rigid_body_set.insert(rigid_body);
        physics_world.collider_set.insert_with_parent(
            collider,
            handle,
            &mut physics_world.rigid_body_set,
        );

        link_handles.insert(link.name.clone(), handle);

        // Spawn visual entity for this link
        let link_color = Color::srgb(link.color[0], link.color[1], link.color[2]);
        let visual_size = Vec2::new(link.size[0] * scale, link.size[1] * scale);

        commands.spawn((
            Sprite {
                color: link_color,
                custom_size: Some(visual_size),
                ..default()
            },
            Transform::from_translation(Vec3::new(x * scale, y * scale, 1.0))
                .with_rotation(Quat::from_rotation_z(angle)),
            RobotLink {
                robot_name: config.name.clone(),
                link_name: link.name.clone(),
                rigid_body_handle: handle,
            },
        ));
    }

    // 2. Create joints between links
    for joint_cfg in &config.joints {
        let child_handle = match link_handles.get(&joint_cfg.child) {
            Some(h) => *h,
            None => {
                tracing::warn!(
                    "Joint '{}' references unknown child link '{}'",
                    joint_cfg.name,
                    joint_cfg.child
                );
                continue;
            }
        };

        let parent_anchor = point![joint_cfg.parent_anchor[0], joint_cfg.parent_anchor[1]];
        let child_anchor = point![joint_cfg.child_anchor[0], joint_cfg.child_anchor[1]];

        // Build the appropriate joint type
        let joint_handle = match &joint_cfg.joint_type {
            Joint2DType::Revolute { limits, motor } => {
                let mut builder = RevoluteJointBuilder::new()
                    .local_anchor1(parent_anchor)
                    .local_anchor2(child_anchor);

                if let Some([min, max]) = limits {
                    builder = builder.limits([*min, *max]);
                }

                if let Some(m) = motor {
                    builder = builder
                        .motor_position(m.target_position, m.stiffness, m.damping)
                        .motor_max_force(m.max_force);
                }

                let joint = builder.build();

                if joint_cfg.parent == "world" {
                    // Fixed to world - create a fixed body at the anchor point
                    let world_body = RigidBodyBuilder::fixed()
                        .translation(vector![
                            config.position[0] + joint_cfg.parent_anchor[0],
                            config.position[1] + joint_cfg.parent_anchor[1]
                        ])
                        .build();
                    let world_handle = physics_world.rigid_body_set.insert(world_body);
                    physics_world
                        .impulse_joint_set
                        .insert(world_handle, child_handle, joint, true)
                } else if let Some(parent_handle) = link_handles.get(&joint_cfg.parent) {
                    physics_world
                        .impulse_joint_set
                        .insert(*parent_handle, child_handle, joint, true)
                } else {
                    tracing::warn!(
                        "Joint '{}' references unknown parent link '{}'",
                        joint_cfg.name,
                        joint_cfg.parent
                    );
                    continue;
                }
            }
            Joint2DType::Prismatic { axis, limits, motor } => {
                let axis_vec = UnitVector::new_normalize(vector![axis[0], axis[1]]);
                let mut builder = PrismaticJointBuilder::new(axis_vec)
                    .local_anchor1(parent_anchor)
                    .local_anchor2(child_anchor);

                if let Some([min, max]) = limits {
                    builder = builder.limits([*min, *max]);
                }

                if let Some(m) = motor {
                    builder = builder
                        .motor_position(m.target_position, m.stiffness, m.damping)
                        .motor_max_force(m.max_force);
                }

                let joint = builder.build();

                if joint_cfg.parent == "world" {
                    let world_body = RigidBodyBuilder::fixed()
                        .translation(vector![
                            config.position[0] + joint_cfg.parent_anchor[0],
                            config.position[1] + joint_cfg.parent_anchor[1]
                        ])
                        .build();
                    let world_handle = physics_world.rigid_body_set.insert(world_body);
                    physics_world
                        .impulse_joint_set
                        .insert(world_handle, child_handle, joint, true)
                } else if let Some(parent_handle) = link_handles.get(&joint_cfg.parent) {
                    physics_world
                        .impulse_joint_set
                        .insert(*parent_handle, child_handle, joint, true)
                } else {
                    continue;
                }
            }
            Joint2DType::Fixed => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(parent_anchor)
                    .local_anchor2(child_anchor)
                    .build();

                if joint_cfg.parent == "world" {
                    let world_body = RigidBodyBuilder::fixed()
                        .translation(vector![
                            config.position[0] + joint_cfg.parent_anchor[0],
                            config.position[1] + joint_cfg.parent_anchor[1]
                        ])
                        .build();
                    let world_handle = physics_world.rigid_body_set.insert(world_body);
                    physics_world
                        .impulse_joint_set
                        .insert(world_handle, child_handle, joint, true)
                } else if let Some(parent_handle) = link_handles.get(&joint_cfg.parent) {
                    physics_world
                        .impulse_joint_set
                        .insert(*parent_handle, child_handle, joint, true)
                } else {
                    continue;
                }
            }
        };

        joint_handles.insert(joint_cfg.name.clone(), joint_handle);
        joint_positions.insert(joint_cfg.name.clone(), joint_cfg.initial_position);
        joint_velocities.insert(joint_cfg.name.clone(), 0.0);

        // Spawn joint marker for visualization
        commands.spawn((
            Sprite {
                color: Color::srgba(1.0, 0.8, 0.0, 0.8),
                custom_size: Some(Vec2::new(8.0, 8.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            JointMarker {
                robot_name: config.name.clone(),
                joint_name: joint_cfg.name.clone(),
            },
        ));
    }

    // 3. Spawn the main articulated robot entity
    let robot_entity = commands
        .spawn(ArticulatedRobot {
            name: config.name.clone(),
            config: config.clone(),
            link_handles,
            joint_handles,
            joint_positions,
            joint_velocities,
        })
        .id();

    tracing::info!(
        "Spawned articulated robot '{}' with {} links and {} joints",
        config.name,
        config.links.len(),
        config.joints.len()
    );

    robot_entity
}

/// Calculate initial position for a link based on parent chain
fn calculate_link_initial_position(
    config: &ArticulatedRobotConfig,
    link_name: &str,
    known_positions: &HashMap<String, (f32, f32, f32)>,
) -> (f32, f32, f32) {
    // Find joint where this link is the child
    let parent_joint = config.joints.iter().find(|j| j.child == link_name);

    match parent_joint {
        None => {
            // This is a root link
            (config.position[0], config.position[1], config.orientation)
        }
        Some(joint) => {
            if joint.parent == "world" {
                // Connected to world
                let x = config.position[0] + joint.parent_anchor[0] - joint.child_anchor[0];
                let y = config.position[1] + joint.parent_anchor[1] - joint.child_anchor[1];
                (x, y, config.orientation + joint.initial_position)
            } else if let Some((px, py, pa)) = known_positions.get(&joint.parent) {
                // Calculate position relative to parent
                let cos_a = pa.cos();
                let sin_a = pa.sin();

                // Parent anchor in world coords
                let pax = px + joint.parent_anchor[0] * cos_a - joint.parent_anchor[1] * sin_a;
                let pay = py + joint.parent_anchor[0] * sin_a + joint.parent_anchor[1] * cos_a;

                // Child angle
                let child_angle = pa + joint.initial_position;
                let cos_c = child_angle.cos();
                let sin_c = child_angle.sin();

                // Child position (offset by child anchor)
                let x = pax - joint.child_anchor[0] * cos_c + joint.child_anchor[1] * sin_c;
                let y = pay - joint.child_anchor[0] * sin_c - joint.child_anchor[1] * cos_c;

                (x, y, child_angle)
            } else {
                // Parent not yet calculated, use default
                (config.position[0], config.position[1], config.orientation)
            }
        }
    }
}

// ============================================================================
// Systems
// ============================================================================

/// System to sync articulated robot link visuals with physics
pub fn articulated_visual_sync_system(
    physics_world: Res<crate::PhysicsWorld>,
    robots: Query<&ArticulatedRobot>,
    mut links: Query<(&RobotLink, &mut Transform)>,
    _ui_state: Res<crate::ui::UiState>,
) {
    let scale = 50.0; // Same scale as main sim2d

    for (link, mut transform) in links.iter_mut() {
        // Find the robot this link belongs to
        if let Some(robot) = robots.iter().find(|r| r.name == link.robot_name) {
            if let Some(handle) = robot.link_handles.get(&link.link_name) {
                if let Some(rigid_body) = physics_world.rigid_body_set.get(*handle) {
                    let pos = rigid_body.translation();
                    let rot = rigid_body.rotation().angle();

                    transform.translation.x = pos.x * scale;
                    transform.translation.y = pos.y * scale;
                    transform.rotation = Quat::from_rotation_z(rot);
                }
            }
        }
    }
}

/// System to sync joint marker positions
pub fn joint_marker_sync_system(
    physics_world: Res<crate::PhysicsWorld>,
    robots: Query<&ArticulatedRobot>,
    mut markers: Query<(&JointMarker, &mut Transform, &mut Visibility)>,
    ui_state: Res<crate::ui::UiState>,
) {
    let scale = 50.0;
    let show_markers = ui_state.show_joint_markers;

    for (marker, mut transform, mut visibility) in markers.iter_mut() {
        *visibility = if show_markers {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        if !show_markers {
            continue;
        }

        // Find the robot and joint
        if let Some(robot) = robots.iter().find(|r| r.name == marker.robot_name) {
            if let Some(joint_cfg) = robot.config.joints.iter().find(|j| j.name == marker.joint_name)
            {
                // Get child link position (joint is at child anchor)
                if let Some(child_handle) = robot.link_handles.get(&joint_cfg.child) {
                    if let Some(rigid_body) = physics_world.rigid_body_set.get(*child_handle) {
                        let pos = rigid_body.translation();
                        let rot = rigid_body.rotation().angle();

                        // Calculate world position of joint
                        let cos_r = rot.cos();
                        let sin_r = rot.sin();
                        let anchor = &joint_cfg.child_anchor;
                        let jx = pos.x + anchor[0] * cos_r - anchor[1] * sin_r;
                        let jy = pos.y + anchor[0] * sin_r + anchor[1] * cos_r;

                        transform.translation.x = jx * scale;
                        transform.translation.y = jy * scale;
                    }
                }
            }
        }
    }
}

/// System to update joint state from physics
pub fn joint_state_update_system(
    physics_world: Res<crate::PhysicsWorld>,
    mut robots: Query<&mut ArticulatedRobot>,
) {
    for mut robot in robots.iter_mut() {
        // Collect updates first to avoid borrow conflicts
        let updates: Vec<(String, f32, f32)> = robot
            .joint_handles
            .iter()
            .filter_map(|(joint_name, joint_handle)| {
                let joint = physics_world.impulse_joint_set.get(*joint_handle)?;

                // Get the bodies
                let (body1, body2) = (joint.body1, joint.body2);
                let rb1 = physics_world.rigid_body_set.get(body1)?;
                let rb2 = physics_world.rigid_body_set.get(body2)?;

                // Calculate relative angle for revolute joints
                let angle1 = rb1.rotation().angle();
                let angle2 = rb2.rotation().angle();
                let relative_angle = angle2 - angle1;

                // Normalize to [-PI, PI]
                let normalized_angle = ((relative_angle + std::f32::consts::PI)
                    % (2.0 * std::f32::consts::PI))
                    - std::f32::consts::PI;

                // Calculate relative angular velocity
                let angvel1 = rb1.angvel();
                let angvel2 = rb2.angvel();

                Some((joint_name.clone(), normalized_angle, angvel2 - angvel1))
            })
            .collect();

        // Apply updates
        for (joint_name, position, velocity) in updates {
            robot.joint_positions.insert(joint_name.clone(), position);
            robot.joint_velocities.insert(joint_name, velocity);
        }
    }
}

/// System to apply joint commands
pub fn joint_command_system(
    _physics_world: ResMut<crate::PhysicsWorld>,
    _robots: Query<&ArticulatedRobot>,
    // Joint commands would come from HORUS communication
    // For now, this is a placeholder for the command handling
) {
    // This will be connected to HORUS communication in Phase 2
}

// ============================================================================
// Preset Configurations
// ============================================================================

/// Create a simple 2-DOF arm configuration
pub fn preset_arm_2dof() -> ArticulatedRobotConfig {
    ArticulatedRobotConfig {
        name: "arm_2dof".to_string(),
        topic_prefix: "/arm".to_string(),
        position: [0.0, 0.0],
        orientation: 0.0,
        fixed_base: true,
        robot_type: ArticulatedRobotType::Arm,
        enable_gravity: false,
        self_collision: false,
        links: vec![
            Link2D {
                name: "base".to_string(),
                size: [0.2, 0.2],
                mass: 5.0,
                color: [0.3, 0.3, 0.3],
                shape: LinkShape::Rectangle,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            Link2D {
                name: "upper_arm".to_string(),
                size: [0.4, 0.08],
                mass: 1.0,
                color: [0.2, 0.6, 0.8],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            Link2D {
                name: "forearm".to_string(),
                size: [0.35, 0.06],
                mass: 0.8,
                color: [0.2, 0.8, 0.6],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
        ],
        joints: vec![
            Joint2D {
                name: "shoulder".to_string(),
                parent: "base".to_string(),
                child: "upper_arm".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2]),
                    motor: Some(JointMotor {
                        stiffness: 100.0,
                        damping: 10.0,
                        max_force: 50.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.1, 0.0],
                child_anchor: [-0.2, 0.0],
                initial_position: 0.0,
            },
            Joint2D {
                name: "elbow".to_string(),
                parent: "upper_arm".to_string(),
                child: "forearm".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([0.0, 2.5]),
                    motor: Some(JointMotor {
                        stiffness: 80.0,
                        damping: 8.0,
                        max_force: 40.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.2, 0.0],
                child_anchor: [-0.175, 0.0],
                initial_position: 0.5,
            },
        ],
    }
}

/// Create a simple 2D humanoid configuration (side view)
pub fn preset_humanoid_simple() -> ArticulatedRobotConfig {
    ArticulatedRobotConfig {
        name: "humanoid_2d".to_string(),
        topic_prefix: "/humanoid".to_string(),
        position: [0.0, 1.0],
        orientation: 0.0,
        fixed_base: false,
        robot_type: ArticulatedRobotType::Humanoid,
        enable_gravity: true,
        self_collision: false,
        links: vec![
            // Torso
            Link2D {
                name: "torso".to_string(),
                size: [0.2, 0.4],
                mass: 10.0,
                color: [0.4, 0.4, 0.6],
                shape: LinkShape::Rectangle,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            // Head
            Link2D {
                name: "head".to_string(),
                size: [0.12, 0.15],
                mass: 2.0,
                color: [0.8, 0.7, 0.6],
                shape: LinkShape::Circle { radius: 0.08 },
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            // Right arm
            Link2D {
                name: "r_upper_arm".to_string(),
                size: [0.2, 0.05],
                mass: 1.0,
                color: [0.5, 0.5, 0.7],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            Link2D {
                name: "r_forearm".to_string(),
                size: [0.18, 0.04],
                mass: 0.7,
                color: [0.5, 0.5, 0.7],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            // Left arm
            Link2D {
                name: "l_upper_arm".to_string(),
                size: [0.2, 0.05],
                mass: 1.0,
                color: [0.5, 0.5, 0.7],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            Link2D {
                name: "l_forearm".to_string(),
                size: [0.18, 0.04],
                mass: 0.7,
                color: [0.5, 0.5, 0.7],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            // Right leg
            Link2D {
                name: "r_thigh".to_string(),
                size: [0.25, 0.07],
                mass: 2.5,
                color: [0.3, 0.3, 0.5],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            Link2D {
                name: "r_shin".to_string(),
                size: [0.22, 0.05],
                mass: 1.5,
                color: [0.3, 0.3, 0.5],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            Link2D {
                name: "r_foot".to_string(),
                size: [0.12, 0.03],
                mass: 0.5,
                color: [0.2, 0.2, 0.3],
                shape: LinkShape::Rectangle,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            // Left leg
            Link2D {
                name: "l_thigh".to_string(),
                size: [0.25, 0.07],
                mass: 2.5,
                color: [0.3, 0.3, 0.5],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            Link2D {
                name: "l_shin".to_string(),
                size: [0.22, 0.05],
                mass: 1.5,
                color: [0.3, 0.3, 0.5],
                shape: LinkShape::Capsule,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
            Link2D {
                name: "l_foot".to_string(),
                size: [0.12, 0.03],
                mass: 0.5,
                color: [0.2, 0.2, 0.3],
                shape: LinkShape::Rectangle,
                com_offset: [0.0, 0.0],
                collision_group: None,
            },
        ],
        joints: vec![
            // Neck
            Joint2D {
                name: "neck".to_string(),
                parent: "torso".to_string(),
                child: "head".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-0.5, 0.5]),
                    motor: Some(JointMotor {
                        stiffness: 50.0,
                        damping: 5.0,
                        max_force: 20.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.0, 0.2],
                child_anchor: [0.0, -0.08],
                initial_position: 0.0,
            },
            // Right arm
            Joint2D {
                name: "r_shoulder".to_string(),
                parent: "torso".to_string(),
                child: "r_upper_arm".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-std::f32::consts::PI, std::f32::consts::PI]),
                    motor: Some(JointMotor {
                        stiffness: 100.0,
                        damping: 10.0,
                        max_force: 50.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.1, 0.15],
                child_anchor: [-0.1, 0.0],
                initial_position: 0.0,
            },
            Joint2D {
                name: "r_elbow".to_string(),
                parent: "r_upper_arm".to_string(),
                child: "r_forearm".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([0.0, 2.5]),
                    motor: Some(JointMotor {
                        stiffness: 80.0,
                        damping: 8.0,
                        max_force: 40.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.1, 0.0],
                child_anchor: [-0.09, 0.0],
                initial_position: 0.3,
            },
            // Left arm
            Joint2D {
                name: "l_shoulder".to_string(),
                parent: "torso".to_string(),
                child: "l_upper_arm".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-std::f32::consts::PI, std::f32::consts::PI]),
                    motor: Some(JointMotor {
                        stiffness: 100.0,
                        damping: 10.0,
                        max_force: 50.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [-0.1, 0.15],
                child_anchor: [0.1, 0.0],
                initial_position: std::f32::consts::PI,
            },
            Joint2D {
                name: "l_elbow".to_string(),
                parent: "l_upper_arm".to_string(),
                child: "l_forearm".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-2.5, 0.0]),
                    motor: Some(JointMotor {
                        stiffness: 80.0,
                        damping: 8.0,
                        max_force: 40.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [-0.1, 0.0],
                child_anchor: [0.09, 0.0],
                initial_position: -0.3,
            },
            // Right leg
            Joint2D {
                name: "r_hip".to_string(),
                parent: "torso".to_string(),
                child: "r_thigh".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-1.0, 1.5]),
                    motor: Some(JointMotor {
                        stiffness: 150.0,
                        damping: 15.0,
                        max_force: 100.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.05, -0.2],
                child_anchor: [0.0, 0.125],
                initial_position: 0.0,
            },
            Joint2D {
                name: "r_knee".to_string(),
                parent: "r_thigh".to_string(),
                child: "r_shin".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-2.5, 0.0]),
                    motor: Some(JointMotor {
                        stiffness: 120.0,
                        damping: 12.0,
                        max_force: 80.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.0, -0.125],
                child_anchor: [0.0, 0.11],
                initial_position: 0.0,
            },
            Joint2D {
                name: "r_ankle".to_string(),
                parent: "r_shin".to_string(),
                child: "r_foot".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-0.8, 0.8]),
                    motor: Some(JointMotor {
                        stiffness: 60.0,
                        damping: 6.0,
                        max_force: 30.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.0, -0.11],
                child_anchor: [-0.04, 0.015],
                initial_position: 0.0,
            },
            // Left leg
            Joint2D {
                name: "l_hip".to_string(),
                parent: "torso".to_string(),
                child: "l_thigh".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-1.5, 1.0]),
                    motor: Some(JointMotor {
                        stiffness: 150.0,
                        damping: 15.0,
                        max_force: 100.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [-0.05, -0.2],
                child_anchor: [0.0, 0.125],
                initial_position: 0.0,
            },
            Joint2D {
                name: "l_knee".to_string(),
                parent: "l_thigh".to_string(),
                child: "l_shin".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([0.0, 2.5]),
                    motor: Some(JointMotor {
                        stiffness: 120.0,
                        damping: 12.0,
                        max_force: 80.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.0, -0.125],
                child_anchor: [0.0, 0.11],
                initial_position: 0.0,
            },
            Joint2D {
                name: "l_ankle".to_string(),
                parent: "l_shin".to_string(),
                child: "l_foot".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-0.8, 0.8]),
                    motor: Some(JointMotor {
                        stiffness: 60.0,
                        damping: 6.0,
                        max_force: 30.0,
                        ..Default::default()
                    }),
                },
                parent_anchor: [0.0, -0.11],
                child_anchor: [0.04, 0.015],
                initial_position: 0.0,
            },
        ],
    }
}

/// Create a 6-DOF arm configuration
pub fn preset_arm_6dof() -> ArticulatedRobotConfig {
    ArticulatedRobotConfig {
        name: "arm_6dof".to_string(),
        topic_prefix: "/arm6".to_string(),
        position: [0.0, 0.0],
        orientation: 0.0,
        fixed_base: true,
        robot_type: ArticulatedRobotType::Arm,
        enable_gravity: false,
        self_collision: false,
        links: vec![
            Link2D {
                name: "base".to_string(),
                size: [0.25, 0.25],
                mass: 8.0,
                color: [0.3, 0.3, 0.3],
                ..Default::default()
            },
            Link2D {
                name: "link1".to_string(),
                size: [0.3, 0.08],
                mass: 1.5,
                color: [0.2, 0.5, 0.8],
                shape: LinkShape::Capsule,
                ..Default::default()
            },
            Link2D {
                name: "link2".to_string(),
                size: [0.28, 0.07],
                mass: 1.2,
                color: [0.3, 0.6, 0.7],
                shape: LinkShape::Capsule,
                ..Default::default()
            },
            Link2D {
                name: "link3".to_string(),
                size: [0.25, 0.06],
                mass: 1.0,
                color: [0.4, 0.7, 0.6],
                shape: LinkShape::Capsule,
                ..Default::default()
            },
            Link2D {
                name: "link4".to_string(),
                size: [0.22, 0.05],
                mass: 0.8,
                color: [0.5, 0.8, 0.5],
                shape: LinkShape::Capsule,
                ..Default::default()
            },
            Link2D {
                name: "link5".to_string(),
                size: [0.18, 0.04],
                mass: 0.6,
                color: [0.6, 0.8, 0.4],
                shape: LinkShape::Capsule,
                ..Default::default()
            },
            Link2D {
                name: "end_effector".to_string(),
                size: [0.1, 0.1],
                mass: 0.3,
                color: [0.8, 0.6, 0.2],
                ..Default::default()
            },
        ],
        joints: vec![
            Joint2D {
                name: "joint1".to_string(),
                parent: "base".to_string(),
                child: "link1".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-std::f32::consts::PI, std::f32::consts::PI]),
                    motor: Some(JointMotor::default()),
                },
                parent_anchor: [0.125, 0.0],
                child_anchor: [-0.15, 0.0],
                initial_position: 0.0,
            },
            Joint2D {
                name: "joint2".to_string(),
                parent: "link1".to_string(),
                child: "link2".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-2.0, 2.0]),
                    motor: Some(JointMotor::default()),
                },
                parent_anchor: [0.15, 0.0],
                child_anchor: [-0.14, 0.0],
                initial_position: 0.3,
            },
            Joint2D {
                name: "joint3".to_string(),
                parent: "link2".to_string(),
                child: "link3".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-2.0, 2.0]),
                    motor: Some(JointMotor::default()),
                },
                parent_anchor: [0.14, 0.0],
                child_anchor: [-0.125, 0.0],
                initial_position: -0.2,
            },
            Joint2D {
                name: "joint4".to_string(),
                parent: "link3".to_string(),
                child: "link4".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-2.5, 2.5]),
                    motor: Some(JointMotor::default()),
                },
                parent_anchor: [0.125, 0.0],
                child_anchor: [-0.11, 0.0],
                initial_position: 0.5,
            },
            Joint2D {
                name: "joint5".to_string(),
                parent: "link4".to_string(),
                child: "link5".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-2.5, 2.5]),
                    motor: Some(JointMotor::default()),
                },
                parent_anchor: [0.11, 0.0],
                child_anchor: [-0.09, 0.0],
                initial_position: -0.3,
            },
            Joint2D {
                name: "joint6".to_string(),
                parent: "link5".to_string(),
                child: "end_effector".to_string(),
                joint_type: Joint2DType::Revolute {
                    limits: Some([-std::f32::consts::PI, std::f32::consts::PI]),
                    motor: Some(JointMotor::default()),
                },
                parent_anchor: [0.09, 0.0],
                child_anchor: [-0.05, 0.0],
                initial_position: 0.0,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arm_2dof_preset() {
        let config = preset_arm_2dof();
        assert_eq!(config.name, "arm_2dof");
        assert_eq!(config.links.len(), 3);
        assert_eq!(config.joints.len(), 2);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_humanoid_preset() {
        let config = preset_humanoid_simple();
        assert_eq!(config.name, "humanoid_2d");
        assert_eq!(config.links.len(), 12);
        assert_eq!(config.joints.len(), 11);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = preset_arm_2dof();

        // Test duplicate link names
        config.links.push(Link2D {
            name: "base".to_string(),
            ..Default::default()
        });
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_joint_motor_defaults() {
        let motor = JointMotor::default();
        assert_eq!(motor.stiffness, 100.0);
        assert_eq!(motor.damping, 10.0);
        assert_eq!(motor.max_force, 50.0);
    }
}
