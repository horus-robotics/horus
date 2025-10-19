use pyo3::prelude::*;

mod hub;
mod node;
mod scheduler;
mod types;

use hub::PyHub;
use node::{PyNode, PyNodeInfo, PyNodeState};
use scheduler::PyScheduler;
use types::{PyMessage, PyNodeConfig, PyNodePriority};

/// HORUS Python Bindings
///
/// This module provides Python bindings for the HORUS robotics framework,
/// allowing Python developers to create and run distributed robotic systems.
#[pymodule]
fn _horus(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Core classes
    m.add_class::<PyNode>()?;
    m.add_class::<PyNodeInfo>()?;
    m.add_class::<PyHub>()?;
    m.add_class::<PyScheduler>()?;

    // Type classes
    m.add_class::<PyMessage>()?;
    m.add_class::<PyNodeState>()?;
    m.add_class::<PyNodePriority>()?;
    m.add_class::<PyNodeConfig>()?;

    // Module-level functions
    m.add_function(wrap_pyfunction!(create_node, m)?)?;
    m.add_function(wrap_pyfunction!(create_hub, m)?)?;
    m.add_function(wrap_pyfunction!(create_scheduler, m)?)?;
    m.add_function(wrap_pyfunction!(get_version, m)?)?;

    Ok(())
}

/// Create a new Python node
#[pyfunction]
fn create_node(name: String) -> PyResult<PyNode> {
    PyNode::new(name)
}

/// Create a new communication hub
#[pyfunction]
#[pyo3(signature = (topic, capacity=None))]
fn create_hub(topic: String, capacity: Option<usize>) -> PyResult<PyHub> {
    PyHub::new(topic, capacity.unwrap_or(1024))
}

/// Create a new scheduler for running nodes
#[pyfunction]
fn create_scheduler() -> PyResult<PyScheduler> {
    PyScheduler::new()
}

/// Get HORUS version information
#[pyfunction]
fn get_version() -> String {
    format!("HORUS Python Bindings v{}", env!("CARGO_PKG_VERSION"))
}
