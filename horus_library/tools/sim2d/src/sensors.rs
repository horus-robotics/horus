//! Advanced sensor models for sim2d
//!
//! Provides realistic sensor noise, GPS, ultrasonic, and contact sensors

use rand_distr::{Distribution, Normal};
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};

/// Noise model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseModel {
    /// Standard deviation for Gaussian noise
    pub std_dev: f32,
    /// Mean offset (bias)
    pub mean: f32,
    /// Enable/disable noise
    pub enabled: bool,
}

impl Default for NoiseModel {
    fn default() -> Self {
        Self {
            std_dev: 0.0,
            mean: 0.0,
            enabled: false,
        }
    }
}

impl NoiseModel {
    /// Apply noise to a value
    pub fn apply(&self, value: f32) -> f32 {
        if !self.enabled {
            return value;
        }

        // If std_dev is 0 or negative, only apply bias/mean
        if self.std_dev <= 0.0 {
            return value + self.mean;
        }

        let mut rng = rand::thread_rng();
        // Normal::new returns an error if std_dev is not finite or is negative
        // We already check for <= 0 above, so this should be safe, but handle error anyway
        let normal = match Normal::new(self.mean, self.std_dev) {
            Ok(n) => n,
            Err(_) => return value + self.mean, // Fall back to just applying mean
        };
        value + normal.sample(&mut rng)
    }

    /// Apply noise to a vector of values
    pub fn apply_vec(&self, values: &[f32]) -> Vec<f32> {
        values.iter().map(|&v| self.apply(v)).collect()
    }

    /// Create a noise model with given standard deviation
    pub fn with_std_dev(std_dev: f32) -> Self {
        Self {
            std_dev,
            mean: 0.0,
            enabled: std_dev > 0.0,
        }
    }

    /// Create a noise model with bias
    pub fn with_bias(std_dev: f32, mean: f32) -> Self {
        Self {
            std_dev,
            mean,
            enabled: std_dev > 0.0 || mean != 0.0,
        }
    }
}

/// GPS sensor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsConfig {
    pub enabled: bool,
    /// Update rate in Hz
    pub update_rate: f32,
    /// Position noise (meters)
    pub position_noise: NoiseModel,
    /// Altitude noise (meters) - always 0 for 2D
    pub altitude_noise: NoiseModel,
    /// Topic to publish GPS data
    pub topic: String,
}

impl Default for GpsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            update_rate: 1.0,                              // 1 Hz typical for GPS
            position_noise: NoiseModel::with_std_dev(2.0), // 2m typical GPS error
            altitude_noise: NoiseModel::default(),
            topic: "gps".to_string(),
        }
    }
}

/// GPS sensor data
#[derive(Debug, Clone)]
pub struct GpsData {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub accuracy: f32,
}

/// GPS sensor state
pub struct GpsSensor {
    config: GpsConfig,
    last_update: f64,
    // Origin for lat/lon conversion (arbitrary)
    origin_lat: f64,
    origin_lon: f64,
}

impl GpsSensor {
    pub fn new(config: GpsConfig) -> Self {
        Self {
            config,
            last_update: -1.0, // Start negative to allow first update immediately
            // Use arbitrary origin (e.g., San Francisco)
            origin_lat: 37.7749,
            origin_lon: -122.4194,
        }
    }

    /// Convert local XY coordinates to GPS lat/lon
    /// Very simplified: 1 meter ≈ 0.00001 degrees at equator
    pub fn xy_to_gps(&self, x: f32, y: f32) -> GpsData {
        let meters_to_degrees = 0.00001;

        let latitude = self.origin_lat + (y as f64 * meters_to_degrees);
        let longitude = self.origin_lon + (x as f64 * meters_to_degrees);

        // Apply noise
        let noisy_lat =
            latitude + (self.config.position_noise.apply(0.0) as f64 * meters_to_degrees);
        let noisy_lon =
            longitude + (self.config.position_noise.apply(0.0) as f64 * meters_to_degrees);

        GpsData {
            latitude: noisy_lat,
            longitude: noisy_lon,
            altitude: 0.0, // 2D simulation
            accuracy: self.config.position_noise.std_dev,
        }
    }

    pub fn update(&mut self, position: &Vector<f32>, time: f64) -> Option<GpsData> {
        if !self.config.enabled {
            return None;
        }

        let dt = time - self.last_update;
        if dt < 1.0 / self.config.update_rate as f64 {
            return None;
        }

        self.last_update = time;
        Some(self.xy_to_gps(position.x, position.y))
    }
}

