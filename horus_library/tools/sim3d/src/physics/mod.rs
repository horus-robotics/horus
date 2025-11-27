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
    BenchmarkConfig, BenchmarkMetrics, BenchmarkPhysicsWorld, BenchmarkReport, BenchmarkResult,
    PhysicsBenchmark, ReferenceAccuracy, ReferenceComparison,
};

// Material system
pub use material::{
    AdvancedMaterial, AnisotropicFriction, FrictionModel, InteractionFlags, MaterialInteraction,
    MaterialInteractionDB, MaterialPreset,
};

// Collider builders and helpers
pub use collider::{
    create_box_collider, create_capsule_collider, create_cylinder_collider, create_ground_collider,
    create_mesh_collider, create_sphere_collider, ColliderBuilder, ColliderShape, PhysicsCollider,
};

// Rigid body components
pub use rigid_body::{
    ContactForce, Damping, ExternalForce, ExternalImpulse, GravityScale, LockedAxes, Mass,
    RigidBodyComponent, RigidBodyType, Sleeping, Velocity,
};

// Joint creation and control
pub use joints::{
    add_joint_motor, add_joint_spring, create_fixed_joint, create_prismatic_joint,
    create_prismatic_joint_with_limits, create_revolute_joint, create_revolute_joint_with_limits,
    create_spherical_joint, JointType, PhysicsJoint,
};

// Controllers
pub use controllers::{ControlMode, JointController, PIDController};

// Differential drive
pub use diff_drive::{CmdVel, DifferentialDrive};

// GPU acceleration
pub use gpu_integration::{setup_gpu_acceleration, GPUPhysicsAdapter};

// Soft body physics
pub use soft_body::{
    Cloth, Particle, ParticleSystem, Rope, SoftBodyMaterial, SoftBodyPlugin, Spring, SpringSystem,
};

// Physics world
pub use world::PhysicsWorld;
