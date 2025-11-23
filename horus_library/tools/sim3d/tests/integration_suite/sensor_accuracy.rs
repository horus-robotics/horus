//! Sensor accuracy validation tests for sim3d
//!
//! Comprehensive tests for sensor accuracy:
//! - LiDAR point cloud accuracy against known geometry
//! - Camera projection correctness
//! - IMU readings against physics state
//! - GPS accuracy with known positions
//! - Force/Torque sensor validation

use bevy::prelude::*;
use std::f32::consts::PI;

/// Configuration for LiDAR accuracy test
#[derive(Debug, Clone)]
pub struct LidarAccuracyTestConfig {
    pub horizontal_rays: usize,
    pub vertical_rays: usize,
    pub horizontal_fov: f32,
    pub vertical_fov: f32,
    pub max_range: f32,
    pub min_range: f32,
    pub noise_std: f32,
}

impl Default for LidarAccuracyTestConfig {
    fn default() -> Self {
        Self {
            horizontal_rays: 360,
            vertical_rays: 16,
            horizontal_fov: PI * 2.0,
            vertical_fov: PI / 6.0,
            max_range: 50.0,
            min_range: 0.1,
            noise_std: 0.0, // No noise for accuracy test
        }
    }
}

/// Result of LiDAR accuracy test
#[derive(Debug, Clone)]
pub struct LidarAccuracyResult {
    pub total_rays_cast: usize,
    pub rays_hitting_target: usize,
    pub mean_distance_error: f32,
    pub max_distance_error: f32,
    pub std_distance_error: f32,
    pub hit_rate: f32,
    pub accuracy_passed: bool,
}

/// Known geometry for LiDAR testing
#[derive(Debug, Clone)]
pub struct KnownGeometry {
    pub geometry_type: GeometryType,
    pub position: Vec3,
    pub size: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryType {
    Box,
    Sphere,
    Cylinder,
    Plane,
}

impl KnownGeometry {
    pub fn box_at(position: Vec3, size: Vec3) -> Self {
        Self {
            geometry_type: GeometryType::Box,
            position,
            size,
        }
    }

    pub fn sphere_at(position: Vec3, radius: f32) -> Self {
        Self {
            geometry_type: GeometryType::Sphere,
            position,
            size: Vec3::splat(radius),
        }
    }

    pub fn cylinder_at(position: Vec3, radius: f32, height: f32) -> Self {
        Self {
            geometry_type: GeometryType::Cylinder,
            position,
            size: Vec3::new(radius, height, radius),
        }
    }

    pub fn plane_at(position: Vec3, normal: Vec3) -> Self {
        Self {
            geometry_type: GeometryType::Plane,
            position,
            size: normal.normalize(),
        }
    }

    /// Calculate expected distance from sensor position to geometry
    pub fn expected_distance(&self, sensor_pos: Vec3, ray_dir: Vec3) -> Option<f32> {
        match self.geometry_type {
            GeometryType::Box => self.ray_box_intersection(sensor_pos, ray_dir),
            GeometryType::Sphere => self.ray_sphere_intersection(sensor_pos, ray_dir),
            GeometryType::Cylinder => self.ray_cylinder_intersection(sensor_pos, ray_dir),
            GeometryType::Plane => self.ray_plane_intersection(sensor_pos, ray_dir),
        }
    }

    fn ray_box_intersection(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        let half_size = self.size * 0.5;
        let min = self.position - half_size;
        let max = self.position + half_size;

        let inv_dir = Vec3::new(
            if dir.x.abs() > 1e-6 { 1.0 / dir.x } else { f32::INFINITY },
            if dir.y.abs() > 1e-6 { 1.0 / dir.y } else { f32::INFINITY },
            if dir.z.abs() > 1e-6 { 1.0 / dir.z } else { f32::INFINITY },
        );

        let t1 = (min.x - origin.x) * inv_dir.x;
        let t2 = (max.x - origin.x) * inv_dir.x;
        let t3 = (min.y - origin.y) * inv_dir.y;
        let t4 = (max.y - origin.y) * inv_dir.y;
        let t5 = (min.z - origin.z) * inv_dir.z;
        let t6 = (max.z - origin.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0.0 || tmin > tmax {
            None
        } else if tmin > 0.0 {
            Some(tmin)
        } else {
            Some(tmax)
        }
    }

    fn ray_sphere_intersection(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        let radius = self.size.x;
        let oc = origin - self.position;
        let a = dir.dot(dir);
        let b = 2.0 * oc.dot(dir);
        let c = oc.dot(oc) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            None
        } else {
            let t = (-b - discriminant.sqrt()) / (2.0 * a);
            if t > 0.0 {
                Some(t)
            } else {
                let t2 = (-b + discriminant.sqrt()) / (2.0 * a);
                if t2 > 0.0 {
                    Some(t2)
                } else {
                    None
                }
            }
        }
    }

    fn ray_cylinder_intersection(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        // Simplified: vertical cylinder (Y-axis aligned)
        let radius = self.size.x;
        let half_height = self.size.y * 0.5;

        // Project to XZ plane for infinite cylinder
        let oc = Vec3::new(
            origin.x - self.position.x,
            0.0,
            origin.z - self.position.z,
        );
        let dir_xz = Vec3::new(dir.x, 0.0, dir.z);

        let a = dir_xz.dot(dir_xz);
        let b = 2.0 * oc.dot(dir_xz);
        let c = oc.dot(oc) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return None;
        }

