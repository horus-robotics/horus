pub mod articulated;
pub mod gazebo;
pub mod robot;
pub mod state;
pub mod urdf_loader;

pub use robot::Robot;

// Re-export articulated robot types
pub use articulated::{
    apply_joint_control_system, apply_robot_joint_commands_system, follow_joint_trajectory_system,
    ArticulatedRobotPlugin, IKSolver, JointCommand, JointTrajectory, RobotJointCommands,
    TrajectoryPoint,
};

// Re-export joint state types
pub use state::{
    detect_joint_state_changes_system, get_robot_joint_position, get_robot_joint_positions,
    update_joint_states_system, update_robot_joint_states_system, ArticulatedRobot, JointLink,
    JointRegistry, JointState, JointStateChangedEvent, RobotJointStates,
};

// Re-export Gazebo extension types
pub use gazebo::{
    GazeboExtensionParser, GazeboExtensions, GazeboMaterial, GazeboPhysics, GazeboPlugin,
    GazeboSensor, SensorType,
};
