use bevy::prelude::*;
use rand::Rng;

use crate::physics::rigid_body::Velocity;

#[derive(Component)]
pub struct IMU {
    pub rate_hz: f32,
    pub last_update: f32,

    // Noise parameters
    pub gyro_noise_std: Vec3,
    pub accel_noise_std: Vec3,
    pub orientation_noise_std: f32,

    // Bias parameters
    pub gyro_bias: Vec3,
    pub accel_bias: Vec3,

    // Previous state for differentiation
    pub last_velocity: Vec3,
    pub last_time: f32,
}

impl Default for IMU {
    fn default() -> Self {
        Self {
            rate_hz: 100.0,
            last_update: 0.0,
            gyro_noise_std: Vec3::new(0.001, 0.001, 0.001),
            accel_noise_std: Vec3::new(0.01, 0.01, 0.01),
            orientation_noise_std: 0.001,
            gyro_bias: Vec3::ZERO,
            accel_bias: Vec3::ZERO,
            last_velocity: Vec3::ZERO,
            last_time: 0.0,
        }
    }
}

impl IMU {
    pub fn new(rate_hz: f32) -> Self {
        Self {
            rate_hz,
            ..default()
        }
    }

    pub fn with_noise(
        mut self,
        gyro_noise: Vec3,
        accel_noise: Vec3,
        orientation_noise: f32,
    ) -> Self {
        self.gyro_noise_std = gyro_noise;
        self.accel_noise_std = accel_noise;
        self.orientation_noise_std = orientation_noise;
        self
    }

    pub fn with_bias(mut self, gyro_bias: Vec3, accel_bias: Vec3) -> Self {
        self.gyro_bias = gyro_bias;
        self.accel_bias = accel_bias;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    pub fn update_time(&mut self, current_time: f32) {
        self.last_update = current_time;
    }
}

#[derive(Component, Clone)]
pub struct IMUData {
    pub timestamp: f32,

    // Orientation (from transform)
    pub orientation: Quat,

    // Angular velocity (rad/s)
    pub angular_velocity: Vec3,

    // Linear acceleration (m/s²)
    pub linear_acceleration: Vec3,