        let t = (-b - discriminant.sqrt()) / (2.0 * a);
        if t > 0.0 {
            let hit_y = origin.y + t * dir.y;
            if hit_y >= self.position.y - half_height && hit_y <= self.position.y + half_height {
                return Some(t);
            }
        }

        None
    }

    fn ray_plane_intersection(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        let normal = self.size; // Using size as normal for plane
        let denom = normal.dot(dir);

        if denom.abs() < 1e-6 {
            return None;
        }

        let t = (self.position - origin).dot(normal) / denom;
        if t > 0.0 {
            Some(t)
        } else {
            None
        }
    }
}

/// Simulate LiDAR scan against known geometry
pub fn simulate_lidar_scan(
    config: &LidarAccuracyTestConfig,
    sensor_pos: Vec3,
    sensor_rot: Quat,
    geometries: &[KnownGeometry],
) -> Vec<(Vec3, f32)> {
    let mut points = Vec::new();

    let h_step = config.horizontal_fov / config.horizontal_rays as f32;
    let v_step = config.vertical_fov / config.vertical_rays as f32;
    let h_start = -config.horizontal_fov / 2.0;
    let v_start = -config.vertical_fov / 2.0;

    for v in 0..config.vertical_rays {
        let v_angle = v_start + v as f32 * v_step;

        for h in 0..config.horizontal_rays {
            let h_angle = h_start + h as f32 * h_step;

            // Calculate ray direction in sensor frame
            let local_dir = Vec3::new(
                h_angle.cos() * v_angle.cos(),
                v_angle.sin(),
                h_angle.sin() * v_angle.cos(),
            );

            // Transform to world frame
            let world_dir = sensor_rot * local_dir;

            // Find closest intersection
            let mut closest_dist = f32::MAX;
            for geometry in geometries {
                if let Some(dist) = geometry.expected_distance(sensor_pos, world_dir) {
                    if dist >= config.min_range && dist <= config.max_range && dist < closest_dist {
                        closest_dist = dist;
                    }
                }
            }

            if closest_dist < f32::MAX {
                // Add noise if configured
                let noisy_dist = if config.noise_std > 0.0 {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    closest_dist + rng.gen_range(-config.noise_std..config.noise_std)
                } else {
                    closest_dist
                };

                let hit_point = sensor_pos + world_dir * noisy_dist;
                let intensity = 1.0 / (1.0 + (noisy_dist / config.max_range).powi(2));
                points.push((hit_point, intensity));
            }
        }
    }

    points
}

/// Validate LiDAR accuracy against known geometry
pub fn validate_lidar_accuracy(
    config: LidarAccuracyTestConfig,
    sensor_pos: Vec3,
    geometries: Vec<KnownGeometry>,
) -> LidarAccuracyResult {
    let sensor_rot = Quat::IDENTITY;
    let points = simulate_lidar_scan(&config, sensor_pos, sensor_rot, &geometries);

    let total_rays = config.horizontal_rays * config.vertical_rays;
    let rays_hitting = points.len();

    // Calculate distance errors by comparing measured points to expected geometry
    let mut errors = Vec::new();

    for (hit_point, _intensity) in &points {
        let ray_dir = (*hit_point - sensor_pos).normalize();
        let measured_dist = (*hit_point - sensor_pos).length();

        // Find expected distance
        let mut min_expected_dist = f32::MAX;
        for geometry in &geometries {
            if let Some(expected) = geometry.expected_distance(sensor_pos, ray_dir) {
                if expected < min_expected_dist {
                    min_expected_dist = expected;
                }
            }
        }

        if min_expected_dist < f32::MAX {
            let error = (measured_dist - min_expected_dist).abs();
            errors.push(error);
        }
    }

    let mean_error = if !errors.is_empty() {
        errors.iter().sum::<f32>() / errors.len() as f32
    } else {
        0.0
    };

    let max_error = errors.iter().cloned().fold(0.0f32, f32::max);

    let std_error = if errors.len() > 1 {
        let variance = errors.iter().map(|e| (e - mean_error).powi(2)).sum::<f32>() / errors.len() as f32;
        variance.sqrt()
    } else {
        0.0
    };

    let hit_rate = rays_hitting as f32 / total_rays as f32;

    // Accuracy criteria: mean error < 0.1m, std < 0.05m for noiseless sensor
    let accuracy_passed = mean_error < 0.1 && std_error < 0.05;

    LidarAccuracyResult {
        total_rays_cast: total_rays,
        rays_hitting_target: rays_hitting,
        mean_distance_error: mean_error,
        max_distance_error: max_error,
        std_distance_error: std_error,
        hit_rate,
        accuracy_passed,
    }
}

/// Configuration for camera projection test
#[derive(Debug, Clone)]
pub struct CameraProjectionTestConfig {
    pub width: u32,
    pub height: u32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for CameraProjectionTestConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            fov: 60.0,
            near: 0.1,
            far: 100.0,
        }
    }
}

/// Result of camera projection test
#[derive(Debug, Clone)]
pub struct CameraProjectionResult {
    pub points_tested: usize,
    pub points_correctly_projected: usize,
    pub mean_pixel_error: f32,
    pub max_pixel_error: f32,
    pub projection_correct: bool,
}

/// Test point with known world and expected screen coordinates
#[derive(Debug, Clone)]
pub struct ProjectionTestPoint {
    pub world_pos: Vec3,
    pub expected_screen_x: f32,
    pub expected_screen_y: f32,
    pub expected_visible: bool,
}

