//! Soft body physics simulation using mass-spring model

pub mod cloth;
pub mod material;
pub mod particle;
pub mod rope;
pub mod spring;

use bevy::prelude::*;

// Re-export soft body types for public API
#[allow(unused_imports)]
pub use cloth::Cloth;
#[allow(unused_imports)]
pub use material::SoftBodyMaterial;
#[allow(unused_imports)]
pub use particle::Particle;
pub use particle::ParticleSystem;
#[allow(unused_imports)]
pub use rope::Rope;
#[allow(unused_imports)]
pub use spring::{Spring, SpringSystem};

/// Soft body physics plugin
pub struct SoftBodyPlugin;

impl Plugin for SoftBodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                particle::integrate_particles_system,
                spring::apply_spring_forces_system,
                particle::apply_constraints_system,
                particle::collide_particles_system,
            )
                .chain(),
        )
        .register_type::<ParticleSystem>()
        .register_type::<cloth::Cloth>()
        .register_type::<rope::Rope>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soft_body_plugin() {
        let mut app = App::new();
        app.add_plugins(SoftBodyPlugin);
        // Plugin should register without errors
    }
}
