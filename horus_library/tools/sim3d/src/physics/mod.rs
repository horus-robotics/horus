pub mod advanced;
pub mod benchmarks;
pub mod collider;
pub mod controllers;
pub mod diff_drive;
pub mod gpu_integration;
pub mod joints;
pub mod material;
pub mod rigid_body;
pub mod soft_body;
pub mod world;

// Advanced physics features
pub use advanced::{
    AdvancedPhysicsPlugin, AnisotropicFrictionComponent, BreakableJoint, BreakableJointManager,
    CCDConfig, CCDEnabled, CCDSolverAdvanced, ContactConfig, ContactConfigComponent,
    ContactEventCallbacks, ContactEventData, ContactEventType, ContactForceVisualizer,
    ContactModel, CoulombFriction, FrictionMaterialPairs, FrictionPyramid, JointBreakEvent,
    PenetrationLimiter, SpringDamperBuilder, SpringDamperConstraint, SpringType, SweepTestResult,
};

// Physics benchmarks
pub use benchmarks::{
    BenchmarkConfig, BenchmarkReport, BenchmarkResult, PhysicsBenchmark,
    BenchmarkMetrics, ReferenceAccuracy, ReferenceComparison, BenchmarkPhysicsWorld,
};

// Material system
pub use material::{
    MaterialPreset, AdvancedMaterial, FrictionModel, AnisotropicFriction,
    MaterialInteractionDB, MaterialInteraction, InteractionFlags,
};

// Collider builders and helpers
pub use collider::{
    PhysicsCollider, ColliderShape, ColliderBuilder,
    create_box_collider, create_sphere_collider, create_capsule_collider,
    create_cylinder_collider, create_mesh_collider, create_ground_collider,
};

// Rigid body components
pub use rigid_body::{
    RigidBodyComponent, ContactForce, RigidBodyType, Velocity, ExternalForce,
    ExternalImpulse, Mass, Damping, Sleeping, GravityScale, LockedAxes,
};

// Joint creation and control
pub use joints::{
    PhysicsJoint, JointType,
    create_revolute_joint, create_revolute_joint_with_limits,
    create_prismatic_joint, create_prismatic_joint_with_limits,
    create_fixed_joint, create_spherical_joint,
    add_joint_motor, add_joint_spring,
};

// Controllers
pub use controllers::{PIDController, JointController, ControlMode};

// Differential drive
pub use diff_drive::{DifferentialDrive, CmdVel};

// GPU acceleration
pub use gpu_integration::{GPUPhysicsAdapter, setup_gpu_acceleration};

// Soft body physics
pub use soft_body::{
    SoftBodyPlugin, Particle, ParticleSystem, Spring, SpringSystem,
    Cloth, Rope, SoftBodyMaterial,
};

// Physics world
pub use world::PhysicsWorld;
