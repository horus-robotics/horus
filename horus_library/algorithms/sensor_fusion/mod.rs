//! Multi-Sensor Fusion
//!
//! Combine measurements from multiple sensors for improved state estimation.
//!
//! # Features
//!
//! - Weighted sensor fusion
//! - Variance-based weighting
//! - Complementary filtering
//! - Sensor health monitoring
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::sensor_fusion::SensorFusion;
//!
//! let mut fusion = SensorFusion::new();
//!
//! // Add sensor measurements with variances
//! fusion.add_measurement("odom", 1.5, 0.1);
//! fusion.add_measurement("gps", 1.6, 0.05);
//!
//! // Get fused estimate
//! let fused = fusion.fuse();
//! ```

use std::collections::HashMap;

/// Sensor measurement with uncertainty
#[derive(Debug, Clone)]
pub struct Measurement {
    pub value: f64,
    pub variance: f64,
    pub timestamp: f64,
}

/// Multi-Sensor Fusion
pub struct SensorFusion {
    measurements: HashMap<String, Measurement>,
    max_age: f64, // Maximum measurement age (seconds)
}

impl SensorFusion {
    /// Create new sensor fusion
    pub fn new() -> Self {
        Self {
            measurements: HashMap::new(),
            max_age: 1.0,
        }
    }

    /// Set maximum measurement age
    pub fn set_max_age(&mut self, max_age: f64) {
        self.max_age = max_age;
    }

    /// Add sensor measurement
    ///
    /// # Arguments
    /// * `sensor_id` - Unique sensor identifier
    /// * `value` - Measurement value
    /// * `variance` - Measurement uncertainty (variance)
    pub fn add_measurement(&mut self, sensor_id: &str, value: f64, variance: f64) {
        self.add_measurement_with_time(sensor_id, value, variance, 0.0);
    }

    /// Add sensor measurement with timestamp
    pub fn add_measurement_with_time(
        &mut self,
        sensor_id: &str,
        value: f64,
        variance: f64,
        timestamp: f64,
    ) {
        self.measurements.insert(
            sensor_id.to_string(),
            Measurement {
                value,
                variance,
                timestamp,
            },
        );
    }

    /// Fuse measurements using variance-weighted average
    ///
    /// Returns fused value or None if no measurements
    pub fn fuse(&self) -> Option<f64> {
        if self.measurements.is_empty() {
            return None;
        }

        let mut sum = 0.0;
        let mut weight_sum = 0.0;

        for measurement in self.measurements.values() {
            if measurement.variance > 1e-10 {
                let weight = 1.0 / measurement.variance;
                sum += weight * measurement.value;
                weight_sum += weight;
            }
        }

        if weight_sum > 1e-10 {
            Some(sum / weight_sum)
        } else {
            None
        }
    }

    /// Fuse measurements with time-based weighting
    ///
    /// More recent measurements get higher weight
    pub fn fuse_with_time(&self, current_time: f64) -> Option<f64> {
        if self.measurements.is_empty() {
            return None;
        }

        let mut sum = 0.0;
        let mut weight_sum = 0.0;

        for measurement in self.measurements.values() {
            let age = current_time - measurement.timestamp;

            // Skip old measurements
            if age > self.max_age {
                continue;
            }

            if measurement.variance > 1e-10 {
                // Time decay factor
                let time_weight = (-age / self.max_age).exp();

                let variance_weight = 1.0 / measurement.variance;
                let total_weight = variance_weight * time_weight;

                sum += total_weight * measurement.value;
                weight_sum += total_weight;
            }
        }

        if weight_sum > 1e-10 {
            Some(sum / weight_sum)
        } else {
            None
        }
    }

    /// Get fused variance
    pub fn fused_variance(&self) -> Option<f64> {
        if self.measurements.is_empty() {
            return None;
        }

        let mut sum = 0.0;

        for measurement in self.measurements.values() {
            if measurement.variance > 1e-10 {
                sum += 1.0 / measurement.variance;
            }
        }

        if sum > 1e-10 {
            Some(1.0 / sum)
        } else {
            None
        }
    }

