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
    pub paused: bool,
    pub simulation_speed: f32,
    pub show_world_section: bool,
    pub show_robot_section: bool,
    pub show_topics_section: bool,
    pub show_camera_section: bool,
    pub show_visual_section: bool,
    pub show_telemetry_section: bool,
    pub reset_simulation: bool,
    pub show_help: bool,
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
    pub show_lidar_rays: bool,
    pub show_trajectory: bool,
    pub trajectory_length: usize,
}

impl Default for VisualPreferences {
    fn default() -> Self {
        Self {
            show_grid: true,
            grid_spacing: 1.0, // 1 meter grid
            grid_color: [0.2, 0.2, 0.2],
            obstacle_color: [0.6, 0.4, 0.2],   // Brown
            wall_color: [0.3, 0.3, 0.3],       // Gray
            background_color: [0.1, 0.1, 0.1], // Dark gray
            show_velocity_arrows: false,
            show_lidar_rays: false,
            show_trajectory: false,
            trajectory_length: 100,
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

/// Robot telemetry data
#[derive(Resource, Default)]
pub struct RobotTelemetry {
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub heading: f32,
    pub angular_velocity: f32,
}

/// Performance metrics
#[derive(Resource)]
pub struct PerformanceMetrics {
    pub fps: f32,
    pub frame_time: f32,
    pub physics_time: f32,
    last_update: std::time::Instant,
    frame_count: u32,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time: 0.0,
            physics_time: 0.0,
            last_update: std::time::Instant::now(),
            frame_count: 0,
        }
    }
}

impl PerformanceMetrics {
    pub fn update(&mut self) {
        self.frame_count += 1;
        let elapsed = self.last_update.elapsed();

        if elapsed.as_secs_f32() >= 0.5 {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_time = 1000.0 / self.fps;
            self.frame_count = 0;
            self.last_update = std::time::Instant::now();
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
            paused: false,
            simulation_speed: 1.0,
            show_world_section: true,
            show_robot_section: true,
            show_topics_section: false,
            show_camera_section: false,
            show_visual_section: false,
            show_telemetry_section: true,
            reset_simulation: false,
            show_help: false,
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
    telemetry: Option<Res<RobotTelemetry>>,
    mut metrics: ResMut<PerformanceMetrics>,
) {
    // Update performance metrics
    metrics.update();

    egui::SidePanel::left("control_panel")
        .min_width(340.0)
        .max_width(400.0)
        .resizable(true)
        .show(contexts.ctx_mut(), |ui| {
            // Header with improved styling
            ui.vertical_centered(|ui| {
                ui.add_space(5.0);
                ui.heading(egui::RichText::new("sim2d").size(28.0).strong());
                ui.label(egui::RichText::new("2D Robotics Simulator").size(11.0).color(egui::Color32::from_rgb(150, 150, 150)));
                ui.add_space(3.0);
            });
            ui.separator();
            ui.add_space(5.0);

            // Simulation Controls - Improved layout
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Simulation Control").strong().size(13.0));
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        let play_pause_text = if ui_state.paused { "Play" } else { "Pause" };
                        let play_pause_color = if ui_state.paused {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::YELLOW
                        };

                        let button = egui::Button::new(egui::RichText::new(play_pause_text).size(14.0).color(play_pause_color))
                            .min_size(egui::vec2(80.0, 30.0));
                        if ui.add(button).clicked() {
                            ui_state.paused = !ui_state.paused;
                            ui_state.status_message = if ui_state.paused {
                                "Simulation paused".to_string()
                            } else {
                                "Simulation resumed".to_string()
                            };
                        }

                        let reset_button = egui::Button::new(egui::RichText::new("Reset").size(14.0))
                            .min_size(egui::vec2(80.0, 30.0));
                        if ui.add(reset_button).clicked() {
                            ui_state.reset_simulation = true;
                            ui_state.status_message = "Resetting simulation...".to_string();
                        }

                        let help_button = egui::Button::new(egui::RichText::new("?").size(14.0))
                            .min_size(egui::vec2(30.0, 30.0));
                        if ui.add(help_button).clicked() {
                            ui_state.show_help = !ui_state.show_help;
                        }
                    });
                });
            });

            ui.horizontal(|ui| {
                ui.label("Speed:");
                if ui.add(egui::Slider::new(&mut ui_state.simulation_speed, 0.1..=5.0).text("x")).changed() {
                    ui_state.status_message = format!("Speed: {:.1}x", ui_state.simulation_speed);
                }
            });

            ui.separator();

            // Scrollable area for sections
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Telemetry Section
                egui::CollapsingHeader::new(egui::RichText::new("Telemetry").size(14.0))
                    .default_open(ui_state.show_telemetry_section)
                    .show(ui, |ui| {
                        ui_state.show_telemetry_section = true;

                        if let Some(telem) = telemetry.as_ref() {
                            ui.label(format!("Position: ({:.2}, {:.2}) m", telem.position.0, telem.position.1));
                            ui.label(format!("Velocity: ({:.2}, {:.2}) m/s", telem.velocity.0, telem.velocity.1));
                            ui.label(format!("Heading: {:.1}°", telem.heading.to_degrees()));
                            ui.label(format!("Angular Vel: {:.2} rad/s", telem.angular_velocity));
                        } else {
                            ui.label(egui::RichText::new("No telemetry data").color(egui::Color32::GRAY));
                        }

                        ui.add_space(5.0);
                        ui.label(egui::RichText::new("Performance").strong());
                        ui.label(format!("FPS: {:.0}", metrics.fps));
                        ui.label(format!("Frame Time: {:.1} ms", metrics.frame_time));
                    });

                // World Configuration Section
                egui::CollapsingHeader::new(egui::RichText::new("World").size(14.0))
                    .default_open(ui_state.show_world_section)
                    .show(ui, |ui| {
                        ui_state.show_world_section = true;

                        if ui.button("Load World File").clicked() {
                            ui_state.show_file_dialog = FileDialogType::WorldConfig;
                        }

                        if let Some(path) = &ui_state.world_config_path {
                            ui.label(egui::RichText::new(format!(
                                "File: {}",
                                path.file_name().unwrap().to_string_lossy()
                            )).color(egui::Color32::LIGHT_GREEN));
                        } else {
                            ui.label(egui::RichText::new("Using default config").color(egui::Color32::GRAY));
                        }

                        ui.add_space(5.0);
                        ui.label(format!("Size: {:.1}m × {:.1}m", app_config.world_config.width, app_config.world_config.height));
                        ui.label(format!("Obstacles: {}", app_config.world_config.obstacles.len()));
                    });

                // Robot Configuration Section
                egui::CollapsingHeader::new(egui::RichText::new("Robot").size(14.0))
                    .default_open(ui_state.show_robot_section)
                    .show(ui, |ui| {
                        ui_state.show_robot_section = true;

                        if ui.button("Load Robot File").clicked() {
                            ui_state.show_file_dialog = FileDialogType::RobotConfig;
                        }

                        if let Some(path) = &ui_state.robot_config_path {
                            ui.label(egui::RichText::new(format!(
                                "File: {}",
                                path.file_name().unwrap().to_string_lossy()
                            )).color(egui::Color32::LIGHT_GREEN));
                        } else {
                            ui.label(egui::RichText::new("Using default config").color(egui::Color32::GRAY));
                        }

                        ui.add_space(5.0);
                        ui.label(egui::RichText::new("Live Parameters").strong());

                        // Edit first robot's parameters (if any robots exist)
                        if let Some(robot_config) = app_config.robots.get_mut(0) {
                            ui.horizontal(|ui| {
                                ui.label("Max Speed:");
                                if ui.add(egui::Slider::new(&mut robot_config.max_speed, 0.1..=10.0).suffix(" m/s")).changed() {
                                    ui_state.status_message = format!("Max speed: {:.1} m/s", robot_config.max_speed);
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Length:");
                                if ui.add(egui::Slider::new(&mut robot_config.length, 0.1..=3.0).suffix(" m")).changed() {
                                    ui_state.status_message = "Robot size updated".to_string();
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Width:");
                                if ui.add(egui::Slider::new(&mut robot_config.width, 0.1..=3.0).suffix(" m")).changed() {
                                    ui_state.status_message = "Robot size updated".to_string();
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                let mut color = egui::Color32::from_rgb(
                                    (robot_config.color[0] * 255.0) as u8,
                                    (robot_config.color[1] * 255.0) as u8,
                                    (robot_config.color[2] * 255.0) as u8,
                                );
                                if ui.color_edit_button_srgba(&mut color).changed() {
                                    robot_config.color = [
                                        color.r() as f32 / 255.0,
                                        color.g() as f32 / 255.0,
                                        color.b() as f32 / 255.0,
                                    ];
                                }
                            });
                        } else {
                            ui.label("No robots configured");
                        }
                    });

                // Topics Section
                egui::CollapsingHeader::new(egui::RichText::new("Topics").size(14.0))
                    .default_open(ui_state.show_topics_section)
                    .show(ui, |ui| {
                        ui_state.show_topics_section = true;

                        ui.horizontal(|ui| {
                            ui.label("Subscribe:");
                            if ui_state.topic_input.is_empty() {
                                ui_state.topic_input = app_config.args.topic.clone();
                            }
                            ui.text_edit_singleline(&mut ui_state.topic_input);
                        });
                        ui.label(egui::RichText::new("Note: Changing topic requires restart").color(egui::Color32::YELLOW).size(10.0));

                        ui.add_space(5.0);
                        ui.label(egui::RichText::new("Active Topics:").strong());
                        if ui_state.active_topics.is_empty() {
                            ui_state.active_topics.push(app_config.args.topic.clone());
                        }
                        for topic in &ui_state.active_topics {
                            ui.label(format!("  - {}", topic));
                        }
                    });

                // Camera Controls Section
                egui::CollapsingHeader::new(egui::RichText::new("Camera").size(14.0))
                    .default_open(ui_state.show_camera_section)
                    .show(ui, |ui| {
                        ui_state.show_camera_section = true;

                        ui.horizontal(|ui| {
                            ui.label("Zoom:");
                            if ui.add(egui::Slider::new(&mut camera_controller.zoom, 0.1..=5.0).text("x")).changed() {
                                ui_state.status_message = format!("Zoom: {:.1}x", camera_controller.zoom);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Pan X:");
                            ui.add(egui::Slider::new(&mut camera_controller.pan_x, -1000.0..=1000.0).text("px"));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Pan Y:");
                            ui.add(egui::Slider::new(&mut camera_controller.pan_y, -1000.0..=1000.0).text("px"));
                        });

                        if ui.button("Reset Camera").clicked() {
                            camera_controller.zoom = 1.0;
                            camera_controller.pan_x = 0.0;
                            camera_controller.pan_y = 0.0;
                            ui_state.status_message = "Camera reset".to_string();
                        }
                    });

                // Visual Customization Section
                egui::CollapsingHeader::new(egui::RichText::new("Visuals").size(14.0))
                    .default_open(ui_state.show_visual_section)
                    .show(ui, |ui| {
                        ui_state.show_visual_section = true;

                        ui.checkbox(&mut visual_prefs.show_grid, "Show Grid");

                        if visual_prefs.show_grid {
                            ui.horizontal(|ui| {
                                ui.label("  Spacing:");
                                ui.add(egui::Slider::new(&mut visual_prefs.grid_spacing, 0.5..=5.0).suffix(" m"));
                            });

                            ui.horizontal(|ui| {
                                ui.label("  Color:");
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

                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.label("Obstacle:");
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
                            ui.label("Wall:");
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

                        ui.add_space(5.0);
                        ui.checkbox(&mut visual_prefs.show_velocity_arrows, "Show Velocity Arrows");
                        ui.checkbox(&mut visual_prefs.show_lidar_rays, "Show LIDAR Rays");
                        ui.checkbox(&mut visual_prefs.show_trajectory, "Show Trajectory Trail");

                        if visual_prefs.show_trajectory {
                            ui.horizontal(|ui| {
                                ui.label("  Trail Length:");
                                ui.add(egui::Slider::new(&mut visual_prefs.trajectory_length, 10..=500));
                            });
                        }
                    });
            });

            // Status Bar - Outside scroll area
            ui.add_space(5.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Status:").strong());
                ui.label(&ui_state.status_message);
            });
        });

    // Help Dialog Window
    if ui_state.show_help {
        egui::Window::new("Help & Controls")
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .show(contexts.ctx_mut(), |ui| {
                ui.heading("Keyboard Shortcuts");
                ui.add_space(10.0);

                ui.label(egui::RichText::new("Camera Controls").strong());
                ui.label("  Middle Mouse + Drag - Pan camera");
                ui.label("  Scroll Wheel - Zoom in/out");
                ui.add_space(5.0);

                ui.label(egui::RichText::new("Simulation").strong());
                ui.label("  Space - Toggle pause/play");
                ui.label("  R - Reset simulation");
                ui.add_space(5.0);

                ui.label(egui::RichText::new("Topics & Commands").strong());
                ui.label(format!("  Listening on: {}", app_config.args.topic));
                ui.label("  Send CmdVel messages to control robot");
                ui.add_space(5.0);

                ui.label(egui::RichText::new("Tips").strong());
                ui.label("  - Adjust simulation speed with slider");
                ui.label("  - Load custom robot/world configs");
                ui.label("  - Toggle visual elements in Visuals panel");
                ui.add_space(10.0);

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        ui_state.show_help = false;
                    }
                });
            });
    }
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
                        // Update first robot or create new robots list with this config
                        if app_config.robots.is_empty() {
                            app_config.robots = vec![config];
                        } else {
                            app_config.robots[0] = config;
                        }
                        ui_state.robot_config_path = Some(path);
                        ui_state.status_message =
                            "Robot config loaded! Changes applied.".to_string();
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
                .add_filter(
                    "All Supported Files",
                    &["yaml", "yml", "toml", "png", "jpg", "jpeg", "pgm"],
                )
                .add_filter(
                    "Image Files (PNG, JPG, PGM)",
                    &["png", "jpg", "jpeg", "pgm"],
                )
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
                if path_str.ends_with(".png")
                    || path_str.ends_with(".jpg")
                    || path_str.ends_with(".jpeg")
                    || path_str.ends_with(".pgm")
                {
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
