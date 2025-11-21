//! Example actuator plugin demonstrating the plugin API

use crate::plugins::traits::*;
use bevy::prelude::*;
use std::any::Any;

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
