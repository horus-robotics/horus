// Python wrappers for diagnostics messages
use horus_library::messages::diagnostics;
use pyo3::prelude::*;

/// Python wrapper for Status
#[pyclass(module = "horus.library._library", name = "Status")]
#[derive(Clone)]
pub struct PyStatus {
    pub(crate) inner: diagnostics::Status,
}

#[pymethods]
impl PyStatus {
    #[new]
    #[pyo3(signature = (level=0, code=0, message=""))]
    fn new(level: u8, code: u32, message: &str) -> Self {
        let status_level = match level {
            0 => diagnostics::StatusLevel::Ok,
            1 => diagnostics::StatusLevel::Warn,
            2 => diagnostics::StatusLevel::Error,
            3 => diagnostics::StatusLevel::Fatal,
            _ => diagnostics::StatusLevel::Ok,
        };
        Self {
            inner: diagnostics::Status::new(status_level, code, message),
        }
    }

    #[staticmethod]
    fn ok(message: &str) -> Self {
        Self {
            inner: diagnostics::Status::ok(message),
        }
    }

    #[staticmethod]
    fn warn(code: u32, message: &str) -> Self {
        Self {
            inner: diagnostics::Status::warn(code, message),
        }
    }

    #[staticmethod]
    fn error(code: u32, message: &str) -> Self {
        Self {
            inner: diagnostics::Status::error(code, message),
        }
    }

    #[staticmethod]
    fn fatal(code: u32, message: &str) -> Self {
        Self {
            inner: diagnostics::Status::fatal(code, message),
        }
    }

    #[getter]
    fn level(&self) -> u8 {
        self.inner.level as u8
    }

    #[getter]
    fn code(&self) -> u32 {
        self.inner.code
    }

    fn message(&self) -> String {
        self.inner.message_str()
    }

    fn component(&self) -> String {
        self.inner.component_str()
    }

    fn with_component(&self, component: &str) -> Self {
        Self {
            inner: self.inner.clone().with_component(component),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Status(level={}, code={}, msg='{}')",
            self.inner.level as u8,
            self.inner.code,
            self.inner.message_str()
        )
    }
}

/// Python wrapper for EmergencyStop
#[pyclass(module = "horus.library._library", name = "EmergencyStop")]
#[derive(Clone)]
pub struct PyEmergencyStop {
    pub(crate) inner: diagnostics::EmergencyStop,
}

#[pymethods]
impl PyEmergencyStop {
    #[new]
    fn new() -> Self {
        Self {
            inner: diagnostics::EmergencyStop::default(),
        }
    }

    #[staticmethod]
    fn engage(reason: &str) -> Self {
        Self {
            inner: diagnostics::EmergencyStop::engage(reason),
        }
    }

    #[staticmethod]
    fn release() -> Self {
        Self {
            inner: diagnostics::EmergencyStop::release(),
        }
    }

    #[getter]
    fn engaged(&self) -> bool {
        self.inner.engaged
    }

    fn reason(&self) -> String {
        self.inner.reason_str()
    }

    fn with_source(&self, source: &str) -> Self {
        Self {
            inner: self.inner.clone().with_source(source),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "EmergencyStop(engaged={}, reason='{}')",
            self.inner.engaged,
            self.inner.reason_str()
        )
    }
}

/// Python wrapper for Heartbeat
#[pyclass(module = "horus.library._library", name = "Heartbeat")]
#[derive(Clone)]
pub struct PyHeartbeat {
    pub(crate) inner: diagnostics::Heartbeat,
}

#[pymethods]
impl PyHeartbeat {
    #[new]
    #[pyo3(signature = (node_name="", node_id=0))]
    fn new(node_name: &str, node_id: u32) -> Self {
        Self {
            inner: diagnostics::Heartbeat::new(node_name, node_id),
        }
    }

    #[getter]
    fn node_id(&self) -> u32 {
        self.inner.node_id
    }

    #[getter]
    fn sequence(&self) -> u64 {
        self.inner.sequence
    }

    #[getter]
    fn alive(&self) -> bool {
        self.inner.alive
    }

    #[getter]
    fn uptime(&self) -> f64 {
        self.inner.uptime
    }

    fn name(&self) -> String {
        self.inner.name()
    }

    fn update(&mut self, uptime: f64) {
        self.inner.update(uptime);
    }

    fn __repr__(&self) -> String {
        format!(
            "Heartbeat(node='{}', seq={}, uptime={:.1}s)",
            self.inner.name(),
            self.inner.sequence,
            self.inner.uptime
        )
    }
}

/// Python wrapper for ResourceUsage
#[pyclass(module = "horus.library._library", name = "ResourceUsage")]
#[derive(Clone)]
pub struct PyResourceUsage {
    pub(crate) inner: diagnostics::ResourceUsage,
}

#[pymethods]
impl PyResourceUsage {
    #[new]
    fn new() -> Self {
        Self {
            inner: diagnostics::ResourceUsage::new(),
        }
    }

    #[getter]
    fn cpu_percent(&self) -> f32 {
        self.inner.cpu_percent
    }

    #[setter]
    fn set_cpu_percent(&mut self, value: f32) {
        self.inner.cpu_percent = value;
    }

    #[getter]
    fn memory_percent(&self) -> f32 {
        self.inner.memory_percent
    }

    #[setter]
    fn set_memory_percent(&mut self, value: f32) {
        self.inner.memory_percent = value;
    }

    #[getter]
    fn memory_bytes(&self) -> u64 {
        self.inner.memory_bytes
    }

    #[setter]
    fn set_memory_bytes(&mut self, value: u64) {
        self.inner.memory_bytes = value;
    }

    #[getter]
    fn temperature(&self) -> f32 {
        self.inner.temperature
    }

    #[setter]
    fn set_temperature(&mut self, value: f32) {
        self.inner.temperature = value;
    }

    fn is_cpu_high(&self, threshold: f32) -> bool {
        self.inner.is_cpu_high(threshold)
    }

    fn is_memory_high(&self, threshold: f32) -> bool {
        self.inner.is_memory_high(threshold)
    }

    fn is_temperature_high(&self, threshold: f32) -> bool {
        self.inner.is_temperature_high(threshold)
    }

    fn __repr__(&self) -> String {
        format!(
            "ResourceUsage(cpu={:.1}%, mem={:.1}%, temp={:.1}°C)",
            self.inner.cpu_percent, self.inner.memory_percent, self.inner.temperature
        )
    }
}
