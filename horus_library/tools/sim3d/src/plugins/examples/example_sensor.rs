//! Example sensor plugin demonstrating the plugin API

use crate::plugins::traits::*;
use bevy::prelude::*;
use std::any::Any;

#[cfg(feature = "visual")]
use bevy_egui::egui;

/// Example proximity sensor component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ProximitySensor {
    pub range: f32,
    pub detected: bool,
    pub distance: f32,
}

impl Default for ProximitySensor {
    fn default() -> Self {
        Self {
            range: 5.0,
            detected: false,
            distance: f32::MAX,
        }
    }
}

/// Example sensor plugin
pub struct ExampleSensorPlugin {
    metadata: PluginMetadata,
    state: PluginState,
}

impl Default for ExampleSensorPlugin {
    fn default() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "proximity_sensor".to_string(),
                version: "1.0.0".to_string(),
                author: "HORUS Team".to_string(),
                description: "Example proximity sensor plugin".to_string(),
                dependencies: vec![],
                tags: vec!["sensor".to_string(), "example".to_string()],
            },
            state: PluginState::Loaded,
        }
    }
}

impl Sim3dPlugin for ExampleSensorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self, app: &mut App) -> Result<(), String> {
        self.register_sensor_components(app);
        self.add_sensor_systems(app);
        self.state = PluginState::Initialized;
        Ok(())
    }

    fn cleanup(&mut self, _app: &mut App) -> Result<(), String> {
        self.state = PluginState::Stopped;
        Ok(())
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SensorPlugin for ExampleSensorPlugin {
    fn register_sensor_components(&mut self, app: &mut App) {
        app.register_type::<ProximitySensor>();
    }

    fn add_sensor_systems(&mut self, app: &mut App) {
        app.add_systems(Update, proximity_sensor_update_system);
    }

    fn update_rate(&self) -> f32 {
        30.0 // 30 Hz
    }

    fn data_format(&self) -> String {
        "detected: bool, distance: f32".to_string()
    }
}

#[cfg(feature = "visual")]
impl UiPlugin for ExampleSensorPlugin {
    fn has_inspector_ui(&self) -> bool {
        true
    }

    fn has_settings_ui(&self) -> bool {
        true
    }

    fn inspector_ui(&self, ui: &mut egui::Ui, world: &World, entity: Entity) {
        // Check if entity has a ProximitySensor component
        if let Some(sensor) = world.get::<ProximitySensor>(entity) {
            egui::CollapsingHeader::new("Proximity Sensor")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Range:");
                        ui.label(format!("{:.2} m", sensor.range));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Status:");
                        if sensor.detected {
                            ui.colored_label(egui::Color32::GREEN, "DETECTED");
                        } else {
                            ui.colored_label(egui::Color32::GRAY, "Clear");
                        }
                    });

                    if sensor.detected {
                        ui.horizontal(|ui| {
                            ui.label("Distance:");
                            ui.label(format!("{:.2} m", sensor.distance));
                        });

                        // Visual indicator bar
                        let progress = 1.0 - (sensor.distance / sensor.range).clamp(0.0, 1.0);
                        ui.add(egui::ProgressBar::new(progress).text("Proximity"));
                    }
                });
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Proximity Sensor Settings");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Update Rate:");
            ui.label(format!("{} Hz", self.update_rate()));
        });

        ui.horizontal(|ui| {
            ui.label("Data Format:");
            ui.label(self.data_format());
        });

        ui.add_space(8.0);
        ui.label("Plugin Info:");
        ui.label(format!("Version: {}", self.metadata.version));
        ui.label(format!("Author: {}", self.metadata.author));
    }

    fn settings_section_name(&self) -> &str {
        "Proximity Sensor"
    }

    fn settings_icon(&self) -> Option<&str> {
        Some("S") // Simple icon for sensor
    }

    fn settings_priority(&self) -> i32 {
        10 // Higher priority (lower number = higher)
    }
}

/// System to update proximity sensors
fn proximity_sensor_update_system(
    mut query: Query<(&mut ProximitySensor, &GlobalTransform)>,
    obstacles: Query<&GlobalTransform, Without<ProximitySensor>>,
) {
    for (mut sensor, sensor_transform) in query.iter_mut() {
        let sensor_pos = sensor_transform.translation();

        let mut min_distance = f32::MAX;
        let mut detected = false;

        // Check distance to all obstacles
        for obstacle_transform in obstacles.iter() {
            let obstacle_pos = obstacle_transform.translation();
            let distance = sensor_pos.distance(obstacle_pos);

            if distance < sensor.range {
                detected = true;
                min_distance = min_distance.min(distance);
            }
        }

        sensor.detected = detected;
        sensor.distance = if detected { min_distance } else { f32::MAX };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proximity_sensor_creation() {
        let sensor = ProximitySensor::default();
        assert_eq!(sensor.range, 5.0);
        assert!(!sensor.detected);
    }

    #[test]
    fn test_example_sensor_plugin_metadata() {
        let plugin = ExampleSensorPlugin::default();
        let metadata = plugin.metadata();
        assert_eq!(metadata.name, "proximity_sensor");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_sensor_plugin_state() {
        let plugin = ExampleSensorPlugin::default();
        assert_eq!(plugin.state(), PluginState::Loaded);
    }
}
