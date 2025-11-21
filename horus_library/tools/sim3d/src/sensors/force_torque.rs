use crate::physics::ContactForce;
use bevy::prelude::*;
use rand::Rng;

/// Force/Torque sensor (6-axis load cell)
#[derive(Component, Clone)]
pub struct ForceTorqueSensor {
    pub rate_hz: f32,
    pub last_update: f32,

    // Measurement ranges (N for force, Nm for torque)
    pub max_force: Vec3,
    pub max_torque: Vec3,

    // Noise parameters
    pub force_noise_std: Vec3,
    pub torque_noise_std: Vec3,

    // Bias/offset
    pub force_bias: Vec3,
    pub torque_bias: Vec3,

    // Temperature drift simulation
    pub temp_drift_coefficient: f32,
    pub current_temperature: f32,

    // Overload protection
    pub enable_overload_protection: bool,
    pub overloaded: bool,
}

impl Default for ForceTorqueSensor {
    fn default() -> Self {
        Self {
            rate_hz: 1000.0, // High rate for force control
            last_update: 0.0,
            max_force: Vec3::new(100.0, 100.0, 200.0), // Typical 6-axis sensor
            max_torque: Vec3::new(10.0, 10.0, 10.0),
            force_noise_std: Vec3::new(0.1, 0.1, 0.1),
            torque_noise_std: Vec3::new(0.01, 0.01, 0.01),
            force_bias: Vec3::ZERO,
            torque_bias: Vec3::ZERO,
            temp_drift_coefficient: 0.001, // 0.1% per degree C
            current_temperature: 25.0,
            enable_overload_protection: true,
            overloaded: false,
        }
    }
}

impl ForceTorqueSensor {
    pub fn new(rate_hz: f32) -> Self {
        Self {
            rate_hz,
            ..default()
        }
    }

    /// High-capacity sensor (industrial)
    pub fn high_capacity() -> Self {
        Self {
            max_force: Vec3::new(500.0, 500.0, 1000.0),
            max_torque: Vec3::new(50.0, 50.0, 50.0),
            force_noise_std: Vec3::new(0.5, 0.5, 0.5),
            torque_noise_std: Vec3::new(0.05, 0.05, 0.05),
            ..default()
        }
    }

    /// Precision sensor (research, assembly)
    pub fn precision() -> Self {
        Self {
            max_force: Vec3::new(50.0, 50.0, 100.0),
            max_torque: Vec3::new(5.0, 5.0, 5.0),
            force_noise_std: Vec3::new(0.01, 0.01, 0.01),
            torque_noise_std: Vec3::new(0.001, 0.001, 0.001),
            temp_drift_coefficient: 0.0005,
            ..default()
        }
    }

    /// Tactile sensor (small forces)
    pub fn tactile() -> Self {
        Self {
            max_force: Vec3::new(10.0, 10.0, 20.0),
            max_torque: Vec3::new(1.0, 1.0, 1.0),
            force_noise_std: Vec3::new(0.01, 0.01, 0.01),
            torque_noise_std: Vec3::new(0.001, 0.001, 0.001),
            rate_hz: 2000.0, // Higher rate for tactile feedback
            ..default()
        }
    }

    pub fn with_noise(mut self, force_noise: Vec3, torque_noise: Vec3) -> Self {
        self.force_noise_std = force_noise;
        self.torque_noise_std = torque_noise;
        self
    }

