use bevy::prelude::*;
use rapier3d::prelude::*;
use std::collections::HashMap;

use crate::physics::joints::{JointType, PhysicsJoint};
use crate::physics::world::PhysicsWorld;

/// Component tracking the state of a single joint
#[derive(Component, Clone, Debug)]
pub struct JointState {
    /// Current position/angle of the joint
    pub position: f32,
    /// Current velocity of the joint
    pub velocity: f32,
    /// Current effort (force/torque) applied to the joint
    pub effort: f32,
    /// Joint limits (min, max)
    pub limits: Option<(f32, f32)>,
    /// Joint name
    pub name: String,
    /// Joint type
    pub joint_type: JointType,
}

impl Default for JointState {
    fn default() -> Self {
        Self {
            position: 0.0,
            velocity: 0.0,
            effort: 0.0,
            limits: None,
            name: String::new(),
            joint_type: JointType::Revolute,
        }
    }
}

impl JointState {
    pub fn new(name: impl Into<String>, joint_type: JointType) -> Self {
        Self {
            name: name.into(),
            joint_type,
            ..default()
        }
    }

    pub fn with_limits(mut self, min: f32, max: f32) -> Self {
        self.limits = Some((min, max));
        self
    }

    /// Check if position is within limits
    pub fn is_within_limits(&self) -> bool {
        if let Some((min, max)) = self.limits {
            self.position >= min && self.position <= max
        } else {
            true
        }
    }

    /// Clamp position to limits
    pub fn clamp_position(&mut self) {
        if let Some((min, max)) = self.limits {
            self.position = self.position.clamp(min, max);
        }
    }
}

/// Component for tracking multiple joints in a robot
#[derive(Component, Clone, Default)]
pub struct RobotJointStates {
    pub joints: HashMap<String, JointState>,
    pub joint_order: Vec<String>,
}

impl RobotJointStates {
    pub fn new() -> Self {
        Self {
            joints: HashMap::new(),
            joint_order: Vec::new(),
        }
    }

    /// Add a joint to track
    pub fn add_joint(&mut self, name: impl Into<String>, joint_type: JointType) {
        let name = name.into();
        if !self.joints.contains_key(&name) {
            self.joint_order.push(name.clone());
        }
        self.joints
            .insert(name.clone(), JointState::new(name, joint_type));
    }

    /// Update joint state
    pub fn update_joint(&mut self, name: &str, position: f32, velocity: f32, effort: f32) {
        if let Some(joint) = self.joints.get_mut(name) {
            joint.position = position;
            joint.velocity = velocity;
            joint.effort = effort;
        }
    }

    /// Get joint state by name
    pub fn get_joint(&self, name: &str) -> Option<&JointState> {
        self.joints.get(name)
    }

    /// Get mutable joint state by name
    pub fn get_joint_mut(&mut self, name: &str) -> Option<&mut JointState> {
        self.joints.get_mut(name)
    }

    /// Get all joint positions as a vector (in joint_order)
    pub fn get_positions(&self) -> Vec<f32> {
        self.joint_order
            .iter()
            .filter_map(|name| self.joints.get(name).map(|j| j.position))
            .collect()
    }

    /// Get all joint velocities as a vector (in joint_order)
    pub fn get_velocities(&self) -> Vec<f32> {
        self.joint_order
            .iter()
            .filter_map(|name| self.joints.get(name).map(|j| j.velocity))
            .collect()
    }

    /// Get all joint efforts as a vector (in joint_order)
    pub fn get_efforts(&self) -> Vec<f32> {
        self.joint_order
            .iter()
            .filter_map(|name| self.joints.get(name).map(|j| j.effort))
            .collect()
    }

    /// Set all joint positions from a vector (in joint_order)
    pub fn set_positions(&mut self, positions: &[f32]) {
        for (i, name) in self.joint_order.iter().enumerate() {
            if let Some(pos) = positions.get(i) {
                if let Some(joint) = self.joints.get_mut(name) {
                    joint.position = *pos;
                }
            }
        }
    }

    /// Set all joint velocities from a vector (in joint_order)
    pub fn set_velocities(&mut self, velocities: &[f32]) {
        for (i, name) in self.joint_order.iter().enumerate() {
            if let Some(vel) = velocities.get(i) {
                if let Some(joint) = self.joints.get_mut(name) {
                    joint.velocity = *vel;
                }
            }
        }
    }

    /// Get number of joints
    pub fn len(&self) -> usize {
        self.joints.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }
}

/// Link between a joint entity and its parent/child bodies
#[derive(Component, Clone)]
pub struct JointLink {
    pub parent_entity: Entity,
    pub child_entity: Entity,
    pub joint_name: String,
}

