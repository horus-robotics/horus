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
    NoiseModel, GaussianNoise, UniformNoise, SaltPepperNoise, DriftNoise,
    CombinedNoise, SensorNoise, patterns as noise_patterns,
};

// Re-export segmentation semantic classes
pub use segmentation::{
    SemanticClass, SemanticClassRegistry, SegmentationCamera, SegmentationImage,
    classes as predefined_classes, ClassId, initialize_default_classes,
};

// Re-export camera types
pub use camera::{RGBCamera, DepthCamera as CameraDepth, CameraImage, DepthImage, CameraVisualization};

// Re-export IMU types
pub use imu::{IMU, IMUData, Gravity};

// Re-export LiDAR types
pub use lidar3d::{Lidar3D, Lidar2D, PointCloud, LaserScan};

// Re-export GPS types
pub use gps::{GPS, GPSData, VelocitySmoothingMethod};

// Re-export encoder types
pub use encoder::{Encoder, EncoderType, EncoderData, IncrementalEncoder, AbsoluteEncoder, QuadratureEncoder, EncoderCalibration};

// Re-export force/torque sensor types
pub use force_torque::{ForceTorqueSensor, ForceTorqueData, ForceTorqueCalibration, ForceTorqueFilter, FilterType};

// Re-export tactile sensor types
pub use tactile::{TactileSensor, TactileSensorType, TaxelReading, TactileData, ContactPoint, GripperForceSensor};

// Re-export event camera types
pub use event_camera::{EventCamera, Event, EventStream};

// Re-export radar types
pub use radar::{RadarSensor, RadarPoint, RadarPointCloud};

// Re-export sonar types
pub use sonar::{SonarSensor, SonarType, SonarMeasurement, SonarArray};

// Re-export thermal camera types
pub use thermal::{ThermalCamera, ThermalImage, ThermalProperties, ThermalStatistics, HeatSource, HeatPattern, Temperature};

// Re-export distortion models
pub use distortion::{LensDistortion, DistortionModel};