/// Ultrasonic range sensor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UltrasonicConfig {
    pub enabled: bool,
    /// Sensor positions relative to robot center (x, y, angle)
    pub sensors: Vec<UltrasonicSensorConfig>,
    /// Topic prefix for publishing
    pub topic_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UltrasonicSensorConfig {
    /// Position relative to robot center
    pub offset: [f32; 2],
    /// Angle relative to robot heading (radians)
    pub angle: f32,
    /// Maximum range (meters)
    pub max_range: f32,
    /// Minimum range (meters)
    pub min_range: f32,
    /// Field of view (radians)
    pub fov: f32,
    /// Range noise
    pub noise: NoiseModel,
}

impl Default for UltrasonicConfig {
    fn default() -> Self {
        // Default: 4 sensors in cardinal directions
        Self {
            enabled: false,
            sensors: vec![
                UltrasonicSensorConfig {
                    offset: [0.3, 0.0],
                    angle: 0.0, // Front
                    max_range: 4.0,
                    min_range: 0.02,
                    fov: 0.5,                              // ~30 degrees
                    noise: NoiseModel::with_std_dev(0.02), // 2cm noise
                },
                UltrasonicSensorConfig {
                    offset: [0.0, 0.3],
                    angle: std::f32::consts::PI / 2.0, // Left
                    max_range: 4.0,
                    min_range: 0.02,
                    fov: 0.5,
                    noise: NoiseModel::with_std_dev(0.02),
                },
                UltrasonicSensorConfig {
                    offset: [-0.3, 0.0],
                    angle: std::f32::consts::PI, // Back
                    max_range: 4.0,
                    min_range: 0.02,
                    fov: 0.5,
                    noise: NoiseModel::with_std_dev(0.02),
                },
                UltrasonicSensorConfig {
                    offset: [0.0, -0.3],
                    angle: -std::f32::consts::PI / 2.0, // Right
                    max_range: 4.0,
                    min_range: 0.02,
                    fov: 0.5,
                    noise: NoiseModel::with_std_dev(0.02),
                },
            ],
            topic_prefix: "ultrasonic".to_string(),
        }
    }
}

/// Contact/bumper sensor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactConfig {
    pub enabled: bool,
    /// Number of contact zones around robot perimeter
    pub num_zones: usize,
    /// Contact detection threshold (force magnitude)
    pub threshold: f32,
    /// Topic for publishing contact events
    pub topic: String,
}

impl Default for ContactConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            num_zones: 8, // 8 zones around perimeter
            threshold: 0.1,
            topic: "contact".to_string(),
        }
    }
}

/// Contact sensor data
#[derive(Debug, Clone)]
pub struct ContactData {
    /// Which zones are in contact (bitmask)
    pub zones: Vec<bool>,
    /// Total contact force
    pub total_force: f32,
}

/// Sensor noise configuration for all sensors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorNoiseConfig {
    /// LIDAR range noise
    pub lidar_noise: NoiseModel,
    /// Odometry position noise (per meter traveled)
    pub odom_position_noise: NoiseModel,
    /// Odometry angular noise (per radian rotated)
    pub odom_angular_noise: NoiseModel,
    /// IMU acceleration noise
    pub imu_accel_noise: NoiseModel,
    /// IMU gyroscope noise
    pub imu_gyro_noise: NoiseModel,
}

impl Default for SensorNoiseConfig {
    fn default() -> Self {
        Self {
            // Realistic noise values for typical sensors
            lidar_noise: NoiseModel::with_std_dev(0.01), // 1cm LIDAR noise
            odom_position_noise: NoiseModel::with_std_dev(0.05), // 5cm per meter
            odom_angular_noise: NoiseModel::with_std_dev(0.01), // ~0.5 degree per radian
            imu_accel_noise: NoiseModel::with_std_dev(0.01), // 0.01 m/s²
            imu_gyro_noise: NoiseModel::with_std_dev(0.001), // 0.001 rad/s
        }
    }
}

impl SensorNoiseConfig {
    /// Create noise config with no noise (ideal sensors)
    pub fn ideal() -> Self {
        Self {
            lidar_noise: NoiseModel::default(),
            odom_position_noise: NoiseModel::default(),
            odom_angular_noise: NoiseModel::default(),
            imu_accel_noise: NoiseModel::default(),
            imu_gyro_noise: NoiseModel::default(),
        }
    }

