//! Plugin system for sim3d extensibility
//!
//! This module provides a flexible plugin architecture for extending sim3d with:
//! - Custom sensors
//! - Custom actuators
//! - Custom world features
//! - Custom physics integrations
//! - Custom rendering pipelines
//! - Custom RL environments
//!
//! # Architecture
//!
//! The plugin system is built on three core components:
//! 1. **Plugin Traits** - Define the interface for different plugin types
//! 2. **Plugin Registry** - Manages plugin lifecycle (load, init, cleanup)
//! 3. **Plugin Loader** - Handles dynamic library loading
//!
//! # Example
//!
//! ```ignore
//! use sim3d::plugins::*;
//!
//! // Create a plugin registry
//! let mut registry = PluginRegistry::new();
//!
//! // Register a plugin
//! let plugin = Box::new(ExampleSensorPlugin::default());
//! registry.register(plugin)?;
//!
//! // Initialize all plugins
//! registry.initialize_all(&mut app)?;
//! ```
//!
//! # Dynamic Loading
//!
//! Plugins can be loaded from dynamic libraries:
//!
//! ```ignore
//! use sim3d::plugins::*;
//!
//! let mut loader = PluginLoader::new();
//! let plugin = unsafe { loader.load_plugin(Path::new("libmyplugin.so"))? };
//!
//! registry.register(plugin)?;
//! ```
//!
//! # Creating Plugins
//!
//! To create a plugin, implement the appropriate trait(s):
//!
//! ```ignore
//! use sim3d::plugins::*;
//!
//! pub struct MyPlugin {
//!     metadata: PluginMetadata,
//!     state: PluginState,
//! }
//!
//! impl Plugin for MyPlugin {
//!     fn metadata(&self) -> &PluginMetadata { &self.metadata }
//!     fn initialize(&mut self, app: &mut App) -> Result<(), String> { Ok(()) }
//!     fn cleanup(&mut self, app: &mut App) -> Result<(), String> { Ok(()) }
//!     fn state(&self) -> PluginState { self.state }
//!     // ...
//! }
//!
//! impl SensorPlugin for MyPlugin {
//!     // Implement sensor-specific methods
//! }
//! ```

pub mod examples;
pub mod loader;
pub mod registry;
pub mod traits;

// Re-export main types
pub use traits::{
    ActuatorPlugin, PhysicsPlugin, PluginConfig, PluginDependency, PluginMetadata, PluginState,
    RLPlugin, RenderingPlugin, SensorPlugin, Sim3dPlugin, WorldPlugin,
};

#[cfg(feature = "visual")]
pub use traits::{PluginPanelInfo, UiPlugin};

pub use registry::{PluginConfigLoader, PluginRegistry, PluginStats};

pub use loader::PluginLoader;

pub use examples::{
    ExampleActuatorPlugin, ExampleSensorPlugin, ExampleWorldPlugin, ProximitySensor,
    ThrusterActuator, WindAffected, WindSystem,
};

use bevy::prelude::*;

/// Plugin system Bevy plugin
pub struct PluginSystemPlugin;

impl bevy::app::Plugin for PluginSystemPlugin {
    fn build(&self, app: &mut App) {
        // Initialize the plugin registry
        let mut registry = PluginRegistry::new();

        // Register built-in example plugins
        let sensor_plugin = Box::new(ExampleSensorPlugin::default());
        let actuator_plugin = Box::new(ExampleActuatorPlugin::default());
        let world_plugin = Box::new(ExampleWorldPlugin::default());

        if let Err(e) = registry.register(sensor_plugin) {
            tracing::warn!("Failed to register sensor plugin: {}", e);
        }
        if let Err(e) = registry.register(actuator_plugin) {
            tracing::warn!("Failed to register actuator plugin: {}", e);
        }
        if let Err(e) = registry.register(world_plugin) {
            tracing::warn!("Failed to register world plugin: {}", e);
        }

        // Initialize all registered plugins
        if let Err(e) = registry.initialize_all(app) {
            tracing::error!("Failed to initialize plugins: {}", e);
        }

        // Insert registry as resource
        app.insert_resource(registry);

        tracing::info!("Plugin system initialized with {} plugins", 3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_system_plugin() {
        let mut app = App::new();
        app.add_plugins(PluginSystemPlugin);

        // Verify registry resource exists
        assert!(app.world().get_resource::<PluginRegistry>().is_some());
    }

    #[test]
    fn test_example_plugins_integration() {
        let mut app = App::new();
        let mut registry = PluginRegistry::new();

        // Register all example plugins
        let sensor_plugin = Box::new(ExampleSensorPlugin::default());
        let actuator_plugin = Box::new(ExampleActuatorPlugin::default());
        let world_plugin = Box::new(ExampleWorldPlugin::default());

        assert!(registry.register(sensor_plugin).is_ok());
        assert!(registry.register(actuator_plugin).is_ok());
        assert!(registry.register(world_plugin).is_ok());

        assert_eq!(registry.count(), 3);

        // Initialize all plugins
        assert!(registry.initialize_all(&mut app).is_ok());

        let stats = registry.get_stats();
        assert_eq!(stats.total_plugins, 3);
        assert_eq!(stats.initialized, 3);
    }
}
