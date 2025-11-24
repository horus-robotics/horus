# Sensors API Reference

This document provides a comprehensive reference for all 16 sensor types available in Sim3D.

## Sensor Architecture

All sensors in Sim3D follow a common pattern:

1. **Configuration Component**: Sensor parameters (rate, noise, range)
2. **Data Component**: Output data storage
3. **Update System**: Bevy system that computes sensor readings

```rust
// General pattern
#[derive(Component)]
pub struct SensorConfig {
    pub rate_hz: f32,
    pub last_update: f32,
    // ... sensor-specific parameters
}

#[derive(Component, Default)]
pub struct SensorData {
    // ... sensor-specific output
}

pub fn sensor_update_system(
    time: Res<Time>,
    mut query: Query<(&mut SensorConfig, &mut SensorData, &GlobalTransform)>,
) {
    // Update sensor readings
}
```

## 1. LiDAR 2D

2D laser scanner for planar distance measurements.

### Configuration

```rust
#[derive(Component)]
pub struct Lidar2D {
    /// Update rate in Hz
    pub rate_hz: f32,

    /// Minimum detection range (m)
    pub range_min: f32,

    /// Maximum detection range (m)
    pub range_max: f32,

    /// Start angle (radians, relative to forward)
    pub angle_min: f32,

    /// End angle (radians)
    pub angle_max: f32,

    /// Angular resolution (radians between rays)
    pub angle_increment: f32,

    /// Range noise standard deviation (m)
    pub range_noise_std: f32,

    /// Last update timestamp
    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct Lidar2DData {
    /// Distance measurements (m), inf for no hit
    pub ranges: Vec<f32>,

    /// Angle for each measurement (radians)
    pub angles: Vec<f32>,

    /// Intensities (0.0 - 1.0)
    pub intensities: Vec<f32>,

    /// Timestamp of last update
    pub timestamp: f32,
}
```

### Usage Example

```rust
// Create 360-degree LiDAR with 1-degree resolution
commands.spawn((
    Lidar2D {
        rate_hz: 10.0,
        range_min: 0.1,
        range_max: 30.0,
        angle_min: -std::f32::consts::PI,
        angle_max: std::f32::consts::PI,
        angle_increment: std::f32::consts::PI / 180.0,  // 1 degree
        range_noise_std: 0.01,
        last_update: 0.0,
    },
    Lidar2DData::default(),
    Transform::from_xyz(0.0, 0.3, 0.0),
));
```

## 2. LiDAR 3D

3D point cloud sensor (e.g., Velodyne, Ouster).

### Configuration

```rust
#[derive(Component)]
pub struct Lidar3D {
    pub rate_hz: f32,
    pub range_min: f32,
    pub range_max: f32,

    /// Horizontal field of view (radians)
    pub horizontal_fov: f32,

    /// Points per horizontal revolution
    pub horizontal_resolution: u32,

    /// Vertical field of view (radians)
    pub vertical_fov: f32,

    /// Number of vertical channels
    pub vertical_channels: u32,

    /// Range noise (m)
    pub range_noise_std: f32,

    /// Intensity noise
    pub intensity_noise_std: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct Lidar3DData {
    /// 3D points in sensor frame
    pub points: Vec<Vec3>,

    /// Intensity per point (0.0 - 1.0)
    pub intensities: Vec<f32>,

    /// Ring/channel index per point
    pub rings: Vec<u8>,

    /// Point cloud timestamp
    pub timestamp: f32,
}
```

### Preset Configurations

