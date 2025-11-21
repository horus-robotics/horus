//! Sensor data generation throughput benchmarks

use bevy::prelude::*;

/// Sensor throughput test
pub struct SensorThroughputTest {
    pub name: String,
    pub sensor_type: SensorType,
    pub resolution: (u32, u32),
    pub update_rate_hz: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorType {
    Camera,
    DepthCamera,
    Lidar,
    Radar,
    Thermal,
    EventCamera,
}

impl SensorThroughputTest {
    pub fn camera_hd() -> Self {
        Self {
            name: "HD Camera (1920x1080)".to_string(),
            sensor_type: SensorType::Camera,
            resolution: (1920, 1080),
            update_rate_hz: 30.0,
        }
    }

    pub fn lidar_64() -> Self {
        Self {
            name: "64-beam Lidar".to_string(),
            sensor_type: SensorType::Lidar,
            resolution: (64, 1024),
            update_rate_hz: 10.0,
        }
    }

    pub fn depth_camera() -> Self {
        Self {
            name: "Depth Camera (640x480)".to_string(),
            sensor_type: SensorType::DepthCamera,
            resolution: (640, 480),
            update_rate_hz: 30.0,
        }
    }

    pub fn thermal_camera() -> Self {
        Self {
            name: "Thermal Camera (320x240)".to_string(),
            sensor_type: SensorType::Thermal,
            resolution: (320, 240),
            update_rate_hz: 60.0,
        }
    }

    /// Calculate expected data rate in MB/s
    pub fn expected_data_rate(&self) -> f32 {
        let pixels = self.resolution.0 * self.resolution.1;
        let bytes_per_pixel = match self.sensor_type {
            SensorType::Camera => 3,       // RGB
            SensorType::DepthCamera => 4,  // float32
            SensorType::Lidar => 12,       // Vec3f
            SensorType::Radar => 8,        // 2 floats
            SensorType::Thermal => 4,      // float32
            SensorType::EventCamera => 16, // event struct
        };

        let bytes_per_frame = pixels * bytes_per_pixel;
        let bytes_per_second = bytes_per_frame as f32 * self.update_rate_hz;
        bytes_per_second / (1024.0 * 1024.0) // Convert to MB/s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_throughput_camera() {
        let test = SensorThroughputTest::camera_hd();
        assert_eq!(test.sensor_type, SensorType::Camera);
        assert_eq!(test.resolution, (1920, 1080));
    }

    #[test]
    fn test_sensor_throughput_lidar() {
        let test = SensorThroughputTest::lidar_64();
        assert_eq!(test.sensor_type, SensorType::Lidar);
    }

    #[test]
    fn test_expected_data_rate() {
        let test = SensorThroughputTest::camera_hd();
        let rate = test.expected_data_rate();
        // HD camera at 30fps ~ 187 MB/s
        assert!(rate > 100.0 && rate < 300.0);
    }

    #[test]
    fn test_sensor_throughput_depth() {
        let test = SensorThroughputTest::depth_camera();
        assert_eq!(test.sensor_type, SensorType::DepthCamera);
    }

    #[test]
    fn test_sensor_throughput_thermal() {
        let test = SensorThroughputTest::thermal_camera();
        assert_eq!(test.sensor_type, SensorType::Thermal);
        assert_eq!(test.update_rate_hz, 60.0);
    }
}