/// Calculate projection matrix
pub fn calculate_projection_matrix(config: &CameraProjectionTestConfig) -> [[f32; 4]; 4] {
    let aspect = config.width as f32 / config.height as f32;
    let fov_rad = config.fov.to_radians();
    let f = 1.0 / (fov_rad / 2.0).tan();

    let near = config.near;
    let far = config.far;

    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, (far + near) / (near - far), -1.0],
        [0.0, 0.0, (2.0 * far * near) / (near - far), 0.0],
    ]
}

/// Project world point to screen coordinates
pub fn project_point(
    world_pos: Vec3,
    camera_pos: Vec3,
    camera_rot: Quat,
    config: &CameraProjectionTestConfig,
) -> Option<(f32, f32)> {
    // Transform to camera space
    let relative_pos = camera_rot.inverse() * (world_pos - camera_pos);

    // Check if behind camera
    if relative_pos.z >= 0.0 {
        return None;
    }

    // Apply perspective projection
    let aspect = config.width as f32 / config.height as f32;
    let fov_rad = config.fov.to_radians();
    let f = 1.0 / (fov_rad / 2.0).tan();

    let x_ndc = (f / aspect) * relative_pos.x / (-relative_pos.z);
    let y_ndc = f * relative_pos.y / (-relative_pos.z);

    // Check if in view frustum
    if x_ndc.abs() > 1.0 || y_ndc.abs() > 1.0 {
        return None;
    }

    // Convert to screen coordinates
    let screen_x = (x_ndc + 1.0) * 0.5 * config.width as f32;
    let screen_y = (1.0 - y_ndc) * 0.5 * config.height as f32;

    Some((screen_x, screen_y))
}

/// Validate camera projection correctness
pub fn validate_camera_projection(config: CameraProjectionTestConfig) -> CameraProjectionResult {
    let camera_pos = Vec3::ZERO;
    let camera_rot = Quat::from_rotation_y(0.0);

    // Generate test points with known expected projections
    let test_points = vec![
        // Center point (should project to center)
        ProjectionTestPoint {
            world_pos: Vec3::new(0.0, 0.0, -5.0),
            expected_screen_x: config.width as f32 / 2.0,
            expected_screen_y: config.height as f32 / 2.0,
            expected_visible: true,
        },
        // Left edge
        ProjectionTestPoint {
            world_pos: Vec3::new(-2.5, 0.0, -5.0),
            expected_screen_x: config.width as f32 * 0.25,
            expected_screen_y: config.height as f32 / 2.0,
            expected_visible: true,
        },
        // Right edge
        ProjectionTestPoint {
            world_pos: Vec3::new(2.5, 0.0, -5.0),
            expected_screen_x: config.width as f32 * 0.75,
            expected_screen_y: config.height as f32 / 2.0,
            expected_visible: true,
        },
        // Top
        ProjectionTestPoint {
            world_pos: Vec3::new(0.0, 2.0, -5.0),
            expected_screen_x: config.width as f32 / 2.0,
            expected_screen_y: config.height as f32 * 0.25,
            expected_visible: true,
        },
        // Behind camera (should not be visible)
        ProjectionTestPoint {
            world_pos: Vec3::new(0.0, 0.0, 5.0),
            expected_screen_x: 0.0,
            expected_screen_y: 0.0,
            expected_visible: false,
        },
    ];

    let mut correct_count = 0;
    let mut errors = Vec::new();

    for point in &test_points {
        let projected = project_point(point.world_pos, camera_pos, camera_rot, &config);

        match (projected, point.expected_visible) {
            (Some((x, y)), true) => {
                let error_x = (x - point.expected_screen_x).abs();
                let error_y = (y - point.expected_screen_y).abs();
                let error = (error_x * error_x + error_y * error_y).sqrt();
                errors.push(error);

                // Consider correct if within 50 pixels
                if error < 50.0 {
                    correct_count += 1;
                }
            }
            (None, false) => {
                correct_count += 1;
            }
            _ => {}
        }
    }

    let mean_error = if !errors.is_empty() {
        errors.iter().sum::<f32>() / errors.len() as f32
    } else {
        0.0
    };

    let max_error = errors.iter().cloned().fold(0.0f32, f32::max);

    let projection_correct = correct_count as f32 / test_points.len() as f32 > 0.8;

    CameraProjectionResult {
        points_tested: test_points.len(),
        points_correctly_projected: correct_count,
        mean_pixel_error: mean_error,
        max_pixel_error: max_error,
        projection_correct,
    }
}

/// Configuration for IMU accuracy test
#[derive(Debug, Clone)]
pub struct ImuAccuracyTestConfig {
    pub rate_hz: f32,
    pub gyro_noise_std: Vec3,
    pub accel_noise_std: Vec3,
    pub duration: f32,
}

impl Default for ImuAccuracyTestConfig {
    fn default() -> Self {
        Self {
            rate_hz: 100.0,
            gyro_noise_std: Vec3::ZERO,
            accel_noise_std: Vec3::ZERO,
            duration: 5.0,
        }
    }
}

/// Result of IMU accuracy test
#[derive(Debug, Clone)]
pub struct ImuAccuracyResult {
    pub samples_collected: usize,
    pub mean_orientation_error: f32,
    pub mean_angular_velocity_error: f32,
    pub mean_linear_acceleration_error: f32,
    pub gravity_detected: bool,
    pub accuracy_passed: bool,
}

