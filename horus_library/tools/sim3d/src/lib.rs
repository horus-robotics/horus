//! Sim3D - 3D Robotics Simulator with RL Support
//!
//! This library provides a high-performance 3D physics simulator
//! with built-in reinforcement learning task support.

// Sim3D - in active development, allow common warnings
#![allow(clippy::all)]
#![allow(deprecated)]
#![allow(unused_imports)]
#![allow(unused_assignments)]
#![allow(unreachable_patterns)]
#![allow(unexpected_cfgs)]

// Re-export main modules
pub mod assets;
pub mod cli;
pub mod editor;
pub mod error;
pub mod gpu;
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
pub mod tf;
pub mod utils;

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
