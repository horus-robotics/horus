//! Simple runtime parameter system for HORUS
//!
//! Provides a straightforward key-value store for runtime configuration

use crate::error::{HorusError, HorusResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Simple runtime parameter store
pub struct RuntimeParams {
    /// Parameter storage - BTreeMap maintains sorted order
    params: Arc<RwLock<BTreeMap<String, Value>>>,
    /// Optional persistence path
    persist_path: Option<PathBuf>,
}

impl RuntimeParams {
    /// Create new parameter store with defaults
    pub fn init() -> HorusResult<Self> {
        let mut initial_params = BTreeMap::new();

        // Try to load from .horus/config/params.yaml in current project
        let params_file = PathBuf::from(".horus/config/params.yaml");
        if params_file.exists() {
            if let Ok(yaml_str) = std::fs::read_to_string(&params_file) {
                if let Ok(loaded) = serde_yaml::from_str::<BTreeMap<String, Value>>(&yaml_str) {
                    initial_params = loaded;
                }
            }
        }

        // If empty, set defaults
        if initial_params.is_empty() {
            // System defaults
            initial_params.insert("tick_rate".to_string(), Value::from(30));
            initial_params.insert("max_memory_mb".to_string(), Value::from(512));

            // Motion defaults
            initial_params.insert("max_speed".to_string(), Value::from(1.0));
            initial_params.insert("max_angular_speed".to_string(), Value::from(1.0));
            initial_params.insert("acceleration_limit".to_string(), Value::from(0.5));

            // Sensor defaults
            initial_params.insert("lidar_rate".to_string(), Value::from(10));
            initial_params.insert("camera_fps".to_string(), Value::from(30));
            initial_params.insert("sensor_timeout_ms".to_string(), Value::from(1000));

            // Safety defaults
            initial_params.insert("emergency_stop_distance".to_string(), Value::from(0.3));
            initial_params.insert("collision_threshold".to_string(), Value::from(0.5));

            // PID defaults
            initial_params.insert("pid_kp".to_string(), Value::from(1.0));
            initial_params.insert("pid_ki".to_string(), Value::from(0.1));
            initial_params.insert("pid_kd".to_string(), Value::from(0.05));
        }

        Ok(Self {
            params: Arc::new(RwLock::new(initial_params)),
            persist_path: Some(params_file),
        })
    }

    /// Set default parameters
    fn set_defaults(&self) -> Result<(), HorusError> {
        // System defaults
        self.set("tick_rate", 30)?;
        self.set("max_memory_mb", 512)?;

        // Motion defaults
        self.set("max_speed", 1.0)?;
        self.set("max_angular_speed", 1.0)?;
        self.set("acceleration_limit", 0.5)?;

        // Sensor defaults
        self.set("lidar_rate", 10)?;
        self.set("camera_fps", 30)?;
        self.set("sensor_timeout_ms", 1000)?;

        // Safety defaults
        self.set("emergency_stop_distance", 0.3)?;
        self.set("collision_threshold", 0.5)?;

        // PID defaults
        self.set("pid_kp", 1.0)?;
        self.set("pid_ki", 0.1)?;
        self.set("pid_kd", 0.05)?;

        Ok(())
    }

    /// Get a parameter value
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        let params = self.params.read().ok()?;
        let value = params.get(key)?;
        serde_json::from_value(value.clone()).ok()
    }

    /// Get parameter with default
    pub fn get_or<T: for<'de> Deserialize<'de>>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    /// Get parameter as f64 with default
    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.get_or(key, default)
    }

    /// Get parameter as i32 with default
    pub fn get_i32(&self, key: &str, default: i32) -> i32 {
        self.get_or(key, default)
    }

    /// Get parameter as bool with default
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.get_or(key, default)
    }

    /// Get parameter as string with default
    pub fn get_string(&self, key: &str, default: &str) -> String {
        self.get_or(key, default.to_string())
    }

    /// Set a parameter value
    pub fn set<T: Serialize>(
        &self,
        key: &str,
        value: T,
    ) -> Result<(), HorusError> {
        let json_value = serde_json::to_value(value)?;
        let mut params = self.params.write()?;
        params.insert(key.to_string(), json_value);
        Ok(())
    }

    /// Get all parameters
    pub fn get_all(&self) -> BTreeMap<String, Value> {
        self.params.read()
            .map(|p| p.clone())
            .unwrap_or_default()
    }

    /// List all parameter keys
    pub fn list_keys(&self) -> Vec<String> {
        self.params.read()
            .map(|p| p.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Check if a parameter exists
    pub fn has(&self, key: &str) -> bool {
        self.params.read()
            .map(|p| p.contains_key(key))
            .unwrap_or(false)
    }

    /// Remove a parameter
    pub fn remove(&self, key: &str) -> Option<Value> {
        self.params.write().ok()?.remove(key)
    }

    /// Clear all parameters and reset to defaults
    pub fn reset(&self) -> Result<(), HorusError> {
        let mut params = self.params.write()?;
        params.clear();
        drop(params);
        self.set_defaults()?;
        Ok(())
    }

    /// Save parameters to YAML file
    pub fn save_to_disk(&self) -> Result<(), HorusError> {
        let path = self
            .persist_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(".horus/config/params.yaml"));

        // Create .horus directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let params = self.params.read()?;
        let yaml = serde_yaml::to_string(&*params)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// Load parameters from YAML file
    pub fn load_from_disk(&self, path: &Path) -> Result<(), HorusError> {
        if path.exists() {
            let yaml_str = std::fs::read_to_string(path)?;
            let loaded: BTreeMap<String, Value> = serde_yaml::from_str(&yaml_str)?;

            let mut params = self.params.write()?;
            *params = loaded;
        }
        Ok(())
    }
}

impl Clone for RuntimeParams {
    fn clone(&self) -> Self {
        Self {
            params: self.params.clone(),
            persist_path: self.persist_path.clone(),
        }
    }
}

impl Default for RuntimeParams {
    fn default() -> Self {
        Self::init().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize RuntimeParams: {}. Using empty params.", e);
            Self {
                params: Arc::new(RwLock::new(BTreeMap::new())),
                persist_path: None,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let params = RuntimeParams::init().unwrap();

        // Test defaults
        assert_eq!(params.get_f64("max_speed", 0.0), 1.0);
        assert_eq!(params.get_i32("tick_rate", 0), 30);

        // Test set/get
        params.set("test_value", 42.5).unwrap();
        assert_eq!(params.get::<f64>("test_value"), Some(42.5));

        // Test overwrite
        params.set("max_speed", 2.0).unwrap();
        assert_eq!(params.get_f64("max_speed", 0.0), 2.0);
    }
}