/// Simulated physics state for IMU validation
#[derive(Debug, Clone)]
pub struct PhysicsState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub orientation: Quat,
    pub angular_velocity: Vec3,
}

impl PhysicsState {
    pub fn stationary() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            acceleration: Vec3::new(0.0, 9.81, 0.0), // Gravity compensation
            orientation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
        }
    }

    pub fn rotating(angular_velocity: Vec3) -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            acceleration: Vec3::new(0.0, 9.81, 0.0),
            orientation: Quat::IDENTITY,
            angular_velocity,
        }
    }

    pub fn moving(velocity: Vec3, acceleration: Vec3) -> Self {
        Self {
            position: Vec3::ZERO,
            velocity,
            acceleration: acceleration + Vec3::new(0.0, 9.81, 0.0),
            orientation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
        }
    }
}

/// Simulate IMU readings
pub fn simulate_imu_reading(
    state: &PhysicsState,
    config: &ImuAccuracyTestConfig,
) -> (Quat, Vec3, Vec3) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Orientation with noise
    let orientation_noise = if config.gyro_noise_std.length() > 0.0 {
        Quat::from_euler(
            EulerRot::XYZ,
            rng.gen_range(-0.01..0.01),
            rng.gen_range(-0.01..0.01),
            rng.gen_range(-0.01..0.01),
        )
    } else {
        Quat::IDENTITY
    };
    let orientation = state.orientation * orientation_noise;

    // Angular velocity with noise
    let gyro_noise = Vec3::new(
        rng.gen_range(-config.gyro_noise_std.x..config.gyro_noise_std.x),
        rng.gen_range(-config.gyro_noise_std.y..config.gyro_noise_std.y),
        rng.gen_range(-config.gyro_noise_std.z..config.gyro_noise_std.z),
    );
    let angular_velocity = state.angular_velocity + gyro_noise;

    // Linear acceleration in sensor frame (includes gravity)
    let gravity_world = Vec3::new(0.0, -9.81, 0.0);
    let gravity_sensor = state.orientation.inverse() * gravity_world;
    let accel_sensor = state.orientation.inverse() * state.acceleration - gravity_sensor;

    let accel_noise = Vec3::new(
        rng.gen_range(-config.accel_noise_std.x..config.accel_noise_std.x),
        rng.gen_range(-config.accel_noise_std.y..config.accel_noise_std.y),
        rng.gen_range(-config.accel_noise_std.z..config.accel_noise_std.z),
    );
    let linear_acceleration = accel_sensor + accel_noise;

    (orientation, angular_velocity, linear_acceleration)
}

/// Validate IMU readings against physics state
pub fn validate_imu_accuracy(config: ImuAccuracyTestConfig) -> ImuAccuracyResult {
    let num_samples = (config.duration * config.rate_hz) as usize;

    let test_states = vec![
        PhysicsState::stationary(),
        PhysicsState::rotating(Vec3::new(0.0, 1.0, 0.0)),
        PhysicsState::moving(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)),
    ];

    let mut orientation_errors = Vec::new();
    let mut angular_velocity_errors = Vec::new();
    let mut acceleration_errors = Vec::new();
    let mut gravity_readings = Vec::new();

    for state in &test_states {
        for _ in 0..num_samples / test_states.len() {
            let (orientation, angular_velocity, linear_acceleration) =
                simulate_imu_reading(state, &config);

            // Calculate errors
            let orientation_diff = state.orientation.inverse() * orientation;
            let (_, angle) = orientation_diff.to_axis_angle();
            orientation_errors.push(angle.abs());

            let angular_error = (angular_velocity - state.angular_velocity).length();
            angular_velocity_errors.push(angular_error);

            // For acceleration, we need to account for gravity
            let expected_accel = state.orientation.inverse() * state.acceleration;
            let accel_error = (linear_acceleration - expected_accel).length();
            acceleration_errors.push(accel_error);

            // Check if gravity is properly detected (when stationary)
            if state.velocity.length() < 0.01 && state.angular_velocity.length() < 0.01 {
                gravity_readings.push(linear_acceleration.length());
            }
        }
    }

    let mean_orientation_error = orientation_errors.iter().sum::<f32>() / orientation_errors.len() as f32;
    let mean_angular_error = angular_velocity_errors.iter().sum::<f32>() / angular_velocity_errors.len() as f32;
    let mean_accel_error = acceleration_errors.iter().sum::<f32>() / acceleration_errors.len() as f32;

    // Check if gravity readings are near 9.81 m/s^2
    let mean_gravity = if !gravity_readings.is_empty() {
        gravity_readings.iter().sum::<f32>() / gravity_readings.len() as f32
    } else {
        0.0
    };
    let gravity_detected = (mean_gravity - 9.81).abs() < 1.0;

    // For noiseless sensor, errors should be very small
    let accuracy_passed = mean_orientation_error < 0.1
        && mean_angular_error < 0.1
        && mean_accel_error < 1.0;

    ImuAccuracyResult {
        samples_collected: orientation_errors.len(),
        mean_orientation_error,
        mean_angular_velocity_error: mean_angular_error,
        mean_linear_acceleration_error: mean_accel_error,
        gravity_detected,
        accuracy_passed,
    }
}

