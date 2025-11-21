use bevy::prelude::*;

/// Extended gizmo utilities for advanced debug rendering
pub struct GizmoUtils;

impl GizmoUtils {
    /// Draw a coordinate frame (X=red, Y=green, Z=blue axes)
    pub fn draw_frame(gizmos: &mut Gizmos, position: Vec3, rotation: Quat, size: f32) {
        // X axis - Red
        let x_end = position + rotation * (Vec3::X * size);
        gizmos.arrow(position, x_end, Color::srgb(1.0, 0.0, 0.0));

        // Y axis - Green
        let y_end = position + rotation * (Vec3::Y * size);
        gizmos.arrow(position, y_end, Color::srgb(0.0, 1.0, 0.0));

        // Z axis - Blue
        let z_end = position + rotation * (Vec3::Z * size);
        gizmos.arrow(position, z_end, Color::srgb(0.0, 0.0, 1.0));
    }

    /// Draw a labeled coordinate frame with axis labels
    pub fn draw_labeled_frame(
        gizmos: &mut Gizmos,
        position: Vec3,
        rotation: Quat,
        size: f32,
        label: Option<&str>,
    ) {
        Self::draw_frame(gizmos, position, rotation, size);

        // Note: Text rendering requires additional Bevy UI setup
        // For now, draw a small sphere at origin to indicate a labeled frame
        if let Some(_label) = label {
            gizmos.sphere(
                Isometry3d::new(position, Quat::IDENTITY),
                size * 0.1,
                Color::srgb(1.0, 1.0, 0.0),
            );
        }
    }

    /// Draw a cylinder
    pub fn draw_cylinder(
        gizmos: &mut Gizmos,
        start: Vec3,
        end: Vec3,
        radius: f32,
        color: Color,
        segments: usize,
    ) {
        let direction = (end - start).normalize();
        let length = (end - start).length();

        // Create perpendicular vectors
        let up = if direction.dot(Vec3::Y).abs() < 0.99 {
            Vec3::Y
        } else {
            Vec3::X
        };
        let right = direction.cross(up).normalize();
        let forward = right.cross(direction).normalize();

        // Draw circles at both ends
        for i in 0..segments {
            let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

            let offset1 = right * angle1.cos() + forward * angle1.sin();
            let offset2 = right * angle2.cos() + forward * angle2.sin();

            // Start circle
            let p1_start = start + offset1 * radius;
            let p2_start = start + offset2 * radius;
            gizmos.line(p1_start, p2_start, color);

            // End circle
            let p1_end = end + offset1 * radius;
            let p2_end = end + offset2 * radius;
            gizmos.line(p1_end, p2_end, color);

            // Connect lines
            gizmos.line(p1_start, p1_end, color);
        }
    }

    /// Draw a cone
    pub fn draw_cone(
        gizmos: &mut Gizmos,
        apex: Vec3,
        base_center: Vec3,
        radius: f32,
        color: Color,
        segments: usize,
    ) {
        let direction = (base_center - apex).normalize();

        // Create perpendicular vectors
        let up = if direction.dot(Vec3::Y).abs() < 0.99 {
            Vec3::Y
        } else {
            Vec3::X
        };
        let right = direction.cross(up).normalize();
        let forward = right.cross(direction).normalize();

        // Draw base circle and lines from apex to base
        for i in 0..segments {
            let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

            let offset1 = right * angle1.cos() + forward * angle1.sin();
            let offset2 = right * angle2.cos() + forward * angle2.sin();

            let p1 = base_center + offset1 * radius;
            let p2 = base_center + offset2 * radius;

            // Base circle
            gizmos.line(p1, p2, color);

            // Lines from apex to base
            gizmos.line(apex, p1, color);
        }
    }

    /// Draw a capsule (cylinder with hemispherical caps)
    pub fn draw_capsule(gizmos: &mut Gizmos, start: Vec3, end: Vec3, radius: f32, color: Color) {
        // Draw central cylinder
        Self::draw_cylinder(gizmos, start, end, radius, color, 16);

        // Draw caps as spheres
        gizmos.sphere(Isometry3d::new(start, Quat::IDENTITY), radius, color);
        gizmos.sphere(Isometry3d::new(end, Quat::IDENTITY), radius, color);
    }

