use pyo3::prelude::*;

mod config;
mod hub;
mod node;
mod scheduler;
mod typed_hub;
mod types;

use config::{PyRobotPreset, PySchedulerConfig};
use hub::PyHub;
use node::{PyNode, PyNodeInfo, PyNodeState};
use scheduler::PyScheduler;
use types::Priority;

/// HORUS Python Bindings
///
/// This module provides Python bindings for the HORUS robotics framework,
/// allowing Python developers to create and run distributed robotic systems.
#[pymodule]
fn _horus(m: &Bound<'_, PyModule>) -> PyResult<()> {
    //  USER-FACING: Core classes that users interact with
    m.add_class::<PyNode>()?;
    m.add_class::<PyNodeInfo>()?;
    m.add_class::<PyHub>()?;
    m.add_class::<PyScheduler>()?;
    m.add_class::<PyNodeState>()?;

    // Configuration classes
    m.add_class::<PyRobotPreset>()?;
    m.add_class::<PySchedulerConfig>()?;

    // Priority constants
    m.add_class::<Priority>()?;

    // Typed hubs for cross-language communication
    typed_hub::register_typed_hubs(m)?;

    //  CHANGED: Priority system now uses u32 instead of enum
    // - Priority class provides constants: CRITICAL=0, HIGH=10, NORMAL=50, LOW=80, BACKGROUND=100
    // - Users can pass any u32 value for fine-grained control
    // - Old PyNodePriority enum removed for flexibility

    //  VERSION: Utility function
    m.add_function(wrap_pyfunction!(get_version, m)?)?;

    Ok(())
}

/// Get HORUS version information
#[pyfunction]
fn get_version() -> String {
    format!("HORUS Python Bindings v{}", env!("CARGO_PKG_VERSION"))
}