    pub fn with_bias(mut self, force_bias: Vec3, torque_bias: Vec3) -> Self {
        self.force_bias = force_bias;
        self.torque_bias = torque_bias;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    pub fn update_time(&mut self, current_time: f32) {
        self.last_update = current_time;
    }

    /// Simulate temperature drift
    pub fn update_temperature(&mut self, dt: f32) {
        let mut rng = rand::thread_rng();

        // Slowly varying temperature (room temp variations)
        self.current_temperature += rng.gen_range(-0.1..0.1) * dt;
        self.current_temperature = self.current_temperature.clamp(20.0, 30.0);

        // Update bias based on temperature
        let temp_delta = self.current_temperature - 25.0;
        let drift_factor = temp_delta * self.temp_drift_coefficient;

        // Apply proportional drift to bias
        let base_force_bias = Vec3::new(0.1, 0.1, 0.2);
        let base_torque_bias = Vec3::new(0.01, 0.01, 0.01);

        self.force_bias = base_force_bias * drift_factor;
        self.torque_bias = base_torque_bias * drift_factor;
    }

    /// Check for overload condition
    pub fn check_overload(&mut self, force: Vec3, torque: Vec3) -> bool {
        if !self.enable_overload_protection {
            return false;
        }

        let force_overload = force.x.abs() > self.max_force.x
            || force.y.abs() > self.max_force.y
            || force.z.abs() > self.max_force.z;

        let torque_overload = torque.x.abs() > self.max_torque.x
            || torque.y.abs() > self.max_torque.y
            || torque.z.abs() > self.max_torque.z;

        self.overloaded = force_overload || torque_overload;
        self.overloaded
    }

    /// Saturate measurements to max range
    pub fn saturate(&self, force: Vec3, torque: Vec3) -> (Vec3, Vec3) {
        let saturated_force = Vec3::new(
            force.x.clamp(-self.max_force.x, self.max_force.x),
            force.y.clamp(-self.max_force.y, self.max_force.y),
            force.z.clamp(-self.max_force.z, self.max_force.z),
        );

        let saturated_torque = Vec3::new(
            torque.x.clamp(-self.max_torque.x, self.max_torque.x),
            torque.y.clamp(-self.max_torque.y, self.max_torque.y),
            torque.z.clamp(-self.max_torque.z, self.max_torque.z),
        );

        (saturated_force, saturated_torque)
    }
}

/// Force/Torque data output
#[derive(Component, Clone, Debug, Default)]
pub struct ForceTorqueData {
    pub timestamp: f32,

    // Measured force (N) in sensor frame
    pub force: Vec3,

    // Measured torque (Nm) in sensor frame
    pub torque: Vec3,

    // Covariances
    pub force_covariance: Vec<f64>,
    pub torque_covariance: Vec<f64>,

    // Sensor status
    pub valid: bool,
    pub overloaded: bool,
}

impl ForceTorqueData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get wrench (force + torque) as 6D vector
    pub fn wrench(&self) -> [f32; 6] {
        [
            self.force.x,
            self.force.y,
            self.force.z,
            self.torque.x,
            self.torque.y,
            self.torque.z,
        ]
    }
}

/// System to update force/torque sensors
/// Reads actual contact forces from physics engine
pub fn force_torque_update_system(
    time: Res<Time>,
    mut query: Query<(
        &mut ForceTorqueSensor,
        &mut ForceTorqueData,
        &GlobalTransform,
        Option<&ContactForce>,
    )>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    for (mut sensor, mut ft_data, transform, contact_force) in query.iter_mut() {
        if !sensor.should_update(current_time) {
            continue;
        }

        sensor.update_time(current_time);
        sensor.update_temperature(dt);

        // Get actual forces from physics contacts
        // Transform from world frame to sensor frame
        let (_, rotation, _) = transform.to_scale_rotation_translation();
        let rotation_inverse = rotation.inverse();

        let (true_force_world, true_torque_world) = if let Some(contact) = contact_force {
            // Use actual contact forces from physics
            (contact.force, contact.torque)
        } else {
            // No contact force component - sensor not attached to physics body
            // This is valid for sensors in kinematic or pure sensor setups
            (Vec3::ZERO, Vec3::ZERO)
        };

        // Transform forces to sensor frame
        let true_force = rotation_inverse * true_force_world;
        let true_torque = rotation_inverse * true_torque_world;

        // Add noise
        let mut rng = rand::thread_rng();

        let force_noise = Vec3::new(
            rng.gen_range(-sensor.force_noise_std.x..sensor.force_noise_std.x),
            rng.gen_range(-sensor.force_noise_std.y..sensor.force_noise_std.y),
            rng.gen_range(-sensor.force_noise_std.z..sensor.force_noise_std.z),
        );

        let torque_noise = Vec3::new(
            rng.gen_range(-sensor.torque_noise_std.x..sensor.torque_noise_std.x),
            rng.gen_range(-sensor.torque_noise_std.y..sensor.torque_noise_std.y),
            rng.gen_range(-sensor.torque_noise_std.z..sensor.torque_noise_std.z),
        );

        let measured_force = true_force + force_noise + sensor.force_bias;
        let measured_torque = true_torque + torque_noise + sensor.torque_bias;

        // Check for overload
        sensor.check_overload(measured_force, measured_torque);

        // Saturate to max range
        let (saturated_force, saturated_torque) = sensor.saturate(measured_force, measured_torque);

        // Update sensor data
        ft_data.force = saturated_force;
        ft_data.torque = saturated_torque;
        ft_data.timestamp = current_time;
        ft_data.valid = !sensor.overloaded;
        ft_data.overloaded = sensor.overloaded;

        // Set covariances
        ft_data.force_covariance = vec![
            (sensor.force_noise_std.x * sensor.force_noise_std.x) as f64,
            0.0,
            0.0,
            0.0,
            (sensor.force_noise_std.y * sensor.force_noise_std.y) as f64,
            0.0,
            0.0,
            0.0,
            (sensor.force_noise_std.z * sensor.force_noise_std.z) as f64,
        ];

        ft_data.torque_covariance = vec![
            (sensor.torque_noise_std.x * sensor.torque_noise_std.x) as f64,
            0.0,
            0.0,
            0.0,
            (sensor.torque_noise_std.y * sensor.torque_noise_std.y) as f64,
            0.0,
            0.0,
            0.0,
            (sensor.torque_noise_std.z * sensor.torque_noise_std.z) as f64,
        ];
    }
}

