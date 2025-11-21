//! Sonar and ultrasonic sensor simulation

use bevy::prelude::*;
use rand::Rng;

/// Sonar sensor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum SonarType {
    /// Ultrasonic sensor (air, short range)
    Ultrasonic,
    /// Underwater sonar
    Underwater,
}

/// Sonar sensor component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SonarSensor {
    /// Sensor type
    pub sonar_type: SonarType,
    /// Maximum detection range (meters)
    pub max_range: f32,
    /// Minimum detection range (meters)
    pub min_range: f32,
    /// Cone angle (radians) - total beam width
    pub cone_angle: f32,
    /// Update rate (Hz)
    pub rate_hz: f32,
    /// Last update time
    pub last_update: f32,
    /// Measurement noise standard deviation (meters)
    pub noise_std: f32,
}

impl Default for SonarSensor {
    fn default() -> Self {
        Self {
            sonar_type: SonarType::Ultrasonic,
            max_range: 4.0,
            min_range: 0.02,
            cone_angle: std::f32::consts::PI / 6.0, // 30 degrees
            rate_hz: 20.0,
            last_update: 0.0,
            noise_std: 0.01,
        }
    }
}

impl SonarSensor {
    pub fn ultrasonic(max_range: f32) -> Self {
        Self {
            sonar_type: SonarType::Ultrasonic,
            max_range,
            ..default()
        }
    }

    pub fn underwater(max_range: f32) -> Self {
        Self {
            sonar_type: SonarType::Underwater,
            max_range,
            cone_angle: std::f32::consts::PI / 4.0, // Wider beam for underwater
            ..default()
        }
    }

    pub fn with_cone_angle(mut self, angle: f32) -> Self {
        self.cone_angle = angle;
        self
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rate_hz = rate_hz;
        self
    }

    pub fn with_noise(mut self, noise_std: f32) -> Self {
        self.noise_std = noise_std;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    /// Get speed of sound based on sensor type
    pub fn speed_of_sound(&self) -> f32 {
        match self.sonar_type {
            SonarType::Ultrasonic => 343.0,  // m/s in air at 20°C
            SonarType::Underwater => 1500.0, // m/s in water
        }
    }
}

/// Sonar measurement data
#[derive(Component, Clone)]
pub struct SonarMeasurement {
    /// Measured distance (meters), f32::MAX if no detection
    pub distance: f32,
    /// Confidence/signal strength (0.0-1.0)
    pub confidence: f32,
    /// Timestamp
    pub timestamp: f32,
    /// Multi-path reflections (additional detections)
    pub multipath: Vec<f32>,
}

impl SonarMeasurement {
    pub fn new() -> Self {
        Self {
            distance: f32::MAX,
            confidence: 0.0,
            timestamp: 0.0,
            multipath: Vec::new(),
        }
    }

    pub fn with_detection(distance: f32, confidence: f32, timestamp: f32) -> Self {
        Self {
            distance,
            confidence,
            timestamp,
            multipath: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.distance.is_finite() && self.confidence > 0.0
    }

    pub fn add_multipath_reflection(&mut self, distance: f32) {
        self.multipath.push(distance);
    }
}

impl Default for SonarMeasurement {
    fn default() -> Self {
        Self::new()
    }
}

/// System to update sonar sensors
pub fn sonar_sensor_update_system(
    time: Res<Time>,
    mut sonars: Query<(&mut SonarSensor, &mut SonarMeasurement, &GlobalTransform)>,
    obstacles: Query<&GlobalTransform, Without<SonarSensor>>,
) {
    let current_time = time.elapsed_secs();
    let mut rng = rand::thread_rng();

    for (mut sonar, mut measurement, sonar_transform) in sonars.iter_mut() {
        if !sonar.should_update(current_time) {
            continue;
        }

        sonar.last_update = current_time;
        measurement.timestamp = current_time;
        measurement.multipath.clear();

        let sonar_pos = sonar_transform.translation();
        let sonar_forward = sonar_transform.forward();

        let mut closest_distance = f32::MAX;
        let mut best_confidence = 0.0;
        let mut all_detections = Vec::new();

        // Check all obstacles
        for obstacle_transform in obstacles.iter() {
            let obstacle_pos = obstacle_transform.translation();
            let to_obstacle = obstacle_pos - sonar_pos;
            let distance = to_obstacle.length();

            // Check if within range
            if distance < sonar.min_range || distance > sonar.max_range {
                continue;
            }

            // Check if within cone
            let direction = to_obstacle.normalize();
            let angle = sonar_forward.dot(direction).acos();

            if angle <= sonar.cone_angle / 2.0 {
                // Calculate confidence based on angle and distance
                let angle_factor = 1.0 - (angle / (sonar.cone_angle / 2.0));
                let distance_factor = 1.0 - (distance / sonar.max_range);
                let confidence = (angle_factor * 0.7 + distance_factor * 0.3).max(0.1);

                all_detections.push((distance, confidence));

                if distance < closest_distance {
                    closest_distance = distance;
                    best_confidence = confidence;
                }
            }
        }

        if closest_distance < f32::MAX {
            // Add noise
            let noise: f32 = rng.gen_range(-sonar.noise_std..sonar.noise_std);
            let noisy_distance = (closest_distance + noise).max(sonar.min_range);

            measurement.distance = noisy_distance;
            measurement.confidence = best_confidence;

            // Add multipath reflections (simplified)
            for (dist, conf) in all_detections {
                if dist > closest_distance && conf > 0.3 {
                    let multipath_noise: f32 =
                        rng.gen_range(-sonar.noise_std * 2.0..sonar.noise_std * 2.0);
                    measurement.add_multipath_reflection(dist + multipath_noise);
                }
            }
        } else {
            // No detection
            measurement.distance = f32::MAX;
            measurement.confidence = 0.0;
        }
    }
}

/// Sonar array for multi-sensor configurations
#[derive(Component)]
pub struct SonarArray {
    /// Number of sensors in the array
    pub sensor_count: usize,
    /// Angular spacing between sensors (radians)
    pub angular_spacing: f32,
    /// Individual measurements
    pub measurements: Vec<SonarMeasurement>,
}

impl SonarArray {
    pub fn new(sensor_count: usize, angular_spacing: f32) -> Self {
        Self {
            sensor_count,
            angular_spacing,
            measurements: vec![SonarMeasurement::new(); sensor_count],
        }
    }