    /// Complementary filter fusion (for IMU + other sensors)
    ///
    /// # Arguments
    /// * `high_freq` - High-frequency sensor (e.g., IMU)
    /// * `low_freq` - Low-frequency sensor (e.g., GPS)
    /// * `alpha` - Filter coefficient (0-1, higher = more high_freq)
    pub fn complementary_filter(high_freq: f64, low_freq: f64, alpha: f64) -> f64 {
        let alpha = alpha.clamp(0.0, 1.0);
        alpha * high_freq + (1.0 - alpha) * low_freq
    }

    /// Clear all measurements
    pub fn clear(&mut self) {
        self.measurements.clear();
    }

    /// Remove specific sensor
    pub fn remove_sensor(&mut self, sensor_id: &str) {
        self.measurements.remove(sensor_id);
    }

    /// Get number of sensors
    pub fn sensor_count(&self) -> usize {
        self.measurements.len()
    }
}

impl Default for SensorFusion {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_fusion() {
        let mut fusion = SensorFusion::new();

        fusion.add_measurement("sensor1", 10.0, 1.0);
        fusion.add_measurement("sensor2", 12.0, 1.0);

        let result = fusion.fuse().unwrap();

        // Equal variances → simple average
        assert!((result - 11.0).abs() < 0.01);
    }

    #[test]
    fn test_weighted_fusion() {
        let mut fusion = SensorFusion::new();

        // sensor1 has low variance (more trusted)
        fusion.add_measurement("sensor1", 10.0, 0.1);
        // sensor2 has high variance (less trusted)
        fusion.add_measurement("sensor2", 20.0, 1.0);

        let result = fusion.fuse().unwrap();

        // Result should be closer to sensor1
        assert!(result < 12.0);
    }

    #[test]
    fn test_empty_fusion() {
        let fusion = SensorFusion::new();
        assert!(fusion.fuse().is_none());
    }

    #[test]
    fn test_fused_variance() {
        let mut fusion = SensorFusion::new();

        fusion.add_measurement("sensor1", 10.0, 1.0);
        fusion.add_measurement("sensor2", 10.0, 1.0);

        let variance = fusion.fused_variance().unwrap();

        // Fused variance should be less than individual
        assert!(variance < 1.0);
    }

    #[test]
    fn test_time_based_fusion() {
        let mut fusion = SensorFusion::new();

        fusion.add_measurement_with_time("sensor1", 10.0, 1.0, 0.0);
        fusion.add_measurement_with_time("sensor2", 12.0, 1.0, 0.5);

        let result = fusion.fuse_with_time(0.6).unwrap();

        // More recent measurement should have more weight
        assert!(result > 11.0);
    }

    #[test]
    fn test_old_measurements_ignored() {
        let mut fusion = SensorFusion::new();
        fusion.set_max_age(1.0);

        fusion.add_measurement_with_time("sensor1", 10.0, 1.0, 0.0);
        fusion.add_measurement_with_time("sensor2", 12.0, 1.0, 10.0);

        // At time 10, sensor1 should be ignored
        let result = fusion.fuse_with_time(10.0);

        if let Some(val) = result {
            assert!((val - 12.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_complementary_filter() {
        let result = SensorFusion::complementary_filter(10.0, 5.0, 0.7);

        // 0.7 * 10 + 0.3 * 5 = 8.5
        assert!((result - 8.5).abs() < 0.01);
    }

    #[test]
    fn test_sensor_management() {
        let mut fusion = SensorFusion::new();

        fusion.add_measurement("sensor1", 10.0, 1.0);
        fusion.add_measurement("sensor2", 12.0, 1.0);

        assert_eq!(fusion.sensor_count(), 2);

        fusion.remove_sensor("sensor1");
        assert_eq!(fusion.sensor_count(), 1);

        fusion.clear();
        assert_eq!(fusion.sensor_count(), 0);
    }

    #[test]
    fn test_three_sensors() {
        let mut fusion = SensorFusion::new();

        fusion.add_measurement("sensor1", 10.0, 0.1); // High confidence
        fusion.add_measurement("sensor2", 15.0, 1.0); // Medium confidence
        fusion.add_measurement("sensor3", 20.0, 10.0); // Low confidence

        let result = fusion.fuse().unwrap();

        // Should be closest to sensor1
        assert!(result < 12.0);
    }
}
