//! Example world plugin demonstrating the plugin API

use crate::plugins::traits::*;
use bevy::prelude::*;
use std::any::Any;

/// Example wind system component
#[derive(Resource)]
pub struct WindSystem {
    pub wind_velocity: Vec3,
    pub turbulence: f32,
    pub enabled: bool,
}

impl Default for WindSystem {
    fn default() -> Self {
        Self {
            wind_velocity: Vec3::new(1.0, 0.0, 0.0),
            turbulence: 0.1,
            enabled: true,
        }
    }
}

/// Example world plugin
pub struct ExampleWorldPlugin {
    metadata: PluginMetadata,
    state: PluginState,
}

impl Default for ExampleWorldPlugin {
    fn default() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "wind_system".to_string(),
                version: "1.0.0".to_string(),
                author: "HORUS Team".to_string(),
                description: "Example wind system world plugin".to_string(),
                dependencies: vec![],
                tags: vec![
                    "world".to_string(),
                    "example".to_string(),
                    "environment".to_string(),
                ],
            },
            state: PluginState::Loaded,
        }
    }
}

impl Sim3dPlugin for ExampleWorldPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self, app: &mut App) -> Result<(), String> {
        self.register_world_components(app);
        self.add_world_systems(app);
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

impl WorldPlugin for ExampleWorldPlugin {
    fn register_world_components(&mut self, app: &mut App) {
        app.insert_resource(WindSystem::default());
    }

    fn add_world_systems(&mut self, app: &mut App) {
        app.add_systems(Update, wind_system_update);
    }

    fn priority(&self) -> i32 {
        10 // Run after most other systems
    }
}

/// Marker component for objects affected by wind
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct WindAffected {
    pub drag_coefficient: f32,
}

impl Default for WindAffected {
    fn default() -> Self {
        Self {
            drag_coefficient: 0.5,
        }
    }
}

/// System to apply wind forces to affected objects
fn wind_system_update(
    wind: Res<WindSystem>,
    time: Res<Time>,
    query: Query<(&WindAffected, &GlobalTransform)>,
) {
    if !wind.enabled {
        return;
    }

    let _dt = time.delta_secs();

    for (_wind_affected, _transform) in query.iter() {
        // In a real implementation, this would apply wind forces to physics bodies
        // For this example, we just validate that the system runs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wind_system_creation() {
        let wind = WindSystem::default();
        assert!(wind.enabled);
        assert_eq!(wind.turbulence, 0.1);
    }

    #[test]
    fn test_example_world_plugin_metadata() {
        let plugin = ExampleWorldPlugin::default();
        let metadata = plugin.metadata();
        assert_eq!(metadata.name, "wind_system");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_world_plugin_priority() {
        let plugin = ExampleWorldPlugin::default();
        assert_eq!(plugin.priority(), 10);
    }

    #[test]
    fn test_wind_affected_creation() {
        let affected = WindAffected::default();
        assert_eq!(affected.drag_coefficient, 0.5);
    }
}