```rust
impl Lidar3D {
    /// Velodyne VLP-16 configuration
    pub fn vlp16() -> Self {
        Self {
            rate_hz: 10.0,
            range_min: 0.5,
            range_max: 100.0,
            horizontal_fov: std::f32::consts::TAU,
            horizontal_resolution: 1875,
            vertical_fov: 0.523,  // 30 degrees
            vertical_channels: 16,
            range_noise_std: 0.03,
            intensity_noise_std: 0.1,
            last_update: 0.0,
        }
    }

    /// Velodyne HDL-64E configuration
    pub fn hdl64e() -> Self {
        Self {
            rate_hz: 10.0,
            range_min: 0.9,
            range_max: 120.0,
            horizontal_fov: std::f32::consts::TAU,
            horizontal_resolution: 2083,
            vertical_fov: 0.454,  // 26 degrees
            vertical_channels: 64,
            range_noise_std: 0.02,
            intensity_noise_std: 0.1,
            last_update: 0.0,
        }
    }
}
```

## 3. RGB Camera

Standard color camera.

### Configuration

```rust
#[derive(Component)]
pub struct Camera {
    pub rate_hz: f32,

    /// Image width (pixels)
    pub width: u32,

    /// Image height (pixels)
    pub height: u32,

    /// Horizontal field of view (radians)
    pub fov_horizontal: f32,

    /// Vertical field of view (radians)
    pub fov_vertical: f32,

    /// Near clipping plane (m)
    pub near_clip: f32,

    /// Far clipping plane (m)
    pub far_clip: f32,

    /// Image noise standard deviation
    pub noise_std: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct RGBCameraData {
    /// Raw image data (RGB, row-major)
    pub data: Vec<u8>,

    /// Image dimensions
    pub width: u32,
    pub height: u32,

    /// Encoding format
    pub encoding: String,

    pub timestamp: f32,
}
```

## 4. Depth Camera

Depth-sensing camera (e.g., Intel RealSense, Kinect).

### Configuration

```rust
#[derive(Component)]
pub struct DepthCamera {
    pub rate_hz: f32,
    pub width: u32,
    pub height: u32,
    pub fov_horizontal: f32,
    pub near_clip: f32,
    pub far_clip: f32,

    /// Depth measurement noise (m)
    pub depth_noise_std: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct DepthCameraData {
    /// Depth values in meters (row-major)
    pub depth: Vec<f32>,

    pub width: u32,
    pub height: u32,
    pub timestamp: f32,
}
```

## 5. RGBD Camera

Combined RGB and depth camera.

### Configuration

```rust
#[derive(Component)]
pub struct RGBDCamera {
    pub rgb: Camera,
    pub depth: DepthCamera,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct RGBDCameraData {
    pub rgb: RGBCameraData,
    pub depth: DepthCameraData,
}
```

## 6. IMU (Inertial Measurement Unit)

Measures orientation, angular velocity, and linear acceleration.

### Configuration

```rust
#[derive(Component)]
pub struct IMU {
    pub rate_hz: f32,

    // Accelerometer noise parameters
    /// Noise density (m/s^2/sqrt(Hz))
    pub accel_noise_density: f32,
    /// Random walk (m/s^3/sqrt(Hz))
    pub accel_random_walk: f32,
    /// Bias stability (m/s^2)
    pub accel_bias_stability: f32,

    // Gyroscope noise parameters
    /// Noise density (rad/s/sqrt(Hz))
    pub gyro_noise_density: f32,
    /// Random walk (rad/s^2/sqrt(Hz))
    pub gyro_random_walk: f32,
    /// Bias stability (rad/s)
    pub gyro_bias_stability: f32,

    // Current bias values (evolve over time)
    pub accel_bias: Vec3,
    pub gyro_bias: Vec3,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct IMUData {
    /// Orientation as quaternion
    pub orientation: Quat,

    /// Orientation covariance (9 elements, row-major)
    pub orientation_covariance: Vec<f64>,

    /// Angular velocity (rad/s)
    pub angular_velocity: Vec3,

    /// Angular velocity covariance
    pub angular_velocity_covariance: Vec<f64>,

    /// Linear acceleration (m/s^2)
    pub linear_acceleration: Vec3,

    /// Linear acceleration covariance
    pub linear_acceleration_covariance: Vec<f64>,

    pub timestamp: f32,
}
```