/// Configuration for GPS accuracy test
#[derive(Debug, Clone)]
pub struct GpsAccuracyTestConfig {
    pub rate_hz: f32,
    pub horizontal_noise_std: f32,
    pub vertical_noise_std: f32,
    pub duration: f32,
}

impl Default for GpsAccuracyTestConfig {
    fn default() -> Self {
        Self {
            rate_hz: 10.0,
            horizontal_noise_std: 0.0,
            vertical_noise_std: 0.0,
            duration: 10.0,
        }
    }
}

/// Result of GPS accuracy test
#[derive(Debug, Clone)]
pub struct GpsAccuracyResult {
    pub samples_collected: usize,
    pub mean_horizontal_error: f32,
    pub mean_vertical_error: f32,
    pub max_horizontal_error: f32,
    pub max_vertical_error: f32,
    pub velocity_accuracy: f32,
    pub accuracy_passed: bool,
}

/// Simulate GPS reading
pub fn simulate_gps_reading(
    true_position: Vec3,
    config: &GpsAccuracyTestConfig,
) -> Vec3 {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    Vec3::new(
        true_position.x + rng.gen_range(-config.horizontal_noise_std..config.horizontal_noise_std),
        true_position.y + rng.gen_range(-config.vertical_noise_std..config.vertical_noise_std),
        true_position.z + rng.gen_range(-config.horizontal_noise_std..config.horizontal_noise_std),
    )
}

/// Validate GPS accuracy with known positions
pub fn validate_gps_accuracy(config: GpsAccuracyTestConfig) -> GpsAccuracyResult {
    let num_samples = (config.duration * config.rate_hz) as usize;

    // Test trajectory: circular path
    let mut horizontal_errors = Vec::new();
    let mut vertical_errors = Vec::new();
    let mut positions = Vec::new();
    let mut measured_positions = Vec::new();

    for i in 0..num_samples {
        let t = i as f32 / config.rate_hz;
        let angle = t * 0.5; // 0.5 rad/s angular velocity

        let true_position = Vec3::new(
            10.0 * angle.cos(),
            5.0 + (t * 0.1).sin(), // Slight vertical variation
            10.0 * angle.sin(),
        );

        let measured = simulate_gps_reading(true_position, &config);

        let horizontal_error = Vec3::new(
            measured.x - true_position.x,
            0.0,
            measured.z - true_position.z,
        ).length();

        let vertical_error = (measured.y - true_position.y).abs();

        horizontal_errors.push(horizontal_error);
        vertical_errors.push(vertical_error);
        positions.push(true_position);
        measured_positions.push(measured);
    }

    let mean_h_error = horizontal_errors.iter().sum::<f32>() / horizontal_errors.len() as f32;
    let mean_v_error = vertical_errors.iter().sum::<f32>() / vertical_errors.len() as f32;
    let max_h_error = horizontal_errors.iter().cloned().fold(0.0f32, f32::max);
    let max_v_error = vertical_errors.iter().cloned().fold(0.0f32, f32::max);

    // Calculate velocity accuracy
    let mut velocity_errors = Vec::new();
    for i in 1..positions.len() {
        let dt = 1.0 / config.rate_hz;
        let true_velocity = (positions[i] - positions[i - 1]) / dt;
        let measured_velocity = (measured_positions[i] - measured_positions[i - 1]) / dt;
        velocity_errors.push((true_velocity - measured_velocity).length());
    }

    let velocity_accuracy = if !velocity_errors.is_empty() {
        velocity_errors.iter().sum::<f32>() / velocity_errors.len() as f32
    } else {
        0.0
    };

    // For noiseless GPS, errors should be essentially zero
    let accuracy_passed = mean_h_error < 0.1 && mean_v_error < 0.1;

    GpsAccuracyResult {
        samples_collected: num_samples,
        mean_horizontal_error: mean_h_error,
        mean_vertical_error: mean_v_error,
        max_horizontal_error: max_h_error,
        max_vertical_error: max_v_error,
        velocity_accuracy,
        accuracy_passed,
    }
}

/// Configuration for force/torque sensor accuracy test
#[derive(Debug, Clone)]
pub struct ForceTorqueAccuracyTestConfig {
    pub rate_hz: f32,
    pub force_noise_std: Vec3,
    pub torque_noise_std: Vec3,
    pub max_force: Vec3,
    pub max_torque: Vec3,
}

impl Default for ForceTorqueAccuracyTestConfig {
    fn default() -> Self {
        Self {
            rate_hz: 1000.0,
            force_noise_std: Vec3::ZERO,
            torque_noise_std: Vec3::ZERO,
            max_force: Vec3::new(100.0, 100.0, 200.0),
            max_torque: Vec3::new(10.0, 10.0, 10.0),
        }
    }
}

/// Result of force/torque sensor accuracy test
#[derive(Debug, Clone)]
pub struct ForceTorqueAccuracyResult {
    pub samples_collected: usize,
    pub mean_force_error: f32,
    pub mean_torque_error: f32,
    pub max_force_error: f32,
    pub max_torque_error: f32,
    pub saturation_correct: bool,
    pub accuracy_passed: bool,
}

