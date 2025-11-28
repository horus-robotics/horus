use bevy::prelude::*;

use crate::physics::diff_drive::DifferentialDrive;
use crate::physics::MaterialPreset;

/// Robot configuration presets
#[derive(Clone, Debug)]
pub struct RobotConfig {
    pub name: String,
    pub base_mass: f32,
    pub base_size: Vec3,
    pub material: MaterialPreset,
}

impl RobotConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_mass: 10.0,
            base_size: Vec3::new(0.4, 0.2, 0.3),
            material: MaterialPreset::aluminum(),
        }
    }

    pub fn with_mass(mut self, mass: f32) -> Self {
        self.base_mass = mass;
        self
    }

    pub fn with_size(mut self, size: Vec3) -> Self {
        self.base_size = size;
        self
    }

    pub fn with_material(mut self, material: MaterialPreset) -> Self {
        self.material = material;
        self
    }
}

/// Articulated robot configuration
#[derive(Clone, Debug)]
pub struct ArticulatedRobotConfig {
    pub name: String,
    pub num_joints: usize,
    pub link_lengths: Vec<f32>,
    pub link_masses: Vec<f32>,
    pub joint_limits: Vec<(f32, f32)>,
    pub max_velocities: Vec<f32>,
    pub max_efforts: Vec<f32>,
}

/// Differential drive robot presets
pub struct DiffDrivePresets;

impl DiffDrivePresets {
    /// TurtleBot3 Burger
    pub fn turtlebot3_burger() -> (RobotConfig, DifferentialDrive) {
        let config = RobotConfig::new("turtlebot3_burger")
            .with_mass(1.0)
            .with_size(Vec3::new(0.14, 0.19, 0.08));

        let diff_drive = DifferentialDrive {
            wheel_separation: 0.16,
            wheel_radius: 0.033,
            max_linear_velocity: 0.22,
            max_angular_velocity: 2.84,
        };

        (config, diff_drive)
    }

    /// TurtleBot3 Waffle
    pub fn turtlebot3_waffle() -> (RobotConfig, DifferentialDrive) {
        let config = RobotConfig::new("turtlebot3_waffle")
            .with_mass(1.8)
            .with_size(Vec3::new(0.281, 0.306, 0.141));

        let diff_drive = DifferentialDrive {
            wheel_separation: 0.287,
            wheel_radius: 0.033,
            max_linear_velocity: 0.26,
            max_angular_velocity: 1.82,
        };

        (config, diff_drive)
    }

    /// Generic small mobile robot
    pub fn small_mobile_robot() -> (RobotConfig, DifferentialDrive) {
        let config = RobotConfig::new("small_mobile_robot")
            .with_mass(5.0)
            .with_size(Vec3::new(0.3, 0.4, 0.2));

        let diff_drive = DifferentialDrive {
            wheel_separation: 0.25,
            wheel_radius: 0.05,
            max_linear_velocity: 0.5,
            max_angular_velocity: 2.0,
        };

        (config, diff_drive)
    }

    /// Warehouse AMR (Autonomous Mobile Robot)
    pub fn warehouse_amr() -> (RobotConfig, DifferentialDrive) {
        let config = RobotConfig::new("warehouse_amr")
            .with_mass(50.0)
            .with_size(Vec3::new(0.6, 0.8, 0.4));

        let diff_drive = DifferentialDrive {
            wheel_separation: 0.5,
            wheel_radius: 0.1,
            max_linear_velocity: 1.5,
            max_angular_velocity: 1.5,
        };

        (config, diff_drive)
    }
}

/// Manipulator robot presets
pub struct ManipulatorPresets;

