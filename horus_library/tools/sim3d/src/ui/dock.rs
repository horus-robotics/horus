//! Dockable Panel System using egui_dock
//!
//! Provides a unified dockable workspace similar to Gazebo and Isaac Sim.
//! Users can drag, dock, and arrange panels as needed.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};

use crate::hframe::TFTree;
use crate::physics::PhysicsWorld;
use crate::ui::controls::SimulationControls;
use crate::ui::stats_panel::{FrameTimeBreakdown, SimulationStats};

/// Tab identifiers for the dock system
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DockTab {
    /// Simulation controls
    Controls,
    /// Statistics and performance
    Stats,
    /// Console/Log output
    Console,
    /// TF (Transform Frame) tree
    TfTree,
    /// Custom plugin tab
    Plugin(String),
}

impl DockTab {
    pub fn title(&self) -> &str {
        match self {
            DockTab::Controls => "Controls",
            DockTab::Stats => "Statistics",
            DockTab::Console => "Console",
            DockTab::TfTree => "TF Tree",
            DockTab::Plugin(name) => name.as_str(),
        }
    }

    pub fn closeable(&self) -> bool {
        true // All tabs can be closed
    }
}

/// Resource to control dock system behavior
#[derive(Resource)]
pub struct DockConfig {
    /// Whether the dock system is enabled (vs. floating windows)
    pub enabled: bool,
    /// Show the menu bar
    pub show_menu_bar: bool,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default - use F7 to enable dockable mode
            show_menu_bar: true,
        }
    }
}

/// Context for rendering dock tabs - holds references to world resources
pub struct DockRenderContext<'a> {
    pub time: &'a Time,
    pub stats: &'a SimulationStats,
    pub frame_time: &'a FrameTimeBreakdown,
    pub controls: &'a SimulationControls,
    pub tf_tree: &'a TFTree,
    pub physics_world: Option<&'a PhysicsWorld>,
}

/// Tab viewer implementation for our dock system
pub struct SimDockViewer<'a> {
    pub ctx: DockRenderContext<'a>,
    /// Console log messages (stored separately for persistence)
    pub console_messages: &'a mut Vec<String>,
}

