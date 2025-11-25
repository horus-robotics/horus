pub mod robot;
pub mod world;

// Re-export robot configuration types and presets
pub use robot::{
    RobotConfig, ArticulatedRobotConfig,
    DiffDrivePresets, ManipulatorPresets, QuadrupedPresets, HumanoidPresets,
};
