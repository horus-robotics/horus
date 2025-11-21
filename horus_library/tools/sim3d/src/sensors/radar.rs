//! Radar sensor simulation with point cloud and Doppler velocity

use bevy::prelude::*;
use rand::Rng;

/// Radar detection point
#[derive(Clone, Copy, Debug)]
pub struct RadarPoint {
    /// Range (distance in meters)
    pub range: f32,
    /// Azimuth angle (radians)
    pub azimuth: f32,
    /// Elevation angle (radians)
    pub elevation: f32,
    /// Radial velocity (m/s, positive = moving away)
    pub doppler_velocity: f32,
    /// Radar cross section (RCS) in dBsm
    pub rcs: f32,
}

impl RadarPoint {
    /// Convert to 3D Cartesian coordinates
    pub fn to_cartesian(&self) -> Vec3 {
        let x = self.range * self.elevation.cos() * self.azimuth.cos();
        let y = self.range * self.elevation.sin();
        let z = self.range * self.elevation.cos() * self.azimuth.sin();
        Vec3::new(x, y, z)
    }
}

/// Radar sensor component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct RadarSensor {
    /// Maximum detection range (meters)
    pub max_range: f32,
    /// Minimum detection range (meters)
    pub min_range: f32,
    /// Horizontal field of view (radians)
    pub horizontal_fov: f32,
    /// Vertical field of view (radians)
    pub vertical_fov: f32,
    /// Range resolution (meters)
    pub range_resolution: f32,
    /// Angular resolution (radians)
    pub angular_resolution: f32,
    /// Doppler velocity resolution (m/s)
    pub velocity_resolution: f32,
    /// Update rate (Hz)
    pub rate_hz: f32,
    /// Last update time
    pub last_update: f32,
    /// Probability of detection threshold
    pub detection_threshold: f32,
    /// False alarm rate
    pub false_alarm_rate: f32,
}

impl Default for RadarSensor {
    fn default() -> Self {
        Self {
            max_range: 100.0,
            min_range: 1.0,
            horizontal_fov: std::f32::consts::PI / 2.0, // 90 degrees
            vertical_fov: std::f32::consts::PI / 6.0,   // 30 degrees
            range_resolution: 0.5,
            angular_resolution: 0.05, // ~3 degrees
            velocity_resolution: 0.1,
            rate_hz: 10.0,
            last_update: 0.0,
            detection_threshold: 0.7,
            false_alarm_rate: 0.01,
        }
    }
}

impl RadarSensor {
    pub fn new(max_range: f32) -> Self {
        Self {
            max_range,
            ..default()
        }
    }

    pub fn with_fov(mut self, horizontal: f32, vertical: f32) -> Self {
        self.horizontal_fov = horizontal;
        self.vertical_fov = vertical;
        self
    }

    pub fn with_resolution(mut self, range_res: f32, angular_res: f32) -> Self {
        self.range_resolution = range_res;
        self.angular_resolution = angular_res;
        self
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rate_hz = rate_hz;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }
}

/// Radar point cloud data
#[derive(Component, Clone)]
pub struct RadarPointCloud {
    /// Detected points
    pub points: Vec<RadarPoint>,
    /// Timestamp
    pub timestamp: f32,
}

impl RadarPointCloud {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            timestamp: 0.0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            points: Vec::with_capacity(capacity),
            timestamp: 0.0,
        }
    }

    pub fn add_point(&mut self, point: RadarPoint) {
        self.points.push(point);
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Filter points by range
    pub fn filter_by_range(&self, min_range: f32, max_range: f32) -> Vec<RadarPoint> {
        self.points
            .iter()
            .filter(|p| p.range >= min_range && p.range <= max_range)
            .copied()
            .collect()
    }

    /// Filter points by velocity (for moving target indication)
    pub fn filter_by_velocity(&self, min_velocity: f32) -> Vec<RadarPoint> {
        self.points
            .iter()
            .filter(|p| p.doppler_velocity.abs() >= min_velocity)
            .copied()
            .collect()
    }

    /// Get points as 3D Cartesian coordinates
    pub fn to_cartesian(&self) -> Vec<Vec3> {
        self.points.iter().map(|p| p.to_cartesian()).collect()
    }
}

impl Default for RadarPointCloud {
    fn default() -> Self {
        Self::new()
    }
}

