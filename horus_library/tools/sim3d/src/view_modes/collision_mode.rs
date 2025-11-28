use bevy::prelude::*;
use rapier3d::geometry::ShapeType;

use crate::physics::collider::PhysicsCollider;
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
    rigid_bodies: Query<(
        &RigidBodyComponent,
        &Transform,
        Option<&CollisionState>,
        Option<&PhysicsCollider>,
    )>,
    time: Res<Time>,
) {
    if !collision_viz.enabled && !controls.show_collision_shapes {
        return;
    }

    let current_time = time.elapsed_secs();

    for (rb, transform, collision_state, collider_component) in rigid_bodies.iter() {
        let position = transform.translation;

        // Determine color based on collision state
        let base_color = if let Some(state) = collision_state {
            if collision_viz.highlight_colliding
                && state.is_recently_colliding(current_time, collision_viz.collision_fade_time)
            {
                // Red for recent collision
                let fade_factor =
                    (current_time - state.last_collision_time) / collision_viz.collision_fade_time;
                Color::srgb(1.0, 1.0 - fade_factor, 1.0 - fade_factor)
            } else {
                Color::srgb(0.0, 1.0, 0.0) // Green when not colliding
            }
        } else {
            Color::srgb(0.0, 0.5, 1.0) // Blue default
        };

        // Draw collision shape based on actual collider type
        if collision_viz.show_collision_shapes {
            draw_collider_shape(
                &mut gizmos,
                &physics_world,
                collider_component,
                transform,
                base_color.with_alpha(0.4),
            );
        }

        // Draw AABB (Axis-Aligned Bounding Box) from physics world
        if collision_viz.show_aabb {
            draw_collider_aabb(
                &mut gizmos,
                &physics_world,
                collider_component,
                transform,
            );
        }

        // Show sleeping indicator
        if collision_viz.show_sleeping_bodies {
            if let Some(rigid_body) = physics_world.rigid_body_set.get(rb.handle) {
                if rigid_body.is_sleeping() {
                    gizmos.sphere(
                        Isometry3d::new(
                            position + Vec3::new(0.0, transform.scale.y * 0.6, 0.0),
                            Quat::IDENTITY,
                        ),
                        0.1,
                        Color::srgb(0.5, 0.5, 0.5), // Gray for sleeping
                    );
                }
            }
        }
    }
}

/// Draw the actual collision shape based on collider type
fn draw_collider_shape(
    gizmos: &mut Gizmos,
    physics_world: &PhysicsWorld,
    collider_component: Option<&PhysicsCollider>,
    transform: &Transform,
    color: Color,
) {
    let position = transform.translation;
    let rotation = transform.rotation;

    // Try to get actual collider shape from physics world
    if let Some(collider_comp) = collider_component {
        if let Some(collider) = physics_world.collider_set.get(collider_comp.handle) {
            let shape = collider.shape();
            let collider_pos = collider.position();

            // Get world position from collider
            let world_pos = Vec3::new(
                collider_pos.translation.x,
                collider_pos.translation.y,
                collider_pos.translation.z,
            );
            let world_rot = Quat::from_xyzw(
                collider_pos.rotation.i,
                collider_pos.rotation.j,
                collider_pos.rotation.k,
                collider_pos.rotation.w,
            );

            match shape.shape_type() {
                ShapeType::Ball => {
                    // Sphere shape
                    if let Some(ball) = shape.as_ball() {
                        gizmos.sphere(
                            Isometry3d::new(world_pos, world_rot),
                            ball.radius,
                            color,
                        );
                    }
                }
                ShapeType::Cuboid => {
                    // Box shape
                    if let Some(cuboid) = shape.as_cuboid() {
                        let half_extents = cuboid.half_extents;
                        gizmos.cuboid(
                            Transform::from_translation(world_pos)
                                .with_rotation(world_rot)
                                .with_scale(Vec3::new(
                                    half_extents.x * 2.0,
                                    half_extents.y * 2.0,
                                    half_extents.z * 2.0,
                                )),
                            color,
                        );
                    }
                }
                ShapeType::Capsule => {
                    // Capsule shape - draw as cylinder + two spheres
                    if let Some(capsule) = shape.as_capsule() {
                        let half_height = capsule.half_height();
                        let radius = capsule.radius;

                        // Draw cylinder body (approximated)
                        draw_cylinder_gizmo(gizmos, world_pos, world_rot, half_height, radius, color);

                        // Draw sphere caps
                        let up = world_rot * Vec3::Y;
                        gizmos.sphere(
                            Isometry3d::new(world_pos + up * half_height, world_rot),
                            radius,
                            color,
                        );
                        gizmos.sphere(
                            Isometry3d::new(world_pos - up * half_height, world_rot),
                            radius,
                            color,
                        );
                    }
                }
                ShapeType::Cylinder => {
                    // Cylinder shape
                    if let Some(cylinder) = shape.as_cylinder() {
                        let half_height = cylinder.half_height;
                        let radius = cylinder.radius;
                        draw_cylinder_gizmo(gizmos, world_pos, world_rot, half_height, radius, color);
                    }
                }
                ShapeType::TriMesh => {
                    // Mesh collider - draw AABB as approximation
                    let aabb = shape.compute_local_aabb();
                    let center = Vec3::new(
                        (aabb.mins.x + aabb.maxs.x) * 0.5,
                        (aabb.mins.y + aabb.maxs.y) * 0.5,
                        (aabb.mins.z + aabb.maxs.z) * 0.5,
                    );
                    let size = Vec3::new(
                        aabb.maxs.x - aabb.mins.x,
                        aabb.maxs.y - aabb.mins.y,
                        aabb.maxs.z - aabb.mins.z,
                    );
                    let world_center = world_pos + world_rot * center;
                    gizmos.cuboid(
                        Transform::from_translation(world_center)
                            .with_rotation(world_rot)
                            .with_scale(size),
                        color.with_alpha(0.2), // More transparent for mesh
                    );
                }
                ShapeType::ConvexPolyhedron => {
                    // Convex hull - draw AABB as approximation
                    let aabb = shape.compute_local_aabb();
                    let center = Vec3::new(
                        (aabb.mins.x + aabb.maxs.x) * 0.5,
                        (aabb.mins.y + aabb.maxs.y) * 0.5,
                        (aabb.mins.z + aabb.maxs.z) * 0.5,
                    );
                    let size = Vec3::new(
                        aabb.maxs.x - aabb.mins.x,
                        aabb.maxs.y - aabb.mins.y,
                        aabb.maxs.z - aabb.mins.z,
                    );
                    let world_center = world_pos + world_rot * center;
                    gizmos.cuboid(
                        Transform::from_translation(world_center)
                            .with_rotation(world_rot)
                            .with_scale(size),
                        color,
                    );
                }
                _ => {
                    // Fallback: draw transform-based box for unknown shapes
                    gizmos.cuboid(
                        Transform::from_translation(position)
                            .with_rotation(rotation)
                            .with_scale(transform.scale),
                        color,
                    );
                }
            }
            return;
        }
    }

    // Fallback: no collider component, use transform-based visualization
    gizmos.cuboid(
        Transform::from_translation(position)
            .with_rotation(rotation)
            .with_scale(transform.scale),
        color,
    );
}