### Preset Configurations

```rust
impl IMU {
    /// High-quality MEMS IMU
    pub fn high_quality() -> Self {
        Self {
            rate_hz: 200.0,
            accel_noise_density: 0.0002,
            accel_random_walk: 0.002,
            accel_bias_stability: 0.00002,
            gyro_noise_density: 0.00005,
            gyro_random_walk: 0.00001,
            gyro_bias_stability: 0.000005,
            accel_bias: Vec3::ZERO,
            gyro_bias: Vec3::ZERO,
            last_update: 0.0,
        }
    }

    /// Consumer-grade IMU
    pub fn consumer_grade() -> Self {
        Self {
            rate_hz: 100.0,
            accel_noise_density: 0.001,
            accel_random_walk: 0.01,
            accel_bias_stability: 0.0001,
            gyro_noise_density: 0.0005,
            gyro_random_walk: 0.0001,
            gyro_bias_stability: 0.00005,
            accel_bias: Vec3::ZERO,
            gyro_bias: Vec3::ZERO,
            last_update: 0.0,
        }
    }
}
```

## 7. GPS

Global positioning system with configurable accuracy.

### Configuration

```rust
#[derive(Component)]
pub struct GPS {
    pub rate_hz: f32,

    /// Horizontal position noise (m)
    pub horizontal_noise_std: f32,

    /// Vertical position noise (m)
    pub vertical_noise_std: f32,

    /// Bias drift per axis
    pub horizontal_bias: Vec2,
    pub vertical_bias: f32,

    /// Bias drift rate (m/s)
    pub bias_drift_rate: f32,

    /// Minimum satellites for fix
    pub min_satellites: u8,

    /// Current visible satellites
    pub current_satellites: u8,

    /// Horizontal dilution of precision
    pub hdop: f32,

    /// Vertical dilution of precision
    pub vdop: f32,

    /// Position history for velocity computation
    position_history: VecDeque<PositionSample>,

    /// History buffer size
    history_size: usize,

    /// Velocity smoothing method
    pub velocity_smoothing: VelocitySmoothingMethod,

    /// Minimum samples for velocity
    pub min_samples_for_velocity: usize,

    pub last_update: f32,
}

pub enum VelocitySmoothingMethod {
    SimpleDifference,
    WeightedAverage,
    LinearRegression,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct GPSData {
    /// Position in world coordinates
    pub position: Vec3,

    /// Position covariance (9 elements)
    pub position_covariance: Vec<f64>,

    /// Visible satellites
    pub satellites_visible: u8,

    pub hdop: f32,
    pub vdop: f32,

    /// Fix quality (0=none, 1=GPS, 2=DGPS, 4=RTK)
    pub fix_quality: u8,

    /// Computed velocity
    pub velocity: Option<Vec3>,

    /// Velocity covariance
    pub velocity_covariance: Vec<f64>,

    pub timestamp: f32,
}
```

### Preset Configurations

```rust
impl GPS {
    /// RTK GPS (centimeter accuracy)
    pub fn high_accuracy() -> Self {
        Self {
            horizontal_noise_std: 0.02,
            vertical_noise_std: 0.03,
            bias_drift_rate: 0.001,
            hdop: 0.8,
            vdop: 1.0,
            ..Default::default()
        }
    }

    /// Standard consumer GPS
    pub fn consumer_grade() -> Self {
        Self {
            horizontal_noise_std: 5.0,
            vertical_noise_std: 10.0,
            bias_drift_rate: 0.05,
            hdop: 2.0,
            vdop: 3.0,
            ..Default::default()
        }
    }

    /// Poor conditions (urban canyon)
    pub fn low_quality() -> Self {
        Self {
            horizontal_noise_std: 15.0,
            vertical_noise_std: 30.0,
            bias_drift_rate: 0.2,
            current_satellites: 4,
            hdop: 4.5,
            vdop: 6.0,
            ..Default::default()
        }
    }
}
```