    /// Create a ring array (360-degree coverage)
    pub fn ring(sensor_count: usize) -> Self {
        let spacing = (2.0 * std::f32::consts::PI) / (sensor_count as f32);
        Self::new(sensor_count, spacing)
    }

    /// Get minimum distance across all sensors
    pub fn get_min_distance(&self) -> Option<f32> {
        self.measurements
            .iter()
            .filter(|m| m.is_valid())
            .map(|m| m.distance)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Get average confidence
    pub fn get_avg_confidence(&self) -> f32 {
        let valid_measurements: Vec<_> =
            self.measurements.iter().filter(|m| m.is_valid()).collect();

        if valid_measurements.is_empty() {
            return 0.0;
        }

        let sum: f32 = valid_measurements.iter().map(|m| m.confidence).sum();
        sum / (valid_measurements.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sonar_sensor_creation() {
        let sonar = SonarSensor::ultrasonic(4.0);
        assert_eq!(sonar.max_range, 4.0);
        assert_eq!(sonar.sonar_type, SonarType::Ultrasonic);
    }

    #[test]
    fn test_underwater_sonar() {
        let sonar = SonarSensor::underwater(100.0);
        assert_eq!(sonar.sonar_type, SonarType::Underwater);
        assert_eq!(sonar.speed_of_sound(), 1500.0);
    }

    #[test]
    fn test_sonar_measurement() {
        let measurement = SonarMeasurement::with_detection(2.5, 0.9, 1.0);
        assert!(measurement.is_valid());
        assert_eq!(measurement.distance, 2.5);
        assert_eq!(measurement.confidence, 0.9);
    }

    #[test]
    fn test_sonar_measurement_invalid() {
        let measurement = SonarMeasurement::new();
        assert!(!measurement.is_valid());
    }

    #[test]
    fn test_sonar_multipath() {
        let mut measurement = SonarMeasurement::with_detection(2.0, 0.8, 0.0);
        measurement.add_multipath_reflection(2.5);
        measurement.add_multipath_reflection(3.0);

        assert_eq!(measurement.multipath.len(), 2);
        assert_eq!(measurement.multipath[0], 2.5);
    }

    #[test]
    fn test_sonar_array() {
        let array = SonarArray::ring(8);
        assert_eq!(array.sensor_count, 8);
        assert!(array.angular_spacing > 0.0);
    }

    #[test]
    fn test_sonar_array_min_distance() {
        let mut array = SonarArray::new(3, 0.5);

        array.measurements[0] = SonarMeasurement::with_detection(1.0, 0.8, 0.0);
        array.measurements[1] = SonarMeasurement::with_detection(2.0, 0.7, 0.0);
        array.measurements[2] = SonarMeasurement::new(); // Invalid

        assert_eq!(array.get_min_distance(), Some(1.0));
    }

    #[test]
    fn test_speed_of_sound() {
        let ultrasonic = SonarSensor::ultrasonic(4.0);
        let underwater = SonarSensor::underwater(100.0);

        assert_eq!(ultrasonic.speed_of_sound(), 343.0);
        assert_eq!(underwater.speed_of_sound(), 1500.0);
    }
}
