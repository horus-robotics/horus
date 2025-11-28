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

// Re-export asset validation

// Re-export cache types

// Re-export resolver
