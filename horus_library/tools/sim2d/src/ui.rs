//! UI module for sim2d control panel

use crate::{recorder::Recorder, scenario::Scenario, AppConfig};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::path::PathBuf;

/// UI state resource
#[derive(Resource)]
pub struct UiState {
    pub robot_config_path: Option<PathBuf>,
    pub world_config_path: Option<PathBuf>,
    pub scenario_path: Option<PathBuf>,
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
    pub show_scenario_section: bool,
    pub show_recording_section: bool,
    pub recording_name: String,
    pub recording_description: String,
    pub recording_path: Option<PathBuf>,
    pub reset_simulation: bool,
    pub show_help: bool,
    pub show_editor_section: bool,
    pub editor_selected_color: [f32; 3],
    pub show_metrics_section: bool,
    pub metrics_export_path: Option<PathBuf>,
    pub show_tutorial_section: bool,
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
    ScenarioSave,
    ScenarioLoad,
    RecordingLoad,
    ExportCSV,
    ExportVideo,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            robot_config_path: None,
            world_config_path: None,
            scenario_path: None,
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
            show_scenario_section: false,
            show_recording_section: false,
            recording_name: "My Recording".to_string(),
            recording_description: String::new(),
            recording_path: None,
            reset_simulation: false,
            show_help: false,
            show_editor_section: false,
            editor_selected_color: [0.6, 0.6, 0.6],
            show_metrics_section: false,
            metrics_export_path: None,
            show_tutorial_section: false,
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
    mut recorder: ResMut<Recorder>,
    mut editor: ResMut<crate::editor::WorldEditor>,
    mut perf_metrics: ResMut<crate::metrics::PerformanceMetrics>,
    mut tutorial_state: ResMut<crate::tutorial::TutorialState>,
) {
    // Update performance metrics
    metrics.update();

    // Handle keyboard shortcuts
    let ctx = contexts.ctx_mut();
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
        ui_state.show_file_dialog = FileDialogType::ScenarioSave;
    }
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
        ui_state.show_file_dialog = FileDialogType::ScenarioLoad;
    }

    // Editor keyboard shortcuts
    if editor.enabled {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            if editor.undo().is_some() {
                ui_state.status_message = "Undone".to_string();
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
            if editor.redo().is_some() {
                ui_state.status_message = "Redone".to_string();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if !editor.selected_obstacles.is_empty() {
                editor.clear_selection();
                ui_state.status_message = "Selection cleared".to_string();
            }
        }
    }

    egui::SidePanel::left("control_panel")
        .min_width(340.0)
        .max_width(400.0)
        .resizable(true)
        .show(contexts.ctx_mut(), |ui| {
            // Header with improved styling
            ui.vertical_centered(|ui| {
                ui.add_space(5.0);
                ui.heading(egui::RichText::new("sim2d").size(28.0).strong());
                ui.label(
                    egui::RichText::new("2D Robotics Simulator")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 150, 150)),
                );
                ui.add_space(3.0);
            });
            ui.separator();
            ui.add_space(5.0);

            // Simulation Controls - Improved layout
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Simulation Control")
                            .strong()
                            .size(13.0),
                    );
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        let play_pause_text = if ui_state.paused { "Play" } else { "Pause" };
                        let play_pause_color = if ui_state.paused {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::YELLOW
                        };

                        let button = egui::Button::new(
                            egui::RichText::new(play_pause_text)
                                .size(14.0)
                                .color(play_pause_color),
                        )
                        .min_size(egui::vec2(80.0, 30.0));
                        if ui.add(button).clicked() {
                            ui_state.paused = !ui_state.paused;
                            ui_state.status_message = if ui_state.paused {
                                "Simulation paused".to_string()
                            } else {
                                "Simulation resumed".to_string()
                            };
                        }

                        let reset_button =
                            egui::Button::new(egui::RichText::new("Reset").size(14.0))
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
                if ui
                    .add(egui::Slider::new(&mut ui_state.simulation_speed, 0.1..=5.0).text("x"))
                    .changed()
                {
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
                            ui.label(format!(
                                "Position: ({:.2}, {:.2}) m",
                                telem.position.0, telem.position.1
                            ));
                            ui.label(format!(
                                "Velocity: ({:.2}, {:.2}) m/s",
                                telem.velocity.0, telem.velocity.1
                            ));
                            ui.label(format!("Heading: {:.1}°", telem.heading.to_degrees()));
                            ui.label(format!("Angular Vel: {:.2} rad/s", telem.angular_velocity));
                        } else {
                            ui.label(
                                egui::RichText::new("No telemetry data").color(egui::Color32::GRAY),
                            );
                        }

                        ui.add_space(5.0);
                        ui.label(egui::RichText::new("Performance").strong());
                        ui.label(format!("FPS: {:.0}", metrics.fps));
                        ui.label(format!("Frame Time: {:.1} ms", metrics.frame_time));
                    });

                // Scenarios Section
                egui::CollapsingHeader::new(egui::RichText::new("💾 Scenarios").size(14.0))
                    .default_open(ui_state.show_scenario_section)
                    .show(ui, |ui| {
                        ui_state.show_scenario_section = true;

                        ui.label(
                            egui::RichText::new("Save/Load complete simulation states")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)),
                        );
                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            let save_button = egui::Button::new(
                                egui::RichText::new("💾 Save Scenario").size(13.0),
                            )
                            .min_size(egui::vec2(140.0, 28.0));

                            if ui.add(save_button).clicked() {
                                ui_state.show_file_dialog = FileDialogType::ScenarioSave;
                            }

                            let load_button = egui::Button::new(
                                egui::RichText::new("📂 Load Scenario").size(13.0),
                            )
                            .min_size(egui::vec2(140.0, 28.0));

                            if ui.add(load_button).clicked() {
                                ui_state.show_file_dialog = FileDialogType::ScenarioLoad;
                            }
                        });

                        if let Some(path) = &ui_state.scenario_path {
                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "Current: {}",
                                    path.file_name().unwrap().to_string_lossy()
                                ))
                                .color(egui::Color32::LIGHT_GREEN),
                            );
                        } else {
                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new("No scenario loaded")
                                    .color(egui::Color32::GRAY),
                            );
                        }

                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new("Keyboard shortcuts:")
                                .size(11.0)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("  Ctrl+S - Quick save")
                                .size(10.0)
                                .color(egui::Color32::from_rgb(180, 180, 180)),
                        );
                        ui.label(
                            egui::RichText::new("  Ctrl+O - Open scenario")
                                .size(10.0)
                                .color(egui::Color32::from_rgb(180, 180, 180)),
                        );
                    });

                // Recording Section
                egui::CollapsingHeader::new(egui::RichText::new("🎬 Recording").size(14.0))
                    .default_open(ui_state.show_recording_section)
                    .show(ui, |ui| {
                        ui_state.show_recording_section = true;

                        ui.label(
                            egui::RichText::new("Record and export simulation runs")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)),
                        );
                        ui.add_space(5.0);

                        // Recording name input
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut ui_state.recording_name);
                        });

                        // Recording description input
                        ui.horizontal(|ui| {
                            ui.label("Desc:");
                            ui.text_edit_singleline(&mut ui_state.recording_description);
                        });

                        ui.add_space(5.0);

                        // Recording controls
                        if recorder.is_recording() {
                            // Show recording status
                            let metadata = recorder.get_metadata().unwrap();
                            ui.label(
                                egui::RichText::new(format!("🔴 Recording: {}", metadata.name))
                                    .color(egui::Color32::RED)
                                    .strong(),
                            );
                            ui.label(format!("Frames: {}", metadata.frame_count));
                            ui.label(format!("Duration: {:.1}s", metadata.duration));

                            ui.add_space(5.0);

                            // Stop button
                            let stop_button = egui::Button::new(
                                egui::RichText::new("⏹ Stop Recording")
                                    .size(13.0)
                                    .color(egui::Color32::RED),
                            )
                            .min_size(egui::vec2(200.0, 28.0));

                            if ui.add(stop_button).clicked() {
                                if let Some(recording) = recorder.stop_recording() {
                                    // Save dialog
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Recording Files", &["yaml", "yml"])
                                        .set_file_name("recording.yaml")
                                        .save_file()
                                    {
                                        match recording.save_to_file(&path) {
                                            Ok(()) => {
                                                ui_state.recording_path = Some(path.clone());
                                                ui_state.status_message = format!(
                                                    "Recording saved: {}",
                                                    path.file_name().unwrap().to_string_lossy()
                                                );
                                            }
                                            Err(e) => {
                                                ui_state.status_message =
                                                    format!("Error saving recording: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // Start button
                            let start_button = egui::Button::new(
                                egui::RichText::new("🔴 Start Recording")
                                    .size(13.0)
                                    .color(egui::Color32::GREEN),
                            )
                            .min_size(egui::vec2(200.0, 28.0));

                            if ui.add(start_button).clicked() {
                                recorder.start_recording(
                                    ui_state.recording_name.clone(),
                                    ui_state.recording_description.clone(),
                                    app_config.robots.clone(),
                                );
                                ui_state.status_message = "Recording started".to_string();
                            }
                        }

                        // Show last saved recording path
                        if let Some(path) = &ui_state.recording_path {
                            ui.add_space(3.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "Last: {}",
                                    path.file_name().unwrap().to_string_lossy()
                                ))
                                .size(10.0)
                                .color(egui::Color32::from_rgb(120, 120, 120)),
                            );
                        }

                        ui.add_space(5.0);
                        ui.separator();

                        // Load recording button
                        ui.label(egui::RichText::new("Playback").size(11.0).strong());
                        if ui.button("📂 Load Recording").clicked() {
                            ui_state.show_file_dialog = FileDialogType::RecordingLoad;
                        }

                        ui.add_space(5.0);
                        ui.separator();

                        // Export options
                        ui.label(egui::RichText::new("Export").size(11.0).strong());
                        if ui.button("📊 Export to CSV").clicked() {
                            ui_state.show_file_dialog = FileDialogType::ExportCSV;
                        }
                        if ui.button("🎥 Export to Video (MP4)").clicked() {
                            ui_state.show_file_dialog = FileDialogType::ExportVideo;
                        }
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
                            ui.label(
                                egui::RichText::new(format!(
                                    "File: {}",
                                    path.file_name().unwrap().to_string_lossy()
                                ))
                                .color(egui::Color32::LIGHT_GREEN),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("Using default config")
                                    .color(egui::Color32::GRAY),
                            );
                        }

                        ui.add_space(5.0);
                        ui.label(format!(
                            "Size: {:.1}m × {:.1}m",
                            app_config.world_config.width, app_config.world_config.height
                        ));
                        ui.label(format!(
                            "Obstacles: {}",
                            app_config.world_config.obstacles.len()
                        ));
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
                            ui.label(
                                egui::RichText::new(format!(
                                    "File: {}",
                                    path.file_name().unwrap().to_string_lossy()
                                ))
                                .color(egui::Color32::LIGHT_GREEN),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("Using default config")
                                    .color(egui::Color32::GRAY),
                            );
                        }

                        ui.add_space(5.0);
                        ui.label(egui::RichText::new("Live Parameters").strong());

                        // Edit first robot's parameters (if any robots exist)
                        if let Some(robot_config) = app_config.robots.get_mut(0) {
                            ui.horizontal(|ui| {
                                ui.label("Max Speed:");
                                if ui
                                    .add(
                                        egui::Slider::new(&mut robot_config.max_speed, 0.1..=10.0)
                                            .suffix(" m/s"),
                                    )
                                    .changed()
                                {
                                    ui_state.status_message =
                                        format!("Max speed: {:.1} m/s", robot_config.max_speed);
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Length:");
                                if ui
                                    .add(
                                        egui::Slider::new(&mut robot_config.length, 0.1..=3.0)
                                            .suffix(" m"),
                                    )
                                    .changed()
                                {
                                    ui_state.status_message = "Robot size updated".to_string();
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Width:");
                                if ui
                                    .add(
                                        egui::Slider::new(&mut robot_config.width, 0.1..=3.0)
                                            .suffix(" m"),
                                    )
                                    .changed()
                                {
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
                        ui.label(
                            egui::RichText::new("Note: Changing topic requires restart")
                                .color(egui::Color32::YELLOW)
                                .size(10.0),
                        );

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
                            if ui
                                .add(
                                    egui::Slider::new(&mut camera_controller.zoom, 0.1..=5.0)
                                        .text("x"),
                                )
                                .changed()
                            {
                                ui_state.status_message =
                                    format!("Zoom: {:.1}x", camera_controller.zoom);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Pan X:");
                            ui.add(
                                egui::Slider::new(&mut camera_controller.pan_x, -1000.0..=1000.0)
                                    .text("px"),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Pan Y:");
                            ui.add(
                                egui::Slider::new(&mut camera_controller.pan_y, -1000.0..=1000.0)
                                    .text("px"),
                            );
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
                                ui.add(
                                    egui::Slider::new(&mut visual_prefs.grid_spacing, 0.5..=5.0)
                                        .suffix(" m"),
                                );
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
                        ui.checkbox(
                            &mut visual_prefs.show_velocity_arrows,
                            "Show Velocity Arrows",
                        );
                        ui.checkbox(&mut visual_prefs.show_lidar_rays, "Show LIDAR Rays");
                        ui.checkbox(&mut visual_prefs.show_trajectory, "Show Trajectory Trail");

                        if visual_prefs.show_trajectory {
                            ui.horizontal(|ui| {
                                ui.label("  Trail Length:");
                                ui.add(egui::Slider::new(
                                    &mut visual_prefs.trajectory_length,
                                    10..=500,
                                ));
                            });
                        }
                    });

                // World Editor Section
                egui::CollapsingHeader::new(egui::RichText::new("🛠 World Editor").size(14.0))
                    .default_open(ui_state.show_editor_section)
                    .show(ui, |ui| {
                        ui_state.show_editor_section = true;

                        ui.label(
                            egui::RichText::new("Create and modify obstacles interactively")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)),
                        );
                        ui.add_space(5.0);

                        // Enable/Disable editor
                        ui.horizontal(|ui| {
                            let enable_text = if editor.enabled {
                                "Disable Editor"
                            } else {
                                "Enable Editor"
                            };
                            let enable_color = if editor.enabled {
                                egui::Color32::RED
                            } else {
                                egui::Color32::GREEN
                            };

                            let button = egui::Button::new(
                                egui::RichText::new(enable_text)
                                    .size(13.0)
                                    .color(enable_color),
                            )
                            .min_size(egui::vec2(200.0, 28.0));

                            if ui.add(button).clicked() {
                                editor.enabled = !editor.enabled;
                                ui_state.status_message = if editor.enabled {
                                    "Editor enabled".to_string()
                                } else {
                                    "Editor disabled".to_string()
                                };
                            }
                        });

                        if editor.enabled {
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new("Tools").strong());

                            // Tool selection buttons
                            ui.horizontal(|ui| {
                                use crate::editor::EditorTool;

                                let select_color = if editor.active_tool == EditorTool::Select {
                                    egui::Color32::LIGHT_BLUE
                                } else {
                                    egui::Color32::GRAY
                                };

                                if ui
                                    .button(egui::RichText::new("Select").color(select_color))
                                    .clicked()
                                {
                                    editor.active_tool = EditorTool::Select;
                                    ui_state.status_message = "Tool: Select".to_string();
                                }

                                let rect_color = if editor.active_tool == EditorTool::Rectangle {
                                    egui::Color32::LIGHT_BLUE
                                } else {
                                    egui::Color32::GRAY
                                };

                                if ui
                                    .button(egui::RichText::new("Rectangle").color(rect_color))
                                    .clicked()
                                {
                                    editor.active_tool = EditorTool::Rectangle;
                                    ui_state.status_message = "Tool: Rectangle".to_string();
                                }
                            });

                            ui.horizontal(|ui| {
                                use crate::editor::EditorTool;

                                let circle_color = if editor.active_tool == EditorTool::Circle {
                                    egui::Color32::LIGHT_BLUE
                                } else {
                                    egui::Color32::GRAY
                                };

                                if ui
                                    .button(egui::RichText::new("Circle").color(circle_color))
                                    .clicked()
                                {
                                    editor.active_tool = EditorTool::Circle;
                                    ui_state.status_message = "Tool: Circle".to_string();
                                }

                                let delete_color = if editor.active_tool == EditorTool::Delete {
                                    egui::Color32::RED
                                } else {
                                    egui::Color32::GRAY
                                };

                                if ui
                                    .button(egui::RichText::new("Delete").color(delete_color))
                                    .clicked()
                                {
                                    editor.active_tool = EditorTool::Delete;
                                    ui_state.status_message = "Tool: Delete".to_string();
                                }
                            });

                            ui.add_space(5.0);
                            ui.separator();

                            // Grid settings
                            ui.label(egui::RichText::new("Grid Settings").strong());
                            ui.checkbox(&mut editor.grid_snap, "Snap to Grid");

                            if editor.grid_snap {
                                ui.horizontal(|ui| {
                                    ui.label("  Grid Size:");
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut editor.grid_size, 0.1..=2.0)
                                                .suffix(" m"),
                                        )
                                        .changed()
                                    {
                                        ui_state.status_message =
                                            format!("Grid size: {:.1}m", editor.grid_size);
                                    }
                                });
                            }

                            ui.add_space(5.0);
                            ui.separator();

                            // Obstacle properties (for new obstacles)
                            ui.label(egui::RichText::new("New Obstacle Color").strong());
                            ui.horizontal(|ui| {
                                let mut color = egui::Color32::from_rgb(
                                    (ui_state.editor_selected_color[0] * 255.0) as u8,
                                    (ui_state.editor_selected_color[1] * 255.0) as u8,
                                    (ui_state.editor_selected_color[2] * 255.0) as u8,
                                );
                                if ui.color_edit_button_srgba(&mut color).changed() {
                                    ui_state.editor_selected_color = [
                                        color.r() as f32 / 255.0,
                                        color.g() as f32 / 255.0,
                                        color.b() as f32 / 255.0,
                                    ];
                                }
                            });

                            ui.add_space(5.0);
                            ui.separator();

                            // Undo/Redo
                            ui.label(egui::RichText::new("History").strong());
                            ui.horizontal(|ui| {
                                let undo_enabled = !editor.undo_stack.is_empty();
                                let redo_enabled = !editor.redo_stack.is_empty();

                                if ui
                                    .add_enabled(undo_enabled, egui::Button::new("↶ Undo"))
                                    .clicked()
                                {
                                    if editor.undo().is_some() {
                                        ui_state.status_message = "Undone".to_string();
                                    }
                                }

                                if ui
                                    .add_enabled(redo_enabled, egui::Button::new("↷ Redo"))
                                    .clicked()
                                {
                                    if editor.redo().is_some() {
                                        ui_state.status_message = "Redone".to_string();
                                    }
                                }
                            });

                            ui.add_space(5.0);

                            // Selection info
                            if !editor.selected_obstacles.is_empty() {
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Selected: {} obstacle(s)",
                                        editor.selected_obstacles.len()
                                    ))
                                    .color(egui::Color32::LIGHT_GREEN),
                                );

                                if ui.button("Clear Selection").clicked() {
                                    editor.clear_selection();
                                    ui_state.status_message = "Selection cleared".to_string();
                                }
                            }

                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new("Keyboard shortcuts:")
                                    .size(11.0)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new("  Ctrl+Z - Undo")
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(180, 180, 180)),
                            );
                            ui.label(
                                egui::RichText::new("  Ctrl+Y - Redo")
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(180, 180, 180)),
                            );
                            ui.label(
                                egui::RichText::new("  Escape - Clear selection")
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(180, 180, 180)),
                            );
                        }
                    });

                // Performance Metrics Section
                egui::CollapsingHeader::new(
                    egui::RichText::new("📊 Performance Metrics").size(14.0),
                )
                .default_open(ui_state.show_metrics_section)
                .show(ui, |ui| {
                    ui_state.show_metrics_section = true;

                    ui.label(
                        egui::RichText::new("Real-time performance tracking and analysis")
                            .size(11.0)
                            .color(egui::Color32::from_rgb(150, 150, 150)),
                    );
                    ui.add_space(5.0);

                    // Goal Status
                    if perf_metrics.goal_reached {
                        ui.label(
                            egui::RichText::new("✅ Goal Reached!")
                                .size(14.0)
                                .color(egui::Color32::GREEN)
                                .strong(),
                        );
                        if let Some(time) = perf_metrics.time_to_goal {
                            ui.label(format!("Time: {:.2}s", time));
                        }
                    } else if perf_metrics.goal_position.is_some() {
                        ui.label(
                            egui::RichText::new("🎯 In Progress...")
                                .size(14.0)
                                .color(egui::Color32::YELLOW)
                                .strong(),
                        );
                        ui.label(format!(
                            "Distance to Goal: {:.2}m",
                            perf_metrics.distance_to_goal
                        ));
                    } else {
                        ui.label(
                            egui::RichText::new("No goal set")
                                .size(12.0)
                                .color(egui::Color32::GRAY),
                        );
                    }

                    ui.add_space(5.0);
                    ui.separator();

                    // Path Metrics
                    egui::CollapsingHeader::new(egui::RichText::new("Path Metrics").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(format!("📏 Path Length: {:.2}m", perf_metrics.path_length));
                            ui.label(format!("⚡ Avg Speed: {:.2} m/s", perf_metrics.avg_speed));
                            ui.label(format!("🚀 Max Speed: {:.2} m/s", perf_metrics.max_speed));

                            ui.add_space(3.0);
                            ui.label("Smoothness:");
                            ui.add(
                                egui::widgets::ProgressBar::new(perf_metrics.path_smoothness)
                                    .text(format!("{:.2}", perf_metrics.path_smoothness)),
                            );
                        });

                    // Safety Metrics
                    egui::CollapsingHeader::new(egui::RichText::new("Safety Metrics").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(format!("💥 Collisions: {}", perf_metrics.collision_count));
                            ui.label(format!("⚠️  Near Misses: {}", perf_metrics.near_miss_count));

                            ui.add_space(3.0);
                            let safety = perf_metrics.safety_score();
                            let safety_color = if safety > 0.8 {
                                egui::Color32::GREEN
                            } else if safety > 0.5 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::RED
                            };

                            ui.colored_label(safety_color, "Safety Score:");
                            ui.add(
                                egui::widgets::ProgressBar::new(safety)
                                    .text(format!("{:.2}", safety))
                                    .fill(safety_color),
                            );
                        });

                    // Resource Usage
                    egui::CollapsingHeader::new(egui::RichText::new("Resource Usage").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(format!(
                                "⏱️  Elapsed Time: {:.2}s",
                                perf_metrics.elapsed_time
                            ));
                            ui.label(format!("🔋 Energy: {:.2} J", perf_metrics.energy_consumed));
                        });

                    // Overall Scores
                    ui.add_space(5.0);
                    ui.separator();
                    ui.label(egui::RichText::new("Overall Scores").strong());

                    let efficiency = perf_metrics.efficiency_score();
                    ui.label("Efficiency:");
                    ui.add(
                        egui::widgets::ProgressBar::new(efficiency)
                            .text(format!("{:.2}", efficiency)),
                    );

                    ui.add_space(3.0);
                    let overall = perf_metrics.overall_score();
                    let overall_color = if overall > 0.8 {
                        egui::Color32::GREEN
                    } else if overall > 0.5 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::LIGHT_RED
                    };

                    ui.colored_label(overall_color, "Overall:");
                    ui.add(
                        egui::widgets::ProgressBar::new(overall)
                            .text(format!("{:.2}", overall))
                            .fill(overall_color),
                    );

                    // Export Options
                    ui.add_space(5.0);
                    ui.separator();
                    ui.label(egui::RichText::new("Export").strong());

                    ui.horizontal(|ui| {
                        if ui.button("📊 Export CSV").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("CSV Files", &["csv"])
                                .set_file_name("metrics.csv")
                                .save_file()
                            {
                                match perf_metrics.export_to_csv(&path) {
                                    Ok(()) => {
                                        ui_state.metrics_export_path = Some(path.clone());
                                        ui_state.status_message = format!(
                                            "Metrics exported: {}",
                                            path.file_name().unwrap().to_string_lossy()
                                        );
                                    }
                                    Err(e) => {
                                        ui_state.status_message =
                                            format!("Error exporting metrics: {}", e);
                                    }
                                }
                            }
                        }

                        if ui.button("📄 Export JSON").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("JSON Files", &["json"])
                                .set_file_name("metrics.json")
                                .save_file()
                            {
                                match perf_metrics.export_to_json(&path) {
                                    Ok(()) => {
                                        ui_state.metrics_export_path = Some(path.clone());
                                        ui_state.status_message = format!(
                                            "Metrics exported: {}",
                                            path.file_name().unwrap().to_string_lossy()
                                        );
                                    }
                                    Err(e) => {
                                        ui_state.status_message =
                                            format!("Error exporting metrics: {}", e);
                                    }
                                }
                            }
                        }
                    });

                    if let Some(path) = &ui_state.metrics_export_path {
                        ui.add_space(3.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Last: {}",
                                path.file_name().unwrap().to_string_lossy()
                            ))
                            .size(10.0)
                            .color(egui::Color32::from_rgb(120, 120, 120)),
                        );
                    }

                    // Reset button
                    ui.add_space(5.0);
                    if ui.button("🔄 Reset Metrics").clicked() {
                        perf_metrics.reset();
                        ui_state.status_message = "Metrics reset".to_string();
                    }
                });

                // Tutorials Section
                egui::CollapsingHeader::new(
                    egui::RichText::new("📚 Interactive Tutorials").size(14.0),
                )
                .default_open(ui_state.show_tutorial_section)
                .show(ui, |ui| {
                    ui_state.show_tutorial_section = true;

                    ui.label(
                        egui::RichText::new("Learn HORUS and sim2d step-by-step")
                            .size(11.0)
                            .color(egui::Color32::from_rgb(150, 150, 150)),
                    );
                    ui.add_space(5.0);

                    // Active tutorial display
                    if let Some(tutorial) = &tutorial_state.active_tutorial {
                        // Tutorial header
                        ui.label(
                            egui::RichText::new(&tutorial.title)
                                .size(16.0)
                                .strong()
                                .color(egui::Color32::from_rgb(100, 150, 255)),
                        );
                        ui.label(
                            egui::RichText::new(&tutorial.description)
                                .size(10.0)
                                .color(egui::Color32::from_rgb(180, 180, 180)),
                        );

                        ui.add_space(5.0);

                        // Progress bar
                        let progress = tutorial.progress();
                        ui.label(format!(
                            "Progress: {} / {}",
                            tutorial.current_step.min(tutorial.steps.len()),
                            tutorial.steps.len()
                        ));
                        ui.add(
                            egui::widgets::ProgressBar::new(progress)
                                .text(format!("{:.0}%", progress * 100.0))
                                .fill(egui::Color32::from_rgb(100, 150, 255)),
                        );

                        ui.add_space(5.0);
                        ui.separator();

                        // Current step
                        if let Some(step) = tutorial.current_step() {
                            ui.label(egui::RichText::new(&step.title).size(14.0).strong());
                            ui.add_space(3.0);

                            ui.label(
                                egui::RichText::new(&step.instruction)
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(220, 220, 220)),
                            );

                            // Show hint if available
                            if let Some(hint) = &step.hint {
                                ui.add_space(3.0);
                                ui.label(
                                    egui::RichText::new(format!("💡 Hint: {}", hint))
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(255, 200, 100))
                                        .italics(),
                                );
                            }

                            ui.add_space(5.0);

                            // Show action type info
                            match &step.action_type {
                                crate::tutorial::TutorialActionType::StartSimulation => {
                                    ui.label(
                                        egui::RichText::new(
                                            "⏯️  Waiting for simulation to start...",
                                        )
                                        .size(10.0)
                                        .color(egui::Color32::YELLOW),
                                    );
                                }
                                crate::tutorial::TutorialActionType::SendCommand {
                                    topic,
                                    min_duration,
                                } => {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "📡 Send command to {} for {:.1}s",
                                            topic, min_duration
                                        ))
                                        .size(10.0)
                                        .color(egui::Color32::LIGHT_BLUE),
                                    );
                                }
                                crate::tutorial::TutorialActionType::ReachPosition {
                                    x,
                                    y,
                                    threshold,
                                } => {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "🎯 Target: ({:.1}, {:.1}) ±{:.1}m",
                                            x, y, threshold
                                        ))
                                        .size(10.0)
                                        .color(egui::Color32::GREEN),
                                    );
                                }
                                crate::tutorial::TutorialActionType::AvoidCollision {
                                    duration,
                                } => {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "🛡️  Stay collision-free for {:.1}s",
                                            duration
                                        ))
                                        .size(10.0)
                                        .color(egui::Color32::LIGHT_RED),
                                    );
                                }
                                crate::tutorial::TutorialActionType::ManualComplete => {
                                    ui.add_space(3.0);
                                    if ui.button("✅ Continue").clicked() {
                                        tutorial_state.complete_current_step();
                                        ui_state.status_message = "Step completed!".to_string();
                                    }
                                }
                            }
                        } else if tutorial.completed {
                            // Tutorial completed
                            ui.label(
                                egui::RichText::new("✅ Tutorial Completed!")
                                    .size(16.0)
                                    .strong()
                                    .color(egui::Color32::GREEN),
                            );
                            ui.label("Great job! You've finished this tutorial.");
                        }

                        ui.add_space(5.0);
                        ui.separator();

                        // Control buttons
                        ui.horizontal(|ui| {
                            if ui.button("⏹️ Stop Tutorial").clicked() {
                                tutorial_state.stop_tutorial();
                                ui_state.status_message = "Tutorial stopped".to_string();
                            }
                        });
                    } else {
                        // Tutorial selector
                        ui.label(
                            egui::RichText::new("Available Tutorials")
                                .size(13.0)
                                .strong(),
                        );
                        ui.add_space(5.0);

                        let tutorials = crate::tutorial::get_available_tutorials();
                        for tutorial in tutorials {
                            let is_completed = tutorial_state.is_completed(&tutorial.id);

                            ui.group(|ui| {
                                ui.set_min_width(ui.available_width() - 10.0);

                                ui.horizontal(|ui| {
                                    // Status indicator
                                    let status = if is_completed {
                                        egui::RichText::new("✅")
                                            .size(16.0)
                                            .color(egui::Color32::GREEN)
                                    } else {
                                        egui::RichText::new("⭕")
                                            .size(16.0)
                                            .color(egui::Color32::GRAY)
                                    };
                                    ui.label(status);

                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(&tutorial.title)
                                                .size(13.0)
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&tutorial.description)
                                                .size(10.0)
                                                .color(egui::Color32::from_rgb(180, 180, 180)),
                                        );
                                    });
                                });

                                ui.add_space(3.0);

                                ui.horizontal(|ui| {
                                    let button_text = if is_completed {
                                        "🔄 Replay"
                                    } else {
                                        "▶️ Start"
                                    };

                                    if ui.button(button_text).clicked() {
                                        tutorial_state.start_tutorial(tutorial.clone());
                                        ui_state.status_message =
                                            format!("Started tutorial: {}", tutorial.title);
                                    }

                                    if is_completed {
                                        ui.label(
                                            egui::RichText::new("Completed")
                                                .size(10.0)
                                                .color(egui::Color32::GREEN),
                                        );
                                    }
                                });
                            });

                            ui.add_space(3.0);
                        }
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

                ui.label(egui::RichText::new("Scenarios").strong());
                ui.label("  Ctrl+S - Save scenario");
                ui.label("  Ctrl+O - Open/load scenario");
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
pub fn file_dialog_system(
    mut ui_state: ResMut<UiState>,
    mut app_config: ResMut<AppConfig>,
    recorder: Res<Recorder>,
) {
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
        FileDialogType::ScenarioSave => {
            let mut dialog = rfd::FileDialog::new()
                .add_filter("Scenario Files", &["yaml", "yml"])
                .set_title("Save Scenario");

            // Try to set default directory to scenarios folder
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let scenarios_path =
                        exe_dir.join("../../../horus_library/tools/sim2d/scenarios");
                    if scenarios_path.exists() {
                        dialog = dialog.set_directory(&scenarios_path);
                    }
                }
            }

            // Set default filename if we have a current scenario
            if let Some(current_path) = &ui_state.scenario_path {
                if let Some(filename) = current_path.file_name() {
                    dialog = dialog.set_file_name(filename.to_string_lossy().as_ref());
                }
            } else {
                dialog = dialog.set_file_name("my_scenario.yaml");
            }

            if let Some(path) = dialog.save_file() {
                // Create scenario from current state
                let scenario = Scenario::from_current_state(
                    path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    "Saved from sim2d".to_string(),
                    &app_config.world_config,
                    &app_config.robots,
                    0.0, // Current time - would need to be tracked
                );

                match scenario.save_to_file(&path) {
                    Ok(()) => {
                        ui_state.scenario_path = Some(path.clone());
                        ui_state.status_message = format!(
                            "Scenario saved: {}",
                            path.file_name().unwrap().to_string_lossy()
                        );
                        info!("💾 Saved scenario to {:?}", path);
                    }
                    Err(e) => {
                        ui_state.status_message = format!("Error saving scenario: {}", e);
                        warn!("❌ Failed to save scenario: {}", e);
                    }
                }
            }
            ui_state.show_file_dialog = FileDialogType::None;
        }
        FileDialogType::ScenarioLoad => {
            let mut dialog = rfd::FileDialog::new()
                .add_filter("Scenario Files", &["yaml", "yml"])
                .set_title("Load Scenario");

            // Try to set default directory to scenarios folder
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let scenarios_path =
                        exe_dir.join("../../../horus_library/tools/sim2d/scenarios");
                    if scenarios_path.exists() {
                        dialog = dialog.set_directory(&scenarios_path);
                    }
                }
            }

            if let Some(path) = dialog.pick_file() {
                match Scenario::load_from_file(&path) {
                    Ok(scenario) => {
                        // Apply scenario to current configuration
                        app_config.world_config = scenario.to_world_config();
                        app_config.robots = scenario.to_robot_configs();

                        ui_state.scenario_path = Some(path.clone());
                        ui_state.status_message = format!(
                            "Scenario loaded: {}",
                            path.file_name().unwrap().to_string_lossy()
                        );
                        info!("📂 Loaded scenario from {:?}", path);
                    }
                    Err(e) => {
                        ui_state.status_message = format!("Error loading scenario: {}", e);
                        warn!("❌ Failed to load scenario: {}", e);
                    }
                }
            }
            ui_state.show_file_dialog = FileDialogType::None;
        }
        FileDialogType::RecordingLoad => {
            let dialog = rfd::FileDialog::new()
                .add_filter("Recording Files", &["yaml", "yml"])
                .set_title("Load Recording");

            if let Some(path) = dialog.pick_file() {
                match crate::recorder::Recording::load_from_file(&path) {
                    Ok(recording) => {
                        ui_state.recording_path = Some(path.clone());
                        ui_state.status_message = format!(
                            "Recording loaded: {} ({} frames)",
                            path.file_name().unwrap().to_string_lossy(),
                            recording.metadata.frame_count
                        );
                        info!("📂 Loaded recording from {:?}", path);
                        // Note: Playback functionality would need to be implemented separately
                    }
                    Err(e) => {
                        ui_state.status_message = format!("Error loading recording: {}", e);
                        warn!("❌ Failed to load recording: {}", e);
                    }
                }
            }
            ui_state.show_file_dialog = FileDialogType::None;
        }
        FileDialogType::ExportCSV => {
            // Check if there's a recording to export
            if let Some(recording) = recorder.get_recording() {
                let dialog = rfd::FileDialog::new()
                    .add_filter("CSV Files", &["csv"])
                    .set_title("Export Recording to CSV")
                    .set_file_name("recording.csv");

                if let Some(path) = dialog.save_file() {
                    match recording.export_to_csv(&path) {
                        Ok(()) => {
                            ui_state.status_message = format!(
                                "Exported to CSV: {}",
                                path.file_name().unwrap().to_string_lossy()
                            );
                            info!("📊 Exported recording to CSV: {:?}", path);
                        }
                        Err(e) => {
                            ui_state.status_message = format!("Error exporting to CSV: {}", e);
                            warn!("❌ Failed to export to CSV: {}", e);
                        }
                    }
                }
            } else {
                ui_state.status_message = "No recording to export".to_string();
            }
            ui_state.show_file_dialog = FileDialogType::None;
        }
        FileDialogType::ExportVideo => {
            // Check if there's a recording to export
            if let Some(recording) = recorder.get_recording() {
                let dialog = rfd::FileDialog::new()
                    .add_filter("MP4 Video", &["mp4"])
                    .set_title("Export Recording to Video")
                    .set_file_name("recording.mp4");

                if let Some(path) = dialog.save_file() {
                    // Use a temp directory for frame images
                    match std::env::temp_dir().canonicalize() {
                        Ok(temp_dir) => {
                            ui_state.status_message =
                                "Exporting video (this may take a while)...".to_string();
                            match recording.export_to_video(&path, &temp_dir) {
                                Ok(()) => {
                                    ui_state.status_message = format!(
                                        "Exported to video: {}",
                                        path.file_name().unwrap().to_string_lossy()
                                    );
                                    info!("🎥 Exported recording to video: {:?}", path);
                                }
                                Err(e) => {
                                    ui_state.status_message =
                                        format!("Error exporting to video: {}", e);
                                    warn!("❌ Failed to export to video: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            ui_state.status_message =
                                format!("Error accessing temp directory: {}", e);
                        }
                    }
                }
            } else {
                ui_state.status_message = "No recording to export".to_string();
            }
            ui_state.show_file_dialog = FileDialogType::None;
        }
        FileDialogType::None => {}
    }
}
