use horus_library::messages::{cmd_vel, geometry, sensor};
use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

/// Python wrapper for Pose2D
#[pyclass(module = "horus.library._library", name = "Pose2D")]
#[derive(Clone)]
pub struct PyPose2D {
    inner: geometry::Pose2D,
}

#[pymethods]
impl PyPose2D {
    #[new]
    #[pyo3(signature = (x, y, theta))]
    fn new(x: f64, y: f64, theta: f64) -> Self {
        Self {
            inner: geometry::Pose2D::new(x, y, theta),
        }
    }

    /// Create pose at origin
    #[staticmethod]
    fn origin() -> Self {
        Self {
            inner: geometry::Pose2D::origin(),
        }
    }

    #[getter]
    fn x(&self) -> f64 {
        self.inner.x
    }

    #[setter]
    fn set_x(&mut self, value: f64) {
        self.inner.x = value;
    }

    #[getter]
    fn y(&self) -> f64 {
        self.inner.y
    }

    #[setter]
    fn set_y(&mut self, value: f64) {
        self.inner.y = value;
    }

    #[getter]
    fn theta(&self) -> f64 {
        self.inner.theta
    }

    #[setter]
    fn set_theta(&mut self, value: f64) {
        self.inner.theta = value;
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.timestamp
    }

    /// Calculate euclidean distance to another pose
    fn distance_to(&self, other: &PyPose2D) -> f64 {
        self.inner.distance_to(&other.inner)
    }

    /// Normalize theta to [-pi, pi]
    fn normalize_angle(&mut self) {
        self.inner.normalize_angle();
    }

