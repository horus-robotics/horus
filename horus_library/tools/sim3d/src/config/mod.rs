pub mod robot;
pub mod world;

// Re-export robot configuration types and presets
pub use robot::{
    ArticulatedRobotConfig, DiffDrivePresets, HumanoidPresets, ManipulatorPresets,
    QuadrupedPresets, RobotConfig,
};
