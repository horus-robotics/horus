use horus::{NodeInfo as CoreNodeInfo, NodeState as CoreNodeState};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

/// Python wrapper for NodeState
#[pyclass]
#[derive(Clone)]
pub struct PyNodeState {
    #[pyo3(get)]
    pub name: String,
}

#[pymethods]
impl PyNodeState {
    #[new]
    fn new(name: String) -> Self {
        PyNodeState { name }
    }

    fn __repr__(&self) -> String {
        format!("NodeState('{}')", self.name)
    }

    fn __str__(&self) -> String {
        self.name.clone()
    }
}

impl From<&CoreNodeState> for PyNodeState {
    fn from(state: &CoreNodeState) -> Self {
        let name = match state {
            CoreNodeState::Uninitialized => "uninitialized",
            CoreNodeState::Initializing => "initializing",
            CoreNodeState::Running => "running",
            CoreNodeState::Paused => "paused",
            CoreNodeState::Stopping => "stopping",
            CoreNodeState::Stopped => "stopped",
            CoreNodeState::Error(_) => "error",
            CoreNodeState::Crashed(_) => "crashed",
        };
        PyNodeState::new(name.to_string())
    }
}

/// Python wrapper for NodeInfo
#[pyclass]
#[derive(Clone)]
pub struct PyNodeInfo {
    pub inner: Arc<Mutex<CoreNodeInfo>>,
}

#[pymethods]
impl PyNodeInfo {
    #[new]
    fn new(name: String) -> Self {
        PyNodeInfo {
            inner: Arc::new(Mutex::new(CoreNodeInfo::new(name, true))),
        }
    }

    #[getter]
    fn name(&self) -> PyResult<String> {
        let info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        Ok(info.name().to_string())
    }

    #[getter]
    fn state(&self) -> PyResult<PyNodeState> {
        let info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        Ok(PyNodeState::from(info.state()))
    }

    fn log_info(&self, message: String) -> PyResult<()> {
        // Take String (owned) instead of &str (borrowed) to avoid PyO3 borrow issues
        let info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.log_info(&message);
        Ok(())
    }

    fn log_warning(&self, message: String) -> PyResult<()> {
        // Take String (owned) instead of &str (borrowed) to avoid PyO3 borrow issues
        let mut info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.log_warning(&message);
        Ok(())
    }

    fn log_error(&self, message: String) -> PyResult<()> {
        // Take String (owned) instead of &str (borrowed) to avoid PyO3 borrow issues
        let mut info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.log_error(&message);
        Ok(())
    }

    fn log_debug(&self, message: String) -> PyResult<()> {
        // Take String (owned) instead of &str (borrowed) to avoid PyO3 borrow issues
        let mut info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.log_debug(&message);
        Ok(())
    }

    fn set_custom_data(&self, key: String, value: String) -> PyResult<()> {
        let mut info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.set_custom_data(key, value);
        Ok(())
    }

    fn get_custom_data(&self, key: String) -> PyResult<Option<String>> {
        let info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        Ok(info.get_custom_data(&key).cloned())
    }

    /// Get total tick count
    fn tick_count(&self) -> PyResult<u64> {
        let info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        Ok(info.metrics().total_ticks)
    }

    /// Get error count
    fn error_count(&self) -> PyResult<u64> {
        let info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        Ok(info.metrics().errors_count)
    }

    /// Transition to error state
    fn transition_to_error(&self, error_msg: String) -> PyResult<()> {
        let mut info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.transition_to_error(error_msg);
        Ok(())
    }

