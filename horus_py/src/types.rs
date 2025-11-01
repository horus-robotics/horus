use horus::{NodeConfig as CoreNodeConfig, NodePriority as CoreNodePriority};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;

/// Python wrapper for messages
#[pyclass]
#[derive(Clone)]
pub struct PyMessage {
    #[pyo3(get, set)]
    pub data: Vec<u8>,
    #[pyo3(get, set)]
    pub topic: String,
    #[pyo3(get, set)]
    pub timestamp: f64,
    #[pyo3(get, set)]
    pub metadata: HashMap<String, String>,
}

#[pymethods]
impl PyMessage {
    #[new]
    fn new(data: Vec<u8>, topic: String) -> Self {
        PyMessage {
            data,
            topic,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            metadata: HashMap::new(),
        }
    }

    fn set_metadata_item(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    fn get_metadata_item(&self, key: String) -> Option<String> {
        self.metadata.get(&key).cloned()
    }

    fn __repr__(&self) -> String {
        format!(
            "Message(topic='{}', size={} bytes, timestamp={})",
            self.topic,
            self.data.len(),
            self.timestamp
        )
    }

    fn __len__(&self) -> usize {
        self.data.len()
    }
}

/// Python wrapper for NodePriority
#[pyclass]
#[derive(Clone, Copy)]
pub struct PyNodePriority {
    value: CoreNodePriority,
}

#[pymethods]
impl PyNodePriority {
    #[new]
    fn new(priority: String) -> PyResult<Self> {
        let value = match priority.to_lowercase().as_str() {
            "critical" => CoreNodePriority::Critical,
            "high" => CoreNodePriority::High,
            "normal" => CoreNodePriority::Normal,
            "low" => CoreNodePriority::Low,
            "background" => CoreNodePriority::Background,
            _ => {
                return Err(PyValueError::new_err(format!(
                "Invalid priority '{}'. Must be one of: critical, high, normal, low, background",
                priority
            )))
            }
        };
        Ok(PyNodePriority { value })
    }

    #[staticmethod]
    fn critical() -> Self {
        PyNodePriority {
            value: CoreNodePriority::Critical,
        }
    }

    #[staticmethod]
    fn high() -> Self {
        PyNodePriority {
            value: CoreNodePriority::High,
        }
    }

    #[staticmethod]
    fn normal() -> Self {
        PyNodePriority {
            value: CoreNodePriority::Normal,
        }
    }

    #[staticmethod]
    fn low() -> Self {
        PyNodePriority {
            value: CoreNodePriority::Low,
        }
    }

    #[staticmethod]
    fn background() -> Self {
        PyNodePriority {
            value: CoreNodePriority::Background,
        }
    }

    fn __repr__(&self) -> String {
        let name = match self.value {
            CoreNodePriority::Critical => "critical",
            CoreNodePriority::High => "high",
            CoreNodePriority::Normal => "normal",
            CoreNodePriority::Low => "low",
            CoreNodePriority::Background => "background",
        };
        format!("NodePriority.{}", name)
    }

    fn __str__(&self) -> String {
        match self.value {
            CoreNodePriority::Critical => "critical".to_string(),
            CoreNodePriority::High => "high".to_string(),
            CoreNodePriority::Normal => "normal".to_string(),
            CoreNodePriority::Low => "low".to_string(),
            CoreNodePriority::Background => "background".to_string(),
        }
    }
}

/// Python wrapper for NodeConfig
#[pyclass]
#[derive(Clone)]
pub struct PyNodeConfig {
    #[pyo3(get, set)]
    pub max_tick_duration_ms: Option<u64>,
    #[pyo3(get, set)]
    pub restart_on_failure: bool,
    #[pyo3(get, set)]
    pub max_restart_attempts: u32,
    #[pyo3(get, set)]
    pub restart_delay_ms: u64,
    #[pyo3(get, set)]
    pub enable_logging: bool,
    #[pyo3(get, set)]
    pub log_level: String,
    #[pyo3(get, set)]
    pub custom_params: HashMap<String, String>,
}

#[pymethods]
impl PyNodeConfig {
    #[new]
    fn new() -> Self {
        let config = CoreNodeConfig::default();
        PyNodeConfig {
            max_tick_duration_ms: config.max_tick_duration_ms,
            restart_on_failure: config.restart_on_failure,
            max_restart_attempts: config.max_restart_attempts,
            restart_delay_ms: config.restart_delay_ms,
            enable_logging: config.enable_logging,
            log_level: config.log_level,
            custom_params: config.custom_params,
        }
    }

    fn set_param(&mut self, key: String, value: String) {
        self.custom_params.insert(key, value);
    }

    fn get_param(&self, key: String) -> Option<String> {
        self.custom_params.get(&key).cloned()
    }

    fn __repr__(&self) -> String {
        format!(
            "NodeConfig(logging={}, log_level='{}')",
            self.enable_logging, self.log_level
        )
    }
}

impl From<PyNodeConfig> for CoreNodeConfig {
    fn from(py_config: PyNodeConfig) -> Self {
        CoreNodeConfig {
            max_tick_duration_ms: py_config.max_tick_duration_ms,
            restart_on_failure: py_config.restart_on_failure,
            max_restart_attempts: py_config.max_restart_attempts,
            restart_delay_ms: py_config.restart_delay_ms,
            enable_logging: py_config.enable_logging,
            log_level: py_config.log_level,
            custom_params: py_config.custom_params,
        }
    }
}
