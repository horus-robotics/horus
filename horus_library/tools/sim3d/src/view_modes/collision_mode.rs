use bevy::prelude::*;

use crate::physics::rigid_body::RigidBodyComponent;
use crate::physics::world::PhysicsWorld;
use crate::ui::controls::SimulationControls;

/// Resource to control collision visualization
#[derive(Resource, Clone)]
pub struct CollisionVisualization {
    pub enabled: bool,
    pub show_collision_shapes: bool,
    pub show_aabb: bool,
    pub show_collision_pairs: bool,
    pub show_sleeping_bodies: bool,
    pub highlight_colliding: bool,
    pub collision_fade_time: f32,
}

impl Default for CollisionVisualization {
    fn default() -> Self {
        Self {
            enabled: false,
            show_collision_shapes: true,
            show_aabb: true,
            show_collision_pairs: true,
            show_sleeping_bodies: false,
            highlight_colliding: true,
            collision_fade_time: 0.5, // Fade collision highlight over 0.5s
        }
    }
}

impl CollisionVisualization {
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

/// Component to track collision state for visualization
#[derive(Component)]
pub struct CollisionState {
    pub is_colliding: bool,
    pub last_collision_time: f32,
    pub collision_count: u32,
}

impl Default for CollisionState {
    fn default() -> Self {
        Self {
            is_colliding: false,
            last_collision_time: 0.0,
            collision_count: 0,
        }
    }
}

impl CollisionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_collision(&mut self, time: f32) {
        self.is_colliding = true;
        self.last_collision_time = time;
        self.collision_count += 1;
    }

    pub fn is_recently_colliding(&self, current_time: f32, fade_time: f32) -> bool {
        current_time - self.last_collision_time < fade_time
    }
}

/// System to visualize collision shapes
pub fn collision_shapes_visualization_system(
    mut gizmos: Gizmos,
    collision_viz: Res<CollisionVisualization>,
    controls: Res<SimulationControls>,
    physics_world: Res<PhysicsWorld>,
    rigid_bodies: Query<(&RigidBodyComponent, &Transform, Option<&CollisionState>)>,
    time: Res<Time>,
) {
    if !collision_viz.enabled && !controls.show_collision_shapes {
        return;
    }

    let current_time = time.elapsed_secs();

    for (rb, transform, collision_state) in rigid_bodies.iter() {
        let position = transform.translation;

        // Determine color based on collision state
        let base_color = if let Some(state) = collision_state {
            if collision_viz.highlight_colliding && state.is_recently_colliding(current_time, collision_viz.collision_fade_time) {
                // Red for recent collision
                let fade_factor = (current_time - state.last_collision_time) / collision_viz.collision_fade_time;
                Color::srgb(1.0, 1.0 - fade_factor, 1.0 - fade_factor)
            } else {
                Color::srgb(0.0, 1.0, 0.0) // Green when not colliding
            }
        } else {
            Color::srgb(0.0, 0.5, 1.0) // Blue default
        };

        // Draw collision shape (simplified as box)
        if collision_viz.show_collision_shapes {
            gizmos.cuboid(
                Transform::from_translation(position)
                    .with_rotation(transform.rotation)
                    .with_scale(transform.scale),
                base_color.with_alpha(0.4),
            );
        }

        // Draw AABB (Axis-Aligned Bounding Box)
        if collision_viz.show_aabb {
            // Calculate AABB from rotated box
            let half_extents = transform.scale * 0.5;
            let corners = [
                Vec3::new(-half_extents.x, -half_extents.y, -half_extents.z),
                Vec3::new(half_extents.x, -half_extents.y, -half_extents.z),
                Vec3::new(-half_extents.x, half_extents.y, -half_extents.z),
                Vec3::new(half_extents.x, half_extents.y, -half_extents.z),
                Vec3::new(-half_extents.x, -half_extents.y, half_extents.z),
                Vec3::new(half_extents.x, -half_extents.y, half_extents.z),
                Vec3::new(-half_extents.x, half_extents.y, half_extents.z),
                Vec3::new(half_extents.x, half_extents.y, half_extents.z),
            ];

            // Transform corners to world space
            let world_corners: Vec<Vec3> = corners
                .iter()
                .map(|&c| position + transform.rotation * c)
                .collect();

            // Find AABB bounds
            let mut min = world_corners[0];
            let mut max = world_corners[0];
            for &corner in &world_corners {
                min = min.min(corner);
                max = max.max(corner);
            }

            let aabb_center = (min + max) * 0.5;
            let aabb_size = max - min;

            gizmos.cuboid(
                Transform::from_translation(aabb_center)
                    .with_scale(aabb_size),
                Color::srgb(1.0, 1.0, 0.0).with_alpha(0.2), // Yellow AABB
            );
        }

        // Show sleeping indicator
        if collision_viz.show_sleeping_bodies {
            if let Some(rigid_body) = physics_world.rigid_body_set.get(rb.handle) {
                if rigid_body.is_sleeping() {
                    gizmos.sphere(
                        Isometry3d::new(position + Vec3::new(0.0, transform.scale.y * 0.6, 0.0), Quat::IDENTITY),
                        0.1,
                        Color::srgb(0.5, 0.5, 0.5), // Gray for sleeping
                    );
                }
            }
        }
    }
}

/// Component to mark collision pairs for visualization
#[derive(Component)]
pub struct CollisionPair {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub contact_point: Vec3,
    pub normal: Vec3,
    pub depth: f32,
}

