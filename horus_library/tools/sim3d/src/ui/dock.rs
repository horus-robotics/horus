//! Dockable Panel System using egui_dock
//!
//! Provides a unified dockable workspace similar to Gazebo and Isaac Sim.
//! Users can drag, dock, and arrange panels as needed.

use bevy::prelude::*;
use bevy_egui::EguiContexts;
// Use egui_dock's re-exported egui to ensure type compatibility
use egui_dock::egui;
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};

/// Tab identifiers for the dock system
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DockTab {
    /// Scene hierarchy tree
    Hierarchy,
    /// Entity inspector
    Inspector,
    /// Plugin settings
    Settings,
    /// Statistics and performance
    Stats,
    /// Console/Log output
    Console,
    /// TF (Transform Frame) tree
    TfTree,
    /// Viewport (3D scene view)
    Viewport,
    /// Asset browser
    Assets,
    /// Custom plugin tab
    Plugin(String),
}

impl DockTab {
    pub fn title(&self) -> &str {
        match self {
            DockTab::Hierarchy => "Hierarchy",
            DockTab::Inspector => "Inspector",
            DockTab::Settings => "Settings",
            DockTab::Stats => "Statistics",
            DockTab::Console => "Console",
            DockTab::TfTree => "TF Tree",
            DockTab::Viewport => "Viewport",
            DockTab::Assets => "Assets",
            DockTab::Plugin(name) => name.as_str(),
        }
    }

    pub fn closeable(&self) -> bool {
        !matches!(self, DockTab::Viewport)
    }
}

/// Context passed to tab viewers for rendering
pub struct DockContext<'a> {
    pub world: &'a World,
}

/// Tab viewer implementation for our dock system
pub struct SimDockViewer<'a> {
    pub context: &'a DockContext<'a>,
    /// Callback functions for rendering each tab type
    pub render_hierarchy: Option<Box<dyn Fn(&mut egui::Ui, &World) + 'a>>,
    pub render_inspector: Option<Box<dyn Fn(&mut egui::Ui, &World) + 'a>>,
    pub render_settings: Option<Box<dyn Fn(&mut egui::Ui, &World) + 'a>>,
    pub render_stats: Option<Box<dyn Fn(&mut egui::Ui, &World) + 'a>>,
    pub render_console: Option<Box<dyn Fn(&mut egui::Ui, &World) + 'a>>,
    pub render_tf_tree: Option<Box<dyn Fn(&mut egui::Ui, &World) + 'a>>,
    pub render_assets: Option<Box<dyn Fn(&mut egui::Ui, &World) + 'a>>,
}

impl<'a> TabViewer for SimDockViewer<'a> {
    type Tab = DockTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            DockTab::Hierarchy => {
                if let Some(ref render) = self.render_hierarchy {
                    render(ui, self.context.world);
                } else {
                    default_hierarchy_ui(ui);
                }
            }
            DockTab::Inspector => {
                if let Some(ref render) = self.render_inspector {
                    render(ui, self.context.world);
                } else {
                    default_inspector_ui(ui);
                }
            }
            DockTab::Settings => {
                if let Some(ref render) = self.render_settings {
                    render(ui, self.context.world);
                } else {
                    default_settings_ui(ui);
                }
            }
            DockTab::Stats => {
                if let Some(ref render) = self.render_stats {
                    render(ui, self.context.world);
                } else {
                    default_stats_ui(ui);
                }
            }
            DockTab::Console => {
                if let Some(ref render) = self.render_console {
                    render(ui, self.context.world);
                } else {
                    default_console_ui(ui);
                }
            }
            DockTab::TfTree => {
                if let Some(ref render) = self.render_tf_tree {
                    render(ui, self.context.world);
                } else {
                    default_tf_tree_ui(ui);
                }
            }
            DockTab::Viewport => {
                // Viewport is handled separately (it's the 3D view)
                ui.centered_and_justified(|ui| {
                    ui.label("3D Viewport");
                });
            }
            DockTab::Assets => {
                if let Some(ref render) = self.render_assets {
                    render(ui, self.context.world);
                } else {
                    default_assets_ui(ui);
                }
            }
            DockTab::Plugin(name) => {
                ui.label(format!("Plugin: {}", name));
            }
        }
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        tab.closeable()
    }

    fn on_close(&mut self, _tab: &mut Self::Tab) -> bool {
        true // Allow closing
    }
}

