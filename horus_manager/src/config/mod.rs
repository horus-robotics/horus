//! Configuration management for HORUS packages
//!
//! Package configuration from Cargo.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod cargo_config;

pub use cargo_config::{CargoConfig, HorusMetadata};

/// Unified package configuration combining all sources
#[derive(Debug, Clone)]
pub struct PackageConfig {
    /// Package name (from Cargo.toml)
    pub name: String,
    /// Package version (from Cargo.toml)
    pub version: String,
    /// Build configuration (from Cargo.toml)
    pub cargo: CargoConfig,
}

impl PackageConfig {
    /// Load configuration from a package directory
    pub fn load(package_dir: &Path) -> Result<Self> {
        // 1. Load Cargo.toml (required)
        let cargo_path = package_dir.join("Cargo.toml");
        let cargo = CargoConfig::load(&cargo_path)
            .with_context(|| format!("Failed to load Cargo.toml from {:?}", cargo_path))?;

        let name = cargo.package.name.clone();
        let version = cargo.package.version.clone();

        Ok(Self {
            name,
            version,
            cargo,
        })
    }

    /// Check if this package is publishable to registry
    pub fn is_publishable(&self) -> bool {
        // Must have proper metadata for publishing
        self.cargo
            .metadata
            .as_ref()
            .and_then(|m| m.horus.as_ref())
            .map(|h| h.publishable)
            .unwrap_or(false)
    }

    /// Get registry metadata for publishing
    pub fn registry_metadata(&self) -> Option<RegistryMetadata> {
        self.cargo
            .metadata
            .as_ref()
            .and_then(|m| m.horus.as_ref())
            .map(|h| RegistryMetadata {
                name: self.name.clone(),
                version: self.version.clone(),
                description: self.cargo.package.description.clone(),
                authors: self.cargo.package.authors.clone().unwrap_or_default(),
                license: self.cargo.package.license.clone(),
                repository: self.cargo.package.repository.clone(),
                tags: h.tags.clone().unwrap_or_default(),
                capabilities: h.capabilities.clone().unwrap_or_default(),
                hardware: h.hardware.clone().unwrap_or_default(),
                min_horus_version: h.min_horus_version.clone(),
            })
    }
}

/// Metadata for package registry/marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub hardware: Vec<String>,
    pub min_horus_version: Option<String>,
}

/// Find the package root directory from current location
pub fn find_package_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        if current.join("Cargo.toml").exists() {
            return Ok(current);
        }

        if !current.pop() {
            return Err(anyhow::anyhow!(
                "No Cargo.toml found in current or parent directories"
            ));
        }
    }
}

/// Configuration precedence for runtime values
/// 1. Environment variables (highest)
/// 2. Cargo.toml metadata
/// 3. Built-in defaults (lowest)
pub fn resolve_runtime_value<T>(env_var: Option<T>, cargo_metadata: Option<T>, default: T) -> T {
    env_var.or(cargo_metadata).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_precedence() {
        // Test environment > cargo > default
        assert_eq!(resolve_runtime_value(Some(1), Some(2), 3), 1);
        assert_eq!(resolve_runtime_value(None, Some(2), 3), 2);
        assert_eq!(resolve_runtime_value::<i32>(None, None, 3), 3);
    }
}