## 8. Force/Torque Sensor

Measures 6-DOF forces and torques at a joint or contact point.

### Configuration

```rust
#[derive(Component)]
pub struct ForceTorqueSensor {
    pub rate_hz: f32,

    /// Force measurement range (N)
    pub force_range: f32,

    /// Torque measurement range (Nm)
    pub torque_range: f32,

    /// Force noise (N)
    pub force_noise_std: f32,

    /// Torque noise (Nm)
    pub torque_noise_std: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct ForceTorqueData {
    /// Force vector (N)
    pub force: Vec3,

    /// Torque vector (Nm)
    pub torque: Vec3,

    pub timestamp: f32,
}
```

## 9. Contact Sensor

Binary contact detection or contact point information.

### Configuration

```rust
#[derive(Component)]
pub struct ContactSensor {
    pub rate_hz: f32,

    /// Report detailed contact points
    pub report_contacts: bool,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct ContactSensorData {
    /// Is there any contact?
    pub in_contact: bool,

    /// Contact points (if report_contacts enabled)
    pub contact_points: Vec<ContactPoint>,

    pub timestamp: f32,
}

pub struct ContactPoint {
    /// Contact position in world frame
    pub position: Vec3,

    /// Contact normal
    pub normal: Vec3,

    /// Penetration depth
    pub depth: f32,

    /// Contact force (if available)
    pub force: Option<Vec3>,
}
```

## 10. Encoder

Measures position and velocity of joints or wheels.

### Configuration

```rust
#[derive(Component)]
pub struct Encoder {
    pub rate_hz: f32,

    /// Counts per revolution
    pub counts_per_rev: u32,

    /// Position noise (rad)
    pub position_noise_std: f32,

    /// Velocity noise (rad/s)
    pub velocity_noise_std: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct EncoderData {
    /// Position (rad)
    pub position: f32,

    /// Velocity (rad/s)
    pub velocity: f32,

    /// Raw tick count
    pub ticks: i64,

    pub timestamp: f32,
}
```

## 11. Magnetometer

Measures magnetic field for heading estimation.

### Configuration

```rust
#[derive(Component)]
pub struct Magnetometer {
    pub rate_hz: f32,

    /// Noise (Tesla)
    pub noise_std: f32,

    /// Hard iron offset
    pub hard_iron_offset: Vec3,

    /// Soft iron matrix (3x3, row-major)
    pub soft_iron_matrix: [f32; 9],

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct MagnetometerData {
    /// Magnetic field vector (Tesla)
    pub magnetic_field: Vec3,

    /// Heading (rad, 0 = North)
    pub heading: f32,

    pub timestamp: f32,
}
```

## 12. Barometer

Measures atmospheric pressure for altitude estimation.

### Configuration

```rust
#[derive(Component)]
pub struct Barometer {
    pub rate_hz: f32,

    /// Pressure noise (Pa)
    pub pressure_noise_std: f32,

    /// Reference pressure at sea level (Pa)
    pub reference_pressure: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct BarometerData {
    /// Absolute pressure (Pa)
    pub pressure: f32,

    /// Estimated altitude (m)
    pub altitude: f32,

    /// Temperature (Celsius)
    pub temperature: f32,

    pub timestamp: f32,
}
```

## 13. Radar

Radio detection for range and velocity measurement.

### Configuration