/// Draw a cylinder gizmo using line segments
fn draw_cylinder_gizmo(
    gizmos: &mut Gizmos,
    position: Vec3,
    rotation: Quat,
    half_height: f32,
    radius: f32,
    color: Color,
) {
    let segments = 16;
    let up = rotation * Vec3::Y;
    let right = rotation * Vec3::X;
    let forward = rotation * Vec3::Z;

    let top_center = position + up * half_height;
    let bottom_center = position - up * half_height;

    // Draw top and bottom circles
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

        let offset1 = right * angle1.cos() * radius + forward * angle1.sin() * radius;
        let offset2 = right * angle2.cos() * radius + forward * angle2.sin() * radius;

        // Top circle
        gizmos.line(top_center + offset1, top_center + offset2, color);
        // Bottom circle
        gizmos.line(bottom_center + offset1, bottom_center + offset2, color);
        // Vertical lines (every 4 segments)
        if i % 4 == 0 {
            gizmos.line(top_center + offset1, bottom_center + offset1, color);
        }
    }
}

/// Draw AABB from actual collider
fn draw_collider_aabb(
    gizmos: &mut Gizmos,
    physics_world: &PhysicsWorld,
    collider_component: Option<&PhysicsCollider>,
    transform: &Transform,
) {
    let aabb_color = Color::srgb(1.0, 1.0, 0.0).with_alpha(0.2); // Yellow AABB

    if let Some(collider_comp) = collider_component {
        if let Some(collider) = physics_world.collider_set.get(collider_comp.handle) {
            // Get world-space AABB from collider
            let aabb = collider.compute_aabb();
            let min = Vec3::new(aabb.mins.x, aabb.mins.y, aabb.mins.z);
            let max = Vec3::new(aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);

            let aabb_center = (min + max) * 0.5;
            let aabb_size = max - min;

            gizmos.cuboid(
                Transform::from_translation(aabb_center).with_scale(aabb_size),
                aabb_color,
            );
            return;
        }
    }

    // Fallback: compute AABB from transform
    let position = transform.translation;
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

    let world_corners: Vec<Vec3> = corners
        .iter()
        .map(|&c| position + transform.rotation * c)
        .collect();

    let mut min = world_corners[0];
    let mut max = world_corners[0];
    for &corner in &world_corners {
        min = min.min(corner);
        max = max.max(corner);
    }

    let aabb_center = (min + max) * 0.5;
    let aabb_size = max - min;

    gizmos.cuboid(
        Transform::from_translation(aabb_center).with_scale(aabb_size),
        aabb_color,
    );
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
        app.init_resource::<CollisionVisualization>().add_systems(
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
