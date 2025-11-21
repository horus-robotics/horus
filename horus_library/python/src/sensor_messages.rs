// Python wrappers for sensor messages
use horus_library::messages::sensor as sensor_msg;
use pyo3::prelude::*;

/// Python wrapper for LaserScan
#[pyclass(module = "horus.library._library", name = "LaserScan")]
#[derive(Clone)]
pub struct PyLaserScan {
    pub(crate) inner: sensor_msg::LaserScan,
}

#[pymethods]
impl PyLaserScan {
    #[new]
    fn new() -> Self {
        Self {
            inner: sensor_msg::LaserScan::new(),
        }
    }

    #[getter]
    fn ranges(&self) -> Vec<f32> {
        self.inner.ranges.to_vec()
    }

    #[setter]
    fn set_ranges(&mut self, ranges: Vec<f32>) {
        if ranges.len() <= 360 {
            for (i, &r) in ranges.iter().enumerate() {
                self.inner.ranges[i] = r;
            }
            // Fill remaining with 0.0
            for i in ranges.len()..360 {
                self.inner.ranges[i] = 0.0;
            }
        }
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
    fn time_increment(&self) -> f32 {
        self.inner.time_increment
    }

    #[setter]
    fn set_time_increment(&mut self, value: f32) {
        self.inner.time_increment = value;
    }

    #[getter]
    fn scan_time(&self) -> f32 {
        self.inner.scan_time
    }

    #[setter]
    fn set_scan_time(&mut self, value: f32) {
        self.inner.scan_time = value;
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.timestamp
    }

    #[setter]
    fn set_timestamp(&mut self, value: u64) {
        self.inner.timestamp = value;
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
            "LaserScan(valid_readings={}/{}, range=[{:.2}, {:.2}]m)",
            self.valid_count(),
            360,
            self.inner.range_min,
            self.inner.range_max
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Python wrapper for Imu
#[pyclass(module = "horus.library._library", name = "Imu")]
#[derive(Clone)]
pub struct PyImu {
    pub(crate) inner: sensor_msg::Imu,
}

#[pymethods]
impl PyImu {
    #[new]
    fn new() -> Self {
        Self {
            inner: sensor_msg::Imu::default(),
        }
    }

    #[getter]
    fn orientation(&self) -> [f64; 4] {
        self.inner.orientation
    }

    #[setter]
    fn set_orientation(&mut self, value: [f64; 4]) {
        self.inner.orientation = value;
    }

    #[getter]
    fn angular_velocity(&self) -> [f64; 3] {
        self.inner.angular_velocity
    }

    #[setter]
    fn set_angular_velocity(&mut self, value: [f64; 3]) {
        self.inner.angular_velocity = value;
    }

    #[getter]
    fn linear_acceleration(&self) -> [f64; 3] {
        self.inner.linear_acceleration
    }

    #[setter]
    fn set_linear_acceleration(&mut self, value: [f64; 3]) {
        self.inner.linear_acceleration = value;
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.timestamp
    }

    #[setter]
    fn set_timestamp(&mut self, value: u64) {
        self.inner.timestamp = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "Imu(orientation=[{:.2},{:.2},{:.2},{:.2}], angular_vel=[{:.2},{:.2},{:.2}])",
            self.inner.orientation[0],
            self.inner.orientation[1],
            self.inner.orientation[2],
            self.inner.orientation[3],
            self.inner.angular_velocity[0],
            self.inner.angular_velocity[1],
            self.inner.angular_velocity[2]
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Python wrapper for Odometry
#[pyclass(module = "horus.library._library", name = "Odometry")]
#[derive(Clone)]
pub struct PyOdometry {
    pub(crate) inner: sensor_msg::Odometry,
}

#[pymethods]
impl PyOdometry {
    #[new]
    fn new() -> Self {
        Self {
            inner: sensor_msg::Odometry::default(),
        }
    }

    /// Position as [x, y, 0.0] - 3D representation of 2D pose
    #[getter]
    fn position(&self) -> [f64; 3] {
        [self.inner.pose.x, self.inner.pose.y, 0.0]
    }

    #[setter]
    fn set_position(&mut self, value: [f64; 3]) {
        self.inner.pose.x = value[0];
        self.inner.pose.y = value[1];
        // value[2] (z) is ignored for 2D pose
    }

    /// 2D pose access: x position in meters
    #[getter]
    fn x(&self) -> f64 {
        self.inner.pose.x
    }

    #[setter]
    fn set_x(&mut self, value: f64) {
        self.inner.pose.x = value;
    }

    /// 2D pose access: y position in meters
    #[getter]
    fn y(&self) -> f64 {
        self.inner.pose.y
    }

    #[setter]
    fn set_y(&mut self, value: f64) {
        self.inner.pose.y = value;
    }

    /// 2D pose access: orientation angle in radians
    #[getter]
    fn theta(&self) -> f64 {
        self.inner.pose.theta
    }

    #[setter]
    fn set_theta(&mut self, value: f64) {
        self.inner.pose.theta = value;
    }

    /// Orientation as quaternion [x, y, z, w] - derived from theta
    #[getter]
    fn orientation(&self) -> [f64; 4] {
        // Convert 2D theta to quaternion (rotation around z-axis)
        let half_theta = self.inner.pose.theta / 2.0;
        [0.0, 0.0, half_theta.sin(), half_theta.cos()]
    }

    #[setter]
    fn set_orientation(&mut self, value: [f64; 4]) {
        // Extract yaw from quaternion (assuming rotation around z-axis)
        // theta = 2 * atan2(z, w)
        self.inner.pose.theta = 2.0 * value[2].atan2(value[3]);
    }

    /// Linear velocity [vx, vy, vz] in m/s
    #[getter]
    fn linear_velocity(&self) -> [f64; 3] {
        self.inner.twist.linear
    }

    #[setter]
    fn set_linear_velocity(&mut self, value: [f64; 3]) {
        self.inner.twist.linear = value;
    }

    /// Angular velocity [wx, wy, wz] in rad/s
    #[getter]
    fn angular_velocity(&self) -> [f64; 3] {
        self.inner.twist.angular
    }

    #[setter]
    fn set_angular_velocity(&mut self, value: [f64; 3]) {
        self.inner.twist.angular = value;
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.timestamp
    }

    #[setter]
    fn set_timestamp(&mut self, value: u64) {
        self.inner.timestamp = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "Odometry(pos=[{:.2},{:.2},{:.2}], vel=[{:.2},{:.2},{:.2}])",
            self.inner.pose.x,
            self.inner.pose.y,
            self.inner.pose.theta,
            self.inner.twist.linear[0],
            self.inner.twist.linear[1],
            self.inner.twist.linear[2]
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}
