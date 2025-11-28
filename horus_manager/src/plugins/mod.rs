//! HORUS Plugin System
//!
//! This module provides CLI plugin management for HORUS.
//! Plugins are packages that extend the `horus` CLI with new subcommands.
//!
//! ## Architecture
//!
//! - **Plugin Registry**: Tracks installed plugins in `plugins.lock` files
//! - **Plugin Resolver**: Resolves plugins from project and global registries
//! - **Plugin Executor**: Discovers, verifies, and executes plugin binaries
//!
//! ## Storage Locations
//!
//! - Global: `~/.horus/plugins.lock` and `~/.horus/bin/`
//! - Project: `.horus/plugins.lock` and `.horus/bin/`
//!
//! ## Plugin Discovery
//!
//! Plugins are discovered by scanning for `horus-*` binaries in:
//! 1. Project `.horus/bin/` (highest priority)
//! 2. Global `~/.horus/bin/` (fallback)

mod executor;
mod registry;
mod resolver;

pub use executor::{PluginExecutor, PluginInfo};
pub use registry::{
    CommandInfo, Compatibility, DisabledPlugin, PluginEntry, PluginRegistry, PluginScope,
    PluginSource,
};
pub use resolver::{PluginResolver, VerificationResult, VerificationStatus};

/// Current schema version for plugins.lock
pub const SCHEMA_VERSION: &str = "1.0";

/// HORUS version for compatibility tracking
pub const HORUS_VERSION: &str = env!("CARGO_PKG_VERSION");
