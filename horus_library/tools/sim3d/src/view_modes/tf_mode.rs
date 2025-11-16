use bevy::prelude::*;

use crate::systems::tf_update::TFPublisher;
use crate::ui::controls::SimulationControls;
use crate::ui::tf_panel::TFPanelConfig;

/// Resource to control TF frame visualization
#[derive(Resource, Clone)]
pub struct TFVisualization {
    pub enabled: bool,
    pub show_frame_axes: bool,
    pub show_frame_labels: bool,
    pub show_parent_links: bool,
    pub show_world_frame: bool,
    pub axis_length: f32,
    pub axis_thickness: f32,
    pub highlight_selected: bool,
}

impl Default for TFVisualization {
    fn default() -> Self {
        Self {
            enabled: true,
            show_frame_axes: true,
            show_frame_labels: false, // Text rendering would need additional setup
            show_parent_links: true,
            show_world_frame: true,
            axis_length: 0.2,
            axis_thickness: 0.01,
            highlight_selected: true,
        }
    }
}

impl TFVisualization {
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

    pub fn with_axis_length(mut self, length: f32) -> Self {
        self.axis_length = length;
        self
    }
}

/// System to visualize TF frames
pub fn tf_frame_visualization_system(
    mut gizmos: Gizmos,
    tf_viz: Res<TFVisualization>,
    controls: Res<SimulationControls>,
    tf_panel: Option<Res<TFPanelConfig>>,
    publishers: Query<(&TFPublisher, &Transform)>,
) {
    if !tf_viz.enabled && !controls.show_tf_frames {
        return;
    }

    // Draw world frame
    if tf_viz.show_world_frame {
        let world_axis_length = tf_viz.axis_length * 2.0; // Larger for world frame

        // X axis - Red
        gizmos.arrow(
            Vec3::ZERO,
            Vec3::new(world_axis_length, 0.0, 0.0),
            Color::srgb(1.0, 0.0, 0.0),
        );

        // Y axis - Green
        gizmos.arrow(
            Vec3::ZERO,
            Vec3::new(0.0, world_axis_length, 0.0),
            Color::srgb(0.0, 1.0, 0.0),
        );

        // Z axis - Blue
        gizmos.arrow(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, world_axis_length),
            Color::srgb(0.0, 0.0, 1.0),
        );
    }

    // Draw each TF frame
    for (publisher, transform) in publishers.iter() {
        let position = transform.translation;

        // Check if this frame is selected
        let is_selected = if let Some(panel) = &tf_panel {
            tf_viz.highlight_selected && panel.is_selected(&publisher.frame_name)
        } else {
            false
        };

        // Scale and brighten axes if selected
        let axis_length = if is_selected {
            tf_viz.axis_length * 1.5
        } else {
            tf_viz.axis_length
        };

        let alpha = if is_selected { 1.0 } else { 0.8 };

        // Draw frame axes
        if tf_viz.show_frame_axes {
            // X axis - Red
            let x_axis = transform.rotation * Vec3::X * axis_length;
            gizmos.arrow(
                position,
                position + x_axis,
                Color::srgb(1.0, 0.0, 0.0).with_alpha(alpha),
            );

            // Y axis - Green
            let y_axis = transform.rotation * Vec3::Y * axis_length;
            gizmos.arrow(
                position,
                position + y_axis,
                Color::srgb(0.0, 1.0, 0.0).with_alpha(alpha),
            );

            // Z axis - Blue
            let z_axis = transform.rotation * Vec3::Z * axis_length;
            gizmos.arrow(
                position,
                position + z_axis,
                Color::srgb(0.0, 0.0, 1.0).with_alpha(alpha),
            );
        }

        // Draw selection indicator
        if is_selected {
            // Draw a sphere at the frame origin
            gizmos.sphere(
                Isometry3d::new(position, Quat::IDENTITY),
                tf_viz.axis_length * 0.3,
                Color::srgb(1.0, 1.0, 0.0), // Yellow
            );
        }
    }
}

/// System to visualize parent-child links in TF tree
pub fn tf_links_visualization_system(
    mut gizmos: Gizmos,
    tf_viz: Res<TFVisualization>,
    publishers: Query<(&TFPublisher, &Transform)>,
) {
    if !tf_viz.enabled || !tf_viz.show_parent_links {
        return;
    }

    // Build a map of frame names to positions
    let mut frame_positions: std::collections::HashMap<String, Vec3> = std::collections::HashMap::new();

    for (publisher, transform) in publishers.iter() {
        frame_positions.insert(publisher.frame_name.clone(), transform.translation);
    }

    // Draw links from each frame to its parent
    for (publisher, transform) in publishers.iter() {
        if publisher.parent_frame.is_empty() || publisher.parent_frame == "world" {
            // Link to world origin
            if tf_viz.show_world_frame {
                gizmos.line(
                    Vec3::ZERO,
                    transform.translation,
                    Color::srgb(0.5, 0.5, 0.5).with_alpha(0.3),
                );
            }
        } else if let Some(&parent_pos) = frame_positions.get(&publisher.parent_frame) {
            // Link to parent frame
            gizmos.line(
                parent_pos,
                transform.translation,
                Color::srgb(0.3, 0.6, 0.9).with_alpha(0.5),
            );
        }
    }
}

/// System to visualize TF frame trails (history of positions)
#[derive(Component)]
pub struct TFTrail {
    pub frame_name: String,
    pub positions: Vec<Vec3>,
    pub max_length: usize,
    pub color: Color,
}

