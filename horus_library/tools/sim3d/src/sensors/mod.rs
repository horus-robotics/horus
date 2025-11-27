pub mod camera;
pub mod depth;
pub mod distortion;
pub mod encoder;
pub mod event_camera;
pub mod force_torque;
pub mod gps;
pub mod imu;
pub mod lidar3d;
pub mod noise;
pub mod radar;
pub mod rgbd;
pub mod segmentation;
pub mod sonar;
pub mod tactile;
pub mod thermal;

// Re-export sensor plugins
pub use depth::DepthCameraPlugin;
pub use imu::IMUPlugin;
pub use rgbd::RGBDCameraPlugin;
pub use segmentation::SegmentationCameraPlugin;
pub use tactile::TactileSensorPlugin;
pub use thermal::ThermalCameraPlugin;

// Re-export noise models for external use
pub use noise::{
    patterns as noise_patterns, CombinedNoise, DriftNoise, GaussianNoise, NoiseModel,
    SaltPepperNoise, SensorNoise, UniformNoise,
};

// Re-export segmentation semantic classes
pub use segmentation::{
    classes as predefined_classes, initialize_default_classes, ClassId, SegmentationCamera,
    SegmentationImage, SemanticClass, SemanticClassRegistry,
};

// Re-export camera types
pub use camera::{
    CameraImage, CameraVisualization, DepthCamera as CameraDepth, DepthImage, RGBCamera,
};

// Re-export IMU types
pub use imu::{Gravity, IMUData, IMU};

// Re-export LiDAR types
pub use lidar3d::{LaserScan, Lidar2D, Lidar3D, PointCloud};

// Re-export GPS types
pub use gps::{GPSData, VelocitySmoothingMethod, GPS};

// Re-export encoder types
pub use encoder::{
    AbsoluteEncoder, Encoder, EncoderCalibration, EncoderData, EncoderType, IncrementalEncoder,
    QuadratureEncoder,
};

// Re-export force/torque sensor types
pub use force_torque::{
    FilterType, ForceTorqueCalibration, ForceTorqueData, ForceTorqueFilter, ForceTorqueSensor,
};

// Re-export tactile sensor types
pub use tactile::{
    ContactPoint, GripperForceSensor, TactileData, TactileSensor, TactileSensorType, TaxelReading,
};

// Re-export event camera types
pub use event_camera::{Event, EventCamera, EventStream};

// Re-export radar types
pub use radar::{RadarPoint, RadarPointCloud, RadarSensor};

// Re-export sonar types
pub use sonar::{SonarArray, SonarMeasurement, SonarSensor, SonarType};

// Re-export thermal camera types
pub use thermal::{
    HeatPattern, HeatSource, Temperature, ThermalCamera, ThermalImage, ThermalProperties,
    ThermalStatistics,
};

// Re-export distortion models
pub use distortion::{DistortionModel, LensDistortion};