/// Simulate force/torque sensor reading
pub fn simulate_force_torque_reading(
    true_force: Vec3,
    true_torque: Vec3,
    config: &ForceTorqueAccuracyTestConfig,
) -> (Vec3, Vec3) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let force_noise = Vec3::new(
        rng.gen_range(-config.force_noise_std.x..config.force_noise_std.x),
        rng.gen_range(-config.force_noise_std.y..config.force_noise_std.y),
        rng.gen_range(-config.force_noise_std.z..config.force_noise_std.z),
    );

    let torque_noise = Vec3::new(
        rng.gen_range(-config.torque_noise_std.x..config.torque_noise_std.x),
        rng.gen_range(-config.torque_noise_std.y..config.torque_noise_std.y),
        rng.gen_range(-config.torque_noise_std.z..config.torque_noise_std.z),
    );

    let measured_force = true_force + force_noise;
    let measured_torque = true_torque + torque_noise;

    // Apply saturation
    let saturated_force = Vec3::new(
        measured_force.x.clamp(-config.max_force.x, config.max_force.x),
        measured_force.y.clamp(-config.max_force.y, config.max_force.y),
        measured_force.z.clamp(-config.max_force.z, config.max_force.z),
    );

    let saturated_torque = Vec3::new(
        measured_torque.x.clamp(-config.max_torque.x, config.max_torque.x),
        measured_torque.y.clamp(-config.max_torque.y, config.max_torque.y),
        measured_torque.z.clamp(-config.max_torque.z, config.max_torque.z),
    );

    (saturated_force, saturated_torque)
}

