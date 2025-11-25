//! Asset loading and management module
//!
//! This module provides comprehensive asset loading capabilities including:
//! - Mesh loading (OBJ, STL, COLLADA, glTF)
//! - Texture loading
//! - Asset caching
//! - Path resolution
//! - YCB object dataset loading
//! - Asset validation (URDF, robot packages)

pub mod asset_validator;
pub mod cache;
pub mod mesh;
pub mod resolver;
pub mod ycb_loader;

// Re-export commonly used types
pub use ycb_loader::{
    YCBLoader, YCBObject, YCBObjectConfig, YCBSpawnOptions, SpawnedYCBObject,
    YCBMeshObjectConfig, YCBPrimitiveConfig,
    spawn_ycb_object_at, spawn_ycb_object_with_transform, create_ycb_clutter,
};

// Re-export asset validation
pub use asset_validator::{
    AssetValidationReport, AssetType, validate_urdf, validate_robot_package,
};

// Re-export cache types
pub use cache::{AssetCache, CacheStats};

// Re-export resolver
pub use resolver::PathResolver;
