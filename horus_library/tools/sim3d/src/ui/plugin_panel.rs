//! Unified Plugin Panel System
//!
//! Provides a centralized panel for plugin UIs following Gazebo/Isaac Sim patterns.
//! Plugins can register inspector UI (shown when entities are selected) and
//! settings UI (shown in the unified settings panel).

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::collections::HashMap;

use crate::plugins::{PluginPanelInfo, PluginRegistry};

/// Configuration for the unified plugin panel
#[derive(Resource, Clone)]
pub struct PluginPanelConfig {
    /// Show the unified settings panel
    pub show_settings_panel: bool,
    /// Show plugin sections in inspector
    pub show_plugin_inspector: bool,
    /// Panel width
    pub panel_width: f32,
    /// Currently expanded plugin sections in settings
    pub expanded_sections: HashMap<String, bool>,
    /// Active tab in the settings panel
    pub active_tab: SettingsTab,
    /// Search filter for settings
    pub search_filter: String,
}

impl Default for PluginPanelConfig {
    fn default() -> Self {
        Self {
            show_settings_panel: false,
            show_plugin_inspector: true,
            panel_width: 320.0,
            expanded_sections: HashMap::new(),
            active_tab: SettingsTab::Plugins,
            search_filter: String::new(),
        }
    }
}

/// Tabs in the unified settings panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Plugins,
    Simulation,
    Rendering,
    Input,
}

impl SettingsTab {
    pub fn label(&self) -> &'static str {
        match self {
            SettingsTab::Plugins => "Plugins",
            SettingsTab::Simulation => "Simulation",
            SettingsTab::Rendering => "Rendering",
            SettingsTab::Input => "Input",
        }
    }

    pub fn all() -> &'static [SettingsTab] {
        &[
            SettingsTab::Plugins,
            SettingsTab::Simulation,
            SettingsTab::Rendering,
            SettingsTab::Input,
        ]
    }
}

/// Registered plugin UI information
#[derive(Resource, Default)]
pub struct PluginUiRegistry {
    /// Registered plugin panel info sorted by priority
    pub panels: Vec<PluginPanelInfo>,
    /// Plugin names that have inspector UI
    pub inspector_plugins: Vec<String>,
    /// Plugin names that have settings UI
    pub settings_plugins: Vec<String>,
}

impl PluginUiRegistry {
    /// Register a plugin's UI capabilities
    pub fn register(&mut self, info: PluginPanelInfo) {
        if info.has_inspector {
            self.inspector_plugins.push(info.plugin_name.clone());
        }
        if info.has_settings {
            self.settings_plugins.push(info.plugin_name.clone());
        }
        self.panels.push(info);
        // Sort by priority
        self.panels.sort_by_key(|p| p.priority);
    }

    /// Check if any plugin has inspector UI
    pub fn has_any_inspector(&self) -> bool {
        !self.inspector_plugins.is_empty()
    }

    /// Check if any plugin has settings UI
    pub fn has_any_settings(&self) -> bool {
        !self.settings_plugins.is_empty()
    }
}

/// Event to toggle the unified settings panel
#[derive(Event)]
pub struct ToggleSettingsPanelEvent;

/// Event to open settings panel to a specific plugin
#[derive(Event)]
pub struct OpenPluginSettingsEvent {
    pub plugin_name: String,
}

/// System to handle settings panel toggle
pub fn handle_settings_toggle(
    mut events: EventReader<ToggleSettingsPanelEvent>,
    mut config: ResMut<PluginPanelConfig>,
) {
    for _ in events.read() {
        config.show_settings_panel = !config.show_settings_panel;
    }
}

/// System to handle opening specific plugin settings
pub fn handle_open_plugin_settings(
    mut events: EventReader<OpenPluginSettingsEvent>,
    mut config: ResMut<PluginPanelConfig>,
) {
    for event in events.read() {
        config.show_settings_panel = true;
        config.active_tab = SettingsTab::Plugins;
        config.expanded_sections.insert(event.plugin_name.clone(), true);
    }
}

