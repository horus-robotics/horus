use crate::tf::TFTree;
use bevy::prelude::*;

#[derive(Resource)]
pub struct TFVisualizer {
    pub enabled: bool,
    pub axis_length: f32,
    pub show_labels: bool,
    pub filter: TFFilter,
}

impl Default for TFVisualizer {
    fn default() -> Self {
        Self {
            enabled: false,
            axis_length: 0.3,
            show_labels: true,
            filter: TFFilter::All,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TFFilter {
    All,
    OnlyRobot,
    OnlySensors,
    Custom(Vec<String>),
}

pub fn render_tf_frames(
    mut gizmos: Gizmos,
    tf_tree: Res<TFTree>,
    config: Option<Res<TFVisualizer>>,
) {
    let config = match config {
        Some(cfg) => cfg,
        None => return,
    };

    if !config.enabled {
        return;
    }

    for (name, frame) in tf_tree.frames.iter() {
        if !should_show_frame(name, &config.filter) {
            continue;
        }

        let transform = isometry_to_bevy_transform(&frame.transform);
        let pos = transform.translation;

        let axis_len = config.axis_length;

        gizmos.arrow(
            pos,
            pos + transform.rotation * Vec3::X * axis_len,
            Color::srgb(1.0, 0.0, 0.0),
        );

        gizmos.arrow(
            pos,
            pos + transform.rotation * Vec3::Y * axis_len,
            Color::srgb(0.0, 1.0, 0.0),
        );

        gizmos.arrow(
            pos,
            pos + transform.rotation * Vec3::Z * axis_len,
            Color::srgb(0.0, 0.0, 1.0),
        );
    }
}

fn isometry_to_bevy_transform(iso: &nalgebra::Isometry3<f32>) -> Transform {
    let translation = Vec3::new(iso.translation.x, iso.translation.y, iso.translation.z);

    let rotation = Quat::from_xyzw(
        iso.rotation.i,
        iso.rotation.j,
        iso.rotation.k,
        iso.rotation.w,
    );

    Transform {
        translation,
        rotation,
        scale: Vec3::ONE,
    }
}

fn should_show_frame(name: &str, filter: &TFFilter) -> bool {
    match filter {
        TFFilter::All => true,
        TFFilter::OnlyRobot => !name.contains("sensor") && name != "world",
        TFFilter::OnlySensors => name.contains("sensor") || name.contains("camera"),
        TFFilter::Custom(names) => names.contains(&name.to_string()),
    }
}
