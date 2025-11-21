//! Example sensor plugin demonstrating the plugin API

use crate::plugins::traits::*;
use bevy::prelude::*;
use std::any::Any;

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
