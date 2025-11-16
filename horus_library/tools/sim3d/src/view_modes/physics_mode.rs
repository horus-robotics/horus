use bevy::prelude::*;

use crate::physics::rigid_body::RigidBodyComponent;
use crate::physics::world::PhysicsWorld;
use crate::ui::controls::SimulationControls;

/// Resource to control physics visualization
#[derive(Resource, Clone)]
pub struct PhysicsVisualization {
    pub enabled: bool,
    pub show_velocity: bool,
    pub show_angular_velocity: bool,
    pub show_forces: bool,
    pub show_collision_shapes: bool,
    pub show_center_of_mass: bool,
    pub show_bounding_boxes: bool,
    pub velocity_scale: f32,
    pub force_scale: f32,
}

impl Default for PhysicsVisualization {
    fn default() -> Self {
        Self {
            enabled: false,
            show_velocity: true,
            show_angular_velocity: false,
            show_forces: true,
            show_collision_shapes: true,
            show_center_of_mass: true,
            show_bounding_boxes: false,
            velocity_scale: 1.0,
            force_scale: 0.01,
        }
    }
}

impl PhysicsVisualization {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

/// System to visualize physics properties
pub fn physics_visualization_system(
    mut gizmos: Gizmos,
    physics_viz: Res<PhysicsVisualization>,
    controls: Res<SimulationControls>,
    physics_world: Res<PhysicsWorld>,
    rigid_bodies: Query<(&RigidBodyComponent, &Transform)>,
) {
    // Check if physics debug is enabled from controls or physics viz
    if !physics_viz.enabled && !controls.show_physics_debug {
        return;
    }

    for (rb, transform) in rigid_bodies.iter() {
        let position = transform.translation;

        // Show center of mass
        if physics_viz.show_center_of_mass {
            gizmos.sphere(
                Isometry3d::new(position, Quat::IDENTITY),
                0.05,
                Color::srgb(1.0, 1.0, 0.0), // Yellow
            );
        }

        // Get rigid body from physics world to access velocities
        let Some(rigid_body) = physics_world.rigid_body_set.get(rb.handle) else {
            continue;
        };

        // Show linear velocity vector
        if physics_viz.show_velocity {
            let linear_velocity = rigid_body.linvel();
            if linear_velocity.norm() > 0.01 {
                let vel_bevy = Vec3::new(linear_velocity.x, linear_velocity.y, linear_velocity.z);
                let velocity_end = position + vel_bevy * physics_viz.velocity_scale;
                gizmos.arrow(
                    position,
                    velocity_end,
                    Color::srgb(0.0, 1.0, 0.0), // Green
                );
            }
        }

        // Show angular velocity (as a circle around the axis)
        if physics_viz.show_angular_velocity {
            let angular_velocity = rigid_body.angvel();
            if angular_velocity.norm() > 0.01 {
                let ang_vel_bevy = Vec3::new(angular_velocity.x, angular_velocity.y, angular_velocity.z);
                let axis = ang_vel_bevy.normalize();
                let magnitude = angular_velocity.norm();
                let radius = 0.2 * magnitude.min(5.0);

                // Compute rotation to align circle with angular velocity axis
                let rotation = Quat::from_rotation_arc(Vec3::Y, axis);
                gizmos.circle(
                    Isometry3d::new(position, rotation),
                    radius,
                    Color::srgb(0.0, 1.0, 1.0), // Cyan
                );
            }
        }

        // Show bounding box
        if physics_viz.show_bounding_boxes {
            // Estimate bounding box from transform scale
            let half_extents = transform.scale * 0.5;

            gizmos.cuboid(
                Transform::from_translation(position)
                    .with_rotation(transform.rotation)
                    .with_scale(transform.scale),
                Color::srgb(0.5, 0.5, 0.5).with_alpha(0.3),
            );
        }

        // Show collision shapes (simplified box representation)
        if physics_viz.show_collision_shapes {
            let half_extents = transform.scale * 0.5;

            gizmos.cuboid(
                Transform::from_translation(position)
                    .with_rotation(transform.rotation)
                    .with_scale(transform.scale * 0.98), // Slightly smaller to differentiate from bounding box
                Color::srgb(0.0, 0.5, 1.0).with_alpha(0.5), // Blue
            );
        }
    }
}

/// System to draw physics grid
pub fn physics_grid_system(
    mut gizmos: Gizmos,
    physics_viz: Res<PhysicsVisualization>,
    controls: Res<SimulationControls>,
) {
    if !physics_viz.enabled && !controls.show_physics_debug {
        return;
    }

    // Draw ground grid
    let grid_size = 20;
    let grid_spacing = 1.0;
    let grid_color = Color::srgb(0.3, 0.3, 0.3);

    for i in -grid_size..=grid_size {
        let offset = i as f32 * grid_spacing;

        // Lines along X axis
        gizmos.line(
            Vec3::new(-grid_size as f32 * grid_spacing, 0.0, offset),
            Vec3::new(grid_size as f32 * grid_spacing, 0.0, offset),
            grid_color,
        );

        // Lines along Z axis
        gizmos.line(
            Vec3::new(offset, 0.0, -grid_size as f32 * grid_spacing),
            Vec3::new(offset, 0.0, grid_size as f32 * grid_spacing),
            grid_color,
        );
    }

    // Draw coordinate axes at origin
    let axis_length = 2.0;

    // X axis - Red
    gizmos.arrow(
        Vec3::ZERO,
        Vec3::new(axis_length, 0.0, 0.0),
        Color::srgb(1.0, 0.0, 0.0),
    );

    // Y axis - Green
    gizmos.arrow(
        Vec3::ZERO,
        Vec3::new(0.0, axis_length, 0.0),
        Color::srgb(0.0, 1.0, 0.0),
    );

    // Z axis - Blue
    gizmos.arrow(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, axis_length),
        Color::srgb(0.0, 0.0, 1.0),
    );
}