    // Covariances
    pub orientation_covariance: Vec<f64>,
    pub angular_velocity_covariance: Vec<f64>,
    pub linear_acceleration_covariance: Vec<f64>,
}

impl Default for IMUData {
    fn default() -> Self {
        Self {
            timestamp: 0.0,
            orientation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
            linear_acceleration: Vec3::ZERO,
            orientation_covariance: vec![0.0; 9],
            angular_velocity_covariance: vec![0.0; 9],
            linear_acceleration_covariance: vec![0.0; 9],
        }
    }
}

impl IMUData {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn imu_update_system(
    time: Res<Time>,
    gravity: Res<Gravity>,
    mut query: Query<(
        &mut IMU,
        &mut IMUData,
        &GlobalTransform,
        Option<&Velocity>,
    )>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    for (mut imu, mut imu_data, transform, velocity_opt) in query.iter_mut() {
        if !imu.should_update(current_time) {
            continue;
        }

        let prev_time = imu.last_time;
        imu.update_time(current_time);
        imu.last_time = current_time;

        let mut rng = rand::thread_rng();

        // Get orientation from transform
        let (_, rotation, _) = transform.to_scale_rotation_translation();

        // Add orientation noise
        let orientation_noise = Quat::from_euler(
            EulerRot::XYZ,
            rng.gen_range(-imu.orientation_noise_std..imu.orientation_noise_std),
            rng.gen_range(-imu.orientation_noise_std..imu.orientation_noise_std),
            rng.gen_range(-imu.orientation_noise_std..imu.orientation_noise_std),
        );
        imu_data.orientation = rotation * orientation_noise;

        // Get angular velocity from velocity component or compute from orientation change
        let angular_velocity = if let Some(velocity) = velocity_opt {
            velocity.angular
        } else {
            Vec3::ZERO
        };

        // Add gyro noise and bias
        let gyro_noise = Vec3::new(
            rng.gen_range(-imu.gyro_noise_std.x..imu.gyro_noise_std.x),
            rng.gen_range(-imu.gyro_noise_std.y..imu.gyro_noise_std.y),
            rng.gen_range(-imu.gyro_noise_std.z..imu.gyro_noise_std.z),
        );
        imu_data.angular_velocity = angular_velocity + imu.gyro_bias + gyro_noise;

        // Compute linear acceleration
        let current_velocity = if let Some(velocity) = velocity_opt {
            velocity.linear
        } else {
            Vec3::ZERO
        };

        let acceleration = if prev_time > 0.0 && dt > 0.0 {
            (current_velocity - imu.last_velocity) / dt
        } else {
            Vec3::ZERO
        };

        imu.last_velocity = current_velocity;

        // Transform gravity to sensor frame
        let gravity_world = Vec3::new(0.0, gravity.0, 0.0);
        let gravity_sensor = rotation.inverse() * gravity_world;

        // Acceleration in sensor frame = measured acceleration - gravity
        let accel_sensor = rotation.inverse() * acceleration - gravity_sensor;

        // Add accelerometer noise and bias
        let accel_noise = Vec3::new(
            rng.gen_range(-imu.accel_noise_std.x..imu.accel_noise_std.x),
            rng.gen_range(-imu.accel_noise_std.y..imu.accel_noise_std.y),
            rng.gen_range(-imu.accel_noise_std.z..imu.accel_noise_std.z),
        );
        imu_data.linear_acceleration = accel_sensor + imu.accel_bias + accel_noise;

        // Set covariances based on noise levels
        imu_data.orientation_covariance = vec![
            (imu.orientation_noise_std * imu.orientation_noise_std) as f64, 0.0, 0.0,
            0.0, (imu.orientation_noise_std * imu.orientation_noise_std) as f64, 0.0,
            0.0, 0.0, (imu.orientation_noise_std * imu.orientation_noise_std) as f64,
        ];

        imu_data.angular_velocity_covariance = vec![
            (imu.gyro_noise_std.x * imu.gyro_noise_std.x) as f64, 0.0, 0.0,
            0.0, (imu.gyro_noise_std.y * imu.gyro_noise_std.y) as f64, 0.0,
            0.0, 0.0, (imu.gyro_noise_std.z * imu.gyro_noise_std.z) as f64,
        ];

        imu_data.linear_acceleration_covariance = vec![
            (imu.accel_noise_std.x * imu.accel_noise_std.x) as f64, 0.0, 0.0,
            0.0, (imu.accel_noise_std.y * imu.accel_noise_std.y) as f64, 0.0,
            0.0, 0.0, (imu.accel_noise_std.z * imu.accel_noise_std.z) as f64,
        ];

        imu_data.timestamp = current_time;
    }
}

#[derive(Resource)]
pub struct Gravity(pub f32);

impl Default for Gravity {
    fn default() -> Self {
        Self(-9.81)
    }
}

pub fn visualize_imu_system(
    mut gizmos: Gizmos,
    query: Query<(&IMU, &IMUData, &GlobalTransform)>,
) {
    for (_imu, imu_data, transform) in query.iter() {
        let pos = transform.translation();

        // Draw orientation axes
        let forward = imu_data.orientation * Vec3::Z;
        let up = imu_data.orientation * Vec3::Y;
        let right = imu_data.orientation * Vec3::X;

        gizmos.line(pos, pos + forward * 0.3, Color::srgb(0.0, 0.0, 1.0));
        gizmos.line(pos, pos + up * 0.3, Color::srgb(0.0, 1.0, 0.0));
        gizmos.line(pos, pos + right * 0.3, Color::srgb(1.0, 0.0, 0.0));

        // Draw angular velocity
        if imu_data.angular_velocity.length() > 0.01 {
            let angvel_dir = imu_data.angular_velocity.normalize_or_zero();
            gizmos.line(
                pos,
                pos + angvel_dir * 0.5,
                Color::srgb(1.0, 1.0, 0.0),
            );
        }

        // Draw acceleration
        if imu_data.linear_acceleration.length() > 0.1 {
            let accel_world = imu_data.orientation * imu_data.linear_acceleration;
            let accel_dir = accel_world.normalize_or_zero();
            gizmos.line(
                pos,
                pos + accel_dir * 0.5,
                Color::srgb(1.0, 0.0, 1.0),
            );
        }
    }
}
