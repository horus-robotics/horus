//! Scene editor and GUI for interactive simulation manipulation

#[cfg(feature = "editor")]
pub mod camera;
#[cfg(feature = "editor")]
pub mod gizmos;
#[cfg(feature = "editor")]
pub mod hierarchy;
#[cfg(feature = "editor")]
pub mod inspector;
#[cfg(feature = "editor")]
pub mod selection;
#[cfg(feature = "editor")]
pub mod ui;
#[cfg(feature = "editor")]
pub mod undo;

#[cfg(feature = "editor")]
use bevy::prelude::*;

#[cfg(feature = "editor")]
use bevy_egui::EguiPlugin;

/// Editor state and configuration
#[cfg(feature = "editor")]
#[derive(Resource, Default)]
pub struct EditorState {
    /// Whether the editor is enabled
    pub enabled: bool,
    /// Show inspector panel
    pub show_inspector: bool,
    /// Show hierarchy panel
    pub show_hierarchy: bool,
    /// Show toolbar
    pub show_toolbar: bool,
    /// Grid snapping enabled
    pub snap_to_grid: bool,
    /// Grid size for snapping
    pub grid_size: f32,
    /// Current gizmo mode
    pub gizmo_mode: GizmoMode,
    /// Editor camera mode
    pub camera_mode: EditorCameraMode,
}

#[cfg(feature = "editor")]
impl EditorState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            show_inspector: true,
            show_hierarchy: true,
            show_toolbar: true,
            snap_to_grid: false,
            grid_size: 0.1,
            gizmo_mode: GizmoMode::Translate,
            camera_mode: EditorCameraMode::Orbit,
        }
    }
}

/// Gizmo manipulation mode
#[cfg(feature = "editor")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
    None,
}

/// Editor camera control mode
#[cfg(feature = "editor")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum EditorCameraMode {
    #[default]
    Orbit,
    Pan,
    Fly,
    Top,
    Side,
    Front,
}

/// Main editor plugin
#[cfg(feature = "editor")]
pub struct EditorPlugin;

#[cfg(feature = "editor")]
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<EditorState>()
            .init_resource::<selection::Selection>()
            .init_resource::<undo::UndoStack>()
            .add_systems(
                Update,
                (
                    ui::editor_ui_system,
                    inspector::inspector_panel_system,
                    hierarchy::hierarchy_panel_system,
                    gizmos::gizmo_system,
                )
                    .run_if(editor_enabled),
            )
            .add_systems(
                Update,
                (
                    selection::selection_system,
                    camera::editor_camera_system,
                    undo::undo_system,
                )
                    .run_if(editor_enabled),
            )
            .register_type::<selection::Selectable>()
            .register_type::<EditorCameraMode>();
    }
}

#[cfg(feature = "editor")]
fn editor_enabled(state: Res<EditorState>) -> bool {
    state.enabled
}

// Non-editor stub implementations
#[cfg(not(feature = "editor"))]
pub struct EditorPlugin;

#[cfg(not(feature = "editor"))]
impl bevy::app::Plugin for EditorPlugin {
    fn build(&self, _app: &mut bevy::app::App) {
        // No-op when editor feature is disabled
    }
}
