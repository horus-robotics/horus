//! Example actuator plugin demonstrating the plugin API

use crate::plugins::traits::*;
use bevy::prelude::*;
use std::any::Any;

#[cfg(feature = "visual")]
use bevy_egui::egui;

/// Example thruster actuator component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ThrusterActuator {
    pub thrust: f32,
    pub max_thrust: f32,
    pub direction: Vec3,
}

impl Default for ThrusterActuator {
    fn default() -> Self {
        Self {
            thrust: 0.0,
            max_thrust: 100.0,
            direction: Vec3::Y,
        }
    }
}

/// Example actuator plugin
pub struct ExampleActuatorPlugin {
    metadata: PluginMetadata,
    state: PluginState,
}

impl Default for ExampleActuatorPlugin {
    fn default() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "thruster_actuator".to_string(),
                version: "1.0.0".to_string(),
                author: "HORUS Team".to_string(),
                description: "Example thruster actuator plugin".to_string(),
                dependencies: vec![],
                tags: vec!["actuator".to_string(), "example".to_string()],
            },
            state: PluginState::Loaded,
        }
    }
}

impl Sim3dPlugin for ExampleActuatorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self, app: &mut App) -> Result<(), String> {
        self.register_actuator_components(app);
        self.add_actuator_systems(app);
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

impl ActuatorPlugin for ExampleActuatorPlugin {
    fn register_actuator_components(&mut self, app: &mut App) {
        app.register_type::<ThrusterActuator>();
    }

    fn add_actuator_systems(&mut self, app: &mut App) {
        app.add_systems(FixedUpdate, thruster_actuator_system);
    }

    fn control_rate(&self) -> f32 {
        240.0 // 240 Hz (matches physics)
    }

    fn control_interface(&self) -> String {
        "thrust: f32 (0.0-max_thrust), direction: Vec3".to_string()
    }
}

#[cfg(feature = "visual")]
impl UiPlugin for ExampleActuatorPlugin {
    fn has_inspector_ui(&self) -> bool {
        true
    }

    fn has_settings_ui(&self) -> bool {
        true
    }

    fn inspector_ui(&self, ui: &mut egui::Ui, world: &World, entity: Entity) {
        // Check if entity has a ThrusterActuator component
        if let Some(thruster) = world.get::<ThrusterActuator>(entity) {
            egui::CollapsingHeader::new("Thruster Actuator")
                .default_open(true)
                .show(ui, |ui| {
                    // Thrust control
                    ui.horizontal(|ui| {
                        ui.label("Thrust:");
                        ui.label(format!("{:.1} / {:.1} N", thruster.thrust, thruster.max_thrust));
                    });

                    // Thrust percentage bar
                    let percentage = thruster.thrust / thruster.max_thrust;
                    ui.add(
                        egui::ProgressBar::new(percentage)
                            .text(format!("{:.0}%", percentage * 100.0)),
                    );

                    // Direction
                    ui.horizontal(|ui| {
                        ui.label("Direction:");
                        ui.label(format!(
                            "[{:.2}, {:.2}, {:.2}]",
                            thruster.direction.x, thruster.direction.y, thruster.direction.z
                        ));
                    });

                    // Status
                    ui.horizontal(|ui| {
                        ui.label("Status:");
                        if thruster.thrust > 0.0 {
                            ui.colored_label(egui::Color32::LIGHT_BLUE, "ACTIVE");
                        } else {
                            ui.colored_label(egui::Color32::GRAY, "Idle");
                        }
                    });
                });
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Thruster Actuator Settings");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Control Rate:");
            ui.label(format!("{} Hz", self.control_rate()));
        });

        ui.horizontal(|ui| {
            ui.label("Interface:");
        });
        ui.label(self.control_interface());

        ui.add_space(8.0);
        ui.label("Plugin Info:");
        ui.label(format!("Version: {}", self.metadata.version));
        ui.label(format!("Author: {}", self.metadata.author));
    }

    fn settings_section_name(&self) -> &str {
        "Thruster Actuator"
    }

    fn settings_icon(&self) -> Option<&str> {
        Some("A") // Simple icon for actuator
    }

    fn settings_priority(&self) -> i32 {
        20 // After sensors
    }
}

/// System to apply thruster forces
fn thruster_actuator_system(mut query: Query<(&mut ThrusterActuator, &GlobalTransform)>) {
    for (mut thruster, _transform) in query.iter_mut() {
        // Clamp thrust to valid range
        thruster.thrust = thruster.thrust.clamp(0.0, thruster.max_thrust);

        // In a real implementation, this would apply forces to the physics body
        // For this example, we just validate the thrust value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thruster_actuator_creation() {
        let thruster = ThrusterActuator::default();
        assert_eq!(thruster.thrust, 0.0);
        assert_eq!(thruster.max_thrust, 100.0);
    }

    #[test]
    fn test_example_actuator_plugin_metadata() {
        let plugin = ExampleActuatorPlugin::default();
        let metadata = plugin.metadata();
        assert_eq!(metadata.name, "thruster_actuator");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_actuator_plugin_control_rate() {
        let plugin = ExampleActuatorPlugin::default();
        assert_eq!(plugin.control_rate(), 240.0);
    }
}