impl JointLink {
    pub fn new(parent: Entity, child: Entity, name: impl Into<String>) -> Self {
        Self {
            parent_entity: parent,
            child_entity: child,
            joint_name: name.into(),
        }
    }
}

/// Marker component for a robot with articulated joints
#[derive(Component)]
pub struct ArticulatedRobot {
    pub name: String,
    pub num_joints: usize,
}

impl ArticulatedRobot {
    pub fn new(name: impl Into<String>, num_joints: usize) -> Self {
        Self {
            name: name.into(),
            num_joints,
        }
    }
}

/// Extract motor effort (force/torque) from a joint's motor
/// Returns the motor's target force/torque if a motor is configured
fn extract_motor_effort(impulse_joint: &ImpulseJoint, axis: JointAxis) -> f32 {
    // Note: Rapier 0.22 doesn't expose motor impulse directly from the solver
    // We extract the motor's maximum force/torque setting as an approximation
    // This represents the commanded effort limit, not the actual applied force

    if let Some(motor) = impulse_joint.data.motor(axis) {
        // Motor exists - return max force/torque as effort approximation
        // For position/velocity motors, this represents the maximum effort available
        motor.max_force
    } else {
        // No motor configured
        0.0
    }
}

/// System to update joint states from physics world
/// Reads position and velocity from Rapier3D's impulse joints
pub fn update_joint_states_system(
    physics_world: Res<PhysicsWorld>,
    mut joint_query: Query<(&PhysicsJoint, &mut JointState)>,
) {
    for (physics_joint, mut joint_state) in joint_query.iter_mut() {
        // Get the joint from Rapier's impulse joint set
        let Some(impulse_joint) = physics_world.impulse_joint_set.get(physics_joint.handle) else {
            continue;
        };

        // Get the two bodies connected by this joint
        let body1_handle = impulse_joint.body1;
        let body2_handle = impulse_joint.body2;

        let Some(body1) = physics_world.rigid_body_set.get(body1_handle) else {
            continue;
        };
        let Some(body2) = physics_world.rigid_body_set.get(body2_handle) else {
            continue;
        };

        // Get body transforms and velocities
        let pos1 = body1.position();
        let pos2 = body2.position();
        let vel1 = body1.linvel();
        let vel2 = body2.linvel();
        let ang_vel1 = body1.angvel();
        let ang_vel2 = body2.angvel();

        // Compute relative transform: body2_to_body1 = body1^-1 * body2
        let relative_transform = pos1.inverse() * pos2;

        // Extract position, velocity, and effort based on joint type
        let (position, velocity, axis) = match physics_joint.joint_type {
            JointType::Revolute => {
                // Revolute joints rotate around AngX axis
                // Extract angle from relative rotation
                let rotation = relative_transform.rotation;
                let axis_angle = rotation.scaled_axis();
                let angle = axis_angle.x; // Rotation around X axis

                // Compute relative angular velocity
                let relative_ang_vel = ang_vel2 - ang_vel1;
                let ang_velocity = relative_ang_vel.x; // Angular velocity around X axis

                (angle, ang_velocity, JointAxis::AngX)
            }
            JointType::Prismatic => {
                // Prismatic joints translate along LinX axis
                // Extract distance from relative translation
                let translation = relative_transform.translation;
                let distance = translation.vector.x; // Translation along X axis

                // Compute relative linear velocity
                let relative_vel = vel2 - vel1;
                let lin_velocity = relative_vel.x; // Velocity along X axis

                (distance, lin_velocity, JointAxis::LinX)
            }
            JointType::Fixed => {
                // Fixed joints have no degrees of freedom
                joint_state.position = 0.0;
                joint_state.velocity = 0.0;
                joint_state.effort = 0.0;
                continue;
            }
            JointType::Spherical => {
                // Spherical joints have 3 angular DOFs
                // Use magnitude of rotation as a simplified position
                let rotation = relative_transform.rotation;
                let axis_angle = rotation.scaled_axis();
                let angle = axis_angle.norm();

                // Use magnitude of relative angular velocity
                let relative_ang_vel = ang_vel2 - ang_vel1;
                let ang_velocity = relative_ang_vel.norm();

                (angle, ang_velocity, JointAxis::AngX)
            }
        };

        // Update joint state
        joint_state.position = position;
        joint_state.velocity = velocity;

        // Extract motor effort (if motor is configured)
        joint_state.effort = extract_motor_effort(impulse_joint, axis);
    }
}

/// System to update robot-level joint states from individual joints
pub fn update_robot_joint_states_system(
    mut robot_query: Query<(&ArticulatedRobot, &mut RobotJointStates)>,
    joint_query: Query<(&JointLink, &JointState)>,
) {
    for (_robot, mut robot_states) in robot_query.iter_mut() {
        for (joint_link, joint_state) in joint_query.iter() {
            robot_states.update_joint(
                &joint_link.joint_name,
                joint_state.position,
                joint_state.velocity,
                joint_state.effort,
            );
        }
    }
}