impl<'a> TabViewer for SimDockViewer<'a> {
    type Tab = DockTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            DockTab::Controls => {
                render_controls_content(ui, &self.ctx);
            }
            DockTab::Stats => {
                render_stats_content(ui, &self.ctx);
            }
            DockTab::Console => {
                render_console_content(ui, self.console_messages);
            }
            DockTab::TfTree => {
                render_tf_tree_content(ui, &self.ctx);
            }
            DockTab::Plugin(name) => {
                ui.heading(format!("Plugin: {}", name));
                ui.separator();
                ui.label("Plugin-specific content goes here");
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

// ============================================================================
// Tab Content Renderers
// ============================================================================

fn render_controls_content(ui: &mut egui::Ui, ctx: &DockRenderContext) {
    ui.heading("Simulation Controls");
    ui.separator();

    // Pause state
    ui.horizontal(|ui| {
        ui.label("Status:");
        if ctx.controls.paused {
            ui.colored_label(egui::Color32::YELLOW, "PAUSED");
        } else {
            ui.colored_label(egui::Color32::GREEN, "RUNNING");
        }
    });

    ui.label(format!("Time Scale: {:.2}x", ctx.controls.time_scale));

    ui.add_space(8.0);
    ui.heading("Visualization");
    ui.separator();

    ui.label(format!(
        "Debug Info: {}",
        if ctx.controls.show_debug_info { "ON" } else { "OFF" }
    ));
    ui.label(format!(
        "Physics Debug: {}",
        if ctx.controls.show_physics_debug { "ON" } else { "OFF" }
    ));
    ui.label(format!(
        "TF Frames: {}",
        if ctx.controls.show_tf_frames { "ON" } else { "OFF" }
    ));
    ui.label(format!(
        "Collision Shapes: {}",
        if ctx.controls.show_collision_shapes { "ON" } else { "OFF" }
    ));

    ui.add_space(8.0);
    ui.label("Hotkeys:");
    ui.label("  Space: Pause/Resume");
    ui.label("  1-5: Time scale");
    ui.label("  D: Toggle debug info");
}

fn render_stats_content(ui: &mut egui::Ui, ctx: &DockRenderContext) {
    ui.heading("Performance");
    ui.separator();

    // FPS and frame time
    let fps = if ctx.time.delta_secs() > 0.0 {
        1.0 / ctx.time.delta_secs()
    } else {
        0.0
    };

    ui.horizontal(|ui| {
        ui.label("FPS:");
        ui.label(format!("{:.1}", fps));
        if fps >= 60.0 {
            ui.colored_label(egui::Color32::GREEN, "[OK]");
        } else if fps >= 30.0 {
            ui.colored_label(egui::Color32::YELLOW, "[WARN]");
        } else {
            ui.colored_label(egui::Color32::RED, "[LOW]");
        }
    });

    ui.label(format!("Frame Time: {:.2}ms", ctx.time.delta_secs() * 1000.0));

    ui.add_space(5.0);
    ui.label("Frame Breakdown:");
    ui.indent("breakdown", |ui| {
        ui.label(format!("  Physics: {:.2}ms", ctx.frame_time.physics_time_ms));
        ui.label(format!("  Sensors: {:.2}ms", ctx.frame_time.sensor_time_ms));
        ui.label(format!("  Rendering: {:.2}ms", ctx.frame_time.rendering_time_ms));
    });

    ui.add_space(10.0);
    ui.heading("Entities");
    ui.separator();

    ui.label(format!("Total: {}", ctx.stats.total_entities));
    ui.label(format!("Robots: {}", ctx.stats.robot_count));
    ui.label(format!("Sensors: {}", ctx.stats.sensor_count));

    ui.add_space(10.0);
    ui.heading("Physics");
    ui.separator();

    ui.label(format!("Rigid Bodies: {}", ctx.stats.rigid_body_count));
    ui.label(format!("Colliders: {}", ctx.stats.collider_count));
    ui.label(format!("Joints: {}", ctx.stats.joint_count));
    ui.label(format!("Contacts: {}", ctx.stats.contact_count));

    ui.add_space(10.0);
    ui.label(format!("Sim Time: {:.2}s", ctx.stats.simulation_time));
    ui.label(format!("Est. Memory: {:.2} MB", ctx.stats.estimated_memory_mb()));
}

fn render_console_content(ui: &mut egui::Ui, messages: &mut Vec<String>) {
    ui.heading("Console");
    ui.separator();

    // Add clear button
    ui.horizontal(|ui| {
        if ui.button("Clear").clicked() {
            messages.clear();
        }
        ui.label(format!("{} messages", messages.len()));
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for msg in messages.iter() {
                // Color code by log level
                if msg.contains("[ERROR]") {
                    ui.colored_label(egui::Color32::RED, msg);
                } else if msg.contains("[WARN]") {
                    ui.colored_label(egui::Color32::YELLOW, msg);
                } else if msg.contains("[INFO]") {
                    ui.label(msg);
                } else {
                    ui.colored_label(egui::Color32::GRAY, msg);
                }
            }
        });
}

fn render_tf_tree_content(ui: &mut egui::Ui, ctx: &DockRenderContext) {
    ui.heading("TF Tree");
    ui.separator();

    // Root frame is always "world" in this implementation
    ui.label("Root: world");

    ui.add_space(8.0);

    // Display frame hierarchy
    egui::ScrollArea::vertical().show(ui, |ui| {
        // Get all frames from the TF tree
        let frames = ctx.tf_tree.get_all_frames();
        if frames.is_empty() {
            ui.label("No transform frames registered");
        } else {
            ui.label(format!("{} frames:", frames.len()));
            for frame in &frames {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(format!("- {}", frame));

                    // Try to get transform info
                    if let Ok(transform) = ctx.tf_tree.lookup_transform("world", frame) {
                        ui.label(format!(
                            "({:.2}, {:.2}, {:.2})",
                            transform.translation.x, transform.translation.y, transform.translation.z
                        ));
                    }
                });
            }
        }
    });
}

