//! Sim3D - 3D Robotics Simulator with RL Support
//!
//! This library provides a high-performance 3D physics simulator
//! with built-in reinforcement learning task support.

// Allow cfg for disabled test feature
#![allow(unexpected_cfgs)]
// Bevy ECS standard patterns - these are idiomatic for Bevy systems
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
// EnhancedError is intentionally large (136 bytes) to carry rich error context
// (file path, line/column, hints, suggestions). Error paths are not hot paths.
#![allow(clippy::result_large_err)]
// Public API: This library exports many types, structs, and functions as public API
// that are not internally consumed but are intended for external users. These are
// legitimate library exports (config presets, asset loaders, physics types, etc.)
#![allow(dead_code)]

// Re-export main modules
pub mod assets;
pub mod cli;
pub mod config;
pub mod editor;
pub mod error;
pub mod gpu;
pub mod hframe;
pub mod horus_bridge;
pub mod multi_robot;
pub mod physics;
pub mod plugins;
pub mod procedural;
pub mod recording;
pub mod rendering;
pub mod rl;
pub mod robot;
pub mod scene;
pub mod sensors;
pub mod systems;

// Backwards-compatible re-export as tf
pub mod tf {
    //! Backwards-compatible re-export of hframe module
    //!
    //! This module is deprecated. Use `hframe` directly instead.
    pub use crate::hframe::*;
}
pub mod utils;
pub mod view_modes;

// UI module (conditional on visual feature due to other module errors)
#[cfg(feature = "visual")]
pub mod ui;

// Re-export Python bindings when the python feature is enabled
#[cfg(feature = "python")]
pub use rl::python::*;

// Python module (for PyO3)
#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn sim3d_rl(m: &Bound<PyModule>) -> PyResult<()> {
    use rl::python::{make_env, make_vec_env, PySim3DEnv, PyVecSim3DEnv};

    m.add_class::<PySim3DEnv>()?;
    m.add_class::<PyVecSim3DEnv>()?;
    m.add_function(wrap_pyfunction!(make_env, m)?)?;
    m.add_function(wrap_pyfunction!(make_vec_env, m)?)?;
    Ok(())
}