/// Event fired when a joint state changes significantly
#[derive(Event, Clone)]
pub struct JointStateChangedEvent {
    pub joint_name: String,
    pub entity: Entity,
    pub position: f32,
    pub velocity: f32,
}

/// System to detect and emit joint state change events
pub fn detect_joint_state_changes_system(
    joint_query: Query<(Entity, &JointState), Changed<JointState>>,
    mut events: EventWriter<JointStateChangedEvent>,
) {
    for (entity, joint_state) in joint_query.iter() {
        events.send(JointStateChangedEvent {
            joint_name: joint_state.name.clone(),
            entity,
            position: joint_state.position,
            velocity: joint_state.velocity,
        });
    }
}

/// Helper to get joint state by name from a robot
pub fn get_robot_joint_position(robot_states: &RobotJointStates, joint_name: &str) -> Option<f32> {
    robot_states.get_joint(joint_name).map(|j| j.position)
}

/// Helper to get multiple joint positions by names
pub fn get_robot_joint_positions(
    robot_states: &RobotJointStates,
    joint_names: &[String],
) -> Vec<f32> {
    joint_names
        .iter()
        .filter_map(|name| get_robot_joint_position(robot_states, name))
        .collect()
}

/// Resource to track all joints in the simulation
#[derive(Resource, Default)]
pub struct JointRegistry {
    pub joints: HashMap<String, Entity>,
}

