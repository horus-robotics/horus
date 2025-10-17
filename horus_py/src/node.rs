use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use std::sync::{Arc, Mutex};
use horus::{Node, NodeInfo as CoreNodeInfo, NodeState as CoreNodeState};

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
pub struct PyNodeInfo {
    inner: Arc<Mutex<CoreNodeInfo>>,
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
        let info = self.inner.lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        Ok(info.name().to_string())
    }
    
    #[getter]
    fn state(&self) -> PyResult<PyNodeState> {
        let info = self.inner.lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        Ok(PyNodeState::from(info.state()))
    }
    
    fn log_info(&self, message: String) -> PyResult<()> {
        let mut info = self.inner.lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.log_info(&message);
        Ok(())
    }
    
    fn log_warning(&self, message: String) -> PyResult<()> {
        let mut info = self.inner.lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.log_warning(&message);
        Ok(())
    }
    
    fn log_error(&self, message: String) -> PyResult<()> {
        let mut info = self.inner.lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.log_error(&message);
        Ok(())
    }
    
    fn log_debug(&self, message: String) -> PyResult<()> {
        let mut info = self.inner.lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.log_debug(&message);
        Ok(())
    }
    
    fn set_custom_data(&self, key: String, value: String) -> PyResult<()> {
        let mut info = self.inner.lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        info.set_custom_data(key, value);
        Ok(())
    }
    
    fn get_custom_data(&self, key: String) -> PyResult<Option<String>> {
        let info = self.inner.lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
        Ok(info.get_custom_data(&key).cloned())
    }
}

/// Python wrapper for HORUS Node
/// 
/// This class allows Python code to implement HORUS nodes
/// by subclassing and implementing the required methods.
#[pyclass(subclass)]
pub struct PyNode {
    #[pyo3(get)]
    pub name: String,
    pub info: Arc<Mutex<CoreNodeInfo>>,
    pub py_callback: Option<PyObject>,
}

#[pymethods]
impl PyNode {
    #[new]
    pub fn new(name: String) -> PyResult<Self> {
        Ok(PyNode {
            name: name.clone(),
            info: Arc::new(Mutex::new(CoreNodeInfo::new(name, true))),
            py_callback: None,
        })
    }
    
    /// Initialize the node
    fn init(&mut self, py: Python) -> PyResult<()> {
        // Call Python init method if it exists
        if let Some(callback) = &self.py_callback {
            let info = PyNodeInfo { inner: self.info.clone() };
            callback.call_method1(py, "init", (info,))?;
        }
        Ok(())
    }
    
    /// Main execution tick
    fn tick(&mut self, py: Python) -> PyResult<()> {
        // Start tick timing
        {
            let mut info = self.info.lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
            info.start_tick();
        }
        
        // Call Python tick method if it exists
        if let Some(callback) = &self.py_callback {
            let info = PyNodeInfo { inner: self.info.clone() };
            callback.call_method1(py, "tick", (info,))?;
        }
        
        // Record tick completion
        {
            let mut info = self.info.lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
            info.record_tick();
        }
        
        Ok(())
    }
    
    /// Shutdown the node
    fn shutdown(&mut self, py: Python) -> PyResult<()> {
        // Call Python shutdown method if it exists
        if let Some(callback) = &self.py_callback {
            let info = PyNodeInfo { inner: self.info.clone() };
            callback.call_method1(py, "shutdown", (info,))?;
        }
        
        {
            let mut info = self.info.lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock NodeInfo: {}", e)))?;
            let _ = info.shutdown();
        }
        
        Ok(())
    }
    
    /// Set the Python callback object (usually 'self' from Python subclass)
    fn set_callback(&mut self, callback: PyObject) -> PyResult<()> {
        self.py_callback = Some(callback);
        Ok(())
    }
    
    /// Get node information
    #[getter]
    fn info(&self) -> PyResult<PyNodeInfo> {
        Ok(PyNodeInfo {
            inner: self.info.clone(),
        })
    }
    
    fn __repr__(&self) -> String {
        format!("Node(name='{}')", self.name)
    }
}

// Bridge struct to implement the Rust Node trait
pub struct PythonNodeBridge {
    pub py_node: Arc<Mutex<PyObject>>,
    pub name: String,
}

impl Node for PythonNodeBridge {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }
    
    fn tick(&mut self, _ctx: Option<&mut CoreNodeInfo>) {
        Python::with_gil(|py| {
            if let Ok(node) = self.py_node.lock() {
                let _ = node.call_method0(py, "tick");
            }
        });
    }
}