    /// Draw a frustum (truncated pyramid)
    pub fn draw_frustum(
        gizmos: &mut Gizmos,
        apex: Vec3,
        direction: Vec3,
        near_distance: f32,
        far_distance: f32,
        near_size: Vec2,
        far_size: Vec2,
        color: Color,
    ) {
        let dir = direction.normalize();
        let right = if dir.dot(Vec3::Y).abs() < 0.99 {
            dir.cross(Vec3::Y).normalize()
        } else {
            dir.cross(Vec3::X).normalize()
        };
        let up = right.cross(dir);

        // Near plane corners
        let near_center = apex + dir * near_distance;
        let near_corners = [
            near_center + right * near_size.x * 0.5 + up * near_size.y * 0.5,
            near_center - right * near_size.x * 0.5 + up * near_size.y * 0.5,
            near_center - right * near_size.x * 0.5 - up * near_size.y * 0.5,
            near_center + right * near_size.x * 0.5 - up * near_size.y * 0.5,
        ];

        // Far plane corners
        let far_center = apex + dir * far_distance;
        let far_corners = [
            far_center + right * far_size.x * 0.5 + up * far_size.y * 0.5,
            far_center - right * far_size.x * 0.5 + up * far_size.y * 0.5,
            far_center - right * far_size.x * 0.5 - up * far_size.y * 0.5,
            far_center + right * far_size.x * 0.5 - up * far_size.y * 0.5,
        ];

        // Draw near and far rectangles
        for i in 0..4 {
            let next = (i + 1) % 4;
            gizmos.line(near_corners[i], near_corners[next], color);
            gizmos.line(far_corners[i], far_corners[next], color);
            gizmos.line(near_corners[i], far_corners[i], color);
        }
    }

    /// Draw a grid on a plane
    pub fn draw_grid(
        gizmos: &mut Gizmos,
        center: Vec3,
        normal: Vec3,
        size: f32,
        divisions: usize,
        color: Color,
    ) {
        let normal = normal.normalize();
        let right = if normal.dot(Vec3::Y).abs() < 0.99 {
            normal.cross(Vec3::Y).normalize()
        } else {
            normal.cross(Vec3::X).normalize()
        };
        let forward = right.cross(normal);

        let cell_size = size / divisions as f32;
        let half_size = size * 0.5;

        // Draw grid lines
        for i in 0..=divisions {
            let offset = -half_size + (i as f32 * cell_size);

            // Lines parallel to right vector
            let start1 = center + forward * offset - right * half_size;
            let end1 = center + forward * offset + right * half_size;
            gizmos.line(start1, end1, color);

            // Lines parallel to forward vector
            let start2 = center + right * offset - forward * half_size;
            let end2 = center + right * offset + forward * half_size;
            gizmos.line(start2, end2, color);
        }
    }

    /// Draw a path as connected line segments
    pub fn draw_path(gizmos: &mut Gizmos, points: &[Vec3], color: Color, closed: bool) {
        if points.len() < 2 {
            return;
        }

        for i in 0..points.len() - 1 {
            gizmos.line(points[i], points[i + 1], color);
        }

        if closed && points.len() > 2 {
            gizmos.line(points[points.len() - 1], points[0], color);
        }
    }

    /// Draw a dashed line
    pub fn draw_dashed_line(
        gizmos: &mut Gizmos,
        start: Vec3,
        end: Vec3,
        dash_length: f32,
        gap_length: f32,
        color: Color,
    ) {
        let direction = end - start;
        let total_length = direction.length();
        let dir_normalized = direction.normalize();
        let segment_length = dash_length + gap_length;

        let mut current_distance = 0.0;
        while current_distance < total_length {
            let dash_start = start + dir_normalized * current_distance;
            let dash_end_distance = (current_distance + dash_length).min(total_length);
            let dash_end = start + dir_normalized * dash_end_distance;

            gizmos.line(dash_start, dash_end, color);

            current_distance += segment_length;
        }
    }

