use pyo3::prelude::*;

mod hub;
mod node;
mod scheduler;
// mod types; // Internal types no longer exposed to Python

use hub::PyHub;
use node::{PyNode, PyNodeInfo, PyNodeState};
use scheduler::PyScheduler;

/// HORUS Python Bindings
///
/// This module provides Python bindings for the HORUS robotics framework,
/// allowing Python developers to create and run distributed robotic systems.
#[pymodule]
fn _horus(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // ✅ USER-FACING: Core classes that users interact with
    m.add_class::<PyNode>()?;
    m.add_class::<PyNodeInfo>()?;
    m.add_class::<PyHub>()?;
    m.add_class::<PyScheduler>()?;
    m.add_class::<PyNodeState>()?;

    // ❌ REMOVED: Internal implementation types
    // - PyMessage (internal wrapper)
    // - PyNodePriority (users pass int, not enum)
    // - PyNodeConfig (internal configuration)

    // ✅ VERSION: Utility function
    m.add_function(wrap_pyfunction!(get_version, m)?)?;

    Ok(())
}

/// Get HORUS version information
#[pyfunction]
fn get_version() -> String {
    format!("HORUS Python Bindings v{}", env!("CARGO_PKG_VERSION"))
}
