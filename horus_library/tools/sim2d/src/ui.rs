//! UI module for sim2d control panel

use crate::AppConfig;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::path::PathBuf;

/// UI state resource
#[derive(Resource)]
pub struct UiState {
    pub robot_config_path: Option<PathBuf>,
    pub world_config_path: Option<PathBuf>,
    pub active_topics: Vec<String>,
    pub active_nodes: Vec<String>,
    pub show_file_dialog: FileDialogType,
    pub status_message: String,
    pub topic_input: String,
}

#[derive(PartialEq)]
pub enum FileDialogType {
    None,
    RobotConfig,
    WorldConfig,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            robot_config_path: None,
            world_config_path: None,
            active_topics: vec![],
            active_nodes: vec![],
            show_file_dialog: FileDialogType::None,
            status_message: "Ready".to_string(),
            topic_input: String::new(),
        }
    }
}

/// System to render the left control panel
pub fn ui_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    app_config: Res<AppConfig>,
) {
    egui::SidePanel::left("control_panel")
        .min_width(300.0)
        .max_width(350.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("sim2d Control Panel");
            ui.separator();

            // World Configuration Section
            ui.group(|ui| {
                ui.label("World Configuration");
                ui.horizontal(|ui| {
                    if ui.button("Load World YAML").clicked() {
                        ui_state.show_file_dialog = FileDialogType::WorldConfig;
                    }
                });

                if let Some(path) = &ui_state.world_config_path {
                    ui.label(format!(
                        "File: {}",
                        path.file_name().unwrap().to_string_lossy()
                    ));
                } else {
                    ui.label("Using default world config");
                }

                ui.label(format!(
                    "Size: {:.1}m × {:.1}m",
                    app_config.world_config.width, app_config.world_config.height
                ));
                ui.label(format!(
                    "Obstacles: {}",
                    app_config.world_config.obstacles.len()
                ));
            });

            ui.add_space(10.0);

            // Robot Configuration Section
            ui.group(|ui| {
                ui.label("Robot Configuration");
                ui.horizontal(|ui| {
                    if ui.button("Load Robot YAML").clicked() {
                        ui_state.show_file_dialog = FileDialogType::RobotConfig;
                    }
                });

                if let Some(path) = &ui_state.robot_config_path {
                    ui.label(format!(
                        "File: {}",
                        path.file_name().unwrap().to_string_lossy()
                    ));
                } else {
                    ui.label("Using default robot config");
                }

                ui.label(format!(
                    "Size: {:.2}m × {:.2}m",
                    app_config.robot_config.length, app_config.robot_config.width
                ));
                ui.label(format!(
                    "Max Speed: {:.1} m/s",
                    app_config.robot_config.max_speed
                ));

                // Color preview
                let color = app_config.robot_config.color;
                let color32 = egui::Color32::from_rgb(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                );
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.colored_label(color32, "███");
                });
            });

            ui.add_space(10.0);

            // Topics Section
            ui.group(|ui| {
                ui.label("HORUS Topics");

                // Topic input
                ui.horizontal(|ui| {
                    ui.label("Subscribe to:");
                    if ui_state.topic_input.is_empty() {
                        ui_state.topic_input = app_config.args.topic.clone();
                    }
                    ui.text_edit_singleline(&mut ui_state.topic_input);
                });
                ui.label("Note: Changing topic requires restart");

                ui.add_space(5.0);

                // Active topics list
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        if ui_state.active_topics.is_empty() {
                            ui_state.active_topics.push(app_config.args.topic.clone());
                        }
                        for topic in &ui_state.active_topics {
                            ui.label(format!("• {}", topic));
                        }
                    });
            });

            ui.add_space(10.0);

            // Nodes Section
            ui.group(|ui| {
                ui.label("Active Nodes");
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        if ui_state.active_nodes.is_empty() {
                            ui.label("• sim2d");
                        } else {
                            for node in &ui_state.active_nodes {
                                ui.label(format!("• {}", node));
                            }
                        }
                    });
            });

            ui.add_space(10.0);

            // Status Bar
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.colored_label(egui::Color32::GREEN, &ui_state.status_message);
            });
        });
}

/// System to handle file dialog actions
pub fn file_dialog_system(mut ui_state: ResMut<UiState>, mut app_config: ResMut<AppConfig>) {
    match ui_state.show_file_dialog {
        FileDialogType::RobotConfig => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("YAML", &["yaml", "yml"])
                .set_title("Load Robot Configuration")
                .pick_file()
            {
                match AppConfig::load_robot_config(path.to_str().unwrap()) {
                    Ok(config) => {
                        app_config.robot_config = config;
                        ui_state.robot_config_path = Some(path);
                        ui_state.status_message = "Robot config loaded!".to_string();
                        info!("✅ Loaded robot config");
                    }
                    Err(e) => {
                        ui_state.status_message = format!("Error: {}", e);
                        warn!("❌ Failed to load robot config: {}", e);
                    }
                }
            }
            ui_state.show_file_dialog = FileDialogType::None;
        }
        FileDialogType::WorldConfig => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("YAML", &["yaml", "yml"])
                .set_title("Load World Configuration")
                .pick_file()
            {
                match AppConfig::load_world_config(path.to_str().unwrap()) {
                    Ok(config) => {
                        app_config.world_config = config;
                        ui_state.world_config_path = Some(path);
                        ui_state.status_message =
                            "World config loaded! (Restart to apply)".to_string();
                        info!("✅ Loaded world config");
                    }
                    Err(e) => {
                        ui_state.status_message = format!("Error: {}", e);
                        warn!("❌ Failed to load world config: {}", e);
                    }
                }
            }
            ui_state.show_file_dialog = FileDialogType::None;
        }
        FileDialogType::None => {}
    }
}
