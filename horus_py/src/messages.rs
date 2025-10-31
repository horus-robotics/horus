/// Phase 3: Python bindings for Rust message types
///
/// Exposes typed message structures from horus_library to Python
/// for optional type-safe communication.

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

/// Command velocity message for robot control (Python wrapper)
///
/// Standard message type for controlling robot movement with linear
/// and angular velocity commands.
///
/// Example:
///     cmd = CmdVel(linear=1.5, angular=0.5)
///     node.send("cmd_vel", cmd)
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdVel {
    #[pyo3(get, set)]
    pub stamp_nanos: u64,

    #[pyo3(get, set)]
    pub linear: f32,  // m/s forward velocity

    #[pyo3(get, set)]
    pub angular: f32, // rad/s turning velocity
}

#[pymethods]
impl CmdVel {
    #[new]
    #[pyo3(signature = (linear=0.0, angular=0.0, stamp_nanos=None))]
    pub fn new(linear: f32, angular: f32, stamp_nanos: Option<u64>) -> Self {
        let stamp = stamp_nanos.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        });

        Self {
            stamp_nanos: stamp,
            linear,
            angular,
        }
    }

    /// Create a zero velocity command (stop)
    #[staticmethod]
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, None)
    }

    /// Get timestamp in seconds (Python-friendly)
    #[getter]
    pub fn timestamp(&self) -> f64 {
        self.stamp_nanos as f64 / 1_000_000_000.0
    }

    /// Get age of message in seconds
    pub fn age(&self) -> f64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        (now - self.stamp_nanos) as f64 / 1_000_000_000.0
    }

    fn __repr__(&self) -> String {
        format!(
            "CmdVel(linear={:.2}, angular={:.2}, age={:.3}s)",
            self.linear,
            self.angular,
            self.age()
        )
    }

    fn __str__(&self) -> String {
        format!("linear: {:.2} m/s, angular: {:.2} rad/s", self.linear, self.angular)
    }

    /// Convert to dict for backward compatibility
    pub fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("linear", self.linear)?;
        dict.set_item("angular", self.angular)?;
        dict.set_item("stamp_nanos", self.stamp_nanos)?;
        Ok(dict.into())
    }

    /// Create from dict
    #[staticmethod]
    pub fn from_dict(dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<Self> {
        let linear = dict.get_item("linear")?
            .and_then(|v| v.extract::<f32>().ok())
            .unwrap_or(0.0);
        let angular = dict.get_item("angular")?
            .and_then(|v| v.extract::<f32>().ok())
            .unwrap_or(0.0);
        let stamp_nanos = dict.get_item("stamp_nanos")?
            .and_then(|v| v.extract::<u64>().ok());

        Ok(Self::new(linear, angular, stamp_nanos))
    }
}

/// IMU (Inertial Measurement Unit) message
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImuMsg {
    #[pyo3(get, set)]
    pub stamp_nanos: u64,

    #[pyo3(get, set)]
    pub accel_x: f32,  // m/s^2

    #[pyo3(get, set)]
    pub accel_y: f32,  // m/s^2

    #[pyo3(get, set)]
    pub accel_z: f32,  // m/s^2

    #[pyo3(get, set)]
    pub gyro_x: f32,  // rad/s

    #[pyo3(get, set)]
    pub gyro_y: f32,  // rad/s

    #[pyo3(get, set)]
    pub gyro_z: f32,  // rad/s
}

#[pymethods]
impl ImuMsg {
    #[new]
    #[pyo3(signature = (accel_x=0.0, accel_y=0.0, accel_z=0.0, gyro_x=0.0, gyro_y=0.0, gyro_z=0.0))]
    pub fn new(
        accel_x: f32,
        accel_y: f32,
        accel_z: f32,
        gyro_x: f32,
        gyro_y: f32,
        gyro_z: f32,
    ) -> Self {
        let stamp_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            stamp_nanos,
            accel_x,
            accel_y,
            accel_z,
            gyro_x,
            gyro_y,
            gyro_z,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ImuMsg(accel=[{:.2}, {:.2}, {:.2}], gyro=[{:.2}, {:.2}, {:.2}])",
            self.accel_x, self.accel_y, self.accel_z, self.gyro_x, self.gyro_y, self.gyro_z
        )
    }
}

/// Register message types with Python module
pub fn register_messages(_py: Python, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<CmdVel>()?;
    module.add_class::<ImuMsg>()?;
    Ok(())
}