impl ManipulatorPresets {
    /// UR5 industrial robot
    pub fn ur5() -> ArticulatedRobotConfig {
        ArticulatedRobotConfig {
            name: "ur5".to_string(),
            num_joints: 6,
            link_lengths: vec![0.425, 0.39225, 0.0, 0.0, 0.0, 0.0],
            link_masses: vec![3.7, 8.393, 2.275, 1.219, 1.219, 0.1889],
            joint_limits: vec![
                (-2.0 * std::f32::consts::PI, 2.0 * std::f32::consts::PI),
                (-2.0 * std::f32::consts::PI, 2.0 * std::f32::consts::PI),
                (-std::f32::consts::PI, std::f32::consts::PI),
                (-2.0 * std::f32::consts::PI, 2.0 * std::f32::consts::PI),
                (-2.0 * std::f32::consts::PI, 2.0 * std::f32::consts::PI),
                (-2.0 * std::f32::consts::PI, 2.0 * std::f32::consts::PI),
            ],
            max_velocities: vec![3.15, 3.15, 3.15, 3.2, 3.2, 3.2],
            max_efforts: vec![150.0, 150.0, 150.0, 28.0, 28.0, 28.0],
        }
    }

    /// Franka Emika Panda
    pub fn franka_panda() -> ArticulatedRobotConfig {
        ArticulatedRobotConfig {
            name: "franka_panda".to_string(),
            num_joints: 7, // 7-DOF
            link_lengths: vec![0.333, 0.316, 0.0825, 0.384, 0.0, 0.088, 0.107],
            link_masses: vec![
                4.970684, 0.646926, 3.228604, 3.587895, 1.225946, 1.666555, 0.735522,
            ],
            joint_limits: vec![
                (-2.8973, 2.8973),
                (-1.7628, 1.7628),
                (-2.8973, 2.8973),
                (-3.0718, -0.0698),
                (-2.8973, 2.8973),
                (-0.0175, 3.7525),
                (-2.8973, 2.8973),
            ],
            max_velocities: vec![2.1750, 2.1750, 2.1750, 2.1750, 2.6100, 2.6100, 2.6100],
            max_efforts: vec![87.0, 87.0, 87.0, 87.0, 12.0, 12.0, 12.0],
        }
    }

    /// Simple 3-DOF arm
    pub fn simple_3dof() -> ArticulatedRobotConfig {
        ArticulatedRobotConfig {
            name: "simple_3dof".to_string(),
            num_joints: 3,
            link_lengths: vec![0.3, 0.3, 0.2],
            link_masses: vec![2.0, 1.5, 1.0],
            joint_limits: vec![
                (-std::f32::consts::PI, std::f32::consts::PI),
                (-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0),
                (-std::f32::consts::PI, std::f32::consts::PI),
            ],
            max_velocities: vec![2.0, 2.0, 2.0],
            max_efforts: vec![50.0, 30.0, 20.0],
        }
    }

    /// 6-DOF generic industrial arm
    pub fn generic_6dof() -> ArticulatedRobotConfig {
        ArticulatedRobotConfig {
            name: "generic_6dof".to_string(),
            num_joints: 6,
            link_lengths: vec![0.4, 0.4, 0.1, 0.3, 0.1, 0.1],
            link_masses: vec![5.0, 4.0, 3.0, 2.0, 1.0, 0.5],
            joint_limits: vec![
                (-std::f32::consts::PI, std::f32::consts::PI),
                (-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0),
                (-std::f32::consts::PI, std::f32::consts::PI),
                (-std::f32::consts::PI, std::f32::consts::PI),
                (-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0),
                (-std::f32::consts::PI, std::f32::consts::PI),
            ],
            max_velocities: vec![2.0, 2.0, 2.0, 3.0, 3.0, 3.0],
            max_efforts: vec![100.0, 80.0, 60.0, 40.0, 20.0, 10.0],
        }
    }
}

/// Quadruped robot presets
pub struct QuadrupedPresets;

impl QuadrupedPresets {
    /// Simple quadruped (12 joints: 3 per leg)
    pub fn simple_quadruped() -> RobotConfig {
        RobotConfig::new("simple_quadruped")
            .with_mass(12.0)
            .with_size(Vec3::new(0.5, 0.3, 0.2))
    }

    /// ANYmal-style quadruped
    pub fn anymal_style() -> RobotConfig {
        RobotConfig::new("anymal")
            .with_mass(30.0)
            .with_size(Vec3::new(0.55, 0.44, 0.24))
    }