    /// Create noise config with high noise (realistic low-cost sensors)
    pub fn high_noise() -> Self {
        Self {
            lidar_noise: NoiseModel::with_std_dev(0.05),        // 5cm
            odom_position_noise: NoiseModel::with_std_dev(0.1), // 10cm per meter
            odom_angular_noise: NoiseModel::with_std_dev(0.05), // ~3 degrees per radian
            imu_accel_noise: NoiseModel::with_std_dev(0.05),    // 0.05 m/s²
            imu_gyro_noise: NoiseModel::with_std_dev(0.01),     // 0.01 rad/s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_model_disabled() {
        let noise = NoiseModel::default();
        assert_eq!(noise.apply(1.0), 1.0);
        assert_eq!(noise.apply(5.0), 5.0);
    }

    #[test]
    fn test_noise_model_enabled() {
        let noise = NoiseModel::with_std_dev(0.1);
        let value = 1.0;
        let noisy = noise.apply(value);
        // Noisy value should be different (with high probability)
        // But we can't assert exact inequality due to randomness
        assert!((noisy - value).abs() < 1.0); // Sanity check
    }

    #[test]
    fn test_noise_model_bias() {
        let noise = NoiseModel::with_bias(0.0, 0.5);
        assert_eq!(noise.apply(1.0), 1.5);
        assert_eq!(noise.apply(2.0), 2.5);
    }

    #[test]
    fn test_noise_model_vec() {
        let noise = NoiseModel::with_bias(0.0, 1.0);
        let values = vec![1.0, 2.0, 3.0];
        let noisy = noise.apply_vec(&values);
        assert_eq!(noisy, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_gps_sensor_creation() {
        let config = GpsConfig::default();
        let gps = GpsSensor::new(config);
        assert_eq!(gps.origin_lat, 37.7749);
        assert_eq!(gps.origin_lon, -122.4194);
    }

    #[test]
    fn test_gps_xy_to_latlon() {
        let config = GpsConfig {
            enabled: true,
            position_noise: NoiseModel::default(), // No noise for testing
            ..Default::default()
        };
        let gps = GpsSensor::new(config);

        let gps_data = gps.xy_to_gps(0.0, 0.0);
        assert!((gps_data.latitude - 37.7749).abs() < 0.0001);
        assert!((gps_data.longitude - (-122.4194)).abs() < 0.0001);

        let gps_data = gps.xy_to_gps(1000.0, 1000.0);
        // 1000m = 0.01 degrees
        assert!((gps_data.latitude - 37.7849).abs() < 0.001);
        assert!((gps_data.longitude - (-122.4094)).abs() < 0.001);
    }

    #[test]
    fn test_gps_update_rate() {
        let config = GpsConfig {
            enabled: true,
            update_rate: 1.0, // 1 Hz
            position_noise: NoiseModel::default(),
            ..Default::default()
        };
        let mut gps = GpsSensor::new(config);

        let pos = vector![0.0, 0.0];

        // First update should return data
        let data = gps.update(&pos, 0.0);
        assert!(data.is_some());

        // Immediate second update should return None (too soon)
        let data = gps.update(&pos, 0.5);
        assert!(data.is_none());

        // After 1 second, should return data again
        let data = gps.update(&pos, 1.0);
        assert!(data.is_some());
    }

    #[test]
    fn test_ultrasonic_config_default() {
        let config = UltrasonicConfig::default();
        assert_eq!(config.sensors.len(), 4); // 4 cardinal directions
        assert_eq!(config.sensors[0].angle, 0.0); // Front
        assert!((config.sensors[1].angle - std::f32::consts::PI / 2.0).abs() < 0.001);
        // Left
    }

    #[test]
    fn test_contact_config() {
        let config = ContactConfig::default();
        assert_eq!(config.num_zones, 8);
        assert_eq!(config.threshold, 0.1);
    }

    #[test]
    fn test_sensor_noise_presets() {
        let ideal = SensorNoiseConfig::ideal();
        assert!(!ideal.lidar_noise.enabled);
        assert!(!ideal.odom_position_noise.enabled);

        let high_noise = SensorNoiseConfig::high_noise();
        assert!(high_noise.lidar_noise.enabled);
        assert_eq!(high_noise.lidar_noise.std_dev, 0.05);
    }
}