/// System to display the unified settings panel
pub fn unified_settings_panel_system(
    mut contexts: EguiContexts,
    mut config: ResMut<PluginPanelConfig>,
    registry: Res<PluginRegistry>,
    ui_registry: Res<PluginUiRegistry>,
) {
    if !config.show_settings_panel {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::Window::new("Settings")
        .default_width(config.panel_width)
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            // Tab bar
            ui.horizontal(|ui| {
                for tab in SettingsTab::all() {
                    let selected = config.active_tab == *tab;
                    if ui.selectable_label(selected, tab.label()).clicked() {
                        config.active_tab = *tab;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("X").clicked() {
                        config.show_settings_panel = false;
                    }
                });
            });

            ui.separator();

            // Search filter (only for Plugins tab)
            if config.active_tab == SettingsTab::Plugins {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut config.search_filter);
                    if ui.small_button("Clear").clicked() {
                        config.search_filter.clear();
                    }
                });
                ui.separator();
            }

            // Tab content
            egui::ScrollArea::vertical().show(ui, |ui| {
                match config.active_tab {
                    SettingsTab::Plugins => {
                        show_plugins_settings(ui, &mut config, &registry, &ui_registry);
                    }
                    SettingsTab::Simulation => {
                        show_simulation_settings(ui);
                    }
                    SettingsTab::Rendering => {
                        show_rendering_settings(ui);
                    }
                    SettingsTab::Input => {
                        show_input_settings(ui);
                    }
                }
            });
        });
}

/// Show plugin settings in the unified panel
fn show_plugins_settings(
    ui: &mut egui::Ui,
    config: &mut ResMut<PluginPanelConfig>,
    registry: &PluginRegistry,
    ui_registry: &PluginUiRegistry,
) {
    let plugin_names = registry.list_plugins();

    if plugin_names.is_empty() {
        ui.label("No plugins loaded");
        return;
    }

    // Plugin overview
    ui.heading("Loaded Plugins");
    ui.label(format!("{} plugin(s) registered", plugin_names.len()));
    ui.add_space(8.0);

    // Filter plugins by search
    let filter = config.search_filter.to_lowercase();

    for name in &plugin_names {
        // Apply search filter
        if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
            continue;
        }

        let is_expanded = config.expanded_sections.get(name).copied().unwrap_or(false);

        // Find panel info if available
        let panel_info = ui_registry.panels.iter().find(|p| &p.plugin_name == name);

        // Build header with icon if available
        let header_text = if let Some(info) = panel_info {
            if let Some(icon) = &info.icon {
                format!("{} {}", icon, info.display_name)
            } else {
                info.display_name.clone()
            }
        } else {
            name.clone()
        };

        // Collapsible section for each plugin
        let header_response = egui::CollapsingHeader::new(&header_text)
            .default_open(is_expanded)
            .show(ui, |ui| {
                // Plugin metadata
                if let Some(metadata) = registry.get_metadata(name) {
                    ui.horizontal(|ui| {
                        ui.label("Version:");
                        ui.label(&metadata.version);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Author:");
                        ui.label(&metadata.author);
                    });
                    if !metadata.description.is_empty() {
                        ui.label(&metadata.description);
                    }

                    // Tags
                    if !metadata.tags.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Tags:");
                            for tag in &metadata.tags {
                                let _ = ui.small_button(tag);
                            }
                        });
                    }
                }

                // Plugin state
                if let Some(state) = registry.get_state(name) {
                    ui.horizontal(|ui| {
                        ui.label("State:");
                        let (state_text, color) = match state {
                            crate::plugins::PluginState::Loaded => {
                                ("Loaded", egui::Color32::YELLOW)
                            }
                            crate::plugins::PluginState::Initialized => {
                                ("Initialized", egui::Color32::GREEN)
                            }
                            crate::plugins::PluginState::Active => {
                                ("Active", egui::Color32::LIGHT_GREEN)
                            }
                            crate::plugins::PluginState::Paused => {
                                ("Paused", egui::Color32::GRAY)
                            }
                            crate::plugins::PluginState::Stopped => {
                                ("Stopped", egui::Color32::DARK_GRAY)
                            }
                            crate::plugins::PluginState::Error => ("Error", egui::Color32::RED),
                        };
                        ui.colored_label(color, state_text);
                    });
                }

                ui.add_space(4.0);

                // Plugin-specific settings UI would be called here
                // This requires access to the actual plugin instance
                if panel_info.map(|p| p.has_settings).unwrap_or(false) {
                    ui.separator();
                    ui.label("Plugin Settings:");
                    ui.label("(Plugin settings UI goes here)");
                }
            });

        // Track expanded state
        config
            .expanded_sections
            .insert(name.clone(), header_response.fully_open());
    }
}