    /// Spot-style quadruped
    pub fn spot_style() -> RobotConfig {
        RobotConfig::new("spot")
            .with_mass(32.0)
            .with_size(Vec3::new(1.1, 0.8, 0.19))
    }
}

/// Humanoid robot presets
pub struct HumanoidPresets;

impl HumanoidPresets {
    /// Simple humanoid
    pub fn simple_humanoid() -> RobotConfig {
        RobotConfig::new("simple_humanoid")
            .with_mass(50.0)
            .with_size(Vec3::new(0.4, 1.6, 0.2))
    }

    /// NAO-style humanoid
    pub fn nao_style() -> RobotConfig {
        RobotConfig::new("nao")
            .with_mass(5.4)
            .with_size(Vec3::new(0.275, 0.573, 0.125))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robot_config_builder() {
        let config = RobotConfig::new("test_robot")
            .with_mass(15.0)
            .with_size(Vec3::new(1.0, 2.0, 0.5));

        assert_eq!(config.name, "test_robot");
        assert_eq!(config.base_mass, 15.0);
        assert_eq!(config.base_size, Vec3::new(1.0, 2.0, 0.5));
    }

    #[test]
    fn test_robot_config_defaults() {
        let config = RobotConfig::new("default_robot");

        assert_eq!(config.name, "default_robot");
        assert_eq!(config.base_mass, 10.0);
        assert_eq!(config.base_size, Vec3::new(0.4, 0.2, 0.3));
    }

    #[test]
    fn test_robot_config_with_material() {
        let config = RobotConfig::new("steel_robot").with_material(MaterialPreset::steel());

        assert_eq!(config.material.density, MaterialPreset::steel().density);
    }

    #[test]
    fn test_turtlebot3_burger_preset() {
        let (config, diff_drive) = DiffDrivePresets::turtlebot3_burger();

        assert_eq!(config.name, "turtlebot3_burger");
        assert_eq!(config.base_mass, 1.0);
        assert_eq!(diff_drive.wheel_separation, 0.16);
        assert_eq!(diff_drive.wheel_radius, 0.033);
        assert_eq!(diff_drive.max_linear_velocity, 0.22);
    }

    #[test]
    fn test_turtlebot3_waffle_preset() {
        let (config, diff_drive) = DiffDrivePresets::turtlebot3_waffle();

        assert_eq!(config.name, "turtlebot3_waffle");
        assert_eq!(config.base_mass, 1.8);
        assert_eq!(diff_drive.wheel_separation, 0.287);
        assert_eq!(diff_drive.max_linear_velocity, 0.26);
    }

    #[test]
    fn test_small_mobile_robot_preset() {
        let (config, diff_drive) = DiffDrivePresets::small_mobile_robot();

        assert_eq!(config.name, "small_mobile_robot");
        assert_eq!(config.base_mass, 5.0);
        assert_eq!(diff_drive.wheel_radius, 0.05);
    }

    #[test]
    fn test_warehouse_amr_preset() {
        let (config, diff_drive) = DiffDrivePresets::warehouse_amr();

        assert_eq!(config.name, "warehouse_amr");
        assert_eq!(config.base_mass, 50.0);
        assert_eq!(diff_drive.max_linear_velocity, 1.5);
    }

    #[test]
    fn test_ur5_manipulator_preset() {
        let config = ManipulatorPresets::ur5();

        assert_eq!(config.name, "ur5");
        assert_eq!(config.num_joints, 6);
        assert_eq!(config.link_lengths.len(), 6);
        assert_eq!(config.link_masses.len(), 6);
        assert_eq!(config.joint_limits.len(), 6);
        assert_eq!(config.max_velocities.len(), 6);
        assert_eq!(config.max_efforts.len(), 6);

        // UR5 first link length
        assert_eq!(config.link_lengths[0], 0.425);
    }

    #[test]
    fn test_franka_panda_preset() {
        let config = ManipulatorPresets::franka_panda();

        assert_eq!(config.name, "franka_panda");
        assert_eq!(config.num_joints, 7); // 7-DOF arm
        assert_eq!(config.link_lengths.len(), 7);
        assert_eq!(config.link_masses.len(), 7);
        assert_eq!(config.joint_limits.len(), 7);
    }

    #[test]
    fn test_simple_3dof_preset() {
        let config = ManipulatorPresets::simple_3dof();

        assert_eq!(config.name, "simple_3dof");
        assert_eq!(config.num_joints, 3);
        assert_eq!(config.link_lengths, vec![0.3, 0.3, 0.2]);
    }

    #[test]
    fn test_generic_6dof_preset() {
        let config = ManipulatorPresets::generic_6dof();

        assert_eq!(config.name, "generic_6dof");
        assert_eq!(config.num_joints, 6);

        // Verify joint limits are symmetric for rotation joints
        for (i, (min, max)) in config.joint_limits.iter().enumerate() {
            assert!(
                *max > *min,
                "Joint {} has invalid limits: min={}, max={}",
                i,
                min,
                max
            );
        }
    }

    #[test]
    fn test_quadruped_presets() {
        let simple = QuadrupedPresets::simple_quadruped();
        assert_eq!(simple.name, "simple_quadruped");
        assert_eq!(simple.base_mass, 12.0);

        let anymal = QuadrupedPresets::anymal_style();
        assert_eq!(anymal.name, "anymal");
        assert_eq!(anymal.base_mass, 30.0);

        let spot = QuadrupedPresets::spot_style();
        assert_eq!(spot.name, "spot");
        assert_eq!(spot.base_mass, 32.0);
    }

    #[test]
    fn test_humanoid_presets() {
        let simple = HumanoidPresets::simple_humanoid();
        assert_eq!(simple.name, "simple_humanoid");
        assert_eq!(simple.base_mass, 50.0);

        let nao = HumanoidPresets::nao_style();
        assert_eq!(nao.name, "nao");
        assert_eq!(nao.base_mass, 5.4);
    }

    #[test]
    fn test_articulated_robot_config_consistency() {
        // Verify that all ArticulatedRobotConfig presets have consistent array lengths
        let presets = vec![
            ManipulatorPresets::ur5(),
            ManipulatorPresets::franka_panda(),
            ManipulatorPresets::simple_3dof(),
            ManipulatorPresets::generic_6dof(),
        ];

        for config in presets {
            assert_eq!(
                config.link_lengths.len(),
                config.num_joints,
                "{} link_lengths mismatch",
                config.name
            );
            assert_eq!(
                config.link_masses.len(),
                config.num_joints,
                "{} link_masses mismatch",
                config.name
            );
            assert_eq!(
                config.joint_limits.len(),
                config.num_joints,
                "{} joint_limits mismatch",
                config.name
            );
            assert_eq!(
                config.max_velocities.len(),
                config.num_joints,
                "{} max_velocities mismatch",
                config.name
            );
            assert_eq!(
                config.max_efforts.len(),
                config.num_joints,
                "{} max_efforts mismatch",
                config.name
            );
        }
    }

    #[test]
    fn test_diff_drive_kinematics_sanity() {
        // Verify wheel separation > wheel radius for all presets
        let presets = vec![
            DiffDrivePresets::turtlebot3_burger(),
            DiffDrivePresets::turtlebot3_waffle(),
            DiffDrivePresets::small_mobile_robot(),
            DiffDrivePresets::warehouse_amr(),
        ];

        for (config, diff_drive) in presets {
            assert!(
                diff_drive.wheel_separation > diff_drive.wheel_radius * 2.0,
                "{}: wheel separation ({}) should be > 2x wheel radius ({})",
                config.name,
                diff_drive.wheel_separation,
                diff_drive.wheel_radius
            );
            assert!(
                diff_drive.max_linear_velocity > 0.0,
                "{}: max_linear_velocity should be positive",
                config.name
            );
            assert!(
                diff_drive.max_angular_velocity > 0.0,
                "{}: max_angular_velocity should be positive",
                config.name
            );
        }
    }
}
