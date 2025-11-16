//! Sim3D - 3D Robotics Simulator with RL Support
//!
//! This library provides a high-performance 3D physics simulator
//! with built-in reinforcement learning task support.

// Re-export main modules
pub mod physics;
pub mod robot;
pub mod sensors;
pub mod systems;
pub mod tf;
pub mod utils;
pub mod rl;

// Re-export Python bindings when the python feature is enabled
#[cfg(feature = "python")]
pub use rl::python::*;

// Python module (for PyO3)
#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn sim3d_rl(m: &Bound<PyModule>) -> PyResult<()> {
    use rl::python::{PySim3DEnv, PyVecSim3DEnv, make_env, make_vec_env};

    m.add_class::<PySim3DEnv>()?;
    m.add_class::<PyVecSim3DEnv>()?;
    m.add_function(wrap_pyfunction!(make_env, m)?)?;
    m.add_function(wrap_pyfunction!(make_vec_env, m)?)?;
    Ok(())
}