    /// Check if values are finite
    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    fn __repr__(&self) -> String {
        format!(
            "Pose2D(x={:.3}, y={:.3}, theta={:.3})",
            self.inner.x, self.inner.y, self.inner.theta
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Pickle support: Provide constructor arguments
    fn __getnewargs__(&self) -> (f64, f64, f64) {
        (self.inner.x, self.inner.y, self.inner.theta)
    }
}

/// Python wrapper for Twist
#[pyclass(module = "horus.library._library", name = "Twist")]
#[derive(Clone)]
pub struct PyTwist {
    inner: geometry::Twist,
}

#[pymethods]
impl PyTwist {
    #[new]
    #[pyo3(signature = (linear, angular))]
    fn new(linear: [f64; 3], angular: [f64; 3]) -> Self {
        Self {
            inner: geometry::Twist::new(linear, angular),
        }
    }

    /// Create a 2D twist (forward velocity and rotation)
    #[staticmethod]
    fn new_2d(linear_x: f64, angular_z: f64) -> Self {
        Self {
            inner: geometry::Twist::new_2d(linear_x, angular_z),
        }
    }

    /// Stop command (all zeros)
    #[staticmethod]
    fn stop() -> Self {
        Self {
            inner: geometry::Twist::stop(),
        }
    }

    #[getter]
    fn linear(&self) -> [f64; 3] {
        self.inner.linear
    }

    #[setter]
    fn set_linear(&mut self, value: [f64; 3]) {
        self.inner.linear = value;
    }

    #[getter]
    fn angular(&self) -> [f64; 3] {
        self.inner.angular
    }

    #[setter]
    fn set_angular(&mut self, value: [f64; 3]) {
        self.inner.angular = value;
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.timestamp
    }

    /// Check if all values are finite
    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    fn __repr__(&self) -> String {
        format!(
            "Twist(linear=[{:.2}, {:.2}, {:.2}], angular=[{:.2}, {:.2}, {:.2}])",
            self.inner.linear[0],
            self.inner.linear[1],
            self.inner.linear[2],
            self.inner.angular[0],
            self.inner.angular[1],
            self.inner.angular[2]
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Pickle support: Provide constructor arguments
    fn __getnewargs__(&self) -> ([f64; 3], [f64; 3]) {
        (self.inner.linear, self.inner.angular)
    }
}

/// Python wrapper for Transform
#[pyclass(module = "horus.library._library", name = "Transform")]
#[derive(Clone)]
pub struct PyTransform {
    inner: geometry::Transform,
}

#[pymethods]
impl PyTransform {
    #[new]
    #[pyo3(signature = (translation, rotation))]
    fn new(translation: [f64; 3], rotation: [f64; 4]) -> Self {
        Self {
            inner: geometry::Transform::new(translation, rotation),
        }
    }

    /// Identity transform (no translation or rotation)
    #[staticmethod]
    fn identity() -> Self {
        Self {
            inner: geometry::Transform::identity(),
        }
    }

    /// Create from 2D pose
    #[staticmethod]
    fn from_pose_2d(pose: &PyPose2D) -> Self {
        Self {
            inner: geometry::Transform::from_pose_2d(&pose.inner),
        }
    }

    #[getter]
    fn translation(&self) -> [f64; 3] {
        self.inner.translation
    }

    #[setter]
    fn set_translation(&mut self, value: [f64; 3]) {
        self.inner.translation = value;
    }

    #[getter]
    fn rotation(&self) -> [f64; 4] {
        self.inner.rotation
    }

    #[setter]
    fn set_rotation(&mut self, value: [f64; 4]) {
        self.inner.rotation = value;
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.timestamp
    }

    /// Check if quaternion is normalized and values are finite
    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    /// Normalize the quaternion component
    fn normalize_rotation(&mut self) {
        self.inner.normalize_rotation();
    }

    fn __repr__(&self) -> String {
        format!(
            "Transform(translation=[{:.2}, {:.2}, {:.2}], rotation=[{:.2}, {:.2}, {:.2}, {:.2}])",
            self.inner.translation[0],
            self.inner.translation[1],
            self.inner.translation[2],
            self.inner.rotation[0],
            self.inner.rotation[1],
            self.inner.rotation[2],
            self.inner.rotation[3]
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Pickle support: Provide constructor arguments
    fn __getnewargs__(&self) -> ([f64; 3], [f64; 4]) {
        (self.inner.translation, self.inner.rotation)
    }
}

/// Python wrapper for Point3
#[pyclass(module = "horus.library._library", name = "Point3")]
#[derive(Clone)]
pub struct PyPoint3 {
    inner: geometry::Point3,
}

#[pymethods]
impl PyPoint3 {
    #[new]
    #[pyo3(signature = (x, y, z))]
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            inner: geometry::Point3::new(x, y, z),
        }
    }

    #[staticmethod]
    fn origin() -> Self {
        Self {
            inner: geometry::Point3::origin(),
        }
    }

    #[getter]
    fn x(&self) -> f64 {
        self.inner.x
    }

    #[setter]
    fn set_x(&mut self, value: f64) {
        self.inner.x = value;
    }

    #[getter]
    fn y(&self) -> f64 {
        self.inner.y
    }

    #[setter]
    fn set_y(&mut self, value: f64) {
        self.inner.y = value;
    }

    #[getter]
    fn z(&self) -> f64 {
        self.inner.z
    }

    #[setter]
    fn set_z(&mut self, value: f64) {
        self.inner.z = value;
    }

    fn distance_to(&self, other: &PyPoint3) -> f64 {
        self.inner.distance_to(&other.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "Point3(x={:.3}, y={:.3}, z={:.3})",
            self.inner.x, self.inner.y, self.inner.z
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Pickle support: Provide constructor arguments
    fn __getnewargs__(&self) -> (f64, f64, f64) {
        (self.inner.x, self.inner.y, self.inner.z)
    }
}

/// Python wrapper for Vector3
#[pyclass(module = "horus.library._library", name = "Vector3")]
#[derive(Clone)]
pub struct PyVector3 {
    inner: geometry::Vector3,
}

#[pymethods]
impl PyVector3 {
    #[new]
    #[pyo3(signature = (x, y, z))]
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            inner: geometry::Vector3::new(x, y, z),
        }
    }

    #[staticmethod]
    fn zero() -> Self {
        Self {
            inner: geometry::Vector3::zero(),
        }
    }

    #[getter]
    fn x(&self) -> f64 {
        self.inner.x
    }

    #[setter]
    fn set_x(&mut self, value: f64) {
        self.inner.x = value;
    }

    #[getter]
    fn y(&self) -> f64 {
        self.inner.y
    }

    #[setter]
    fn set_y(&mut self, value: f64) {
        self.inner.y = value;
    }

    #[getter]
    fn z(&self) -> f64 {
        self.inner.z
    }

    #[setter]
    fn set_z(&mut self, value: f64) {
        self.inner.z = value;
    }

    fn magnitude(&self) -> f64 {
        self.inner.magnitude()
    }

    fn normalize(&mut self) {
        self.inner.normalize();
    }

    fn dot(&self, other: &PyVector3) -> f64 {
        self.inner.dot(&other.inner)
    }

    fn cross(&self, other: &PyVector3) -> PyVector3 {
        PyVector3 {
            inner: self.inner.cross(&other.inner),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Vector3(x={:.3}, y={:.3}, z={:.3})",
            self.inner.x, self.inner.y, self.inner.z
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Pickle support: Provide constructor arguments
    fn __getnewargs__(&self) -> (f64, f64, f64) {
        (self.inner.x, self.inner.y, self.inner.z)
    }
}

/// Python wrapper for Quaternion
#[pyclass(module = "horus.library._library", name = "Quaternion")]
#[derive(Clone)]
pub struct PyQuaternion {
    inner: geometry::Quaternion,
}

#[pymethods]
impl PyQuaternion {
    #[new]
    #[pyo3(signature = (x, y, z, w))]
    fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self {
            inner: geometry::Quaternion::new(x, y, z, w),
        }
    }

    #[staticmethod]
    fn identity() -> Self {
        Self {
            inner: geometry::Quaternion::identity(),
        }
    }

    #[staticmethod]
    fn from_euler(roll: f64, pitch: f64, yaw: f64) -> Self {
        Self {
            inner: geometry::Quaternion::from_euler(roll, pitch, yaw),
        }
    }

    #[getter]
    fn x(&self) -> f64 {
        self.inner.x
    }

    #[setter]
    fn set_x(&mut self, value: f64) {
        self.inner.x = value;
    }

    #[getter]
    fn y(&self) -> f64 {
        self.inner.y
    }

    #[setter]
    fn set_y(&mut self, value: f64) {
        self.inner.y = value;
    }

    #[getter]
    fn z(&self) -> f64 {
        self.inner.z
    }

    #[setter]
    fn set_z(&mut self, value: f64) {
        self.inner.z = value;
    }

    #[getter]
    fn w(&self) -> f64 {
        self.inner.w
    }

    #[setter]
    fn set_w(&mut self, value: f64) {
        self.inner.w = value;
    }

    fn normalize(&mut self) {
        self.inner.normalize();
    }

    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    fn __repr__(&self) -> String {
        format!(
            "Quaternion(x={:.3}, y={:.3}, z={:.3}, w={:.3})",
            self.inner.x, self.inner.y, self.inner.z, self.inner.w
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Pickle support: Provide constructor arguments
    fn __getnewargs__(&self) -> (f64, f64, f64, f64) {
        (self.inner.x, self.inner.y, self.inner.z, self.inner.w)
    }
}

/// Python wrapper for CmdVel (2D velocity command)
#[pyclass(module = "horus.library._library", name = "CmdVel")]
#[derive(Clone)]
pub struct PyCmdVel {
    inner: cmd_vel::CmdVel,
}

#[pymethods]
impl PyCmdVel {
    #[new]
    #[pyo3(signature = (linear, angular))]
    fn new(linear: f32, angular: f32) -> Self {
        Self {
            inner: cmd_vel::CmdVel::new(linear, angular),
        }
    }

    /// Create a zero velocity command (stop)
    #[staticmethod]
    fn zero() -> Self {
        Self {
            inner: cmd_vel::CmdVel::zero(),
        }
    }

    #[getter]
    fn linear(&self) -> f32 {
        self.inner.linear
    }

    #[setter]
    fn set_linear(&mut self, value: f32) {
        self.inner.linear = value;
    }

    #[getter]
    fn angular(&self) -> f32 {
        self.inner.angular
    }

    #[setter]
    fn set_angular(&mut self, value: f32) {
        self.inner.angular = value;
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.stamp_nanos
    }

    fn __repr__(&self) -> String {
        format!(
            "CmdVel(linear={:.2}, angular={:.2})",
            self.inner.linear, self.inner.angular
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Pickle support: Provide constructor arguments
    fn __getnewargs__(&self) -> (f32, f32) {
        (self.inner.linear, self.inner.angular)
    }
}

/// Python wrapper for LaserScan (2D lidar data)
#[pyclass(module = "horus.library._library", name = "LaserScan")]
#[derive(Clone)]
pub struct PyLaserScan {
    inner: sensor::LaserScan,
}

#[pymethods]
impl PyLaserScan {
    #[new]
    fn new() -> Self {
        Self {
            inner: sensor::LaserScan::new(),
        }
    }

    /// Get ranges as NumPy array (zero-copy view)
    #[getter]
    fn ranges<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        PyArray1::from_slice_bound(py, &self.inner.ranges)
    }

    /// Set ranges from Python list or NumPy array
    #[setter]
    fn set_ranges(&mut self, _py: Python, value: &Bound<'_, PyAny>) -> PyResult<()> {
        // Try extracting as Vec<f32> - works for both NumPy arrays and lists
        if let Ok(vec) = value.extract::<Vec<f32>>() {
            if vec.len() == 360 {
                self.inner.ranges.copy_from_slice(&vec);
                return Ok(());
            } else {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Input must have exactly 360 elements, got {}",
                    vec.len()
                )));
            }
        }

        Err(pyo3::exceptions::PyTypeError::new_err(
            "ranges must be a NumPy array or Python list of floats",
        ))
    }

    #[getter]
    fn angle_min(&self) -> f32 {
        self.inner.angle_min
    }

    #[setter]
    fn set_angle_min(&mut self, value: f32) {
        self.inner.angle_min = value;
    }

    #[getter]
    fn angle_max(&self) -> f32 {
        self.inner.angle_max
    }

    #[setter]
    fn set_angle_max(&mut self, value: f32) {
        self.inner.angle_max = value;
    }

    #[getter]
    fn range_min(&self) -> f32 {
        self.inner.range_min
    }

    #[setter]
    fn set_range_min(&mut self, value: f32) {
        self.inner.range_min = value;
    }

    #[getter]
    fn range_max(&self) -> f32 {
        self.inner.range_max
    }

    #[setter]
    fn set_range_max(&mut self, value: f32) {
        self.inner.range_max = value;
    }

    #[getter]
    fn angle_increment(&self) -> f32 {
        self.inner.angle_increment
    }

    #[setter]
    fn set_angle_increment(&mut self, value: f32) {
        self.inner.angle_increment = value;
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.timestamp
    }

    /// Get the angle for a specific range index
    fn angle_at(&self, index: usize) -> f32 {
        self.inner.angle_at(index)
    }

    /// Check if a range reading is valid
    fn is_range_valid(&self, index: usize) -> bool {
        self.inner.is_range_valid(index)
    }

    /// Count valid range readings
    fn valid_count(&self) -> usize {
        self.inner.valid_count()
    }

    /// Get minimum valid range reading
    fn min_range(&self) -> Option<f32> {
        self.inner.min_range()
    }

    fn __repr__(&self) -> String {
        format!(
            "LaserScan(ranges={}, valid={}, min={:.2}m)",
            self.inner.ranges.len(),
            self.inner.valid_count(),
            self.inner.min_range().unwrap_or(0.0)
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __len__(&self) -> usize {
        360
    }

    /// Pickle support: Return empty args since LaserScan() has no constructor args,
    /// but we preserve state via __getstate__/__setstate__ pattern
    fn __getnewargs__<'py>(&self, py: Python<'py>) -> Bound<'py, PyTuple> {
        PyTuple::empty_bound(py)
    }

    /// Pickle support: Return the full state for unpickling
    fn __getstate__(&self, py: Python) -> PyResult<PyObject> {
        let state = PyDict::new_bound(py);
        state.set_item("ranges", self.inner.ranges.to_vec())?;
        state.set_item("angle_min", self.inner.angle_min)?;
        state.set_item("angle_max", self.inner.angle_max)?;
        state.set_item("range_min", self.inner.range_min)?;
        state.set_item("range_max", self.inner.range_max)?;
        state.set_item("angle_increment", self.inner.angle_increment)?;
        state.set_item("timestamp", self.inner.timestamp)?;
        Ok(state.into())
    }

    /// Pickle support: Restore the full state from unpickling
    fn __setstate__(&mut self, state: &Bound<'_, PyDict>) -> PyResult<()> {
        let ranges: Vec<f32> = state.get_item("ranges")?.unwrap().extract()?;
        let angle_min: f32 = state.get_item("angle_min")?.unwrap().extract()?;
        let angle_max: f32 = state.get_item("angle_max")?.unwrap().extract()?;
        let range_min: f32 = state.get_item("range_min")?.unwrap().extract()?;
        let range_max: f32 = state.get_item("range_max")?.unwrap().extract()?;
        let angle_increment: f32 = state.get_item("angle_increment")?.unwrap().extract()?;
        let timestamp: u64 = state.get_item("timestamp")?.unwrap().extract()?;

        self.inner.ranges.copy_from_slice(&ranges);
        self.inner.angle_min = angle_min;
        self.inner.angle_max = angle_max;
        self.inner.range_min = range_min;
        self.inner.range_max = range_max;
        self.inner.angle_increment = angle_increment;
        self.inner.timestamp = timestamp;

        Ok(())
    }
}

/// HORUS Library Python Module
#[pymodule]
fn _library(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Geometry messages
    m.add_class::<PyPose2D>()?;
    m.add_class::<PyTwist>()?;
    m.add_class::<PyTransform>()?;
    m.add_class::<PyPoint3>()?;
    m.add_class::<PyVector3>()?;
    m.add_class::<PyQuaternion>()?;

    // Control messages
    m.add_class::<PyCmdVel>()?;

    // Sensor messages
    m.add_class::<PyLaserScan>()?;

    Ok(())
}