// Default UI implementations for each tab

fn default_hierarchy_ui(ui: &mut egui::Ui) {
    ui.heading("Scene Hierarchy");
    ui.separator();
    ui.label("(Scene tree will be rendered here)");
}

fn default_inspector_ui(ui: &mut egui::Ui) {
    ui.heading("Inspector");
    ui.separator();
    ui.label("Select an entity to inspect its components.");
    ui.add_space(8.0);
    ui.label("Components will be displayed as collapsible sections:");
    ui.label("  - Transform");
    ui.label("  - Mesh");
    ui.label("  - Material");
    ui.label("  - Physics");
    ui.label("  - Plugin-specific components");
}

fn default_settings_ui(ui: &mut egui::Ui) {
    ui.heading("Settings");
    ui.separator();

    egui::CollapsingHeader::new("Simulation")
        .default_open(true)
        .show(ui, |ui| {
            ui.label("Time scale: 1.0x");
            ui.label("Physics rate: 240 Hz");
        });

    egui::CollapsingHeader::new("Rendering")
        .default_open(false)
        .show(ui, |ui| {
            ui.label("Shadows: Enabled");
            ui.label("MSAA: 4x");
        });

    egui::CollapsingHeader::new("Plugins")
        .default_open(true)
        .show(ui, |ui| {
            ui.label("(Plugin settings will appear here)");
        });
}

fn default_stats_ui(ui: &mut egui::Ui) {
    ui.heading("Statistics");
    ui.separator();
    ui.label("FPS: --");
    ui.label("Frame time: -- ms");
    ui.label("Entities: --");
    ui.label("Physics bodies: --");
}

fn default_console_ui(ui: &mut egui::Ui) {
    ui.heading("Console");
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label("[INFO] Simulation started");
        ui.label("[INFO] Loaded 0 entities");
    });
}

fn default_tf_tree_ui(ui: &mut egui::Ui) {
    ui.heading("TF Tree");
    ui.separator();
    ui.label("Transform frame hierarchy:");
    ui.label("  world");
    ui.label("    +-- robot_base");
    ui.label("        +-- sensor_frame");
}

fn default_assets_ui(ui: &mut egui::Ui) {
    ui.heading("Assets");
    ui.separator();
    ui.label("Loaded assets will appear here");
}

/// Dock state resource for persistence
#[derive(Resource)]
pub struct DockWorkspace {
    pub state: DockState<DockTab>,
    pub style: Style,
}

impl Default for DockWorkspace {
    fn default() -> Self {
        Self::new_default_layout()
    }
}

impl DockWorkspace {
    /// Create workspace with default layout (similar to Unity/Unreal)
    pub fn new_default_layout() -> Self {
        // Create initial dock state with tabs
        let mut state = DockState::new(vec![DockTab::Viewport]);
        let tree = state.main_surface_mut();

        // Split: Left panel (Hierarchy)
        let [_viewport, _left] = tree.split_left(NodeIndex::root(), 0.2, vec![DockTab::Hierarchy]);

        // Split: Right panel (Inspector + Settings as tabs)
        let [_viewport, _right] = tree.split_right(
            NodeIndex::root(),
            0.25,
            vec![DockTab::Inspector, DockTab::Settings],
        );

        // Split: Bottom panel (Console + Stats as tabs)
        let [_viewport, _bottom] = tree.split_below(
            NodeIndex::root(),
            0.25,
            vec![DockTab::Console, DockTab::Stats],
        );

        // Add TF Tree to left panel
        tree.push_to_focused_leaf(DockTab::TfTree);

        Self {
            state,
            style: Style::from_egui(&egui::Style::default()),
        }
    }

    /// Create a minimal layout (just viewport + inspector)
    pub fn new_minimal_layout() -> Self {
        let mut state = DockState::new(vec![DockTab::Viewport]);
        let tree = state.main_surface_mut();

        let [_viewport, _right] =
            tree.split_right(NodeIndex::root(), 0.3, vec![DockTab::Inspector]);

        Self {
            state,
            style: Style::from_egui(&egui::Style::default()),
        }
    }