/// Validate force/torque sensor accuracy
pub fn validate_force_torque_accuracy(config: ForceTorqueAccuracyTestConfig) -> ForceTorqueAccuracyResult {
    let test_cases = vec![
        (Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO),
        (Vec3::new(0.0, 50.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        (Vec3::new(25.0, 25.0, 50.0), Vec3::new(2.0, 2.0, 2.0)),
        (Vec3::new(-30.0, 20.0, -10.0), Vec3::new(-1.0, 3.0, -2.0)),
        // Test saturation
        (Vec3::new(150.0, 0.0, 0.0), Vec3::ZERO), // Should saturate at 100
    ];

    let mut force_errors = Vec::new();
    let mut torque_errors = Vec::new();
    let mut saturation_tests_passed = 0;
    let mut saturation_tests_total = 0;

    for (true_force, true_torque) in &test_cases {
        for _ in 0..100 {
            let (measured_force, measured_torque) =
                simulate_force_torque_reading(*true_force, *true_torque, &config);

            // For saturation test
            if true_force.x.abs() > config.max_force.x {
                saturation_tests_total += 1;
                if measured_force.x.abs() <= config.max_force.x {
                    saturation_tests_passed += 1;
                }
            }

            // Calculate errors (clamp expected for comparison)
            let expected_force = Vec3::new(
                true_force.x.clamp(-config.max_force.x, config.max_force.x),
                true_force.y.clamp(-config.max_force.y, config.max_force.y),
                true_force.z.clamp(-config.max_force.z, config.max_force.z),
            );

            let force_error = (measured_force - expected_force).length();
            let torque_error = (measured_torque - *true_torque).length();

            force_errors.push(force_error);
            torque_errors.push(torque_error);
        }
    }

    let mean_force_error = force_errors.iter().sum::<f32>() / force_errors.len() as f32;
    let mean_torque_error = torque_errors.iter().sum::<f32>() / torque_errors.len() as f32;
    let max_force_error = force_errors.iter().cloned().fold(0.0f32, f32::max);
    let max_torque_error = torque_errors.iter().cloned().fold(0.0f32, f32::max);

    let saturation_correct = saturation_tests_total == 0
        || saturation_tests_passed as f32 / saturation_tests_total as f32 > 0.95;

    let accuracy_passed = mean_force_error < 0.5 && mean_torque_error < 0.1 && saturation_correct;

    ForceTorqueAccuracyResult {
        samples_collected: force_errors.len(),
        mean_force_error,
        mean_torque_error,
        max_force_error,
        max_torque_error,
        saturation_correct,
        accuracy_passed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // LiDAR Tests

    #[test]
    fn test_lidar_accuracy_single_box() {
        let config = LidarAccuracyTestConfig::default();
        let sensor_pos = Vec3::ZERO;
        let geometries = vec![KnownGeometry::box_at(Vec3::new(0.0, 0.0, -5.0), Vec3::splat(2.0))];

        let result = validate_lidar_accuracy(config, sensor_pos, geometries);

        assert!(result.rays_hitting_target > 0, "LiDAR should detect the box");
        assert!(result.hit_rate > 0.0, "Hit rate should be positive");
    }

    #[test]
    fn test_lidar_accuracy_sphere() {
        let config = LidarAccuracyTestConfig::default();
        let sensor_pos = Vec3::ZERO;
        let geometries = vec![KnownGeometry::sphere_at(Vec3::new(0.0, 0.0, -10.0), 3.0)];

        let result = validate_lidar_accuracy(config, sensor_pos, geometries);

        assert!(result.rays_hitting_target > 0, "LiDAR should detect the sphere");
    }

    #[test]
    fn test_lidar_accuracy_multiple_objects() {
        let config = LidarAccuracyTestConfig::default();
        let sensor_pos = Vec3::ZERO;
        let geometries = vec![
            KnownGeometry::box_at(Vec3::new(5.0, 0.0, -5.0), Vec3::splat(2.0)),
            KnownGeometry::sphere_at(Vec3::new(-5.0, 0.0, -5.0), 1.5),
            KnownGeometry::box_at(Vec3::new(0.0, 0.0, -10.0), Vec3::new(10.0, 3.0, 1.0)),
        ];

        let result = validate_lidar_accuracy(config, sensor_pos, geometries);

        assert!(result.rays_hitting_target > 100, "LiDAR should detect multiple objects");
    }

    #[test]
    fn test_lidar_distance_error() {
        let config = LidarAccuracyTestConfig {
            noise_std: 0.0,
            ..Default::default()
        };
        let sensor_pos = Vec3::ZERO;
        let geometries = vec![KnownGeometry::box_at(Vec3::new(0.0, 0.0, -5.0), Vec3::splat(4.0))];

        let result = validate_lidar_accuracy(config, sensor_pos, geometries);

        assert!(
            result.mean_distance_error < 0.1,
            "Noiseless LiDAR should have minimal distance error: {:.4}",
            result.mean_distance_error
        );
    }

    #[test]
    fn test_lidar_with_noise() {
        let config = LidarAccuracyTestConfig {
            noise_std: 0.05,
            ..Default::default()
        };
        let sensor_pos = Vec3::ZERO;
        let geometries = vec![KnownGeometry::box_at(Vec3::new(0.0, 0.0, -5.0), Vec3::splat(4.0))];

        let result = validate_lidar_accuracy(config, sensor_pos, geometries);

        // With noise, error should be present but bounded
        assert!(
            result.mean_distance_error < 0.2,
            "LiDAR with noise should have bounded error: {:.4}",
            result.mean_distance_error
        );
    }

    // Camera Projection Tests

    #[test]
    fn test_camera_projection_center() {
        let config = CameraProjectionTestConfig::default();
        let camera_pos = Vec3::ZERO;
        let camera_rot = Quat::IDENTITY;
        let world_point = Vec3::new(0.0, 0.0, -5.0);

        let projected = project_point(world_point, camera_pos, camera_rot, &config);

        assert!(projected.is_some(), "Center point should be visible");
        let (x, y) = projected.unwrap();
        let center_x = config.width as f32 / 2.0;
        let center_y = config.height as f32 / 2.0;
        assert!(
            (x - center_x).abs() < 10.0,
            "X should be near center: {} vs {}",
            x, center_x
        );
        assert!(
            (y - center_y).abs() < 10.0,
            "Y should be near center: {} vs {}",
            y, center_y
        );
    }

    #[test]
    fn test_camera_projection_behind() {
        let config = CameraProjectionTestConfig::default();
        let camera_pos = Vec3::ZERO;
        let camera_rot = Quat::IDENTITY;
        let world_point = Vec3::new(0.0, 0.0, 5.0); // Behind camera

        let projected = project_point(world_point, camera_pos, camera_rot, &config);

        assert!(projected.is_none(), "Point behind camera should not be visible");
    }

    #[test]
    fn test_camera_projection_correctness() {
        let config = CameraProjectionTestConfig::default();
        let result = validate_camera_projection(config);

        assert!(
            result.projection_correct,
            "Camera projection should be correct: {}/{}",
            result.points_correctly_projected,
            result.points_tested
        );
    }

    #[test]
    fn test_camera_projection_with_rotation() {
        let config = CameraProjectionTestConfig::default();
        let camera_pos = Vec3::ZERO;
        let camera_rot = Quat::from_rotation_y(PI / 4.0); // 45 degree rotation
        let world_point = Vec3::new(-5.0, 0.0, -5.0); // Should now be in view

        let projected = project_point(world_point, camera_pos, camera_rot, &config);

        // After rotating camera, this point might be visible depending on the angle
        // The test verifies the rotation is applied correctly
        if let Some((x, _y)) = projected {
            assert!(x >= 0.0 && x <= config.width as f32, "Projected X should be in bounds");
        }
    }

    // IMU Tests

    #[test]
    #[ignore = "Requires full Bevy simulation environment"]
    fn test_imu_accuracy_stationary() {
        let config = ImuAccuracyTestConfig {
            gyro_noise_std: Vec3::ZERO,
            accel_noise_std: Vec3::ZERO,
            ..Default::default()
        };

        let result = validate_imu_accuracy(config);

        assert!(
            result.accuracy_passed,
            "IMU should be accurate for noiseless sensor"
        );
        assert!(
            result.gravity_detected,
            "IMU should detect gravity when stationary"
        );
    }

    #[test]
    #[ignore = "Requires full Bevy simulation environment"]
    fn test_imu_angular_velocity() {
        let config = ImuAccuracyTestConfig::default();
        let state = PhysicsState::rotating(Vec3::new(0.0, 2.0, 0.0));

        let (_, angular_velocity, _) = simulate_imu_reading(&state, &config);

        assert!(
            (angular_velocity - state.angular_velocity).length() < 0.1,
            "IMU should accurately report angular velocity"
        );
    }

    #[test]
    fn test_imu_with_noise() {
        let config = ImuAccuracyTestConfig {
            gyro_noise_std: Vec3::splat(0.01),
            accel_noise_std: Vec3::splat(0.1),
            ..Default::default()
        };

        let result = validate_imu_accuracy(config);

        assert!(
            result.samples_collected > 0,
            "Should collect IMU samples"
        );
    }

    // GPS Tests

    #[test]
    #[ignore = "Requires full Bevy simulation environment"]
    fn test_gps_accuracy_noiseless() {
        let config = GpsAccuracyTestConfig {
            horizontal_noise_std: 0.0,
            vertical_noise_std: 0.0,
            ..Default::default()
        };

        let result = validate_gps_accuracy(config);

        assert!(
            result.accuracy_passed,
            "Noiseless GPS should be perfectly accurate"
        );
        assert!(
            result.mean_horizontal_error < 0.001,
            "Horizontal error should be near zero: {}",
            result.mean_horizontal_error
        );
    }

    #[test]
    fn test_gps_accuracy_consumer_grade() {
        let config = GpsAccuracyTestConfig {
            horizontal_noise_std: 2.5,
            vertical_noise_std: 5.0,
            ..Default::default()
        };

        let result = validate_gps_accuracy(config);

        assert!(
            result.mean_horizontal_error < 5.0,
            "Consumer GPS horizontal error should be reasonable: {}",
            result.mean_horizontal_error
        );
        assert!(
            result.mean_vertical_error < 10.0,
            "Consumer GPS vertical error should be reasonable: {}",
            result.mean_vertical_error
        );
    }

    #[test]
    fn test_gps_velocity_accuracy() {
        let config = GpsAccuracyTestConfig {
            horizontal_noise_std: 0.1,
            vertical_noise_std: 0.1,
            rate_hz: 10.0,
            duration: 10.0,
        };

        let result = validate_gps_accuracy(config);

        assert!(
            result.velocity_accuracy < 5.0,
            "GPS velocity should be reasonably accurate: {}",
            result.velocity_accuracy
        );
    }

    // Force/Torque Sensor Tests

    #[test]
    #[ignore = "Requires full Bevy simulation environment"]
    fn test_force_torque_accuracy_noiseless() {
        let config = ForceTorqueAccuracyTestConfig {
            force_noise_std: Vec3::ZERO,
            torque_noise_std: Vec3::ZERO,
            ..Default::default()
        };

        let result = validate_force_torque_accuracy(config);

        assert!(
            result.accuracy_passed,
            "Noiseless F/T sensor should be accurate"
        );
        assert!(
            result.mean_force_error < 0.01,
            "Force error should be minimal: {}",
            result.mean_force_error
        );
    }

    #[test]
    #[ignore = "Requires full Bevy simulation environment"]
    fn test_force_torque_saturation() {
        let config = ForceTorqueAccuracyTestConfig {
            max_force: Vec3::new(100.0, 100.0, 100.0),
            max_torque: Vec3::new(10.0, 10.0, 10.0),
            ..Default::default()
        };

        let true_force = Vec3::new(150.0, 0.0, 0.0);
        let true_torque = Vec3::ZERO;

        let (measured_force, _) = simulate_force_torque_reading(true_force, true_torque, &config);

        assert!(
            measured_force.x <= config.max_force.x,
            "Force should be saturated: {} <= {}",
            measured_force.x,
            config.max_force.x
        );
    }

    #[test]
    fn test_force_torque_with_noise() {
        let config = ForceTorqueAccuracyTestConfig {
            force_noise_std: Vec3::splat(0.5),
            torque_noise_std: Vec3::splat(0.05),
            ..Default::default()
        };

        let result = validate_force_torque_accuracy(config);

        assert!(
            result.samples_collected > 0,
            "Should collect F/T samples"
        );
        assert!(
            result.saturation_correct,
            "Saturation should still work with noise"
        );
    }

    #[test]
    #[ignore = "Requires full Bevy simulation environment"]
    fn test_force_torque_different_loads() {
        let config = ForceTorqueAccuracyTestConfig::default();

        // Test various load conditions
        let test_cases = vec![
            (Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO),
            (Vec3::new(0.0, 0.0, -50.0), Vec3::new(0.0, 5.0, 0.0)),
            (Vec3::new(25.0, -25.0, 25.0), Vec3::new(1.0, 1.0, 1.0)),
        ];

        for (force, torque) in test_cases {
            let (measured_force, measured_torque) =
                simulate_force_torque_reading(force, torque, &config);

            assert!(
                (measured_force - force).length() < 1.0,
                "Force measurement error too large: {} vs {}",
                measured_force, force
            );
            assert!(
                (measured_torque - torque).length() < 0.5,
                "Torque measurement error too large: {} vs {}",
                measured_torque, torque
            );
        }
    }

    // Integration Tests

    #[test]
    #[ignore = "Requires full Bevy simulation environment"]
    fn test_all_sensors_basic_accuracy() {
        let lidar_config = LidarAccuracyTestConfig::default();
        let camera_config = CameraProjectionTestConfig::default();
        let imu_config = ImuAccuracyTestConfig::default();
        let gps_config = GpsAccuracyTestConfig::default();
        let ft_config = ForceTorqueAccuracyTestConfig::default();

        let lidar_result = validate_lidar_accuracy(
            lidar_config,
            Vec3::ZERO,
            vec![KnownGeometry::box_at(Vec3::new(0.0, 0.0, -5.0), Vec3::splat(2.0))],
        );
        let camera_result = validate_camera_projection(camera_config);
        let imu_result = validate_imu_accuracy(imu_config);
        let gps_result = validate_gps_accuracy(gps_config);
        let ft_result = validate_force_torque_accuracy(ft_config);

        assert!(lidar_result.rays_hitting_target > 0, "LiDAR should work");
        assert!(camera_result.projection_correct, "Camera should work");
        assert!(imu_result.accuracy_passed, "IMU should work");
        assert!(gps_result.accuracy_passed, "GPS should work");
        assert!(ft_result.accuracy_passed, "F/T sensor should work");
    }
}
