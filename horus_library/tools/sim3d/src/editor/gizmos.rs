//! Transform gizmos for visual manipulation

use super::{selection::Selection, EditorState, GizmoMode};
use bevy::prelude::*;

/// Gizmo visualization component
#[derive(Component)]
pub struct TransformGizmo {
    pub entity: Entity,
    pub mode: GizmoMode,
}

/// System to update and render transform gizmos
pub fn gizmo_system(
    state: Res<EditorState>,
    selection: Res<Selection>,
    mut gizmos: Gizmos,
    transforms: Query<&GlobalTransform>,
) {
    if state.gizmo_mode == GizmoMode::None {
        return;
    }

    // Draw gizmo for primary selection
    if let Some(entity) = selection.primary {
        if let Ok(transform) = transforms.get(entity) {
            let position = transform.translation();
            let rotation = transform.to_scale_rotation_translation().1;

            match state.gizmo_mode {
                GizmoMode::Translate => draw_translation_gizmo(&mut gizmos, position, rotation),
                GizmoMode::Rotate => draw_rotation_gizmo(&mut gizmos, position, rotation),
                GizmoMode::Scale => draw_scale_gizmo(&mut gizmos, position, rotation),
                GizmoMode::None => {}
            }
        }
    }
}

/// Draw translation gizmo (3 arrows)
fn draw_translation_gizmo(gizmos: &mut Gizmos, position: Vec3, rotation: Quat) {
    let length = 1.0;

    // X axis (red)
    let x_dir = rotation * Vec3::X;
    gizmos.arrow(
        position,
        position + x_dir * length,
        Color::srgb(1.0, 0.0, 0.0),
    );

    // Y axis (green)
    let y_dir = rotation * Vec3::Y;
    gizmos.arrow(
        position,
        position + y_dir * length,
        Color::srgb(0.0, 1.0, 0.0),
    );

    // Z axis (blue)
    let z_dir = rotation * Vec3::Z;
    gizmos.arrow(
        position,
        position + z_dir * length,
        Color::srgb(0.0, 0.0, 1.0),
    );
}

/// Draw rotation gizmo (3 circles)
fn draw_rotation_gizmo(gizmos: &mut Gizmos, position: Vec3, rotation: Quat) {
    let radius = 1.0;
    let segments = 32;

    // X axis circle (red)
    draw_circle(
        gizmos,
        position,
        rotation * Vec3::X,
        radius,
        segments,
        Color::srgb(1.0, 0.0, 0.0),
    );

    // Y axis circle (green)
    draw_circle(
        gizmos,
        position,
        rotation * Vec3::Y,
        radius,
        segments,
        Color::srgb(0.0, 1.0, 0.0),
    );

    // Z axis circle (blue)
    draw_circle(
        gizmos,
        position,
        rotation * Vec3::Z,
        radius,
        segments,
        Color::srgb(0.0, 0.0, 1.0),
    );
}

/// Draw scale gizmo (3 axes with cubes)
fn draw_scale_gizmo(gizmos: &mut Gizmos, position: Vec3, rotation: Quat) {
    let length = 1.0;
    let cube_size = 0.1;

    // X axis (red)
    let x_dir = rotation * Vec3::X;
    gizmos.line(
        position,
        position + x_dir * length,
        Color::srgb(1.0, 0.0, 0.0),
    );
    draw_cube(
        gizmos,
        position + x_dir * length,
        cube_size,
        Color::srgb(1.0, 0.0, 0.0),
    );

    // Y axis (green)
    let y_dir = rotation * Vec3::Y;
    gizmos.line(
        position,
        position + y_dir * length,
        Color::srgb(0.0, 1.0, 0.0),
    );
    draw_cube(
        gizmos,
        position + y_dir * length,
        cube_size,
        Color::srgb(0.0, 1.0, 0.0),
    );

    // Z axis (blue)
    let z_dir = rotation * Vec3::Z;
    gizmos.line(
        position,
        position + z_dir * length,
        Color::srgb(0.0, 0.0, 1.0),
    );
    draw_cube(
        gizmos,
        position + z_dir * length,
        cube_size,
        Color::srgb(0.0, 0.0, 1.0),
    );
}

/// Helper to draw a circle
fn draw_circle(
    gizmos: &mut Gizmos,
    center: Vec3,
    normal: Vec3,
    radius: f32,
    segments: usize,
    color: Color,
) {
    let perpendicular = if normal.dot(Vec3::Y).abs() < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };

    let tangent = normal.cross(perpendicular).normalize();
    let bitangent = normal.cross(tangent).normalize();

    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

        let p1 = center + (tangent * angle1.cos() + bitangent * angle1.sin()) * radius;
        let p2 = center + (tangent * angle2.cos() + bitangent * angle2.sin()) * radius;

        gizmos.line(p1, p2, color);
    }
}

/// Helper to draw a cube outline
fn draw_cube(gizmos: &mut Gizmos, center: Vec3, size: f32, color: Color) {
    let half_size = size / 2.0;

    let vertices = [
        center + Vec3::new(-half_size, -half_size, -half_size),
        center + Vec3::new(half_size, -half_size, -half_size),
        center + Vec3::new(half_size, half_size, -half_size),
        center + Vec3::new(-half_size, half_size, -half_size),
        center + Vec3::new(-half_size, -half_size, half_size),
        center + Vec3::new(half_size, -half_size, half_size),
        center + Vec3::new(half_size, half_size, half_size),
        center + Vec3::new(-half_size, half_size, half_size),
    ];

    // Bottom face
    gizmos.line(vertices[0], vertices[1], color);
    gizmos.line(vertices[1], vertices[2], color);
    gizmos.line(vertices[2], vertices[3], color);
    gizmos.line(vertices[3], vertices[0], color);

    // Top face
    gizmos.line(vertices[4], vertices[5], color);
    gizmos.line(vertices[5], vertices[6], color);
    gizmos.line(vertices[6], vertices[7], color);
    gizmos.line(vertices[7], vertices[4], color);

    // Vertical edges
    gizmos.line(vertices[0], vertices[4], color);
    gizmos.line(vertices[1], vertices[5], color);
    gizmos.line(vertices[2], vertices[6], color);
    gizmos.line(vertices[3], vertices[7], color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gizmo_component() {
        let entity = Entity::from_raw(1);
        let gizmo = TransformGizmo {
            entity,
            mode: GizmoMode::Translate,
        };

        assert_eq!(gizmo.entity, entity);
        assert_eq!(gizmo.mode, GizmoMode::Translate);
    }
}
