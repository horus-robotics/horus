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
    create_ycb_clutter, spawn_ycb_object_at, spawn_ycb_object_with_transform, SpawnedYCBObject,
    YCBLoader, YCBMeshObjectConfig, YCBObject, YCBObjectConfig, YCBPrimitiveConfig,
    YCBSpawnOptions,
};

// Re-export asset validation
pub use asset_validator::{
    validate_robot_package, validate_urdf, AssetType, AssetValidationReport,
};

// Re-export cache types
pub use cache::{AssetCache, CacheStats};

// Re-export resolver
pub use resolver::PathResolver;
