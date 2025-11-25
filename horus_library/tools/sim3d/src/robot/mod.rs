pub mod articulated;
pub mod gazebo;
pub mod robot;
pub mod state;
pub mod urdf_loader;

pub use robot::Robot;

// Re-export articulated robot types
pub use articulated::{
    JointCommand, RobotJointCommands, TrajectoryPoint, JointTrajectory, IKSolver,
    ArticulatedRobotPlugin, apply_joint_control_system, apply_robot_joint_commands_system,
    follow_joint_trajectory_system,
};

// Re-export joint state types
pub use state::{
    JointState, RobotJointStates, JointLink, ArticulatedRobot, JointStateChangedEvent, JointRegistry,
    update_joint_states_system, update_robot_joint_states_system, detect_joint_state_changes_system,
    get_robot_joint_position, get_robot_joint_positions,
};

// Re-export Gazebo extension types
pub use gazebo::{
    GazeboExtensions, GazeboMaterial, GazeboPhysics, GazeboSensor, SensorType,
    GazeboPlugin, GazeboExtensionParser,
};
