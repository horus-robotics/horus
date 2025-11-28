//! Plugin registration and lifecycle management

use super::traits::{PluginConfig, PluginMetadata, PluginState, Sim3dPlugin};
use bevy::prelude::*;
use semver::{Version, VersionReq};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Plugin registry - manages all loaded plugins
#[derive(Resource)]
pub struct PluginRegistry {
    /// Loaded plugins (name → plugin)
    plugins: HashMap<String, Arc<RwLock<Box<dyn Sim3dPlugin>>>>,
    /// Plugin metadata cache
    metadata: HashMap<String, PluginMetadata>,
    /// Plugin states
    states: HashMap<String, PluginState>,
    /// Plugin load order
    load_order: Vec<String>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            metadata: HashMap::new(),
            states: HashMap::new(),
            load_order: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn Sim3dPlugin>) -> Result<(), String> {
        let metadata = plugin.metadata().clone();
        let name = metadata.name.clone();

        // Check if plugin already registered
        if self.plugins.contains_key(&name) {
            return Err(format!("Plugin '{}' is already registered", name));
        }

        // Validate dependencies
        self.validate_dependencies(&metadata)?;

        // Register plugin
        self.plugins
            .insert(name.clone(), Arc::new(RwLock::new(plugin)));
        self.metadata.insert(name.clone(), metadata);
        self.states.insert(name.clone(), PluginState::Loaded);
        self.load_order.push(name.clone());

        tracing::info!("Registered plugin: {}", name);
        Ok(())
    }

    /// Initialize a plugin
    pub fn initialize(&mut self, name: &str, app: &mut App) -> Result<(), String> {
        let plugin = self
            .plugins
            .get(name)
            .ok_or_else(|| format!("Plugin '{}' not found", name))?;

        let mut plugin_lock = plugin
            .write()
            .map_err(|e| format!("Failed to acquire plugin lock: {}", e))?;

        // Initialize plugin
        plugin_lock.initialize(app)?;

        // Update state
        self.states
            .insert(name.to_string(), PluginState::Initialized);

        tracing::info!("Initialized plugin: {}", name);
        Ok(())
    }

    /// Initialize all plugins
    pub fn initialize_all(&mut self, app: &mut App) -> Result<(), String> {
        let plugins_to_init: Vec<String> = self.load_order.clone();

        for name in plugins_to_init {
            if let Err(e) = self.initialize(&name, app) {
                tracing::error!("Failed to initialize plugin '{}': {}", name, e);
                self.states.insert(name.clone(), PluginState::Error);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister(&mut self, name: &str, app: &mut App) -> Result<(), String> {
        // First cleanup the plugin
        {
            let plugin = self
                .plugins
                .get(name)
                .ok_or_else(|| format!("Plugin '{}' not found", name))?;

            let mut plugin_lock = plugin
                .write()
                .map_err(|e| format!("Failed to acquire plugin lock: {}", e))?;

            plugin_lock.cleanup(app)?;
        } // Drop the lock here

        // Now remove from registry
        self.plugins.remove(name);
        self.metadata.remove(name);
        self.states.remove(name);
        self.load_order.retain(|n| n != name);

        tracing::info!("Unregistered plugin: {}", name);
        Ok(())
    }

    /// Get plugin by name
    pub fn get(&self, name: &str) -> Option<Arc<RwLock<Box<dyn Sim3dPlugin>>>> {
        self.plugins.get(name).cloned()
    }

    /// Get plugin metadata
    pub fn get_metadata(&self, name: &str) -> Option<&PluginMetadata> {
        self.metadata.get(name)
    }

    /// Get plugin state
    pub fn get_state(&self, name: &str) -> Option<PluginState> {
        self.states.get(name).copied()
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<String> {
        self.load_order.clone()
    }

    /// Get plugin count
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Check if plugin is registered
    pub fn contains(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    /// Validate plugin dependencies
    fn validate_dependencies(&self, metadata: &PluginMetadata) -> Result<(), String> {
        for dep in &metadata.dependencies {
            // Check if dependency exists
            if !self.plugins.contains_key(&dep.name) {
                return Err(format!(
                    "Missing dependency: {} (required by {})",
                    dep.name, metadata.name
                ));
            }

            // Get the cached metadata for the dependency
            let dep_metadata = self.metadata.get(&dep.name).ok_or_else(|| {
                format!(
                    "Metadata not found for dependency: {} (required by {})",
                    dep.name, metadata.name
                )
            })?;

            // Parse the dependency version requirement
            let version_req = VersionReq::parse(&dep.version_requirement).map_err(|e| {
                format!(
                    "Invalid version requirement '{}' for dependency {}: {}",
                    dep.version_requirement, dep.name, e
                )
            })?;

            // Parse the actual plugin version
            let actual_version = Version::parse(&dep_metadata.version).map_err(|e| {
                format!(
                    "Invalid version '{}' for plugin {}: {}",
                    dep_metadata.version, dep.name, e
                )
            })?;

            // Check if the actual version satisfies the requirement
            if !version_req.matches(&actual_version) {
                return Err(format!(
                    "Version mismatch for dependency {}: required {}, found {}",
                    dep.name, dep.version_requirement, dep_metadata.version
                ));
            }
        }

        Ok(())
    }

    /// Get plugin statistics
    pub fn get_stats(&self) -> PluginStats {
        let mut stats = PluginStats {
            total_plugins: self.plugins.len(),
            ..Default::default()
        };

        for state in self.states.values() {
            match state {
                PluginState::Loaded => stats.loaded += 1,
                PluginState::Initialized => stats.initialized += 1,
                PluginState::Active => stats.active += 1,
                PluginState::Paused => stats.paused += 1,
                PluginState::Stopped => stats.stopped += 1,
                PluginState::Error => stats.error += 1,
            }
        }

        stats
    }
}

/// Plugin statistics
#[derive(Debug, Default, Clone)]
pub struct PluginStats {
    pub total_plugins: usize,
    pub loaded: usize,
    pub initialized: usize,
    pub active: usize,
    pub paused: usize,
    pub stopped: usize,
    pub error: usize,
}

impl PluginStats {
    pub fn print(&self) {
        tracing::info!("Plugin Statistics:");
        tracing::info!("  Total: {}", self.total_plugins);
        tracing::info!("  Loaded: {}", self.loaded);
        tracing::info!("  Initialized: {}", self.initialized);
        tracing::info!("  Active: {}", self.active);
        tracing::info!("  Paused: {}", self.paused);
        tracing::info!("  Stopped: {}", self.stopped);
        tracing::info!("  Error: {}", self.error);
    }
}

/// Plugin configuration loader
pub struct PluginConfigLoader;

impl PluginConfigLoader {
    /// Load plugin configs from YAML file
    pub fn load_from_yaml(path: &str) -> Result<Vec<PluginConfig>, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let configs: Vec<PluginConfig> =
            serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))?;

        Ok(configs)
    }

    /// Load plugin configs from TOML file
    pub fn load_from_toml(path: &str) -> Result<Vec<PluginConfig>, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let configs: Vec<PluginConfig> =
            toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))?;

        Ok(configs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::traits::*;
    use std::any::Any;

    // Mock plugin for testing
    struct MockPlugin {
        metadata: PluginMetadata,
        state: PluginState,
    }

    impl Sim3dPlugin for MockPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        fn initialize(&mut self, _app: &mut App) -> Result<(), String> {
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

    fn create_mock_plugin(name: &str) -> Box<dyn Sim3dPlugin> {
        Box::new(MockPlugin {
            metadata: PluginMetadata {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                author: "Test".to_string(),
                description: "Test plugin".to_string(),
                dependencies: vec![],
                tags: vec![],
            },
            state: PluginState::Loaded,
        })
    }

    #[test]
    fn test_registry_register() {
        let mut registry = PluginRegistry::new();
        let plugin = create_mock_plugin("test_plugin");

        assert!(registry.register(plugin).is_ok());
        assert_eq!(registry.count(), 1);
        assert!(registry.contains("test_plugin"));
    }

    #[test]
    fn test_registry_duplicate() {
        let mut registry = PluginRegistry::new();
        let plugin1 = create_mock_plugin("test_plugin");
        let plugin2 = create_mock_plugin("test_plugin");

        assert!(registry.register(plugin1).is_ok());
        assert!(registry.register(plugin2).is_err());
    }

    #[test]
    fn test_registry_list() {
        let mut registry = PluginRegistry::new();
        registry.register(create_mock_plugin("plugin1")).unwrap();
        registry.register(create_mock_plugin("plugin2")).unwrap();

        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 2);
        assert!(plugins.contains(&"plugin1".to_string()));
        assert!(plugins.contains(&"plugin2".to_string()));
    }

    #[test]
    fn test_plugin_stats() {
        let mut registry = PluginRegistry::new();
        registry.register(create_mock_plugin("plugin1")).unwrap();

        let stats = registry.get_stats();
        assert_eq!(stats.total_plugins, 1);
        assert_eq!(stats.loaded, 1);
    }
}
