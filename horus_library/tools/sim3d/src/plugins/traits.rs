//! Plugin trait definitions for sim3d extensibility

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    /// Plugin version (semantic versioning)
    pub version: String,
    /// Plugin author
    pub author: String,
    /// Short description
    pub description: String,
    /// Plugin dependencies (name → version requirement)
    pub dependencies: Vec<PluginDependency>,
    /// Plugin capabilities/tags
    pub tags: Vec<String>,
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub name: String,
    pub version_requirement: String, // e.g., ">=1.0.0", "^2.0"
}

/// Plugin lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin loaded but not initialized
    Loaded,
    /// Plugin initialized and ready
    Initialized,
    /// Plugin running
    Active,
    /// Plugin paused
    Paused,
    /// Plugin stopped
    Stopped,
    /// Plugin encountered an error
    Error,
}

/// Base plugin trait - all plugins must implement this
pub trait Sim3dPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize plugin (called once after load)
    fn initialize(&mut self, app: &mut App) -> Result<(), String>;

    /// Cleanup plugin (called before unload)
    fn cleanup(&mut self, app: &mut App) -> Result<(), String>;

    /// Get current plugin state
    fn state(&self) -> PluginState;

    /// Cast to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Cast to Any (mutable) for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Sensor plugin trait for custom sensors
pub trait SensorPlugin: Sim3dPlugin {
    /// Register sensor components with Bevy ECS
    fn register_sensor_components(&mut self, app: &mut App);

    /// Add sensor update systems to Bevy schedule
    fn add_sensor_systems(&mut self, app: &mut App);

    /// Get sensor update rate (Hz)
    fn update_rate(&self) -> f32 {
        60.0 // Default 60 Hz
    }

    /// Get sensor data format description
    fn data_format(&self) -> String;
}

/// Actuator plugin trait for custom actuators
pub trait ActuatorPlugin: Sim3dPlugin {
    /// Register actuator components with Bevy ECS
    fn register_actuator_components(&mut self, app: &mut App);

    /// Add actuator update systems to Bevy schedule
    fn add_actuator_systems(&mut self, app: &mut App);

    /// Get actuator control rate (Hz)
    fn control_rate(&self) -> f32 {
        240.0 // Default 240 Hz (matches physics)
    }

    /// Get actuator control interface description
    fn control_interface(&self) -> String;
}

/// World plugin trait for custom world features
pub trait WorldPlugin: Sim3dPlugin {
    /// Register world components with Bevy ECS
    fn register_world_components(&mut self, app: &mut App);

    /// Add world update systems to Bevy schedule
    fn add_world_systems(&mut self, app: &mut App);

    /// Get world plugin priority (lower runs first)
    fn priority(&self) -> i32 {
        0
    }
}

/// Physics plugin trait for custom physics integrations
pub trait PhysicsPlugin: Sim3dPlugin {
    /// Initialize physics integration
    fn initialize_physics(&mut self, app: &mut App) -> Result<(), String>;

    /// Step physics (called each physics tick)
    fn step_physics(&mut self, dt: f32);

    /// Get physics tick rate (Hz)
    fn physics_rate(&self) -> f32 {
        240.0
    }
}

/// Rendering plugin trait for custom rendering features
pub trait RenderingPlugin: Sim3dPlugin {
    /// Initialize rendering pipeline
    fn initialize_rendering(&mut self, app: &mut App) -> Result<(), String>;

    /// Add render systems
    fn add_render_systems(&mut self, app: &mut App);

    /// Get rendering priority (lower runs first)
    fn render_priority(&self) -> i32 {
        0
    }
}

/// AI/RL plugin trait for custom RL environments
pub trait RLPlugin: Sim3dPlugin {
    /// Get observation space shape
    fn observation_space(&self) -> Vec<usize>;

    /// Get action space shape
    fn action_space(&self) -> Vec<usize>;

    /// Compute reward for current state
    fn compute_reward(&self, app: &App) -> f32;

    /// Check if episode is done
    fn is_done(&self, app: &App) -> bool;

    /// Reset environment for new episode
    fn reset(&mut self, app: &mut App);
}

/// Plugin configuration (loaded from YAML/TOML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin library path
    pub library_path: String,
    /// Plugin enable/disable flag
    pub enabled: bool,
    /// Plugin-specific configuration (JSON)
    pub config: serde_json::Value,
    /// Load priority (lower loads first)
    pub priority: i32,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            library_path: String::new(),
            enabled: true,
            config: serde_json::Value::Null,
            priority: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata() {
        let metadata = PluginMetadata {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            author: "Test Author".to_string(),
            description: "A test plugin".to_string(),
            dependencies: vec![],
            tags: vec!["sensor".to_string()],
        };

        assert_eq!(metadata.name, "test_plugin");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_plugin_state() {
        assert_eq!(PluginState::Loaded, PluginState::Loaded);
        assert_ne!(PluginState::Loaded, PluginState::Initialized);
    }

    #[test]
    fn test_plugin_config() {
        let config = PluginConfig::default();
        assert!(config.enabled);
        assert_eq!(config.priority, 0);
    }
}
