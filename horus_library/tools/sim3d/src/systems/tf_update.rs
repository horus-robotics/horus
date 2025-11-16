use bevy::prelude::*;
use nalgebra::{Isometry3, Translation3, UnitQuaternion};

use crate::physics::rigid_body::RigidBodyComponent;
use crate::physics::world::PhysicsWorld;
use crate::robot::state::{ArticulatedRobot, JointLink, RobotJointStates};
use crate::tf::tree::TFTree;

/// Component to mark an entity that should publish its transform to the TF tree
#[derive(Component, Clone)]
pub struct TFPublisher {
    /// Frame name for this entity
    pub frame_name: String,
    /// Parent frame name
    pub parent_frame: String,
    /// Update rate in Hz (0 = every frame)
    pub rate_hz: f32,
    /// Last update time
    pub last_update: f32,
}

impl TFPublisher {
    pub fn new(frame_name: impl Into<String>, parent_frame: impl Into<String>) -> Self {
        Self {
            frame_name: frame_name.into(),
            parent_frame: parent_frame.into(),
            rate_hz: 0.0,
            last_update: -f32::INFINITY, // Start with negative infinity so first update always succeeds
        }
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rate_hz = rate_hz;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        if self.rate_hz <= 0.0 {
            return true;
        }
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    pub fn update_time(&mut self, current_time: f32) {
        self.last_update = current_time;
    }
}

/// System to update TF tree from Bevy transforms
pub fn tf_update_system(
    time: Res<Time>,
    mut tf_tree: ResMut<TFTree>,
    mut query: Query<(&mut TFPublisher, &GlobalTransform)>,
) {
    let current_time = time.elapsed_secs();

    for (mut publisher, transform) in query.iter_mut() {
        if !publisher.should_update(current_time) {
            continue;
        }

        publisher.update_time(current_time);

        // Convert Bevy transform to nalgebra Isometry3
        let translation = transform.translation();
        let rotation = transform.to_scale_rotation_translation().1;

        let nalgebra_translation = Translation3::new(translation.x, translation.y, translation.z);
        let nalgebra_rotation = UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
            rotation.w,
            rotation.x,
            rotation.y,
            rotation.z,
        ));
        let isometry = Isometry3::from_parts(nalgebra_translation.into(), nalgebra_rotation);

        // Update or add frame to TF tree
        if tf_tree.frames.contains_key(&publisher.frame_name) {
            tf_tree
                .update_frame(&publisher.frame_name, isometry)
                .ok();
        } else {
            tf_tree
                .add_frame(&publisher.frame_name, &publisher.parent_frame, isometry)
                .ok();
        }
    }
}

/// System to update TF tree from physics rigid bodies
pub fn tf_update_from_physics_system(
    time: Res<Time>,
    physics_world: Res<PhysicsWorld>,
    mut tf_tree: ResMut<TFTree>,
    query: Query<(&RigidBodyComponent, &TFPublisher)>,
) {
    let current_time = time.elapsed_secs();

    for (rb_component, publisher) in query.iter() {
        if !publisher.should_update(current_time) {
            continue;
        }

        // Get rigid body position from physics world
        if let Some(rb) = physics_world.rigid_body_set.get(rb_component.handle) {
            let position = rb.position();

            // Convert to nalgebra Isometry3
            let isometry = Isometry3::from_parts(
                Translation3::new(
                    position.translation.x,
                    position.translation.y,
                    position.translation.z,
                )
                .into(),
                UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
                    position.rotation.w,
                    position.rotation.i,
                    position.rotation.j,
                    position.rotation.k,
                )),
            );

            // Update TF tree
            if tf_tree.frames.contains_key(&publisher.frame_name) {
                tf_tree
                    .update_frame(&publisher.frame_name, isometry)
                    .ok();
            } else {
                tf_tree
                    .add_frame(&publisher.frame_name, &publisher.parent_frame, isometry)
                    .ok();
            }
        }
    }
}

/// System to update TF tree for articulated robots (joint frames)
pub fn tf_update_robot_joints_system(
    mut tf_tree: ResMut<TFTree>,
    robot_query: Query<(&ArticulatedRobot, &RobotJointStates)>,
    joint_query: Query<&JointLink>,
) {
    for (_robot, joint_states) in robot_query.iter() {
        for joint_link in joint_query.iter() {
            // Get joint state
            if let Some(joint_state) = joint_states.get_joint(&joint_link.joint_name) {
                // Create transform based on joint position
                // For revolute joints: rotation around an axis
                // For prismatic joints: translation along an axis
                let isometry = match joint_state.joint_type {
                    crate::physics::joints::JointType::Revolute => {
                        // Rotation around Z-axis (assuming that's the joint axis)
                        let rotation = UnitQuaternion::from_euler_angles(0.0, 0.0, joint_state.position);
                        Isometry3::from_parts(Translation3::new(0.0, 0.0, 0.0).into(), rotation)
                    }
                    crate::physics::joints::JointType::Prismatic => {
                        // Translation along Z-axis (assuming that's the joint axis)
                        Isometry3::from_parts(
                            Translation3::new(0.0, 0.0, joint_state.position).into(),
                            UnitQuaternion::identity(),
                        )
                    }
                    _ => Isometry3::identity(),
                };

                // Update TF tree with joint transform
                // Frame name would be the child link name
                let child_frame = format!("{}_frame", joint_link.joint_name);

                if tf_tree.frames.contains_key(&child_frame) {
                    tf_tree.update_frame(&child_frame, isometry).ok();
                } else {
                    // Parent frame would be the parent link
                    let parent_frame = format!("link_{}", joint_link.parent_entity.index());
                    tf_tree
                        .add_frame(&child_frame, &parent_frame, isometry)
                        .ok();
                }
            }
        }
    }
}

