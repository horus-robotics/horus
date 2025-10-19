use horus::communication::Hub;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Generic message type that can be serialized between Rust and Python
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenericMessage {
    pub data: Vec<u8>,
    pub metadata: Option<String>,
}

/// Python wrapper for HORUS Hub (Publisher)
///
/// This class provides publish/subscribe communication between nodes.
/// Messages are serialized using bincode for efficient transmission.
#[pyclass]
#[derive(Clone)]
pub struct PyHub {
    topic: String,
    hub: Arc<Mutex<Hub<GenericMessage>>>,
}

#[pymethods]
impl PyHub {
    #[new]
    pub fn new(topic: String, capacity: usize) -> PyResult<Self> {
        let hub = Hub::new_with_capacity(&topic, capacity)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create hub: {}", e)))?;

        Ok(PyHub {
            topic,
            hub: Arc::new(Mutex::new(hub)),
        })
    }

    /// Send a message to all subscribers
    ///
    /// The message can be any Python object that can be serialized to bytes.
    /// Common types like dict, list, str, int, float are automatically handled.
    fn send(&self, py: Python, message: PyObject) -> PyResult<bool> {
        // Convert Python object to bytes
        let data = if let Ok(bytes) = message.extract::<Vec<u8>>(py) {
            bytes
        } else if let Ok(string) = message.extract::<String>(py) {
            string.into_bytes()
        } else if let Ok(dict) = message.downcast_bound::<PyDict>(py) {
            // Serialize dict as JSON
            let json_str =
                serde_json::to_string(&pythonize::depythonize::<serde_json::Value>(dict)?)
                    .map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to serialize dict: {}", e))
                    })?;
            json_str.into_bytes()
        } else {
            // Try to pickle the object as fallback
            let pickle = py.import_bound("pickle")?;
            let pickled = pickle.call_method1("dumps", (message,))?;
            pickled.extract::<Vec<u8>>()?
        };

        let msg = GenericMessage {
            data,
            metadata: None,
        };

        let hub = self
            .hub
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock hub: {}", e)))?;

        match hub.send(msg, None) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Send raw bytes
    fn send_bytes(&self, data: Vec<u8>) -> PyResult<bool> {
        let msg = GenericMessage {
            data,
            metadata: None,
        };

        let hub = self
            .hub
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock hub: {}", e)))?;

        match hub.send(msg, None) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Send with metadata
    fn send_with_metadata(&self, data: Vec<u8>, metadata: String) -> PyResult<bool> {
        let msg = GenericMessage {
            data,
            metadata: Some(metadata),
        };

        let hub = self
            .hub
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock hub: {}", e)))?;

        match hub.send(msg, None) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Try to receive a message
    fn recv(&self, py: Python) -> PyResult<Option<PyObject>> {
        let hub = self
            .hub
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock hub: {}", e)))?;

        if let Some(msg) = hub.recv(None) {
            // Convert bytes back to Python object
            let bytes = PyBytes::new_bound(py, &msg.data);
            Ok(Some(bytes.into()))
        } else {
            Ok(None)
        }
    }

    /// Try to receive a message with metadata
    fn recv_with_metadata(&self, py: Python) -> PyResult<Option<(PyObject, Option<String>)>> {
        let hub = self
            .hub
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock hub: {}", e)))?;

        if let Some(msg) = hub.recv(None) {
            let bytes = PyBytes::new_bound(py, &msg.data);
            Ok(Some((bytes.into(), msg.metadata)))
        } else {
            Ok(None)
        }
    }

    /// Get the topic name
    #[getter]
    fn topic(&self) -> String {
        self.topic.clone()
    }

    fn __repr__(&self) -> String {
        format!("Hub(topic='{}')", self.topic)
    }

    fn __str__(&self) -> String {
        self.topic.clone()
    }
}

/// Python wrapper for creating typed hubs
#[pyclass]
pub struct PyTypedHub {
    topic: String,
    hub_type: String,
    hub: Arc<Mutex<Hub<GenericMessage>>>,
}

#[pymethods]
impl PyTypedHub {
    #[new]
    fn new(topic: String, hub_type: String) -> PyResult<Self> {
        let hub = Hub::new(&topic)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create hub: {}", e)))?;

        Ok(PyTypedHub {
            topic,
            hub_type,
            hub: Arc::new(Mutex::new(hub)),
        })
    }

    /// Send a typed message
    fn send(&self, py: Python, message: PyObject) -> PyResult<bool> {
        // Validate type if needed
        // For now, just serialize and send
        let data = if let Ok(bytes) = message.extract::<Vec<u8>>(py) {
            bytes
        } else {
            // Serialize using pickle or json
            let pickle = py.import_bound("pickle")?;
            let pickled = pickle.call_method1("dumps", (message,))?;
            pickled.extract::<Vec<u8>>()?
        };

        let msg = GenericMessage {
            data,
            metadata: Some(self.hub_type.clone()),
        };

        let hub = self
            .hub
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock hub: {}", e)))?;

        match hub.send(msg, None) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Receive a typed message
    fn recv(&self, py: Python) -> PyResult<Option<PyObject>> {
        let hub = self
            .hub
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock hub: {}", e)))?;

        if let Some(msg) = hub.recv(None) {
            // Validate type if metadata matches
            if let Some(metadata) = &msg.metadata {
                if metadata != &self.hub_type {
                    return Err(PyRuntimeError::new_err(format!(
                        "Type mismatch: expected '{}', got '{}'",
                        self.hub_type, metadata
                    )));
                }
            }

            // Deserialize back to Python object
            let pickle = py.import_bound("pickle")?;
            let bytes = PyBytes::new_bound(py, &msg.data);
            let obj = pickle.call_method1("loads", (bytes,))?;
            Ok(Some(obj.into()))
        } else {
            Ok(None)
        }
    }
}