impl TFTrail {
    pub fn new(frame_name: impl Into<String>, max_length: usize) -> Self {
        Self {
            frame_name: frame_name.into(),
            positions: Vec::new(),
            max_length,
            color: Color::srgb(0.5, 0.5, 1.0),
        }
    }

    pub fn add_position(&mut self, position: Vec3) {
        self.positions.push(position);
        if self.positions.len() > self.max_length {
            self.positions.remove(0);
        }
    }
}

/// System to update and visualize TF trails
pub fn tf_trail_system(
    mut gizmos: Gizmos,
    tf_viz: Res<TFVisualization>,
    mut trails: Query<&mut TFTrail>,
    publishers: Query<(&TFPublisher, &Transform)>,
) {
    if !tf_viz.enabled {
        return;
    }

    // Update trails with current positions
    for mut trail in trails.iter_mut() {
        for (publisher, transform) in publishers.iter() {
            if publisher.frame_name == trail.frame_name {
                trail.add_position(transform.translation);
                break;
            }
        }

        // Draw the trail
        if trail.positions.len() >= 2 {
            for i in 0..trail.positions.len() - 1 {
                // Fade older positions
                let alpha = (i as f32 / trail.positions.len() as f32) * 0.8;
                gizmos.line(
                    trail.positions[i],
                    trail.positions[i + 1],
                    trail.color.with_alpha(alpha),
                );
            }
        }
    }
}

/// System to visualize TF transform chains
pub fn tf_chain_visualization_system(
    mut gizmos: Gizmos,
    tf_viz: Res<TFVisualization>,
    tf_panel: Option<Res<TFPanelConfig>>,
    publishers: Query<(&TFPublisher, &Transform)>,
) {
    if !tf_viz.enabled {
        return;
    }

    // If a frame is selected, highlight its entire chain to root
    if let Some(panel) = tf_panel {
        if let Some(selected_frame) = &panel.selected_frame {
            // Build parent chain
            let mut current_frame = selected_frame.clone();
            let mut chain_positions = Vec::new();

            // Build frame map
            let mut frame_map: std::collections::HashMap<String, (String, Vec3)> = std::collections::HashMap::new();
            for (publisher, transform) in publishers.iter() {
                frame_map.insert(
                    publisher.frame_name.clone(),
                    (publisher.parent_frame.clone(), transform.translation),
                );
            }

            // Traverse up to root
            chain_positions.push(
                frame_map
                    .get(&current_frame)
                    .map(|(_, pos)| *pos)
                    .unwrap_or(Vec3::ZERO),
            );

            for _ in 0..20 {
                // Max depth limit
                if let Some((parent, pos)) = frame_map.get(&current_frame) {
                    if parent.is_empty() || parent == "world" {
                        chain_positions.push(Vec3::ZERO);
                        break;
                    }
                    current_frame = parent.clone();
                    chain_positions.push(*pos);
                } else {
                    break;
                }
            }

            // Draw highlighted chain
            for i in 0..chain_positions.len() - 1 {
                gizmos.line(
                    chain_positions[i],
                    chain_positions[i + 1],
                    Color::srgb(1.0, 1.0, 0.0).with_alpha(0.8), // Yellow
                );
            }
        }
    }
}

/// Keyboard shortcuts for TF visualization
pub fn tf_viz_keyboard_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tf_viz: ResMut<TFVisualization>,
) {
    // T: Toggle TF visualization
    if keyboard.just_pressed(KeyCode::KeyT) && keyboard.pressed(KeyCode::ControlLeft) {
        tf_viz.toggle();
    }

    // Shift+T: Toggle frame axes
    if keyboard.just_pressed(KeyCode::KeyT) && keyboard.pressed(KeyCode::ShiftLeft) {
        tf_viz.show_frame_axes = !tf_viz.show_frame_axes;
    }

    // Shift+L: Toggle parent links
    if keyboard.just_pressed(KeyCode::KeyL) && keyboard.pressed(KeyCode::ControlLeft) {
        tf_viz.show_parent_links = !tf_viz.show_parent_links;
    }

    // Shift+W: Toggle world frame
    if keyboard.just_pressed(KeyCode::KeyW) && keyboard.pressed(KeyCode::ControlLeft) {
        tf_viz.show_world_frame = !tf_viz.show_world_frame;
    }
}

/// Plugin to register TF visualization systems
pub struct TFVisualizationPlugin;

impl Plugin for TFVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TFVisualization>()
            .add_systems(
                Update,
                (
                    tf_frame_visualization_system,
                    tf_links_visualization_system,
                    tf_trail_system,
                    tf_chain_visualization_system,
                    tf_viz_keyboard_system,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tf_visualization() {
        let mut viz = TFVisualization::new();
        assert!(viz.enabled);

        viz.toggle();
        assert!(!viz.enabled);

        viz.disable();
        assert!(!viz.enabled);

        viz.enable();
        assert!(viz.enabled);
    }

    #[test]
    fn test_tf_trail() {
        let mut trail = TFTrail::new("test_frame", 10);
        assert_eq!(trail.positions.len(), 0);

        for i in 0..15 {
            trail.add_position(Vec3::new(i as f32, 0.0, 0.0));
        }

        // Should be limited to max_length
        assert_eq!(trail.positions.len(), 10);
        assert_eq!(trail.positions[0], Vec3::new(5.0, 0.0, 0.0)); // Oldest kept
        assert_eq!(trail.positions[9], Vec3::new(14.0, 0.0, 0.0)); // Newest
    }

    #[test]
    fn test_default_settings() {
        let viz = TFVisualization::default();
        assert!(viz.show_frame_axes);
        assert!(viz.show_parent_links);
        assert!(viz.show_world_frame);
        assert_eq!(viz.axis_length, 0.2);
    }
}
