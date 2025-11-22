//! Asset loading and management module
//!
//! This module provides comprehensive asset loading capabilities including:
//! - Mesh loading (OBJ, STL, COLLADA, glTF)
//! - Texture loading
//! - Asset caching
//! - Path resolution

pub mod asset_validator;
pub mod cache;
pub mod mesh;
pub mod resolver;
