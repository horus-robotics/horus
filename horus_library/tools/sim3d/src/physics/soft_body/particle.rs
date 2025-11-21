//! Particle system for soft body physics using Verlet integration

use bevy::prelude::*;

/// Individual particle in a soft body
#[derive(Debug, Clone, Copy, Reflect)]
pub struct Particle {
    /// Current position
    pub position: Vec3,
    /// Previous position (for Verlet integration)
    pub prev_position: Vec3,
    /// Accumulated forces
    pub force: Vec3,
    /// Mass
    pub mass: f32,
    /// Whether this particle is fixed (immovable)
    pub fixed: bool,
    /// Damping coefficient
    pub damping: f32,
}

impl Particle {
    pub fn new(position: Vec3, mass: f32) -> Self {
        Self {
            position,
            prev_position: position,
            force: Vec3::ZERO,
            mass,
            fixed: false,
            damping: 0.99,
        }
    }

    pub fn with_fixed(mut self, fixed: bool) -> Self {
        self.fixed = fixed;
        self
    }

    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }

    /// Get velocity using Verlet integration
    pub fn velocity(&self) -> Vec3 {
        self.position - self.prev_position
    }

    /// Set velocity (updates prev_position)
    pub fn set_velocity(&mut self, velocity: Vec3) {
        self.prev_position = self.position - velocity;
    }

    /// Apply force to particle
    pub fn apply_force(&mut self, force: Vec3) {
        if !self.fixed {
            self.force += force;
        }
    }

    /// Reset accumulated forces
    pub fn reset_forces(&mut self) {
        self.force = Vec3::ZERO;
    }
}

/// Component for soft body particle systems
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ParticleSystem {
    /// All particles
    pub particles: Vec<Particle>,
    /// Global gravity
    pub gravity: Vec3,
    /// Ground plane height
    pub ground_height: f32,
    /// Ground elasticity (0 = no bounce, 1 = perfect bounce)
    pub ground_elasticity: f32,
    /// Friction coefficient
    pub friction: f32,
}

impl ParticleSystem {
    pub fn new(gravity: Vec3) -> Self {
        Self {
            particles: Vec::new(),
            gravity,
            ground_height: 0.0,
            ground_elasticity: 0.3,
            friction: 0.5,
        }
    }

    /// Add a particle
    pub fn add_particle(&mut self, particle: Particle) -> usize {
        self.particles.push(particle);
        self.particles.len() - 1
    }

    /// Get particle count
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Get particle by index
    pub fn get_particle(&self, index: usize) -> Option<&Particle> {
        self.particles.get(index)
    }

    /// Get mutable particle by index
    pub fn get_particle_mut(&mut self, index: usize) -> Option<&mut Particle> {
        self.particles.get_mut(index)
    }

    /// Apply global forces (gravity)
    pub fn apply_global_forces(&mut self) {
        for particle in &mut self.particles {
            if !particle.fixed {
                particle.apply_force(self.gravity * particle.mass);
            }
        }
    }

    /// Reset all particle forces
    pub fn reset_forces(&mut self) {
        for particle in &mut self.particles {
            particle.reset_forces();
        }
    }
}

/// System to integrate particles using Verlet integration
pub fn integrate_particles_system(mut systems: Query<&mut ParticleSystem>, time: Res<Time>) {
    let dt = time.delta_secs();
    let dt_sq = dt * dt;

    for mut system in systems.iter_mut() {
        // Apply global forces
        system.apply_global_forces();

        // Integrate particles
        for particle in &mut system.particles {
            if particle.fixed {
                continue;
            }

            // Verlet integration: x(t+dt) = 2*x(t) - x(t-dt) + a*dt^2
            let acceleration = particle.force / particle.mass;
            let new_position =
                particle.position * 2.0 - particle.prev_position + acceleration * dt_sq;

            // Apply damping
            let velocity = particle.position - particle.prev_position;
            let damped_position = particle.position + velocity * particle.damping;

            particle.prev_position = particle.position;
            particle.position = new_position.lerp(damped_position, 1.0 - particle.damping);
        }

        // Reset forces for next frame
        system.reset_forces();
    }
}

/// System to apply constraints (ground collision, etc.)
pub fn apply_constraints_system(mut systems: Query<&mut ParticleSystem>) {
    for mut system in systems.iter_mut() {
        // Extract values to avoid borrow checker issues
        let ground_height = system.ground_height;
        let ground_elasticity = system.ground_elasticity;
        let friction = system.friction;

        for particle in &mut system.particles {
            if particle.fixed {
                continue;
            }

            // Ground collision
            if particle.position.y < ground_height {
                particle.position.y = ground_height;

                // Apply bounce
                let mut velocity = particle.velocity();
                velocity.y *= -ground_elasticity;

                // Apply friction
                velocity.x *= 1.0 - friction;
                velocity.z *= 1.0 - friction;

                particle.set_velocity(velocity);
            }
        }
    }
}

