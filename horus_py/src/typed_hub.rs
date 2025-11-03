use horus::communication::Hub;
use horus_library::messages::{cmd_vel, geometry, sensor};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

/// Python wrapper for Hub<Pose2D> - enables cross-language communication
#[pyclass(module = "horus._horus")]
#[derive(Clone)]
pub struct PyPose2DHub {
    hub: Hub<geometry::Pose2D>,
}

#[pymethods]
impl PyPose2DHub {
    #[new]
    pub fn new(topic: String) -> PyResult<Self> {
        let hub = Hub::new(&topic)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create hub: {}", e)))?;

        Ok(PyPose2DHub { hub })
    }

    /// Send a Pose2D message (compatible with Rust Hub<Pose2D>)
    fn send(&self, py: Python, pose: PyObject) -> PyResult<bool> {
        // Extract x, y, theta from Python Pose2D object
        let x: f64 = pose.getattr(py, "x")?.extract(py)?;
        let y: f64 = pose.getattr(py, "y")?.extract(py)?;
        let theta: f64 = pose.getattr(py, "theta")?.extract(py)?;

        let rust_pose = geometry::Pose2D::new(x, y, theta);

        match self.hub.send(rust_pose, None) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Receive a Pose2D message (compatible with Rust Hub<Pose2D>)
    fn recv(&self, py: Python) -> PyResult<Option<PyObject>> {
        if let Some(rust_pose) = self.hub.recv(None) {
            // Import Pose2D class from horus.library
            let library_mod = py.import_bound("horus.library")?;
            let pose2d_class = library_mod.getattr("Pose2D")?;

            // Create Python Pose2D object
            let py_pose = pose2d_class.call1((rust_pose.x, rust_pose.y, rust_pose.theta))?;
            Ok(Some(py_pose.into()))
        } else {
            Ok(None)
        }
    }
}

/// Python wrapper for Hub<CmdVel> - enables cross-language communication
#[pyclass(module = "horus._horus")]
#[derive(Clone)]
pub struct PyCmdVelHub {
    hub: Hub<cmd_vel::CmdVel>,
}

#[pymethods]
impl PyCmdVelHub {
    #[new]
    pub fn new(topic: String) -> PyResult<Self> {
        let hub = Hub::new(&topic)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create hub: {}", e)))?;

        Ok(PyCmdVelHub { hub })
    }

    /// Send a CmdVel message (compatible with Rust Hub<CmdVel>)
    fn send(&self, py: Python, cmd: PyObject) -> PyResult<bool> {
        // Extract linear, angular from Python CmdVel object
        let linear: f32 = cmd.getattr(py, "linear")?.extract(py)?;
        let angular: f32 = cmd.getattr(py, "angular")?.extract(py)?;

        let rust_cmd = cmd_vel::CmdVel::new(linear, angular);

        match self.hub.send(rust_cmd, None) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Receive a CmdVel message (compatible with Rust Hub<CmdVel>)
    fn recv(&self, py: Python) -> PyResult<Option<PyObject>> {
        if let Some(rust_cmd) = self.hub.recv(None) {
            // Import CmdVel class from horus.library
            let library_mod = py.import_bound("horus.library")?;
            let cmdvel_class = library_mod.getattr("CmdVel")?;

            // Create Python CmdVel object
            let py_cmd = cmdvel_class.call1((rust_cmd.linear, rust_cmd.angular))?;
            Ok(Some(py_cmd.into()))
        } else {
            Ok(None)
        }
    }
}

/// Python wrapper for Hub<LaserScan> - enables cross-language communication
#[pyclass(module = "horus._horus")]
#[derive(Clone)]
pub struct PyLaserScanHub {
    hub: Hub<sensor::LaserScan>,
}

#[pymethods]
impl PyLaserScanHub {
    #[new]
    pub fn new(topic: String) -> PyResult<Self> {
        let hub = Hub::new(&topic)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create hub: {}", e)))?;

        Ok(PyLaserScanHub { hub })
    }

    /// Send a LaserScan message (compatible with Rust Hub<LaserScan>)
    fn send(&self, py: Python, scan: PyObject) -> PyResult<bool> {
        // Extract ranges array from Python LaserScan object
        let ranges_attr = scan.getattr(py, "ranges")?;
        let ranges_vec: Vec<f32> = ranges_attr.extract(py)?;

        // Create Rust LaserScan and copy ranges
        let mut rust_scan = sensor::LaserScan::new();
        if ranges_vec.len() == 360 {
            rust_scan.ranges.copy_from_slice(&ranges_vec);
        } else {
            return Err(PyRuntimeError::new_err(format!(
                "LaserScan ranges must have 360 elements, got {}",
                ranges_vec.len()
            )));
        }

        // Copy other attributes
        rust_scan.angle_min = scan.getattr(py, "angle_min")?.extract(py)?;
        rust_scan.angle_max = scan.getattr(py, "angle_max")?.extract(py)?;
        rust_scan.range_min = scan.getattr(py, "range_min")?.extract(py)?;
        rust_scan.range_max = scan.getattr(py, "range_max")?.extract(py)?;
        rust_scan.angle_increment = scan.getattr(py, "angle_increment")?.extract(py)?;

        match self.hub.send(rust_scan, None) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Receive a LaserScan message (compatible with Rust Hub<LaserScan>)
    fn recv(&self, py: Python) -> PyResult<Option<PyObject>> {
        if let Some(rust_scan) = self.hub.recv(None) {
            // Import LaserScan class from horus.library
            let library_mod = py.import_bound("horus.library")?;
            let laserscan_class = library_mod.getattr("LaserScan")?;

            // Create Python LaserScan object
            let py_scan = laserscan_class.call0()?;

            // Set ranges
            let ranges_list: Vec<f32> = rust_scan.ranges.to_vec();
            py_scan.setattr("ranges", ranges_list)?;

            // Set other attributes
            py_scan.setattr("angle_min", rust_scan.angle_min)?;
            py_scan.setattr("angle_max", rust_scan.angle_max)?;
            py_scan.setattr("range_min", rust_scan.range_min)?;
            py_scan.setattr("range_max", rust_scan.range_max)?;
            py_scan.setattr("angle_increment", rust_scan.angle_increment)?;

            Ok(Some(py_scan.into()))
        } else {
            Ok(None)
        }
    }
}

pub fn register_typed_hubs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPose2DHub>()?;
    m.add_class::<PyCmdVelHub>()?;
    m.add_class::<PyLaserScanHub>()?;
    Ok(())
}