    /// Draw an arc
    pub fn draw_arc(
        gizmos: &mut Gizmos,
        center: Vec3,
        normal: Vec3,
        start_direction: Vec3,
        angle: f32,
        radius: f32,
        color: Color,
        segments: usize,
    ) {
        let normal = normal.normalize();
        let start_dir = start_direction.normalize();

        for i in 0..segments {
            let angle1 = (i as f32 / segments as f32) * angle;
            let angle2 = ((i + 1) as f32 / segments as f32) * angle;

            let rot1 = Quat::from_axis_angle(normal, angle1);
            let rot2 = Quat::from_axis_angle(normal, angle2);

            let p1 = center + rot1 * start_dir * radius;
            let p2 = center + rot2 * start_dir * radius;

            gizmos.line(p1, p2, color);
        }
    }

    /// Draw a wireframe box
    pub fn draw_wireframe_box(
        gizmos: &mut Gizmos,
        center: Vec3,
        size: Vec3,
        rotation: Quat,
        color: Color,
    ) {
        let half_size = size * 0.5;

        // Define 8 corners of the box
        let corners = [
            Vec3::new(-half_size.x, -half_size.y, -half_size.z),
            Vec3::new(half_size.x, -half_size.y, -half_size.z),
            Vec3::new(half_size.x, half_size.y, -half_size.z),
            Vec3::new(-half_size.x, half_size.y, -half_size.z),
            Vec3::new(-half_size.x, -half_size.y, half_size.z),
            Vec3::new(half_size.x, -half_size.y, half_size.z),
            Vec3::new(half_size.x, half_size.y, half_size.z),
            Vec3::new(-half_size.x, half_size.y, half_size.z),
        ];

        // Transform corners
        let transformed: Vec<Vec3> = corners
            .iter()
            .map(|&corner| center + rotation * corner)
            .collect();

        // Draw 12 edges
        let edges = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0), // Bottom face
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4), // Top face
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7), // Vertical edges
        ];

        for (i, j) in edges {
            gizmos.line(transformed[i], transformed[j], color);
        }
    }

    /// Draw a 3D cross (XYZ axes marker)
    pub fn draw_cross_3d(gizmos: &mut Gizmos, center: Vec3, size: f32, color: Color) {
        gizmos.line(center - Vec3::X * size, center + Vec3::X * size, color);
        gizmos.line(center - Vec3::Y * size, center + Vec3::Y * size, color);
        gizmos.line(center - Vec3::Z * size, center + Vec3::Z * size, color);
    }

    /// Draw a billboard (always faces camera) - simplified version
    pub fn draw_billboard_cross(gizmos: &mut Gizmos, position: Vec3, size: f32, color: Color) {
        // Draw a simple cross marker
        Self::draw_cross_3d(gizmos, position, size, color);
    }
}

/// Persistent gizmo storage for drawing across frames
#[derive(Resource, Default)]
pub struct PersistentGizmos {
    lines: Vec<(Vec3, Vec3, Color, f32)>, // start, end, color, lifetime
}

impl PersistentGizmos {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a persistent line
    pub fn add_line(&mut self, start: Vec3, end: Vec3, color: Color, lifetime: f32) {
        self.lines.push((start, end, color, lifetime));
    }

    /// Update and draw persistent gizmos
    pub fn update_and_draw(&mut self, gizmos: &mut Gizmos, delta_time: f32) {
        // Update lifetimes and remove expired
        self.lines.retain_mut(|(start, end, color, lifetime)| {
            *lifetime -= delta_time;
            if *lifetime > 0.0 {
                // Draw with fading alpha
                let alpha = (*lifetime).min(1.0);
                gizmos.line(*start, *end, color.with_alpha(alpha));
                true
            } else {
                false
            }
        });
    }

    /// Clear all persistent gizmos
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Get count of active gizmos
    pub fn count(&self) -> usize {
        self.lines.len()
    }
}

/// System to update persistent gizmos
pub fn persistent_gizmos_system(
    mut gizmos: Gizmos,
    mut persistent: ResMut<PersistentGizmos>,
    time: Res<Time>,
) {
    persistent.update_and_draw(&mut gizmos, time.delta_secs());
}

/// Plugin to register gizmo systems
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PersistentGizmos>()
            .add_systems(Update, persistent_gizmos_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistent_gizmos() {
        let mut persistent = PersistentGizmos::new();
        assert_eq!(persistent.count(), 0);

        persistent.add_line(Vec3::ZERO, Vec3::X, Color::WHITE, 1.0);

        assert_eq!(persistent.count(), 1);

        persistent.clear();
        assert_eq!(persistent.count(), 0);
    }
}