/// Resource to configure TF update behavior
#[derive(Resource, Clone)]
pub struct TFUpdateConfig {
    /// Enable TF updates
    pub enabled: bool,
    /// Update from physics rigid bodies
    pub update_from_physics: bool,
    /// Update robot joint frames
    pub update_robot_joints: bool,
    /// Default update rate (Hz), 0 = every frame
    pub default_rate: f32,
}

impl Default for TFUpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_from_physics: true,
            update_robot_joints: true,
            default_rate: 100.0, // 100 Hz default for TF
        }
    }
}

impl TFUpdateConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rate(mut self, rate: f32) -> Self {
        self.default_rate = rate;
        self
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..default()
        }
    }
}

/// Event fired when TF tree is updated
#[derive(Event)]
pub struct TFTreeUpdatedEvent {
    pub frame_count: usize,
    pub update_time: f32,
}

/// System to emit TF update events
pub fn emit_tf_updated_event(
    time: Res<Time>,
    tf_tree: Res<TFTree>,
    mut events: EventWriter<TFTreeUpdatedEvent>,
) {
    events.send(TFTreeUpdatedEvent {
        frame_count: tf_tree.frames.len(),
        update_time: time.elapsed_secs(),
    });
}

/// Resource to track TF update statistics
#[derive(Resource, Default)]
pub struct TFUpdateStats {
    pub frame_count: usize,
    pub update_count: u64,
    pub last_update_time: f32,
    pub frames_updated: u64,
}

impl TFUpdateStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, frame_count: usize, frames_updated: u64, time: f32) {
        self.frame_count = frame_count;
        self.update_count += 1;
        self.frames_updated += frames_updated;
        self.last_update_time = time;
    }

    pub fn reset(&mut self) {
        self.update_count = 0;
        self.frames_updated = 0;
    }
}

/// System to track TF update statistics
pub fn track_tf_stats_system(
    time: Res<Time>,
    tf_tree: Res<TFTree>,
    mut stats: ResMut<TFUpdateStats>,
    query: Query<&TFPublisher>,
) {
    let frames_updated = query.iter().count() as u64;
    stats.update(tf_tree.frames.len(), frames_updated, time.elapsed_secs());
}

/// Plugin to register all TF update systems
pub struct TFUpdatePlugin;

impl Plugin for TFUpdatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TFUpdateConfig>()
            .init_resource::<TFUpdateStats>()
            .add_event::<TFTreeUpdatedEvent>()
            .add_systems(
                Update,
                (
                    tf_update_system,
                    tf_update_from_physics_system,
                    tf_update_robot_joints_system,
                    emit_tf_updated_event,
                    track_tf_stats_system,
                )
                    .chain(),
            );
    }
}

/// Helper function to create a TF publisher for an entity
pub fn add_tf_publisher(
    commands: &mut Commands,
    entity: Entity,
    frame_name: impl Into<String>,
    parent_frame: impl Into<String>,
) {
    commands.entity(entity).insert(TFPublisher::new(frame_name, parent_frame));
}

/// Helper function to create a TF publisher with custom rate
pub fn add_tf_publisher_with_rate(
    commands: &mut Commands,
    entity: Entity,
    frame_name: impl Into<String>,
    parent_frame: impl Into<String>,
    rate_hz: f32,
) {
    commands
        .entity(entity)
        .insert(TFPublisher::new(frame_name, parent_frame).with_rate(rate_hz));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tf_publisher() {
        let mut publisher = TFPublisher::new("test_frame", "world");
        assert_eq!(publisher.frame_name, "test_frame");
        assert_eq!(publisher.parent_frame, "world");
        assert_eq!(publisher.rate_hz, 0.0);

        publisher = publisher.with_rate(50.0);
        assert_eq!(publisher.rate_hz, 50.0);
    }

    #[test]
    fn test_tf_publisher_rate_limiting() {
        let mut publisher = TFPublisher::new("test", "world").with_rate(10.0);

        assert!(publisher.should_update(0.0));
        publisher.update_time(0.0);

        assert!(!publisher.should_update(0.05)); // 50ms < 100ms
        assert!(publisher.should_update(0.11));  // 110ms > 100ms
    }

    #[test]
    fn test_tf_update_config() {
        let config = TFUpdateConfig::new().with_rate(200.0);
        assert_eq!(config.default_rate, 200.0);
        assert!(config.enabled);

        let disabled = TFUpdateConfig::disabled();
        assert!(!disabled.enabled);
    }

    #[test]
    fn test_tf_update_stats() {
        let mut stats = TFUpdateStats::new();
        assert_eq!(stats.update_count, 0);

        stats.update(10, 5, 1.0);
        assert_eq!(stats.frame_count, 10);
        assert_eq!(stats.update_count, 1);
        assert_eq!(stats.frames_updated, 5);

        stats.reset();
        assert_eq!(stats.update_count, 0);
        assert_eq!(stats.frames_updated, 0);
    }
}