/// System to visualize collision pairs
pub fn collision_pairs_visualization_system(
    mut gizmos: Gizmos,
    collision_viz: Res<CollisionVisualization>,
    collision_pairs: Query<&CollisionPair>,
) {
    if !collision_viz.enabled || !collision_viz.show_collision_pairs {
        return;
    }

    for pair in collision_pairs.iter() {
        // Draw contact point
        gizmos.sphere(
            Isometry3d::new(pair.contact_point, Quat::IDENTITY),
            0.05,
            Color::srgb(1.0, 0.0, 0.0), // Red
        );

        // Draw normal
        let normal_end = pair.contact_point + pair.normal * 0.3;
        gizmos.arrow(
            pair.contact_point,
            normal_end,
            Color::srgb(1.0, 0.5, 0.0), // Orange
        );

        // Draw penetration depth
        if pair.depth > 0.001 {
            let depth_end = pair.contact_point - pair.normal * pair.depth;
            gizmos.line(
                pair.contact_point,
                depth_end,
                Color::srgb(1.0, 0.0, 1.0), // Magenta
            );
        }
    }
}

/// System to visualize collision islands (groups of interacting bodies)
pub fn collision_islands_visualization_system(
    mut gizmos: Gizmos,
    collision_viz: Res<CollisionVisualization>,
    physics_world: Res<PhysicsWorld>,
    rigid_bodies: Query<(&RigidBodyComponent, &Transform)>,
) {
    if !collision_viz.enabled {
        return;
    }

    // Visualize collision islands by grouping bodies into active/sleeping groups
    // Active (awake) bodies are potentially interacting with each other
    // Sleeping bodies are in separate, stable islands

    for (rb, transform) in rigid_bodies.iter() {
        if let Some(rigid_body) = physics_world.rigid_body_set.get(rb.handle) {
            // Visualize active vs sleeping islands with different colors
            let (color, size_mult) = if rigid_body.is_sleeping() {
                // Sleeping bodies - gray
                (Color::srgb(0.6, 0.6, 0.6).with_alpha(0.2), 0.5)
            } else {
                // Active bodies - cyan (part of active island)
                (Color::srgb(0.0, 1.0, 1.0).with_alpha(0.3), 0.6)
            };

            // Draw a sphere around the body to indicate its island state
            gizmos.sphere(
                Isometry3d::new(transform.translation, Quat::IDENTITY),
                transform.scale.x * size_mult,
                color,
            );
        }
    }
}

/// System to visualize broadphase collision detection grid
pub fn broadphase_grid_visualization_system(
    mut gizmos: Gizmos,
    collision_viz: Res<CollisionVisualization>,
) {
    if !collision_viz.enabled {
        return;
    }

    // Draw a simplified spatial partitioning grid
    let grid_size = 10;
    let cell_size = 5.0;
    let grid_color = Color::srgb(0.2, 0.2, 0.5).with_alpha(0.2);

    for i in -grid_size..=grid_size {
        for j in -grid_size..=grid_size {
            let x = i as f32 * cell_size;
            let z = j as f32 * cell_size;

            // Draw cell outline on ground
            let corners = [
                Vec3::new(x, 0.0, z),
                Vec3::new(x + cell_size, 0.0, z),
                Vec3::new(x + cell_size, 0.0, z + cell_size),
                Vec3::new(x, 0.0, z + cell_size),
            ];

            for i in 0..4 {
                let next = (i + 1) % 4;
                gizmos.line(corners[i], corners[next], grid_color);
            }
        }
    }
}

/// Keyboard shortcuts for collision visualization
pub fn collision_viz_keyboard_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut collision_viz: ResMut<CollisionVisualization>,
) {
    // C: Toggle collision visualization
    if keyboard.just_pressed(KeyCode::KeyC) && keyboard.pressed(KeyCode::ControlLeft) {
        collision_viz.toggle();
    }

    // Shift+C: Toggle collision shapes
    if keyboard.just_pressed(KeyCode::KeyC) && keyboard.pressed(KeyCode::ShiftLeft) {
        collision_viz.show_collision_shapes = !collision_viz.show_collision_shapes;
    }

    // Shift+A: Toggle AABB
    if keyboard.just_pressed(KeyCode::KeyA) && keyboard.pressed(KeyCode::ControlLeft) {
        collision_viz.show_aabb = !collision_viz.show_aabb;
    }

    // Shift+P: Toggle collision pairs
    if keyboard.just_pressed(KeyCode::KeyP) && keyboard.pressed(KeyCode::ControlLeft) {
        collision_viz.show_collision_pairs = !collision_viz.show_collision_pairs;
    }
}

/// Plugin to register collision visualization systems
pub struct CollisionVisualizationPlugin;

impl Plugin for CollisionVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionVisualization>()
            .add_systems(
                Update,
                (
                    collision_shapes_visualization_system,
                    collision_pairs_visualization_system,
                    collision_islands_visualization_system,
                    broadphase_grid_visualization_system,
                    collision_viz_keyboard_system,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collision_visualization() {
        let mut viz = CollisionVisualization::new();
        assert!(!viz.enabled);

        viz.toggle();
        assert!(viz.enabled);

        viz.disable();
        assert!(!viz.enabled);

        viz.enable();
        assert!(viz.enabled);
    }

    #[test]
    fn test_collision_state() {
        let mut state = CollisionState::new();
        assert!(!state.is_colliding);
        assert_eq!(state.collision_count, 0);

        state.record_collision(1.0);
        assert!(state.is_colliding);
        assert_eq!(state.collision_count, 1);
        assert!(state.is_recently_colliding(1.2, 0.5));
        assert!(!state.is_recently_colliding(2.0, 0.5));
    }

    #[test]
    fn test_default_settings() {
        let viz = CollisionVisualization::default();
        assert!(viz.show_collision_shapes);
        assert!(viz.show_aabb);
        assert!(viz.show_collision_pairs);
        assert!(viz.highlight_colliding);
        assert_eq!(viz.collision_fade_time, 0.5);
    }
}