```rust
#[derive(Component)]
pub struct Radar {
    pub rate_hz: f32,

    /// Maximum range (m)
    pub range_max: f32,

    /// Horizontal field of view (rad)
    pub fov_horizontal: f32,

    /// Vertical field of view (rad)
    pub fov_vertical: f32,

    /// Range resolution (m)
    pub range_resolution: f32,

    /// Velocity resolution (m/s)
    pub velocity_resolution: f32,

    /// Range noise (m)
    pub range_noise_std: f32,

    /// Velocity noise (m/s)
    pub velocity_noise_std: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct RadarData {
    /// Detected targets
    pub targets: Vec<RadarTarget>,

    pub timestamp: f32,
}

pub struct RadarTarget {
    /// Range to target (m)
    pub range: f32,

    /// Azimuth angle (rad)
    pub azimuth: f32,

    /// Elevation angle (rad)
    pub elevation: f32,

    /// Radial velocity (m/s, positive = approaching)
    pub velocity: f32,

    /// Radar cross-section (dBsm)
    pub rcs: f32,
}
```

## 14. Sonar

Acoustic range sensor.

### Configuration

```rust
#[derive(Component)]
pub struct Sonar {
    pub rate_hz: f32,

    /// Minimum range (m)
    pub range_min: f32,

    /// Maximum range (m)
    pub range_max: f32,

    /// Beam width (rad)
    pub beam_width: f32,

    /// Range noise (m)
    pub range_noise_std: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct SonarData {
    /// Measured range (m)
    pub range: f32,

    /// Echo strength (0-1)
    pub intensity: f32,

    pub timestamp: f32,
}
```

## 15. Thermal Camera

Infrared thermal imaging.

### Configuration

```rust
#[derive(Component)]
pub struct ThermalCamera {
    pub rate_hz: f32,
    pub width: u32,
    pub height: u32,
    pub fov_horizontal: f32,

    /// Temperature range (Kelvin)
    pub temp_min: f32,
    pub temp_max: f32,

    /// Temperature noise (K)
    pub temp_noise_std: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct ThermalCameraData {
    /// Temperature values (Kelvin, row-major)
    pub temperatures: Vec<f32>,

    pub width: u32,
    pub height: u32,
    pub timestamp: f32,
}
```

## 16. Event Camera

Neuromorphic vision sensor that outputs asynchronous events.

### Configuration

```rust
#[derive(Component)]
pub struct EventCamera {
    pub rate_hz: f32,
    pub width: u32,
    pub height: u32,

    /// Contrast threshold for event generation
    pub contrast_threshold: f32,

    /// Refractory period (s)
    pub refractory_period: f32,

    pub last_update: f32,
}
```

### Data Output

```rust
#[derive(Component, Default)]
pub struct EventCameraData {
    /// Events since last update
    pub events: Vec<Event>,

    pub timestamp: f32,
}

pub struct Event {
    /// Pixel x coordinate
    pub x: u16,

    /// Pixel y coordinate
    pub y: u16,

    /// Event polarity (true = ON, false = OFF)
    pub polarity: bool,

    /// Event timestamp (s)
    pub timestamp: f32,
}
```

## Adding Sensors to Robots

### General Pattern

```rust
fn add_sensors_to_robot(
    mut commands: Commands,
    robot_entity: Entity,
) {
    commands.entity(robot_entity).with_children(|parent| {
        // Add LiDAR on top
        parent.spawn((
            Lidar2D::default(),
            Lidar2DData::default(),
            Transform::from_xyz(0.0, 0.3, 0.0),
            Name::new("lidar"),
        ));

        // Add IMU at center
        parent.spawn((
            IMU::high_quality(),
            IMUData::default(),
            Transform::default(),
            Name::new("imu"),
        ));

        // Add front camera
        parent.spawn((
            Camera {
                width: 640,
                height: 480,
                fov_horizontal: 1.22,
                ..Default::default()
            },
            RGBCameraData::default(),
            Transform::from_xyz(0.15, 0.1, 0.0)
                .looking_to(Vec3::X, Vec3::Y),
            Name::new("front_camera"),
        ));
    });
}
```

### Registering Sensor Systems

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<PhysicsWorld>()
        // Add sensor update systems
        .add_systems(Update, (
            lidar2d_update_system,
            lidar3d_update_system,
            camera_update_system,
            imu_update_system,
            gps_update_system,
            // ... other sensor systems
        ))
        .run();
}
```