/// Visualize force/torque measurements
pub fn visualize_force_torque_system(
    mut gizmos: Gizmos,
    query: Query<(&ForceTorqueData, &GlobalTransform)>,
) {
    for (ft_data, transform) in query.iter() {
        if !ft_data.valid {
            continue;
        }

        let pos = transform.translation();
        let (_, rotation, _) = transform.to_scale_rotation_translation();

        // Draw force vector (scaled for visibility)
        let force_scale = 0.01; // 1cm per Newton
        let force_world = rotation * ft_data.force * force_scale;

        let force_color = if ft_data.overloaded {
            Color::srgb(1.0, 0.0, 0.0)
        } else {
            Color::srgb(0.0, 1.0, 1.0)
        };

        if force_world.length() > 0.001 {
            gizmos.arrow(pos, pos + force_world, force_color);
        }

        // Draw torque vector (perpendicular representation)
        let torque_scale = 0.1; // 10cm per Nm
        let torque_world = rotation * ft_data.torque * torque_scale;

        let torque_color = if ft_data.overloaded {
            Color::srgb(1.0, 0.5, 0.0)
        } else {
            Color::srgb(1.0, 1.0, 0.0)
        };

        if torque_world.length() > 0.001 {
            // Draw circular arc to represent torque (perpendicular to torque axis)
            if let Ok(dir) = bevy::math::Dir3::new(torque_world) {
                use bevy::math::Isometry3d;
                // Create rotation from default Z-up to torque direction
                let rotation = Quat::from_rotation_arc(Vec3::Z, *dir);
                let isometry = Isometry3d::new(pos, rotation);
                gizmos.circle(isometry, 0.1, torque_color);
            }
        }
    }
}

/// Calibration for force/torque sensor
#[derive(Component, Clone)]
pub struct ForceTorqueCalibration {
    pub force_offset: Vec3,
    pub torque_offset: Vec3,
    pub force_scale: Vec3,
    pub torque_scale: Vec3,
    pub cross_coupling: [[f32; 6]; 6], // 6x6 calibration matrix
}

impl Default for ForceTorqueCalibration {
    fn default() -> Self {
        Self {
            force_offset: Vec3::ZERO,
            torque_offset: Vec3::ZERO,
            force_scale: Vec3::ONE,
            torque_scale: Vec3::ONE,
            cross_coupling: [[0.0; 6]; 6], // Identity initially
        }
    }
}

impl ForceTorqueCalibration {
    pub fn new() -> Self {
        let mut calib = Self::default();

        // Initialize cross-coupling as identity
        for i in 0..6 {
            calib.cross_coupling[i][i] = 1.0;
        }

        calib
    }

    /// Apply calibration to raw measurements
    pub fn apply(&self, raw_force: Vec3, raw_torque: Vec3) -> (Vec3, Vec3) {
        // Apply offset correction
        let force = raw_force - self.force_offset;
        let torque = raw_torque - self.torque_offset;

        // Apply scaling
        let force = Vec3::new(
            force.x * self.force_scale.x,
            force.y * self.force_scale.y,
            force.z * self.force_scale.z,
        );

        let torque = Vec3::new(
            torque.x * self.torque_scale.x,
            torque.y * self.torque_scale.y,
            torque.z * self.torque_scale.z,
        );

        // Apply cross-coupling correction
        let wrench = [force.x, force.y, force.z, torque.x, torque.y, torque.z];
        let mut calibrated = [0.0; 6];

        for i in 0..6 {
            for j in 0..6 {
                calibrated[i] += self.cross_coupling[i][j] * wrench[j];
            }
        }

        let calibrated_force = Vec3::new(calibrated[0], calibrated[1], calibrated[2]);
        let calibrated_torque = Vec3::new(calibrated[3], calibrated[4], calibrated[5]);

        (calibrated_force, calibrated_torque)
    }

