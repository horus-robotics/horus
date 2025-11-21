use bevy::prelude::*;

use crate::sensors::{camera, encoder, force_torque, gps, imu, lidar3d};

/// System set for sensor updates - ensures sensors run after physics
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SensorSystemSet {
    /// Core sensor data acquisition
    Update,
    /// Sensor visualization (runs after Update)
    Visualization,
}

/// Plugin that registers all sensor update systems
pub struct SensorUpdatePlugin;

impl Plugin for SensorUpdatePlugin {
    fn build(&self, app: &mut App) {
        // Initialize resources needed by sensors
        app.init_resource::<imu::Gravity>();

        // Configure system sets
        app.configure_sets(
            Update,
            (SensorSystemSet::Update, SensorSystemSet::Visualization).chain(),
        );

        // Add sensor update systems
        app.add_systems(
            Update,
            (
                imu::imu_update_system,
                gps::gps_update_system,
                encoder::encoder_update_system,
                force_torque::force_torque_update_system,
                lidar3d::lidar3d_update_system,
                lidar3d::lidar2d_update_system,
                camera::update_camera_timestamps_system,
                camera::extract_camera_images_system,
            )
                .chain()
                .in_set(SensorSystemSet::Update),
        );

        // Add visualization systems
        app.add_systems(
            Update,
            (
                imu::visualize_imu_system,
                gps::visualize_gps_system,
                force_torque::visualize_force_torque_system,
                lidar3d::visualize_lidar_system,
                lidar3d::visualize_lidar2d_system,
                camera::visualize_camera_system,
            )
                .chain()
                .in_set(SensorSystemSet::Visualization),
        );
    }
}

// Legacy sensor_update_system() removed - use SensorUpdatePlugin instead
// The no-op function has been removed and main.rs now properly uses the plugin

/// Resource for tracking sensor update statistics
#[derive(Resource, Default)]
pub struct SensorUpdateStats {
    pub imu_update_count: u64,
    pub gps_update_count: u64,
    pub encoder_update_count: u64,
    pub force_torque_update_count: u64,
    pub lidar_update_count: u64,
    pub camera_update_count: u64,
    pub last_update_time: f32,
}

impl SensorUpdateStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_updates(&self) -> u64 {
        self.imu_update_count
            + self.gps_update_count
            + self.encoder_update_count
            + self.force_torque_update_count
            + self.lidar_update_count
            + self.camera_update_count
    }

    pub fn reset(&mut self) {
        self.imu_update_count = 0;
        self.gps_update_count = 0;
        self.encoder_update_count = 0;
        self.force_torque_update_count = 0;
        self.lidar_update_count = 0;
        self.camera_update_count = 0;
    }
}

/// System to track sensor update statistics (optional, for debugging)
pub fn track_sensor_stats_system(
    time: Res<Time>,
    mut stats: ResMut<SensorUpdateStats>,
    imu_query: Query<&imu::IMUData, Changed<imu::IMUData>>,
    gps_query: Query<&gps::GPSData, Changed<gps::GPSData>>,
    encoder_query: Query<&encoder::EncoderData, Changed<encoder::EncoderData>>,
    ft_query: Query<&force_torque::ForceTorqueData, Changed<force_torque::ForceTorqueData>>,
    lidar_query: Query<&lidar3d::PointCloud, Changed<lidar3d::PointCloud>>,
) {
    stats.last_update_time = time.elapsed_secs();

    // Count updated sensors
    stats.imu_update_count += imu_query.iter().count() as u64;
    stats.gps_update_count += gps_query.iter().count() as u64;
    stats.encoder_update_count += encoder_query.iter().count() as u64;
    stats.force_torque_update_count += ft_query.iter().count() as u64;
    stats.lidar_update_count += lidar_query.iter().count() as u64;
}

/// Event fired when all sensors have completed their updates for a frame
#[derive(Event)]
pub struct SensorsUpdatedEvent {
    pub frame_number: u32,
    pub update_time: f32,
}

