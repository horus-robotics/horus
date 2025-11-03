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
    pub show_file_dialog: FileDialogType,
    pub status_message: String,
    pub topic_input: String,
}

/// Visual preferences for the simulator
#[derive(Resource)]
pub struct VisualPreferences {
    pub show_grid: bool,
    pub grid_spacing: f32,
    pub grid_color: [f32; 3],
    pub obstacle_color: [f32; 3],
    pub wall_color: [f32; 3],
    pub background_color: [f32; 3],
    pub show_velocity_arrows: bool,
}

impl Default for VisualPreferences {
    fn default() -> Self {
        Self {
            show_grid: true,
            grid_spacing: 1.0, // 1 meter grid
            grid_color: [0.2, 0.2, 0.2],
            obstacle_color: [0.6, 0.4, 0.2], // Brown
            wall_color: [0.3, 0.3, 0.3],     // Gray
            background_color: [0.1, 0.1, 0.1], // Dark gray
            show_velocity_arrows: false,
        }
    }
}

/// Camera controller for zoom and pan
#[derive(Resource)]
pub struct CameraController {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
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
    mut app_config: ResMut<AppConfig>,
    mut camera_controller: ResMut<CameraController>,
    mut visual_prefs: ResMut<VisualPreferences>,
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
                    if ui.button("Load World File").clicked() {
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
                    if ui.button("Load Robot File").clicked() {
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

            // Camera Controls Section
            ui.group(|ui| {
                ui.label("Camera Controls");

                ui.horizontal(|ui| {
                    ui.label("Zoom:");
                    if ui.add(egui::Slider::new(&mut camera_controller.zoom, 0.1..=5.0)
                        .text("x"))
                        .changed() {
                        ui_state.status_message = format!("Zoom: {:.1}x", camera_controller.zoom);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Pan X:");
                    ui.add(egui::Slider::new(&mut camera_controller.pan_x, -1000.0..=1000.0)
                        .text("px"));
                });

                ui.horizontal(|ui| {
                    ui.label("Pan Y:");
                    ui.add(egui::Slider::new(&mut camera_controller.pan_y, -1000.0..=1000.0)
                        .text("px"));
                });

                if ui.button("Reset Camera").clicked() {
                    camera_controller.zoom = 1.0;
                    camera_controller.pan_x = 0.0;
                    camera_controller.pan_y = 0.0;
                    ui_state.status_message = "Camera reset".to_string();
                }
            });

            ui.add_space(10.0);

            // Live Robot Parameters Section
            ui.group(|ui| {
                ui.label("Live Robot Parameters");

                ui.horizontal(|ui| {
                    ui.label("Max Speed:");
                    if ui.add(egui::Slider::new(&mut app_config.robot_config.max_speed, 0.1..=10.0)
                        .suffix(" m/s"))
                        .changed() {
                        ui_state.status_message = format!("Max speed: {:.1} m/s", app_config.robot_config.max_speed);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Length:");
                    if ui.add(egui::Slider::new(&mut app_config.robot_config.length, 0.1..=3.0)
                        .suffix(" m"))
                        .changed() {
                        ui_state.status_message = "Robot dimensions updated".to_string();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Width:");
                    if ui.add(egui::Slider::new(&mut app_config.robot_config.width, 0.1..=3.0)
                        .suffix(" m"))
                        .changed() {
                        ui_state.status_message = "Robot dimensions updated".to_string();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Robot Color:");
                    let mut color = egui::Color32::from_rgb(
                        (app_config.robot_config.color[0] * 255.0) as u8,
                        (app_config.robot_config.color[1] * 255.0) as u8,
                        (app_config.robot_config.color[2] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        app_config.robot_config.color = [
                            color.r() as f32 / 255.0,
                            color.g() as f32 / 255.0,
                            color.b() as f32 / 255.0,
                        ];
                        ui_state.status_message = "Robot color updated".to_string();
                    }
                });
            });

            ui.add_space(10.0);

            // Visual Customization Section
            ui.group(|ui| {
                ui.label("Visual Customization");

                ui.checkbox(&mut visual_prefs.show_grid, "Show Grid");

                if visual_prefs.show_grid {
                    ui.horizontal(|ui| {
                        ui.label("Grid Spacing:");
                        ui.add(egui::Slider::new(&mut visual_prefs.grid_spacing, 0.5..=5.0)
                            .suffix(" m"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Grid Color:");
                        let mut color = egui::Color32::from_rgb(
                            (visual_prefs.grid_color[0] * 255.0) as u8,
                            (visual_prefs.grid_color[1] * 255.0) as u8,
                            (visual_prefs.grid_color[2] * 255.0) as u8,
                        );
                        if ui.color_edit_button_srgba(&mut color).changed() {
                            visual_prefs.grid_color = [
                                color.r() as f32 / 255.0,
                                color.g() as f32 / 255.0,
                                color.b() as f32 / 255.0,
                            ];
                        }
                    });
                }

                ui.horizontal(|ui| {
                    ui.label("Obstacle Color:");
                    let mut color = egui::Color32::from_rgb(
                        (visual_prefs.obstacle_color[0] * 255.0) as u8,
                        (visual_prefs.obstacle_color[1] * 255.0) as u8,
                        (visual_prefs.obstacle_color[2] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        visual_prefs.obstacle_color = [
                            color.r() as f32 / 255.0,
                            color.g() as f32 / 255.0,
                            color.b() as f32 / 255.0,
                        ];
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Wall Color:");
                    let mut color = egui::Color32::from_rgb(
                        (visual_prefs.wall_color[0] * 255.0) as u8,
                        (visual_prefs.wall_color[1] * 255.0) as u8,
                        (visual_prefs.wall_color[2] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        visual_prefs.wall_color = [
                            color.r() as f32 / 255.0,
                            color.g() as f32 / 255.0,
                            color.b() as f32 / 255.0,
                        ];
                    }
                });

                ui.checkbox(&mut visual_prefs.show_velocity_arrows, "Show Velocity Arrows");
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
            let mut dialog = rfd::FileDialog::new()
                .add_filter("Config Files", &["yaml", "yml", "toml"])
                .add_filter("YAML Files", &["yaml", "yml"])
                .add_filter("TOML Files", &["toml"])
                .set_title("Load Robot Configuration (YAML/TOML only)");

            // Try to set default directory to configs folder
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let configs_path = exe_dir.join("../../../horus_library/tools/sim2d/configs");
                    if configs_path.exists() {
                        dialog = dialog.set_directory(&configs_path);
                    }
                }
            }

            if let Some(path) = dialog.pick_file() {
                match AppConfig::load_robot_config(path.to_str().unwrap()) {
                    Ok(config) => {
                        app_config.robot_config = config;
                        ui_state.robot_config_path = Some(path);
                        ui_state.status_message = "Robot config loaded! Changes applied.".to_string();
                        info!(" Loaded robot config - changes applied live");
                    }
                    Err(e) => {
                        ui_state.status_message = format!("Error: {}", e);
                        warn!(" Failed to load robot config: {}", e);
                    }
                }
            }
            ui_state.show_file_dialog = FileDialogType::None;
        }
        FileDialogType::WorldConfig => {
            let mut dialog = rfd::FileDialog::new()
                .add_filter("All Supported Files", &["yaml", "yml", "toml", "png", "jpg", "jpeg", "pgm"])
                .add_filter("Image Files (PNG, JPG, PGM)", &["png", "jpg", "jpeg", "pgm"])
                .add_filter("Config Files (YAML, TOML)", &["yaml", "yml", "toml"])
                .set_title("Load World Configuration or Image");

            // Try to set default directory to configs folder
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let configs_path = exe_dir.join("../../../horus_library/tools/sim2d/configs");
                    if configs_path.exists() {
                        dialog = dialog.set_directory(&configs_path);
                    }
                }
            }

            if let Some(path) = dialog.pick_file() {
                let path_str = path.to_str().unwrap();

                // Check if it's an image file
                if path_str.ends_with(".png") || path_str.ends_with(".jpg") ||
                   path_str.ends_with(".jpeg") || path_str.ends_with(".pgm") {
                    // Load from image with default resolution and threshold
                    match AppConfig::load_world_from_image(path_str, 0.05, 128) {
                        Ok(config) => {
                            app_config.world_config = config;
                            ui_state.world_config_path = Some(path);
                            ui_state.status_message =
                                "World loaded from image! Reloading...".to_string();
                            info!(" Loaded world from image - reloading world");
                        }
                        Err(e) => {
                            ui_state.status_message = format!("Error: {}", e);
                            warn!(" Failed to load world from image: {}", e);
                        }
                    }
                } else {
                    // Load from config file
                    match AppConfig::load_world_config(path_str) {
                        Ok(config) => {
                            app_config.world_config = config;
                            ui_state.world_config_path = Some(path);
                            ui_state.status_message =
                                "World config loaded! Reloading...".to_string();
                            info!(" Loaded world config - reloading world");
                        }
                        Err(e) => {
                            ui_state.status_message = format!("Error: {}", e);
                            warn!(" Failed to load world config: {}", e);
                        }
                    }
                }
            }
            ui_state.show_file_dialog = FileDialogType::None;
        }
        FileDialogType::None => {}
    }
}