// ============================================================================
// Dock System Resources and Systems
// ============================================================================

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
    /// Create workspace with default layout
    pub fn new_default_layout() -> Self {
        // Start with Stats + Controls as tabs
        let mut state = DockState::new(vec![DockTab::Stats, DockTab::Controls]);
        let tree = state.main_surface_mut();

        // Split: Bottom panel (Console + TfTree as tabs)
        let [_main, _bottom] =
            tree.split_below(NodeIndex::root(), 0.65, vec![DockTab::Console, DockTab::TfTree]);

        Self {
            state,
            style: Style::default(),
        }
    }

    /// Create a minimal layout (just stats)
    pub fn new_minimal_layout() -> Self {
        let state = DockState::new(vec![DockTab::Stats]);

        Self {
            state,
            style: Style::default(),
        }
    }

    /// Create a development layout (all panels)
    pub fn new_dev_layout() -> Self {
        // Left: Stats + Controls
        let mut state = DockState::new(vec![DockTab::Stats, DockTab::Controls]);
        let tree = state.main_surface_mut();

        // Right: TF Tree
        let [_left, _right] = tree.split_right(NodeIndex::root(), 0.65, vec![DockTab::TfTree]);

        // Bottom: Console
        let [_main, _bottom] = tree.split_below(NodeIndex::root(), 0.7, vec![DockTab::Console]);

        Self {
            state,
            style: Style::default(),
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

/// Console messages storage
#[derive(Resource, Default)]
pub struct ConsoleMessages {
    pub messages: Vec<String>,
}

impl ConsoleMessages {
    pub fn add(&mut self, message: String) {
        self.messages.push(message);
        // Keep last 1000 messages
        if self.messages.len() > 1000 {
            self.messages.remove(0);
        }
    }

    pub fn info(&mut self, msg: &str) {
        self.add(format!("[INFO] {}", msg));
    }

    pub fn warn(&mut self, msg: &str) {
        self.add(format!("[WARN] {}", msg));
    }

    pub fn error(&mut self, msg: &str) {
        self.add(format!("[ERROR] {}", msg));
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

/// Event to toggle dock mode
#[derive(Event)]
pub struct ToggleDockModeEvent;

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

/// System to toggle dock mode
pub fn handle_toggle_dock_mode(
    mut events: EventReader<ToggleDockModeEvent>,
    mut config: ResMut<DockConfig>,
) {
    for _ in events.read() {
        config.enabled = !config.enabled;
        tracing::info!(
            "Dock mode: {}",
            if config.enabled { "enabled" } else { "disabled" }
        );
    }
}

/// Keyboard shortcuts for dock system
pub fn dock_keyboard_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut layout_events: EventWriter<ChangeDockLayoutEvent>,
    mut toggle_events: EventWriter<ToggleDockModeEvent>,
) {
    // F7: Toggle dock mode
    if keyboard.just_pressed(KeyCode::F7) {
        toggle_events.send(ToggleDockModeEvent);
    }

    // F8: Reset to default layout
    if keyboard.just_pressed(KeyCode::F8) {
        layout_events.send(ChangeDockLayoutEvent {
            preset: DockLayoutPreset::Default,
        });
    }

    // F9: Minimal layout
    if keyboard.just_pressed(KeyCode::F9) {
        layout_events.send(ChangeDockLayoutEvent {
            preset: DockLayoutPreset::Minimal,
        });
    }

    // F10: Development layout
    if keyboard.just_pressed(KeyCode::F10) {
        layout_events.send(ChangeDockLayoutEvent {
            preset: DockLayoutPreset::Development,
        });
    }
}

/// Main dock UI rendering system
#[cfg(feature = "visual")]
pub fn dock_ui_system(
    mut contexts: EguiContexts,
    config: Res<DockConfig>,
    mut workspace: ResMut<DockWorkspace>,
    mut console: ResMut<ConsoleMessages>,
    time: Res<Time>,
    stats: Res<SimulationStats>,
    frame_time: Res<FrameTimeBreakdown>,
    controls: Res<SimulationControls>,
    tf_tree: Res<TFTree>,
    physics_world: Option<Res<PhysicsWorld>>,
    mut layout_events: EventWriter<ChangeDockLayoutEvent>,
) {
    if !config.enabled {
        return;
    }

    let egui_ctx = contexts.ctx_mut();

    // Menu bar
    if config.show_menu_bar {
        egui::TopBottomPanel::top("dock_menu_bar").show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("View", |ui| {
                    if ui.button("Default Layout (F8)").clicked() {
                        layout_events.send(ChangeDockLayoutEvent {
                            preset: DockLayoutPreset::Default,
                        });
                        ui.close_menu();
                    }
                    if ui.button("Minimal Layout (F9)").clicked() {
                        layout_events.send(ChangeDockLayoutEvent {
                            preset: DockLayoutPreset::Minimal,
                        });
                        ui.close_menu();
                    }
                    if ui.button("Development Layout (F10)").clicked() {
                        layout_events.send(ChangeDockLayoutEvent {
                            preset: DockLayoutPreset::Development,
                        });
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label("F7: Toggle dock mode");
                });

                ui.menu_button("Panels", |ui| {
                    if ui.button("Add Controls").clicked() {
                        workspace
                            .state
                            .main_surface_mut()
                            .push_to_focused_leaf(DockTab::Controls);
                        ui.close_menu();
                    }
                    if ui.button("Add Statistics").clicked() {
                        workspace
                            .state
                            .main_surface_mut()
                            .push_to_focused_leaf(DockTab::Stats);
                        ui.close_menu();
                    }
                    if ui.button("Add Console").clicked() {
                        workspace
                            .state
                            .main_surface_mut()
                            .push_to_focused_leaf(DockTab::Console);
                        ui.close_menu();
                    }
                    if ui.button("Add TF Tree").clicked() {
                        workspace
                            .state
                            .main_surface_mut()
                            .push_to_focused_leaf(DockTab::TfTree);
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("sim3d - Dockable UI");
                });
            });
        });
    }

    // Build render context
    let render_ctx = DockRenderContext {
        time: &time,
        stats: &stats,
        frame_time: &frame_time,
        controls: &controls,
        tf_tree: &tf_tree,
        physics_world: physics_world.as_deref(),
    };

    // Create tab viewer
    let mut viewer = SimDockViewer {
        ctx: render_ctx,
        console_messages: &mut console.messages,
    };

    // Render dock area in a side panel (left side) to preserve viewport
    // Clone style first to avoid borrow conflict with mutable state borrow
    let style = workspace.style.clone();
    egui::SidePanel::left("dock_panel")
        .default_width(350.0)
        .min_width(200.0)
        .max_width(600.0)
        .resizable(true)
        .show(egui_ctx, |ui| {
            DockArea::new(&mut workspace.state)
                .style(style)
                .show_inside(ui, &mut viewer);
        });
}

#[cfg(not(feature = "visual"))]
pub fn dock_ui_system() {}

/// Bevy plugin for the dock system
pub struct DockPlugin;

impl Plugin for DockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DockConfig>()
            .init_resource::<DockWorkspace>()
            .init_resource::<ConsoleMessages>()
            .add_event::<ChangeDockLayoutEvent>()
            .add_event::<AddPluginTabEvent>()
            .add_event::<ToggleDockModeEvent>()
            .add_systems(
                Update,
                (
                    handle_layout_change,
                    handle_add_plugin_tab,
                    handle_toggle_dock_mode,
                    dock_keyboard_system,
                    dock_ui_system,
                )
                    .chain(),
            );

        tracing::info!("Dock system initialized - Press F7 to toggle, F8-F10 for layouts");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_tab_titles() {
        assert_eq!(DockTab::Controls.title(), "Controls");
        assert_eq!(DockTab::Stats.title(), "Statistics");
        assert_eq!(DockTab::Plugin("Test".to_string()).title(), "Test");
    }

    #[test]
    fn test_dock_tab_closeable() {
        assert!(DockTab::Controls.closeable());
        assert!(DockTab::Stats.closeable());
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

    #[test]
    fn test_console_messages() {
        let mut console = ConsoleMessages::default();
        console.info("Test message");
        console.warn("Warning message");
        console.error("Error message");

        assert_eq!(console.messages.len(), 3);
        assert!(console.messages[0].contains("[INFO]"));
        assert!(console.messages[1].contains("[WARN]"));
        assert!(console.messages[2].contains("[ERROR]"));
    }
}