    /// Create a development layout (all panels)
    pub fn new_dev_layout() -> Self {
        let mut state = DockState::new(vec![DockTab::Viewport]);
        let tree = state.main_surface_mut();

        // Left: Hierarchy + Assets
        let [_viewport, _left] = tree.split_left(
            NodeIndex::root(),
            0.2,
            vec![DockTab::Hierarchy, DockTab::Assets, DockTab::TfTree],
        );

        // Right: Inspector + Settings
        let [_viewport, _right] = tree.split_right(
            NodeIndex::root(),
            0.25,
            vec![DockTab::Inspector, DockTab::Settings],
        );

        // Bottom: Console + Stats
        let [_viewport, _bottom] = tree.split_below(
            NodeIndex::root(),
            0.2,
            vec![DockTab::Console, DockTab::Stats],
        );

        Self {
            state,
            style: Style::from_egui(&egui::Style::default()),
        }
    }

    /// Add a custom plugin tab
    pub fn add_plugin_tab(&mut self, plugin_name: String) {
        self.state
            .main_surface_mut()
            .push_to_focused_leaf(DockTab::Plugin(plugin_name));
    }

    /// Reset to default layout
    pub fn reset_layout(&mut self) {
        *self = Self::new_default_layout();
    }
}

/// Layout preset enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockLayoutPreset {
    #[default]
    Default,
    Minimal,
    Development,
}

impl DockLayoutPreset {
    pub fn label(&self) -> &'static str {
        match self {
            DockLayoutPreset::Default => "Default",
            DockLayoutPreset::Minimal => "Minimal",
            DockLayoutPreset::Development => "Development",
        }
    }

    pub fn apply(&self) -> DockWorkspace {
        match self {
            DockLayoutPreset::Default => DockWorkspace::new_default_layout(),
            DockLayoutPreset::Minimal => DockWorkspace::new_minimal_layout(),
            DockLayoutPreset::Development => DockWorkspace::new_dev_layout(),
        }
    }
}

/// Event to change dock layout
#[derive(Event)]
pub struct ChangeDockLayoutEvent {
    pub preset: DockLayoutPreset,
}

/// Event to add a plugin tab
#[derive(Event)]
pub struct AddPluginTabEvent {
    pub plugin_name: String,
}

/// System to handle layout changes
pub fn handle_layout_change(
    mut events: EventReader<ChangeDockLayoutEvent>,
    mut workspace: ResMut<DockWorkspace>,
) {
    for event in events.read() {
        *workspace = event.preset.apply();
        tracing::info!("Dock layout changed to: {}", event.preset.label());
    }
}

/// System to handle adding plugin tabs
pub fn handle_add_plugin_tab(
    mut events: EventReader<AddPluginTabEvent>,
    mut workspace: ResMut<DockWorkspace>,
) {
    for event in events.read() {
        workspace.add_plugin_tab(event.plugin_name.clone());
        tracing::info!("Added plugin tab: {}", event.plugin_name);
    }
}

/// Bevy plugin for the dock system
pub struct DockPlugin;

impl Plugin for DockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DockWorkspace>()
            .add_event::<ChangeDockLayoutEvent>()
            .add_event::<AddPluginTabEvent>()
            .add_systems(Update, (handle_layout_change, handle_add_plugin_tab));

        tracing::info!("Dock system initialized");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_tab_titles() {
        assert_eq!(DockTab::Hierarchy.title(), "Hierarchy");
        assert_eq!(DockTab::Inspector.title(), "Inspector");
        assert_eq!(DockTab::Plugin("Test".to_string()).title(), "Test");
    }

    #[test]
    fn test_dock_tab_closeable() {
        assert!(DockTab::Hierarchy.closeable());
        assert!(!DockTab::Viewport.closeable());
    }

    #[test]
    fn test_workspace_default() {
        let workspace = DockWorkspace::default();
        assert!(!workspace.state.main_surface().is_empty());
    }

    #[test]
    fn test_layout_presets() {
        let _default = DockLayoutPreset::Default.apply();
        let _minimal = DockLayoutPreset::Minimal.apply();
        let _dev = DockLayoutPreset::Development.apply();
    }
}
