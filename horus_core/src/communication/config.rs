/// Configuration file support for HORUS
///
/// Allows Hub creation from TOML/YAML config files instead of hardcoded strings.
/// Supports auto-detection of file format and multiple search paths.
use crate::error::{HorusError, HorusResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// HORUS Hub configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubConfig {
    /// Hub name/topic
    pub name: String,

    /// Endpoint string (e.g., "camera@router", "sensor@192.168.1.5:9000")
    #[serde(default)]
    pub endpoint: Option<String>,

    /// Transport type (overrides endpoint parsing)
    #[serde(default)]
    pub transport: Option<String>,

    /// Host address (for direct/router transports)
    #[serde(default)]
    pub host: Option<String>,

    /// Port number
    #[serde(default)]
    pub port: Option<u16>,

    /// Enable TLS for this hub (router transport only)
    #[serde(default)]
    pub tls: Option<bool>,

    /// Path to TLS certificate file (PEM format)
    #[serde(default)]
    pub tls_cert: Option<String>,

    /// Path to TLS private key file (PEM format)
    #[serde(default)]
    pub tls_key: Option<String>,

    /// Additional options
    #[serde(flatten)]
    pub options: std::collections::HashMap<String, serde_yaml::Value>,
}

/// Full configuration file with multiple hub definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorusConfig {
    /// Map of hub name -> hub config
    pub hubs: std::collections::HashMap<String, HubConfig>,
}

impl HubConfig {
    /// Get the endpoint string for this hub
    pub fn get_endpoint(&self) -> String {
        // If explicit endpoint is provided, use it
        if let Some(ref ep) = self.endpoint {
            return ep.clone();
        }

        // Build endpoint from components
        match self.transport.as_deref() {
            Some("local") | None => {
                // Local shared memory
                self.name.clone()
            }
            Some("localhost") => {
                // Unix socket
                if let Some(port) = self.port {
                    format!("{}@localhost:{}", self.name, port)
                } else {
                    format!("{}@localhost", self.name)
                }
            }
            Some("router") => {
                // Router
                if let Some(ref host) = self.host {
                    if let Some(port) = self.port {
                        format!("{}@{}:{}", self.name, host, port)
                    } else {
                        format!("{}@{}", self.name, host)
                    }
                } else if let Some(port) = self.port {
                    format!("{}@router:{}", self.name, port)
                } else {
                    format!("{}@router", self.name)
                }
            }
            Some("multicast") => {
                // Multicast discovery
                format!("{}@*", self.name)
            }
            Some("direct") => {
                // Direct UDP
                if let Some(ref host) = self.host {
                    if let Some(port) = self.port {
                        format!("{}@{}:{}", self.name, host, port)
                    } else {
                        format!("{}@{}", self.name, host)
                    }
                } else {
                    format!("{}@router", self.name) // Fallback
                }
            }
            Some(unknown) => {
                // Unknown transport, use as-is
                format!("{}@{}", self.name, unknown)
            }
        }
    }
}