    /// Simple gravity compensation
    pub fn compensate_gravity(
        &self,
        force: Vec3,
        torque: Vec3,
        mass: f32,
        com_offset: Vec3,
    ) -> (Vec3, Vec3) {
        let gravity = Vec3::new(0.0, -9.81 * mass, 0.0);

        let compensated_force = force - gravity;
        let compensated_torque = torque - com_offset.cross(gravity);

        (compensated_force, compensated_torque)
    }
}

/// Filter for force/torque data
#[derive(Component, Clone)]
pub struct ForceTorqueFilter {
    pub force_buffer: Vec<Vec3>,
    pub torque_buffer: Vec<Vec3>,
    pub buffer_size: usize,
    pub filter_type: FilterType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterType {
    MovingAverage,
    LowPass { alpha: f32 },
    MedianFilter,
}

impl ForceTorqueFilter {
    pub fn new(buffer_size: usize, filter_type: FilterType) -> Self {
        Self {
            force_buffer: Vec::new(),
            torque_buffer: Vec::new(),
            buffer_size,
            filter_type,
        }
    }

    pub fn moving_average(window_size: usize) -> Self {
        Self::new(window_size, FilterType::MovingAverage)
    }

    pub fn low_pass(alpha: f32) -> Self {
        Self::new(2, FilterType::LowPass { alpha })
    }

    /// Add measurement and get filtered output
    pub fn filter(&mut self, force: Vec3, torque: Vec3) -> (Vec3, Vec3) {
        self.force_buffer.push(force);
        self.torque_buffer.push(torque);

        // Limit buffer size
        if self.force_buffer.len() > self.buffer_size {
            self.force_buffer.remove(0);
        }
        if self.torque_buffer.len() > self.buffer_size {
            self.torque_buffer.remove(0);
        }

        match self.filter_type {
            FilterType::MovingAverage => {
                let filtered_force = self.moving_average_vec3(&self.force_buffer);
                let filtered_torque = self.moving_average_vec3(&self.torque_buffer);
                (filtered_force, filtered_torque)
            }
            FilterType::LowPass { alpha } => {
                if self.force_buffer.len() < 2 {
                    (force, torque)
                } else {
                    let prev_force = self.force_buffer[self.force_buffer.len() - 2];
                    let prev_torque = self.torque_buffer[self.torque_buffer.len() - 2];

                    let filtered_force = prev_force * (1.0 - alpha) + force * alpha;
                    let filtered_torque = prev_torque * (1.0 - alpha) + torque * alpha;

                    (filtered_force, filtered_torque)
                }
            }
            FilterType::MedianFilter => {
                let filtered_force = self.median_vec3(&self.force_buffer);
                let filtered_torque = self.median_vec3(&self.torque_buffer);
                (filtered_force, filtered_torque)
            }
        }
    }

    fn moving_average_vec3(&self, buffer: &[Vec3]) -> Vec3 {
        if buffer.is_empty() {
            return Vec3::ZERO;
        }

        let sum: Vec3 = buffer.iter().sum();
        sum / buffer.len() as f32
    }