/// Component to visualize contact points
#[derive(Component)]
pub struct ContactPoint {
    pub position: Vec3,
    pub normal: Vec3,
    pub depth: f32,
    pub impulse: f32,
}

/// System to visualize contact points
pub fn contact_visualization_system(
    mut gizmos: Gizmos,
    physics_viz: Res<PhysicsVisualization>,
    contacts: Query<&ContactPoint>,
) {
    if !physics_viz.enabled {
        return;
    }

    for contact in contacts.iter() {
        // Draw contact point
        gizmos.sphere(
            Isometry3d::new(contact.position, Quat::IDENTITY),
            0.03,
            Color::srgb(1.0, 0.0, 0.0), // Red
        );

        // Draw contact normal
        let normal_end = contact.position + contact.normal * 0.2;
        gizmos.arrow(
            contact.position,
            normal_end,
            Color::srgb(1.0, 0.5, 0.0), // Orange
        );

        // Draw penetration depth (if any)
        if contact.depth > 0.001 {
            let depth_end = contact.position - contact.normal * contact.depth;
            gizmos.line(
                contact.position,
                depth_end,
                Color::srgb(1.0, 0.0, 1.0), // Magenta
            );
        }
    }
}

/// Keyboard shortcuts for physics visualization
pub fn physics_viz_keyboard_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut physics_viz: ResMut<PhysicsVisualization>,
) {
    // P: Toggle physics visualization
    if keyboard.just_pressed(KeyCode::KeyP) {
        physics_viz.toggle();
    }

    // V: Toggle velocity vectors
    if keyboard.just_pressed(KeyCode::KeyV) && keyboard.pressed(KeyCode::ShiftLeft) {
        physics_viz.show_velocity = !physics_viz.show_velocity;
    }

    // A: Toggle angular velocity
    if keyboard.just_pressed(KeyCode::KeyA) && keyboard.pressed(KeyCode::ShiftLeft) {
        physics_viz.show_angular_velocity = !physics_viz.show_angular_velocity;
    }

    // F: Toggle forces
    if keyboard.just_pressed(KeyCode::KeyF) && keyboard.pressed(KeyCode::ShiftLeft) {
        physics_viz.show_forces = !physics_viz.show_forces;
    }

    // C: Toggle collision shapes
    if keyboard.just_pressed(KeyCode::KeyC) && keyboard.pressed(KeyCode::ShiftLeft) {
        physics_viz.show_collision_shapes = !physics_viz.show_collision_shapes;
    }

    // M: Toggle center of mass
    if keyboard.just_pressed(KeyCode::KeyM) && keyboard.pressed(KeyCode::ShiftLeft) {
        physics_viz.show_center_of_mass = !physics_viz.show_center_of_mass;
    }

    // B: Toggle bounding boxes
    if keyboard.just_pressed(KeyCode::KeyB) && keyboard.pressed(KeyCode::ShiftLeft) {
        physics_viz.show_bounding_boxes = !physics_viz.show_bounding_boxes;
    }
}

/// Plugin to register physics visualization systems
pub struct PhysicsVisualizationPlugin;

impl Plugin for PhysicsVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhysicsVisualization>()
            .add_systems(
                Update,
                (
                    physics_visualization_system,
                    physics_grid_system,
                    contact_visualization_system,
                    physics_viz_keyboard_system,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_visualization() {
        let mut viz = PhysicsVisualization::new();
        assert!(!viz.enabled);

        viz.toggle();
        assert!(viz.enabled);

        viz.disable();
        assert!(!viz.enabled);

        viz.enable();
        assert!(viz.enabled);
    }

    #[test]
    fn test_default_settings() {
        let viz = PhysicsVisualization::default();
        assert!(viz.show_velocity);
        assert!(viz.show_collision_shapes);
        assert!(viz.show_center_of_mass);
        assert!(!viz.show_angular_velocity);
        assert_eq!(viz.velocity_scale, 1.0);
        assert_eq!(viz.force_scale, 0.01);
    }
}