impl JointRegistry {
    pub fn new() -> Self {
        Self {
            joints: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: impl Into<String>, entity: Entity) {
        self.joints.insert(name.into(), entity);
    }

    pub fn get(&self, name: &str) -> Option<Entity> {
        self.joints.get(name).copied()
    }

    pub fn remove(&mut self, name: &str) -> Option<Entity> {
        self.joints.remove(name)
    }

    pub fn len(&self) -> usize {
        self.joints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }
}

/// Plugin to add joint state tracking systems
pub struct JointStatePlugin;

impl Plugin for JointStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JointRegistry>()
            .add_event::<JointStateChangedEvent>()
            .add_systems(
                Update,
                (
                    update_joint_states_system,
                    update_robot_joint_states_system,
                    detect_joint_state_changes_system,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joint_state() {
        let mut joint = JointState::new("joint1", JointType::Revolute).with_limits(-1.57, 1.57);

        joint.position = 1.0;
        assert!(joint.is_within_limits());

        joint.position = 2.0;
        assert!(!joint.is_within_limits());

        joint.clamp_position();
        assert_eq!(joint.position, 1.57);
    }

    #[test]
    fn test_joint_state_default() {
        let joint = JointState::default();
        assert_eq!(joint.position, 0.0);
        assert_eq!(joint.velocity, 0.0);
        assert_eq!(joint.effort, 0.0);
        assert!(joint.limits.is_none());
        assert!(joint.is_within_limits()); // No limits = always within
    }

    #[test]
    fn test_joint_state_limits() {
        let mut joint = JointState::new("test", JointType::Revolute).with_limits(-3.14, 3.14);

        assert_eq!(joint.limits, Some((-3.14, 3.14)));

        // Test within limits
        joint.position = 0.0;
        assert!(joint.is_within_limits());

        // Test at boundaries
        joint.position = -3.14;
        assert!(joint.is_within_limits());

        joint.position = 3.14;
        assert!(joint.is_within_limits());

        // Test outside limits
        joint.position = -4.0;
        assert!(!joint.is_within_limits());

        // Test clamping
        joint.clamp_position();
        assert_eq!(joint.position, -3.14);
    }

    #[test]
    fn test_robot_joint_states() {
        let mut robot = RobotJointStates::new();

        robot.add_joint("joint1", JointType::Revolute);
        robot.add_joint("joint2", JointType::Revolute);

        robot.update_joint("joint1", 1.0, 0.5, 0.1);
        robot.update_joint("joint2", 2.0, 1.0, 0.2);

        let positions = robot.get_positions();
        assert_eq!(positions, vec![1.0, 2.0]);

        let velocities = robot.get_velocities();
        assert_eq!(velocities, vec![0.5, 1.0]);

        let efforts = robot.get_efforts();
        assert_eq!(efforts, vec![0.1, 0.2]);
    }

    #[test]
    fn test_robot_joint_states_set_positions() {
        let mut robot = RobotJointStates::new();
        robot.add_joint("joint1", JointType::Revolute);
        robot.add_joint("joint2", JointType::Revolute);

        robot.set_positions(&[1.5, 2.5]);

        assert_eq!(robot.get_joint("joint1").unwrap().position, 1.5);
        assert_eq!(robot.get_joint("joint2").unwrap().position, 2.5);
    }

    #[test]
    fn test_robot_joint_states_set_velocities() {
        let mut robot = RobotJointStates::new();
        robot.add_joint("joint1", JointType::Revolute);
        robot.add_joint("joint2", JointType::Revolute);

        robot.set_velocities(&[0.5, 1.5]);

        assert_eq!(robot.get_joint("joint1").unwrap().velocity, 0.5);
        assert_eq!(robot.get_joint("joint2").unwrap().velocity, 1.5);
    }

    #[test]
    fn test_robot_joint_states_ordering() {
        let mut robot = RobotJointStates::new();

        // Add joints in specific order
        robot.add_joint("joint_b", JointType::Revolute);
        robot.add_joint("joint_a", JointType::Revolute);
        robot.add_joint("joint_c", JointType::Revolute);

        robot.update_joint("joint_b", 1.0, 0.0, 0.0);
        robot.update_joint("joint_a", 2.0, 0.0, 0.0);
        robot.update_joint("joint_c", 3.0, 0.0, 0.0);

        // Order should match addition order, not alphabetical
        let positions = robot.get_positions();
        assert_eq!(positions, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_joint_registry() {
        let mut registry = JointRegistry::new();

        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);

        registry.register("joint1", entity1);
        registry.register("joint2", entity2);

        assert_eq!(registry.get("joint1"), Some(entity1));
        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());

        registry.remove("joint1");
        assert_eq!(registry.get("joint1"), None);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_joint_link() {
        let parent = Entity::from_raw(1);
        let child = Entity::from_raw(2);

        let link = JointLink::new(parent, child, "test_joint");

        assert_eq!(link.parent_entity, parent);
        assert_eq!(link.child_entity, child);
        assert_eq!(link.joint_name, "test_joint");
    }

    #[test]
    fn test_articulated_robot() {
        let robot = ArticulatedRobot::new("test_robot", 6);

        assert_eq!(robot.name, "test_robot");
        assert_eq!(robot.num_joints, 6);
    }

    #[test]
    fn test_get_robot_joint_position() {
        let mut robot = RobotJointStates::new();
        robot.add_joint("joint1", JointType::Revolute);
        robot.update_joint("joint1", 1.5, 0.0, 0.0);

        let position = get_robot_joint_position(&robot, "joint1");
        assert_eq!(position, Some(1.5));

        let missing = get_robot_joint_position(&robot, "nonexistent");
        assert_eq!(missing, None);
    }

    #[test]
    fn test_get_robot_joint_positions() {
        let mut robot = RobotJointStates::new();
        robot.add_joint("joint1", JointType::Revolute);
        robot.add_joint("joint2", JointType::Revolute);
        robot.add_joint("joint3", JointType::Revolute);

        robot.update_joint("joint1", 1.0, 0.0, 0.0);
        robot.update_joint("joint2", 2.0, 0.0, 0.0);
        robot.update_joint("joint3", 3.0, 0.0, 0.0);

        let names = vec![
            "joint1".to_string(),
            "joint3".to_string(),
            "joint2".to_string(),
        ];
        let positions = get_robot_joint_positions(&robot, &names);

        assert_eq!(positions, vec![1.0, 3.0, 2.0]);
    }

    #[test]
    fn test_joint_type_matching() {
        // Test that all joint types are handled
        let revolute = JointState::new("rev", JointType::Revolute);
        let prismatic = JointState::new("pris", JointType::Prismatic);
        let fixed = JointState::new("fix", JointType::Fixed);
        let spherical = JointState::new("sphere", JointType::Spherical);

        assert!(matches!(revolute.joint_type, JointType::Revolute));
        assert!(matches!(prismatic.joint_type, JointType::Prismatic));
        assert!(matches!(fixed.joint_type, JointType::Fixed));
        assert!(matches!(spherical.joint_type, JointType::Spherical));
    }

    #[test]
    fn test_robot_joint_states_empty() {
        let robot = RobotJointStates::new();

        assert!(robot.is_empty());
        assert_eq!(robot.len(), 0);
        assert_eq!(robot.get_positions(), Vec::<f32>::new());
        assert_eq!(robot.get_velocities(), Vec::<f32>::new());
        assert_eq!(robot.get_efforts(), Vec::<f32>::new());
    }

    #[test]
    fn test_joint_state_changed_event() {
        let entity = Entity::from_raw(42);
        let event = JointStateChangedEvent {
            joint_name: "test_joint".to_string(),
            entity,
            position: 1.57,
            velocity: 0.5,
        };

        assert_eq!(event.joint_name, "test_joint");
        assert_eq!(event.entity, entity);
        assert_eq!(event.position, 1.57);
        assert_eq!(event.velocity, 0.5);
    }
}