/// System to emit sensor update completion events
pub fn emit_sensors_updated_event(
    time: Res<Time>,
    mut events: EventWriter<SensorsUpdatedEvent>,
    mut frame_counter: Local<u32>,
) {
    *frame_counter += 1;
    events.send(SensorsUpdatedEvent {
        frame_number: *frame_counter,
        update_time: time.elapsed_secs(),
    });
}

/// Configuration for sensor update behavior
#[derive(Resource, Clone)]
pub struct SensorUpdateConfig {
    /// Enable sensor visualization
    pub enable_visualization: bool,
    /// Enable sensor statistics tracking
    pub enable_stats: bool,
    /// Emit sensor update events
    pub emit_events: bool,
    /// Maximum sensor update rate (Hz), 0 = unlimited
    pub max_update_rate: f32,
}

impl Default for SensorUpdateConfig {
    fn default() -> Self {
        Self {
            enable_visualization: true,
            enable_stats: false,
            emit_events: false,
            max_update_rate: 0.0,
        }
    }
}

impl SensorUpdateConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_visualization(mut self, enable: bool) -> Self {
        self.enable_visualization = enable;
        self
    }

    pub fn with_stats(mut self, enable: bool) -> Self {
        self.enable_stats = enable;
        self
    }

    pub fn with_events(mut self, enable: bool) -> Self {
        self.emit_events = enable;
        self
    }

    pub fn with_max_rate(mut self, rate: f32) -> Self {
        self.max_update_rate = rate;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_stats() {
        let mut stats = SensorUpdateStats::new();
        assert_eq!(stats.total_updates(), 0);

        stats.imu_update_count = 10;
        stats.gps_update_count = 5;
        assert_eq!(stats.total_updates(), 15);

        stats.reset();
        assert_eq!(stats.total_updates(), 0);
    }

    #[test]
    fn test_sensor_config() {
        let config = SensorUpdateConfig::new()
            .with_visualization(false)
            .with_stats(true)
            .with_max_rate(100.0);

        assert!(!config.enable_visualization);
        assert!(config.enable_stats);
        assert_eq!(config.max_update_rate, 100.0);
    }

    #[test]
    fn test_sensor_config_defaults() {
        let config = SensorUpdateConfig::default();
        assert!(config.enable_visualization);
        assert!(!config.enable_stats);
        assert!(!config.emit_events);
        assert_eq!(config.max_update_rate, 0.0);
    }

    #[test]
    fn test_sensor_config_builder() {
        let config = SensorUpdateConfig::new()
            .with_visualization(true)
            .with_stats(true)
            .with_events(true)
            .with_max_rate(60.0);

        assert!(config.enable_visualization);
        assert!(config.enable_stats);
        assert!(config.emit_events);
        assert_eq!(config.max_update_rate, 60.0);
    }

    #[test]
    fn test_sensor_stats_incremental() {
        let mut stats = SensorUpdateStats::new();

        stats.imu_update_count += 1;
        assert_eq!(stats.total_updates(), 1);

        stats.gps_update_count += 2;
        assert_eq!(stats.total_updates(), 3);

        stats.force_torque_update_count += 3;
        assert_eq!(stats.total_updates(), 6);
    }

    #[test]
    fn test_sensor_plugin_can_be_created() {
        let _plugin = SensorUpdatePlugin;
        // If this compiles, the plugin structure is valid
    }

    #[test]
    fn test_system_set_enum_values() {
        // Ensure system set variants can be created
        let _update = SensorSystemSet::Update;
        let _viz = SensorSystemSet::Visualization;

        // Test Debug formatting
        let debug_str = format!("{:?}", SensorSystemSet::Update);
        assert!(debug_str.contains("Update"));
    }

    #[test]
    fn test_system_set_equality() {
        assert_eq!(SensorSystemSet::Update, SensorSystemSet::Update);
        assert_ne!(SensorSystemSet::Update, SensorSystemSet::Visualization);
    }
}
