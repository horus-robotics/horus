//! Asset loading and management module
//!
//! This module provides comprehensive asset loading capabilities including:
//! - Mesh loading (OBJ, STL, COLLADA, glTF)
//! - Texture loading
//! - Asset caching
//! - Path resolution
//! - YCB object dataset loading

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