/// Show simulation settings
fn show_simulation_settings(ui: &mut egui::Ui) {
    ui.heading("Simulation Settings");
    ui.add_space(8.0);

    egui::CollapsingHeader::new("Physics")
        .default_open(true)
        .show(ui, |ui| {
            ui.label("Physics engine: Rapier3D");
            ui.label("Timestep: 1/240s (fixed)");
            ui.add_space(4.0);
            ui.label("Gravity: [0, -9.81, 0]");
        });

    egui::CollapsingHeader::new("Time")
        .default_open(true)
        .show(ui, |ui| {
            ui.label("Simulation speed: 1.0x");
            ui.label("Pause: No");
        });
}

/// Show rendering settings
fn show_rendering_settings(ui: &mut egui::Ui) {
    ui.heading("Rendering Settings");
    ui.add_space(8.0);

    egui::CollapsingHeader::new("Quality")
        .default_open(true)
        .show(ui, |ui| {
            ui.label("Shadows: Enabled");
            ui.label("MSAA: 4x");
            ui.label("Ambient Occlusion: Enabled");
        });

    egui::CollapsingHeader::new("Debug Visualization")
        .default_open(false)
        .show(ui, |ui| {
            ui.label("Show Colliders: No");
            ui.label("Show Wireframe: No");
            ui.label("Show Normals: No");
        });
}

/// Show input settings
fn show_input_settings(ui: &mut egui::Ui) {
    ui.heading("Input Settings");
    ui.add_space(8.0);

    egui::CollapsingHeader::new("Camera Controls")
        .default_open(true)
        .show(ui, |ui| {
            ui.label("Orbit: Right Mouse + Drag");
            ui.label("Pan: Middle Mouse + Drag");
            ui.label("Zoom: Scroll Wheel");
        });

    egui::CollapsingHeader::new("Keyboard Shortcuts")
        .default_open(false)
        .show(ui, |ui| {
            ui.label("F: Focus selected");
            ui.label("G: Toggle grid");
            ui.label("Space: Pause/Resume");
        });
}

/// Bevy plugin for the unified panel system
pub struct PluginPanelPlugin;

impl Plugin for PluginPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PluginPanelConfig>()
            .init_resource::<PluginUiRegistry>()
            .add_event::<ToggleSettingsPanelEvent>()
            .add_event::<OpenPluginSettingsEvent>()
            .add_systems(
                Update,
                (
                    handle_settings_toggle,
                    handle_open_plugin_settings,
                    unified_settings_panel_system,
                    settings_hotkey_system,
                ),
            );

        tracing::info!("Plugin panel system initialized");
    }
}

/// System to handle hotkey for opening settings
fn settings_hotkey_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut toggle_events: EventWriter<ToggleSettingsPanelEvent>,
) {
    // Ctrl+, to toggle settings (common convention)
    if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight) {
        if keyboard.just_pressed(KeyCode::Comma) {
            toggle_events.send(ToggleSettingsPanelEvent);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_panel_config_default() {
        let config = PluginPanelConfig::default();
        assert!(!config.show_settings_panel);
        assert!(config.show_plugin_inspector);
        assert_eq!(config.active_tab, SettingsTab::Plugins);
    }

    #[test]
    fn test_plugin_ui_registry() {
        let mut registry = PluginUiRegistry::default();

        let info = PluginPanelInfo {
            plugin_name: "test_plugin".to_string(),
            display_name: "Test Plugin".to_string(),
            icon: Some("T".to_string()),
            priority: 50,
            has_inspector: true,
            has_settings: true,
        };

        registry.register(info);

        assert!(registry.has_any_inspector());
        assert!(registry.has_any_settings());
        assert_eq!(registry.panels.len(), 1);
    }

    #[test]
    fn test_settings_tabs() {
        assert_eq!(SettingsTab::all().len(), 4);
        assert_eq!(SettingsTab::Plugins.label(), "Plugins");
    }
}