/// System to update radar sensors
pub fn radar_sensor_update_system(
    time: Res<Time>,
    mut radars: Query<(&mut RadarSensor, &mut RadarPointCloud, &GlobalTransform)>,
    targets: Query<(&GlobalTransform, Option<&Velocity>), Without<RadarSensor>>,
) {
    let current_time = time.elapsed_secs();
    let mut rng = rand::thread_rng();

    for (mut radar, mut point_cloud, radar_transform) in radars.iter_mut() {
        if !radar.should_update(current_time) {
            continue;
        }

        radar.last_update = current_time;
        point_cloud.clear();
        point_cloud.timestamp = current_time;

        let radar_pos = radar_transform.translation();
        let radar_rot = radar_transform.to_scale_rotation_translation().1;

        // Scan targets
        for (target_transform, velocity_opt) in targets.iter() {
            let target_pos = target_transform.translation();
            let relative_pos = target_pos - radar_pos;

            // Transform to radar local frame
            let local_pos = radar_rot.inverse() * relative_pos;

            // Calculate spherical coordinates
            let range = local_pos.length();
            if range < radar.min_range || range > radar.max_range {
                continue;
            }

            let azimuth = local_pos.z.atan2(local_pos.x);
            let elevation = local_pos
                .y
                .atan2((local_pos.x.powi(2) + local_pos.z.powi(2)).sqrt());

            // Check if within FOV
            if azimuth.abs() > radar.horizontal_fov / 2.0
                || elevation.abs() > radar.vertical_fov / 2.0
            {
                continue;
            }

            // Calculate Doppler velocity
            let radial_velocity = if let Some(velocity) = velocity_opt {
                let velocity_vec =
                    Vec3::new(velocity.linear.x, velocity.linear.y, velocity.linear.z);
                let radial_dir = relative_pos.normalize();
                velocity_vec.dot(radial_dir)
            } else {
                0.0
            };

            // Add realistic noise and detection probability
            let detection_prob = calculate_detection_probability(range, radar.max_range);
            let random_val: f32 = rng.gen();

            if random_val < detection_prob {
                // Add measurement noise
                let range_noise = rng.gen_range(-radar.range_resolution..radar.range_resolution);
                let angle_noise =
                    rng.gen_range(-radar.angular_resolution..radar.angular_resolution);
                let velocity_noise =
                    rng.gen_range(-radar.velocity_resolution..radar.velocity_resolution);

                let point = RadarPoint {
                    range: (range + range_noise).max(0.0),
                    azimuth: azimuth + angle_noise,
                    elevation: elevation + angle_noise,
                    doppler_velocity: radial_velocity + velocity_noise,
                    rcs: calculate_rcs(range), // Simplified RCS model
                };

                point_cloud.add_point(point);
            }
        }

        // Add false alarms (clutter)
        let num_false_alarms = (radar.false_alarm_rate * 100.0) as usize;
        for _ in 0..num_false_alarms {
            let false_point = RadarPoint {
                range: rng.gen_range(radar.min_range..radar.max_range),
                azimuth: rng.gen_range(-radar.horizontal_fov / 2.0..radar.horizontal_fov / 2.0),
                elevation: rng.gen_range(-radar.vertical_fov / 2.0..radar.vertical_fov / 2.0),
                doppler_velocity: rng.gen_range(-5.0..5.0),
                rcs: rng.gen_range(-20.0..0.0),
            };
            point_cloud.add_point(false_point);
        }
    }
}

/// Velocity component for Doppler calculation
#[derive(Component)]
pub struct Velocity {
    pub linear: Vec3,
    pub angular: Vec3,
}

/// Calculate detection probability based on range
fn calculate_detection_probability(range: f32, max_range: f32) -> f32 {
    // Simple range-dependent detection model
    let normalized_range = range / max_range;
    (1.0 - normalized_range).max(0.1)
}

/// Calculate radar cross section (simplified model)
fn calculate_rcs(range: f32) -> f32 {
    // Simplified: RCS decreases with range
    // Real RCS depends on target geometry, material, frequency, etc.
    10.0 * (1.0 / (range + 1.0)).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radar_sensor_creation() {
        let radar = RadarSensor::new(100.0);
        assert_eq!(radar.max_range, 100.0);
        assert!(radar.min_range > 0.0);
    }

    #[test]
    fn test_radar_point_cartesian() {
        let point = RadarPoint {
            range: 10.0,
            azimuth: 0.0,
            elevation: 0.0,
            doppler_velocity: 0.0,
            rcs: 0.0,
        };

        let cartesian = point.to_cartesian();
        assert!((cartesian.x - 10.0).abs() < 0.01);
        assert!(cartesian.y.abs() < 0.01);
        assert!(cartesian.z.abs() < 0.01);
    }

    #[test]
    fn test_radar_point_cloud() {
        let mut cloud = RadarPointCloud::new();

        let point = RadarPoint {
            range: 50.0,
            azimuth: 0.1,
            elevation: 0.0,
            doppler_velocity: 5.0,
            rcs: 10.0,
        };

        cloud.add_point(point);
        assert_eq!(cloud.len(), 1);
        assert!(!cloud.is_empty());
    }

    #[test]
    fn test_filter_by_range() {
        let mut cloud = RadarPointCloud::new();

        cloud.add_point(RadarPoint {
            range: 10.0,
            azimuth: 0.0,
            elevation: 0.0,
            doppler_velocity: 0.0,
            rcs: 0.0,
        });

        cloud.add_point(RadarPoint {
            range: 50.0,
            azimuth: 0.0,
            elevation: 0.0,
            doppler_velocity: 0.0,
            rcs: 0.0,
        });

        let filtered = cloud.filter_by_range(20.0, 60.0);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].range, 50.0);
    }

    #[test]
    fn test_filter_by_velocity() {
        let mut cloud = RadarPointCloud::new();

        cloud.add_point(RadarPoint {
            range: 10.0,
            azimuth: 0.0,
            elevation: 0.0,
            doppler_velocity: 0.5,
            rcs: 0.0,
        });

        cloud.add_point(RadarPoint {
            range: 20.0,
            azimuth: 0.0,
            elevation: 0.0,
            doppler_velocity: 10.0,
            rcs: 0.0,
        });

        let moving = cloud.filter_by_velocity(5.0);
        assert_eq!(moving.len(), 1);
        assert_eq!(moving[0].doppler_velocity, 10.0);
    }
}
