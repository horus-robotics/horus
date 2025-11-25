//! Plugin trait definitions for sim3d extensibility

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;

#[cfg(feature = "visual")]
use bevy_egui::egui;

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

/// UI plugin trait for plugins that provide inspector/settings UI
///
/// This trait enables plugins to contribute UI elements to the unified
/// inspector panel, following patterns from Gazebo and Isaac Sim.
///
/// # Example
/// ```ignore
/// impl UiPlugin for MySensorPlugin {
///     fn has_inspector_ui(&self) -> bool { true }
///
///     fn inspector_ui(&self, ui: &mut egui::Ui, world: &World, entity: Entity) {
///         if let Some(sensor) = world.get::<MySensor>(entity) {
///             ui.label(format!("Reading: {:.2}", sensor.value));
///         }
///     }
///
///     fn settings_ui(&mut self, ui: &mut egui::Ui) {
///         ui.checkbox(&mut self.enabled, "Enable sensor");
///     }
/// }
/// ```
#[cfg(feature = "visual")]
pub trait UiPlugin: Sim3dPlugin {
    /// Whether this plugin provides inspector UI for entities
    fn has_inspector_ui(&self) -> bool {
        false
    }

    /// Whether this plugin provides global settings UI
    fn has_settings_ui(&self) -> bool {
        false
    }

    /// Display inspector UI for a selected entity
    /// Called when an entity with this plugin's components is selected
    fn inspector_ui(&self, _ui: &mut egui::Ui, _world: &World, _entity: Entity) {}

    /// Display global plugin settings UI
    /// Shown in the unified settings panel under a collapsible section
    fn settings_ui(&mut self, _ui: &mut egui::Ui) {}

    /// Get the display name for the settings section
    fn settings_section_name(&self) -> &str {
        self.metadata().name.as_str()
    }

    /// Get icon for the settings section (optional, Unicode or emoji)
    fn settings_icon(&self) -> Option<&str> {
        None
    }

    /// Priority for ordering in the settings panel (lower = higher priority)
    fn settings_priority(&self) -> i32 {
        100
    }
}

/// UI panel registration info for the unified panel system
#[cfg(feature = "visual")]
#[derive(Debug, Clone)]
pub struct PluginPanelInfo {
    /// Plugin name
    pub plugin_name: String,
    /// Display name for the panel section
    pub display_name: String,
    /// Icon (optional)
    pub icon: Option<String>,
    /// Priority for ordering (lower = higher)
    pub priority: i32,
    /// Whether plugin provides inspector UI
    pub has_inspector: bool,
    /// Whether plugin provides settings UI
    pub has_settings: bool,
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