impl HorusConfig {
    /// Load config from a file (auto-detect format)
    pub fn from_file<P: AsRef<Path>>(path: P) -> HorusResult<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)
            .map_err(|e| HorusError::config(format!("Failed to read config file: {}", e)))?;

        // Auto-detect format based on extension
        let extension = path.extension().and_then(|s| s.to_str());
        match extension {
            Some("toml") => Self::from_toml(&contents),
            Some("yaml") | Some("yml") => Self::from_yaml(&contents),
            _ => {
                // Try both formats
                Self::from_toml(&contents).or_else(|_| Self::from_yaml(&contents))
            }
        }
    }

    /// Parse config from TOML string
    pub fn from_toml(contents: &str) -> HorusResult<Self> {
        toml::from_str(contents)
            .map_err(|e| HorusError::config(format!("Failed to parse TOML: {}", e)))
    }

    /// Parse config from YAML string
    pub fn from_yaml(contents: &str) -> HorusResult<Self> {
        serde_yaml::from_str(contents)
            .map_err(|e| HorusError::config(format!("Failed to parse YAML: {}", e)))
    }

    /// Find and load config file from standard search paths
    ///
    /// Search order:
    /// 1. ./horus.toml or ./horus.yaml
    /// 2. ~/.horus/config.toml or ~/.horus/config.yaml
    /// 3. /etc/horus/config.toml or /etc/horus/config.yaml
    pub fn find_and_load() -> HorusResult<Self> {
        let search_paths = Self::get_search_paths();

        for path in search_paths {
            if path.exists() {
                return Self::from_file(&path);
            }
        }

        Err(HorusError::config(
            "No config file found in standard locations",
        ))
    }

    /// Get standard config file search paths
    pub fn get_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Current directory
        paths.push(PathBuf::from("horus.toml"));
        paths.push(PathBuf::from("horus.yaml"));
        paths.push(PathBuf::from("horus.yml"));

        // User config directory (~/.horus/)
        if let Some(home) = dirs::home_dir() {
            let horus_dir = home.join(".horus");
            paths.push(horus_dir.join("config.toml"));
            paths.push(horus_dir.join("config.yaml"));
            paths.push(horus_dir.join("config.yml"));
        }

        // System config directory (/etc/horus/)
        paths.push(PathBuf::from("/etc/horus/config.toml"));
        paths.push(PathBuf::from("/etc/horus/config.yaml"));

        paths
    }

    /// Get a hub config by name
    pub fn get_hub(&self, name: &str) -> HorusResult<&HubConfig> {
        self.hubs
            .get(name)
            .ok_or_else(|| HorusError::config(format!("Hub '{}' not found in config", name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
            [hubs.camera]
            name = "camera"
            endpoint = "camera@router"

            [hubs.sensor]
            name = "sensor"
            transport = "direct"
            host = "192.168.1.5"
            port = 9000
        "#;

        let config = HorusConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.hubs.len(), 2);

        let camera = config.get_hub("camera").unwrap();
        assert_eq!(camera.get_endpoint(), "camera@router");

        let sensor = config.get_hub("sensor").unwrap();
        assert_eq!(sensor.get_endpoint(), "sensor@192.168.1.5:9000");
    }

    #[test]
    fn test_parse_yaml() {
        let yaml_str = r#"
            hubs:
              camera:
                name: camera
                endpoint: camera@router
              sensor:
                name: sensor
                transport: direct
                host: 192.168.1.5
                port: 9000
        "#;

        let config = HorusConfig::from_yaml(yaml_str).unwrap();
        assert_eq!(config.hubs.len(), 2);

        let camera = config.get_hub("camera").unwrap();
        assert_eq!(camera.get_endpoint(), "camera@router");

        let sensor = config.get_hub("sensor").unwrap();
        assert_eq!(sensor.get_endpoint(), "sensor@192.168.1.5:9000");
    }

    #[test]
    fn test_local_transport() {
        let config = HubConfig {
            name: "local_topic".to_string(),
            endpoint: None,
            transport: Some("local".to_string()),
            host: None,
            port: None,
            tls: None,
            tls_cert: None,
            tls_key: None,
            options: std::collections::HashMap::new(),
        };

        assert_eq!(config.get_endpoint(), "local_topic");
    }

    #[test]
    fn test_router_transport() {
        let config = HubConfig {
            name: "my_topic".to_string(),
            endpoint: None,
            transport: Some("router".to_string()),
            host: None,
            port: Some(8888),
            tls: None,
            tls_cert: None,
            tls_key: None,
            options: std::collections::HashMap::new(),
        };

        assert_eq!(config.get_endpoint(), "my_topic@router:8888");
    }

    #[test]
    fn test_multicast_transport() {
        let config = HubConfig {
            name: "broadcast".to_string(),
            endpoint: None,
            transport: Some("multicast".to_string()),
            host: None,
            port: None,
            tls: None,
            tls_cert: None,
            tls_key: None,
            options: std::collections::HashMap::new(),
        };

        assert_eq!(config.get_endpoint(), "broadcast@*");
    }
}
