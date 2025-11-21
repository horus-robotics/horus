//! Main editor UI using egui

use super::{EditorState, GizmoMode};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

/// Main editor UI system
pub fn editor_ui_system(mut contexts: EguiContexts, mut state: ResMut<EditorState>) {
    let ctx = contexts.ctx_mut();

    // Top toolbar
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        toolbar_ui(ui, &mut state);
    });

    // Status bar at bottom
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        status_bar_ui(ui, &state);
    });
}

/// Toolbar UI with tool buttons
fn toolbar_ui(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.heading("Scene Editor");

        ui.separator();

        // Gizmo mode buttons
        ui.label("Transform:");
        if ui
            .selectable_label(state.gizmo_mode == GizmoMode::Translate, "Translate")
            .clicked()
        {
            state.gizmo_mode = GizmoMode::Translate;
        }

        if ui
            .selectable_label(state.gizmo_mode == GizmoMode::Rotate, "Rotate")
            .clicked()
        {
            state.gizmo_mode = GizmoMode::Rotate;
        }

        if ui
            .selectable_label(state.gizmo_mode == GizmoMode::Scale, "Scale")
            .clicked()
        {
            state.gizmo_mode = GizmoMode::Scale;
        }

        ui.separator();

        // Snap to grid
        ui.checkbox(&mut state.snap_to_grid, "Snap to Grid");
        if state.snap_to_grid {
            ui.add(
                egui::DragValue::new(&mut state.grid_size)
                    .speed(0.01)
                    .clamp_range(0.01..=10.0)
                    .prefix("Size: "),
            );
        }

        ui.separator();

        // Panel toggles
        ui.checkbox(&mut state.show_hierarchy, "Hierarchy");
        ui.checkbox(&mut state.show_inspector, "Inspector");
    });
}

/// Status bar showing editor state
fn status_bar_ui(ui: &mut egui::Ui, state: &EditorState) {
    ui.horizontal(|ui| {
        ui.label(format!("Mode: {:?}", state.gizmo_mode));
        ui.separator();
        ui.label(format!("Camera: {:?}", state.camera_mode));
        ui.separator();

        if state.snap_to_grid {
            ui.label(format!("Grid: {:.2}m", state.grid_size));
        } else {
            ui.label("Grid: Off");
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label("Press H for help");
        });
    });
}

/// Keyboard shortcuts reference
pub fn show_shortcuts_window(ctx: &egui::Context, show: &mut bool) {
    egui::Window::new("Keyboard Shortcuts")
        .open(show)
        .collapsible(true)
        .show(ctx, |ui| {
            ui.heading("Selection");
            ui.label("Left Click: Select object");
            ui.label("Ctrl+A: Select all");
            ui.label("Escape: Deselect all");

            ui.add_space(10.0);
            ui.heading("Transform");
            ui.label("G: Move (Translate)");
            ui.label("R: Rotate");
            ui.label("S: Scale");

            ui.add_space(10.0);
            ui.heading("Camera");
            ui.label("Middle Mouse: Orbit");
            ui.label("Shift+Middle Mouse: Pan");
            ui.label("Scroll: Zoom");
            ui.label("F: Frame selected");

            ui.add_space(10.0);
            ui.heading("Edit");
            ui.label("Ctrl+Z: Undo");
            ui.label("Ctrl+Shift+Z / Ctrl+Y: Redo");
            ui.label("Ctrl+D: Duplicate");
            ui.label("Delete: Delete selected");

            ui.add_space(10.0);
            ui.heading("View");
            ui.label("Numpad 7: Top view");
            ui.label("Numpad 1: Front view");
            ui.label("Numpad 3: Side view");
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gizmo_mode_equality() {
        assert_eq!(GizmoMode::Translate, GizmoMode::Translate);
        assert_ne!(GizmoMode::Translate, GizmoMode::Rotate);
    }
}