    fn median_vec3(&self, buffer: &[Vec3]) -> Vec3 {
        if buffer.is_empty() {
            return Vec3::ZERO;
        }

        let mut x_values: Vec<f32> = buffer.iter().map(|v| v.x).collect();
        let mut y_values: Vec<f32> = buffer.iter().map(|v| v.y).collect();
        let mut z_values: Vec<f32> = buffer.iter().map(|v| v.z).collect();

        x_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        y_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        z_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mid = buffer.len() / 2;
        Vec3::new(x_values[mid], y_values[mid], z_values[mid])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_torque_sensor() {
        let mut sensor = ForceTorqueSensor::default();

        let force = Vec3::new(50.0, 50.0, 100.0);
        let torque = Vec3::new(5.0, 5.0, 5.0);

        assert!(!sensor.check_overload(force, torque));

        let large_force = Vec3::new(200.0, 0.0, 0.0);
        assert!(sensor.check_overload(large_force, torque));
    }

    #[test]
    fn test_saturation() {
        let sensor = ForceTorqueSensor::default();

        let force = Vec3::new(150.0, 50.0, 250.0);
        let torque = Vec3::new(15.0, 5.0, 5.0);

        let (sat_force, sat_torque) = sensor.saturate(force, torque);

        assert_eq!(sat_force.x, 100.0); // Clamped to max
        assert_eq!(sat_force.y, 50.0); // Within range
        assert_eq!(sat_force.z, 200.0); // Clamped to max
        assert_eq!(sat_torque.x, 10.0); // Clamped
    }

    #[test]
    fn test_moving_average_filter() {
        let mut filter = ForceTorqueFilter::moving_average(3);

        let (f1, _) = filter.filter(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO);
        assert_eq!(f1.x, 1.0);

        let (f2, _) = filter.filter(Vec3::new(2.0, 0.0, 0.0), Vec3::ZERO);
        assert_eq!(f2.x, 1.5);

        let (f3, _) = filter.filter(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO);
        assert_eq!(f3.x, 2.0);
    }

    #[test]
    fn test_contact_force_integration() {
        use crate::physics::ContactForce;

        let mut contact = ContactForce::new();
        assert!(!contact.is_in_contact());

        // Add a contact force
        let force = Vec3::new(10.0, 0.0, 0.0);
        let point = Vec3::new(0.0, 1.0, 0.0);
        let normal = Vec3::X;
        let com = Vec3::ZERO;

        contact.add_contact(force, point, normal, com);

        assert!(contact.is_in_contact());
        assert_eq!(contact.contact_count, 1);
        assert_eq!(contact.force, force);

        // Torque should be (point - com) × force
        // (0, 1, 0) × (10, 0, 0) = (0, 0, -10)
        assert!((contact.torque.z + 10.0).abs() < 0.01);
    }

    #[test]
    fn test_contact_force_accumulation() {
        use crate::physics::ContactForce;

        let mut contact = ContactForce::new();

        // Add multiple contact forces
        contact.add_contact(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::X,
            Vec3::ZERO,
        );
        contact.add_contact(
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::X,
            Vec3::ZERO,
        );

        assert_eq!(contact.contact_count, 2);
        assert_eq!(contact.force.x, 8.0); // 5 + 3 = 8

        // Torques should partially cancel: -5 + 3 = -2
        assert!((contact.torque.z + 2.0).abs() < 0.01);
    }

    #[test]
    fn test_contact_force_reset() {
        use crate::physics::ContactForce;

        let mut contact = ContactForce::new();
        contact.add_contact(Vec3::X, Vec3::ZERO, Vec3::X, Vec3::ZERO);

        assert!(contact.is_in_contact());

        contact.reset();

        assert!(!contact.is_in_contact());
        assert_eq!(contact.contact_count, 0);
        assert_eq!(contact.force, Vec3::ZERO);
        assert_eq!(contact.torque, Vec3::ZERO);
    }

    #[test]
    fn test_force_sensor_with_contact() {
        use crate::physics::ContactForce;

        let mut sensor = ForceTorqueSensor::default();

        // Simulate contact force
        let mut contact = ContactForce::new();
        contact.add_contact(
            Vec3::new(0.0, 50.0, 0.0), // 50N upward (supporting weight)
            Vec3::ZERO,
            Vec3::Y,
            Vec3::ZERO,
        );

        // Sensor should read this force (plus noise/bias)
        assert_eq!(contact.force.y, 50.0);
        assert!(!sensor.check_overload(contact.force, contact.torque));
    }

    #[test]
    fn test_average_contact_point() {
        use crate::physics::ContactForce;

        let mut contact = ContactForce::new();
        contact.add_contact(Vec3::X, Vec3::new(1.0, 0.0, 0.0), Vec3::X, Vec3::ZERO);
        contact.add_contact(Vec3::X, Vec3::new(3.0, 0.0, 0.0), Vec3::X, Vec3::ZERO);

        let avg = contact.average_contact_point().unwrap();
        assert_eq!(avg, Vec3::new(2.0, 0.0, 0.0));
    }

    #[test]
    fn test_contact_force_magnitudes() {
        use crate::physics::ContactForce;

        let mut contact = ContactForce::new();
        contact.add_contact(Vec3::new(3.0, 4.0, 0.0), Vec3::ZERO, Vec3::X, Vec3::ZERO);

        assert_eq!(contact.force_magnitude(), 5.0); // sqrt(9 + 16) = 5
    }

    #[test]
    fn test_gravity_compensation() {
        let calib = ForceTorqueCalibration::new();

        // 1kg mass creates ~9.81N downward force
        let measured_force = Vec3::new(0.0, -9.81, 0.0);
        let measured_torque = Vec3::ZERO;

        let (compensated_force, _) = calib.compensate_gravity(
            measured_force,
            measured_torque,
            1.0, // 1kg
            Vec3::ZERO,
        );

        // After gravity compensation, force should be near zero
        assert!(compensated_force.length() < 0.1);
    }
}
