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
pub use advanced::AdvancedPhysicsPlugin;

// Physics benchmarks

// Material system
pub use material::MaterialPreset;

// Collider builders and helpers

// Rigid body components
pub use rigid_body::ContactForce;

// Joint creation and control

// Controllers

// Differential drive

// GPU acceleration

// Soft body physics

// Physics world
pub use world::PhysicsWorld;