    /// Log a publish operation with IPC timing
    fn log_pub(&self, topic: String, data_repr: String, ipc_ns: u64) -> PyResult<()> {
        // Take String (owned) instead of &str (borrowed) to avoid PyO3 borrow issues
        let info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;

        if info.config().enable_logging {
            // Format everything to owned Strings first to avoid lifetime issues
            let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
            let node_name = info.name().to_string();

            // Build the log message safely
            let msg = format!(
                "\r\n\x1b[36m[{}]\x1b[0m \x1b[32m[IPC: {}ns]\x1b[0m \x1b[33m{}\x1b[0m \x1b[1;32m--PUB-->\x1b[0m \x1b[35m'{}'\x1b[0m = {}\r\n",
                timestamp,
                ipc_ns,
                node_name,
                topic,
                data_repr
            );

            use std::io::{self, Write};
            let _ = io::stdout().write_all(msg.as_bytes());
            let _ = io::stdout().flush();
        }

        Ok(())
    }

    /// Log a subscribe operation with IPC timing
    fn log_sub(&self, topic: String, data_repr: String, ipc_ns: u64) -> PyResult<()> {
        // Take String (owned) instead of &str (borrowed) to avoid PyO3 borrow issues
        let info = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;

        if info.config().enable_logging {
            // Format everything to owned Strings first to avoid lifetime issues
            let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
            let node_name = info.name().to_string();

            // Build the log message safely
            let msg = format!(
                "\x1b[36m[{}]\x1b[0m \x1b[32m[IPC: {}ns]\x1b[0m \x1b[33m{}\x1b[0m \x1b[1;34m<--SUB--\x1b[0m \x1b[35m'{}'\x1b[0m = {}\n",
                timestamp,
                ipc_ns,
                node_name,
                topic,
                data_repr
            );

            use std::io::{self, Write};
            let _ = io::stdout().write_all(msg.as_bytes());
            let _ = io::stdout().flush();
        }

        Ok(())
    }

    fn __repr__(&self) -> PyResult<String> {
        if let Ok(info) = self.inner.lock() {
            Ok(format!(
                "NodeInfo(name='{}', state='{}', ticks={}, errors={})",
                info.name(),
                info.state(),
                info.metrics().total_ticks,
                info.metrics().errors_count
            ))
        } else {
            Ok("NodeInfo(locked)".to_string())
        }
    }
}

/// Python wrapper for HORUS Node
///
/// This class allows Python code to implement HORUS nodes
/// by subclassing and implementing the required methods.
///
/// NOTE: PyNode no longer creates its own NodeInfo. The scheduler will provide one.
#[pyclass(subclass)]
pub struct PyNode {
    #[pyo3(get)]
    pub name: String,
    pub py_callback: Option<PyObject>,
}

#[pymethods]
impl PyNode {
    #[new]
    pub fn new(name: String) -> PyResult<Self> {
        Ok(PyNode {
            name: name.clone(),
            py_callback: None,
        })
    }

    /// Initialize the node
    /// The scheduler passes NodeInfo, which we forward to the Python callback
    fn init(&mut self, py: Python, info: PyNodeInfo) -> PyResult<()> {
        if let Some(callback) = &self.py_callback {
            callback.call_method1(py, "init", (info,))?;
        }
        Ok(())
    }

    /// Main execution tick
    /// The scheduler passes NodeInfo, which we forward to the Python callback
    fn tick(&mut self, py: Python, info: PyNodeInfo) -> PyResult<()> {
        if let Some(callback) = &self.py_callback {
            callback.call_method1(py, "tick", (info,))?;
        }
        Ok(())
    }

    /// Shutdown the node
    /// The scheduler passes NodeInfo, which we forward to the Python callback
    fn shutdown(&mut self, py: Python, info: PyNodeInfo) -> PyResult<()> {
        if let Some(callback) = &self.py_callback {
            callback.call_method1(py, "shutdown", (info,))?;
        }
        Ok(())
    }

    /// Set the Python callback object (usually 'self' from Python subclass)
    fn set_callback(&mut self, callback: PyObject) -> PyResult<()> {
        self.py_callback = Some(callback);
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("Node(name='{}')", self.name)
    }
}

// Bridge struct to implement the Rust Node trait