/// System for particle-particle collisions
pub fn collide_particles_system(mut systems: Query<&mut ParticleSystem>) {
    const PARTICLE_RADIUS: f32 = 0.05;

    for mut system in systems.iter_mut() {
        let particle_count = system.particles.len();

        // Simple O(n^2) collision detection
        for i in 0..particle_count {
            for j in (i + 1)..particle_count {
                // Get positions (can't borrow two mutable particles at once)
                let pos_i = system.particles[i].position;
                let pos_j = system.particles[j].position;
                let fixed_i = system.particles[i].fixed;
                let fixed_j = system.particles[j].fixed;

                let diff = pos_j - pos_i;
                let distance = diff.length();
                let min_distance = PARTICLE_RADIUS * 2.0;

                if distance < min_distance && distance > 0.0001 {
                    let correction = diff / distance * (min_distance - distance) * 0.5;

                    // Apply corrections based on which particles are fixed
                    if !fixed_i && !fixed_j {
                        system.particles[i].position -= correction;
                        system.particles[j].position += correction;
                    } else if !fixed_i {
                        system.particles[i].position -= correction * 2.0;
                    } else if !fixed_j {
                        system.particles[j].position += correction * 2.0;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_creation() {
        let particle = Particle::new(Vec3::ZERO, 1.0);
        assert_eq!(particle.position, Vec3::ZERO);
        assert_eq!(particle.mass, 1.0);
        assert!(!particle.fixed);
    }

    #[test]
    fn test_particle_fixed() {
        let particle = Particle::new(Vec3::ZERO, 1.0).with_fixed(true);
        assert!(particle.fixed);
    }

    #[test]
    fn test_particle_velocity() {
        let mut particle = Particle::new(Vec3::new(1.0, 0.0, 0.0), 1.0);
        particle.prev_position = Vec3::ZERO;

        let velocity = particle.velocity();
        assert_eq!(velocity, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_particle_apply_force() {
        let mut particle = Particle::new(Vec3::ZERO, 1.0);
        particle.apply_force(Vec3::new(10.0, 0.0, 0.0));

        assert_eq!(particle.force, Vec3::new(10.0, 0.0, 0.0));
    }

    #[test]
    fn test_particle_fixed_no_force() {
        let mut particle = Particle::new(Vec3::ZERO, 1.0).with_fixed(true);
        particle.apply_force(Vec3::new(10.0, 0.0, 0.0));

        assert_eq!(particle.force, Vec3::ZERO);
    }

    #[test]
    fn test_particle_system_creation() {
        let system = ParticleSystem::new(Vec3::new(0.0, -9.81, 0.0));
        assert_eq!(system.particle_count(), 0);
        assert_eq!(system.gravity, Vec3::new(0.0, -9.81, 0.0));
    }

    #[test]
    fn test_particle_system_add() {
        let mut system = ParticleSystem::new(Vec3::ZERO);
        let particle = Particle::new(Vec3::ZERO, 1.0);

        let index = system.add_particle(particle);
        assert_eq!(index, 0);
        assert_eq!(system.particle_count(), 1);
    }

    #[test]
    fn test_particle_system_get() {
        let mut system = ParticleSystem::new(Vec3::ZERO);
        system.add_particle(Particle::new(Vec3::new(1.0, 2.0, 3.0), 1.0));

        let particle = system.get_particle(0).unwrap();
        assert_eq!(particle.position, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_particle_system_global_forces() {
        let mut system = ParticleSystem::new(Vec3::new(0.0, -10.0, 0.0));
        system.add_particle(Particle::new(Vec3::ZERO, 2.0));

        system.apply_global_forces();

        let particle = system.get_particle(0).unwrap();
        assert_eq!(particle.force, Vec3::new(0.0, -20.0, 0.0));
    }

    #[test]
    fn test_particle_system_reset_forces() {
        let mut system = ParticleSystem::new(Vec3::new(0.0, -10.0, 0.0));
        system.add_particle(Particle::new(Vec3::ZERO, 1.0));

        system.apply_global_forces();
        system.reset_forces();

        let particle = system.get_particle(0).unwrap();
        assert_eq!(particle.force, Vec3::ZERO);
    }
}
