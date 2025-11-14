// Type-based Hub implementation for Python bindings
//
// New API matches Rust exactly:
//   from horus import Hub, CmdVel, Pose2D
//   hub = Hub(CmdVel)  # Type determines everything

use horus::communication::hub::Hub;
use horus_library::messages::GenericMessage;
use horus_library::messages::cmd_vel::CmdVel;
use horus_library::messages::geometry::Pose2D;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::{Arc, Mutex};

/// Internal enum tracking which Rust type the Hub wraps
enum HubType {
    CmdVel(Arc<Mutex<Hub<CmdVel>>>),
    Pose2D(Arc<Mutex<Hub<Pose2D>>>),
    Generic(Arc<Mutex<Hub<GenericMessage>>>),
}

/// Python Hub - type-safe wrapper that creates the right Rust Hub<T>
///
/// Examples:
///     hub = Hub(CmdVel)       # Creates Hub<CmdVel> - zero overhead!
///     hub = Hub(Pose2D)       # Creates Hub<Pose2D>
///     hub = Hub("custom")     # Generic hub (fallback, slower)
#[pyclass(name = "Hub")]  // Export as "Hub" in Python, not "PyHub"
pub struct PyHub {
    hub_type: HubType,
    topic: String,
}

#[pymethods]
impl PyHub {
    /// Create a new Hub for a specific message type
    ///
    /// Args:
    ///     msg_type: Message class (CmdVel, Pose2D) or string for generic hub
    ///     capacity: Optional buffer capacity (default: 1024 if not specified)
    ///
    /// Examples:
    ///     hub = Hub(CmdVel)           # Default capacity (1024)
    ///     hub = Hub(Pose2D, 2048)     # Custom capacity
    ///     hub = Hub("custom")         # Generic hub, default capacity
    #[new]
    #[pyo3(signature = (msg_type, capacity=None))]
    fn new(py: Python, msg_type: PyObject, capacity: Option<usize>) -> PyResult<Self> {
        // Get type name from the Python object
        let type_name = if let Ok(name) = msg_type.getattr(py, "__name__") {
            name.extract::<String>(py)?
        } else if let Ok(s) = msg_type.extract::<String>(py) {
            s  // String fallback for generic hubs
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "Hub() requires a message type (CmdVel, Pose2D) or topic string"
            ));
        };

        // Get topic name from type's __topic_name__, or default to lowercase type name
        let topic = if let Ok(topic_attr) = msg_type.getattr(py, "__topic_name__") {
            topic_attr.extract::<String>(py)?
        } else {
            type_name.to_lowercase()
        };

        // Create the appropriate typed Hub (with optional custom capacity)
        let hub_type = match type_name.as_str() {
            "CmdVel" => {
                let hub = if let Some(cap) = capacity {
                    Hub::<CmdVel>::new_with_capacity(&topic, cap)
                } else {
                    Hub::<CmdVel>::new(&topic)
                }.map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to create Hub<CmdVel>"))?;
                HubType::CmdVel(Arc::new(Mutex::new(hub)))
            }
            "Pose2D" => {
                let hub = if let Some(cap) = capacity {
                    Hub::<Pose2D>::new_with_capacity(&topic, cap)
                } else {
                    Hub::<Pose2D>::new(&topic)
                }.map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to create Hub<Pose2D>"))?;
                HubType::Pose2D(Arc::new(Mutex::new(hub)))
            }
            _ => {
                // Fallback to GenericMessage for unknown types
                let hub = if let Some(cap) = capacity {
                    Hub::<GenericMessage>::new_with_capacity(&topic, cap)
                } else {
                    Hub::<GenericMessage>::new(&topic)
                }.map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to create Hub<GenericMessage>"))?;
                HubType::Generic(Arc::new(Mutex::new(hub)))
            }
        };

        Ok(Self { hub_type, topic })
    }

    /// Send a message (type must match Hub's type)
    ///
    /// Args:
    ///     message: Message object (CmdVel, Pose2D, etc.)
    ///     node: Optional Node for automatic logging with IPC timing
    ///
    /// Returns:
    ///     True if sent successfully, False otherwise
    ///
    /// Examples:
    ///     hub.send(CmdVel(1.5, 0.5), node)      # With logging
    ///     hub.send(Pose2D(1.0, 2.0, 0.5))       # Without logging
    #[pyo3(signature = (message, node=None))]
    fn send(&self, py: Python, message: PyObject, node: Option<PyObject>) -> PyResult<bool> {
        use std::time::Instant;
        let start = Instant::now();

        let result = match &self.hub_type {
            HubType::CmdVel(hub) => {
                // Extract fields from Python CmdVel object
                let linear: f32 = message.getattr(py, "linear")?.extract(py)?;
                let angular: f32 = message.getattr(py, "angular")?.extract(py)?;
                let stamp_nanos: u64 = message.getattr(py, "stamp_nanos")?.extract(py)?;

                // Create Rust CmdVel - zero-copy!
                let cmd = CmdVel::with_timestamp(linear, angular, stamp_nanos);

                // Send via typed Hub<CmdVel>
                let hub = hub.lock().unwrap();
                let success = hub.send(cmd.clone(), None).is_ok();

                // Log if node provided (use LogSummary trait!)
                if let Some(node_obj) = node {
                    let ipc_ns = start.elapsed().as_nanos() as u64;
                    if let Ok(info) = node_obj.getattr(py, "info") {
                        if !info.is_none(py) {
                            use horus::core::LogSummary;
                            let log_msg = cmd.log_summary();
                            let _ = info.call_method1(py, "log_pub", (&self.topic, log_msg, ipc_ns));
                        }
                    }
                }

                success
            }
            HubType::Pose2D(hub) => {
                // Extract fields from Python Pose2D object
                let x: f64 = message.getattr(py, "x")?.extract(py)?;
                let y: f64 = message.getattr(py, "y")?.extract(py)?;
                let theta: f64 = message.getattr(py, "theta")?.extract(py)?;
                let timestamp: u64 = message.getattr(py, "timestamp")?.extract(py)?;

                // Create Rust Pose2D - zero-copy!
                let pose = Pose2D { x, y, theta, timestamp };

                // Send via typed Hub<Pose2D>
                let hub = hub.lock().unwrap();
                let success = hub.send(pose.clone(), None).is_ok();

                // Log if node provided (use LogSummary trait!)
                if let Some(node_obj) = node {
                    let ipc_ns = start.elapsed().as_nanos() as u64;
                    if let Ok(info) = node_obj.getattr(py, "info") {
                        if !info.is_none(py) {
                            use horus::core::LogSummary;
                            let log_msg = pose.log_summary();
                            let _ = info.call_method1(py, "log_pub", (&self.topic, log_msg, ipc_ns));
                        }
                    }
                }

                success
            }
            HubType::Generic(hub) => {
                // Convert Python object to MessagePack via pythonize
                let bound = message.bind(py);
                let value: serde_json::Value = pythonize::depythonize_bound(bound.clone())
                    .map_err(|e| pyo3::exceptions::PyTypeError::new_err(
                        format!("Failed to convert Python object: {}", e)
                    ))?;

                // Serialize to MessagePack
                let msgpack_bytes = rmp_serde::to_vec(&value)
                    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(
                        format!("Failed to serialize to MessagePack: {}", e)
                    ))?;

                // Create GenericMessage (with size validation)
                let msg = GenericMessage::new(msgpack_bytes)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;

                // Send via Hub<GenericMessage>
                let hub = hub.lock().unwrap();
                let success = hub.send(msg, None).is_ok();

                // Log if node provided
                if let Some(node_obj) = node {
                    let ipc_ns = start.elapsed().as_nanos() as u64;
                    if let Ok(info) = node_obj.getattr(py, "info") {
                        if !info.is_none(py) {
                            use horus::core::LogSummary;
                            let log_msg = msg.log_summary();
                            let _ = info.call_method1(py, "log_pub", (&self.topic, log_msg, ipc_ns));
                        }
                    }
                }

                success
            }
        };

        Ok(result)
    }

    /// Receive a message (returns typed object matching Hub's type)
    ///
    /// Args:
    ///     node: Optional Node for automatic logging with IPC timing
    ///
    /// Returns:
    ///     CmdVel/Pose2D object if available, None otherwise
    ///
    /// Examples:
    ///     cmd = hub.recv(node)       # With logging
    ///     pose = hub.recv()          # Without logging
    #[pyo3(signature = (node=None))]
    fn recv(&self, py: Python, node: Option<PyObject>) -> PyResult<Option<PyObject>> {
        use std::time::Instant;
        let start = Instant::now();

        match &self.hub_type {
            HubType::CmdVel(hub) => {
                let hub = hub.lock().unwrap();
                if let Some(cmd) = hub.recv(None) {
                    let ipc_ns = start.elapsed().as_nanos() as u64;

                    // Log if node provided (use LogSummary trait!)
                    if let Some(node_obj) = &node {
                        if let Ok(info) = node_obj.getattr(py, "info") {
                            if !info.is_none(py) {
                                use horus::core::LogSummary;
                                let log_msg = cmd.log_summary();
                                let _ = info.call_method1(py, "log_sub", (&self.topic, log_msg, ipc_ns));
                            }
                        }
                    }

                    // Create Python CmdVel object
                    let horus_module = py.import_bound("horus")?;
                    let cmdvel_class = horus_module.getattr("CmdVel")?;
                    let py_cmd = cmdvel_class.call1((cmd.linear, cmd.angular, cmd.stamp_nanos))?;
                    Ok(Some(py_cmd.into()))
                } else {
                    Ok(None)
                }
            }
            HubType::Pose2D(hub) => {
                let hub = hub.lock().unwrap();
                if let Some(pose) = hub.recv(None) {
                    let ipc_ns = start.elapsed().as_nanos() as u64;

                    // Log if node provided (use LogSummary trait!)
                    if let Some(node_obj) = &node {
                        if let Ok(info) = node_obj.getattr(py, "info") {
                            if !info.is_none(py) {
                                use horus::core::LogSummary;
                                let log_msg = pose.log_summary();
                                let _ = info.call_method1(py, "log_sub", (&self.topic, log_msg, ipc_ns));
                            }
                        }
                    }

                    // Create Python Pose2D object
                    let horus_module = py.import_bound("horus")?;
                    let pose2d_class = horus_module.getattr("Pose2D")?;
                    let py_pose = pose2d_class.call1((pose.x, pose.y, pose.theta, pose.timestamp))?;
                    Ok(Some(py_pose.into()))
                } else {
                    Ok(None)
                }
            }
            HubType::Generic(hub) => {
                let hub = hub.lock().unwrap();
                if let Some(msg) = hub.recv(None) {
                    let ipc_ns = start.elapsed().as_nanos() as u64;

                    // Log if node provided
                    if let Some(node_obj) = &node {
                        if let Ok(info) = node_obj.getattr(py, "info") {
                            if !info.is_none(py) {
                                use horus::core::LogSummary;
                                let log_msg = msg.log_summary();
                                let _ = info.call_method1(py, "log_sub", (&self.topic, log_msg, ipc_ns));
                            }
                        }
                    }

                    // Deserialize MessagePack to serde_json::Value
                    let data = msg.data();
                    let value: serde_json::Value = rmp_serde::from_slice(&data)
                        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(
                            format!("Failed to deserialize MessagePack: {}", e)
                        ))?;

                    // Convert serde Value to Python object
                    let py_obj = pythonize::pythonize(py, &value)
                        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(
                            format!("Failed to convert to Python: {}", e)
                        ))?;

                    Ok(Some(py_obj.into()))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Get the topic name
    fn topic(&self) -> String {
        self.topic.clone()
    }
}
