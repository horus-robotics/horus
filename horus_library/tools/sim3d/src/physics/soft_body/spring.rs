//! Spring constraints for connecting particles

use super::particle::ParticleSystem;
use bevy::prelude::*;

/// Spring connecting two particles
#[derive(Debug, Clone, Copy, Reflect)]
pub struct Spring {
    /// Index of first particle
    pub particle_a: usize,
    /// Index of second particle
    pub particle_b: usize,
    /// Rest length
    pub rest_length: f32,
    /// Spring stiffness (k)
    pub stiffness: f32,
    /// Damping coefficient
    pub damping: f32,
}

impl Spring {
    pub fn new(particle_a: usize, particle_b: usize, rest_length: f32, stiffness: f32) -> Self {
        Self {
            particle_a,
            particle_b,
            rest_length,
            stiffness,
            damping: 0.1,
        }
    }

    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }

    /// Calculate spring force using Hooke's law
    pub fn calculate_force(&self, system: &ParticleSystem) -> Option<(Vec3, Vec3)> {
        let particle_a = system.get_particle(self.particle_a)?;
        let particle_b = system.get_particle(self.particle_b)?;

        let diff = particle_b.position - particle_a.position;
        let distance = diff.length();

        if distance < 0.0001 {
            return Some((Vec3::ZERO, Vec3::ZERO));
        }

        let direction = diff / distance;

        // Hooke's law: F = -k * (x - rest_length)
        let spring_force = direction * self.stiffness * (distance - self.rest_length);

        // Damping force
        let relative_velocity = particle_b.velocity() - particle_a.velocity();
        let damping_force = direction * direction.dot(relative_velocity) * self.damping;

        let total_force = spring_force + damping_force;

        Some((total_force, -total_force))
    }
}

/// Component for spring systems
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct SpringSystem {
    /// All springs
    pub springs: Vec<Spring>,
}

impl SpringSystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a spring
    pub fn add_spring(&mut self, spring: Spring) {
        self.springs.push(spring);
    }

    /// Add spring from particle indices
    pub fn connect(
        &mut self,
        particle_a: usize,
        particle_b: usize,
        rest_length: f32,
        stiffness: f32,
    ) {
        self.add_spring(Spring::new(particle_a, particle_b, rest_length, stiffness));
    }

    /// Get spring count
    pub fn spring_count(&self) -> usize {
        self.springs.len()
    }
}

/// System to apply spring forces
pub fn apply_spring_forces_system(mut query: Query<(&mut ParticleSystem, &SpringSystem)>) {
    for (mut particle_system, spring_system) in query.iter_mut() {
        for spring in &spring_system.springs {
            if let Some((force_a, force_b)) = spring.calculate_force(&particle_system) {
                if let Some(particle_a) = particle_system.get_particle_mut(spring.particle_a) {
                    particle_a.apply_force(force_a);
                }
                if let Some(particle_b) = particle_system.get_particle_mut(spring.particle_b) {
                    particle_b.apply_force(force_b);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::soft_body::particle::Particle;

    #[test]
    fn test_spring_creation() {
        let spring = Spring::new(0, 1, 1.0, 100.0);
        assert_eq!(spring.particle_a, 0);
        assert_eq!(spring.particle_b, 1);
        assert_eq!(spring.rest_length, 1.0);
        assert_eq!(spring.stiffness, 100.0);
    }

    #[test]
    fn test_spring_force_at_rest() {
        let mut system = ParticleSystem::new(Vec3::ZERO);
        system.add_particle(Particle::new(Vec3::ZERO, 1.0));
        system.add_particle(Particle::new(Vec3::new(1.0, 0.0, 0.0), 1.0));

        let spring = Spring::new(0, 1, 1.0, 100.0);
        let (force_a, force_b) = spring.calculate_force(&system).unwrap();

        // At rest length, force should be near zero
        assert!(force_a.length() < 0.1);
        assert!(force_b.length() < 0.1);
    }

    #[test]
    fn test_spring_force_stretched() {
        let mut system = ParticleSystem::new(Vec3::ZERO);
        system.add_particle(Particle::new(Vec3::ZERO, 1.0));
        system.add_particle(Particle::new(Vec3::new(2.0, 0.0, 0.0), 1.0));

        let spring = Spring::new(0, 1, 1.0, 100.0);
        let (force_a, force_b) = spring.calculate_force(&system).unwrap();

        // Stretched by 1.0, should pull with force of 100.0
        assert!((force_a.x - 100.0).abs() < 0.1);
        assert!((force_b.x + 100.0).abs() < 0.1);
    }

    #[test]
    fn test_spring_force_compressed() {
        let mut system = ParticleSystem::new(Vec3::ZERO);
        system.add_particle(Particle::new(Vec3::ZERO, 1.0));
        system.add_particle(Particle::new(Vec3::new(0.5, 0.0, 0.0), 1.0));

        let spring = Spring::new(0, 1, 1.0, 100.0);
        let (force_a, force_b) = spring.calculate_force(&system).unwrap();

        // Compressed by 0.5, should push with force of 50.0
        assert!((force_a.x + 50.0).abs() < 0.1);
        assert!((force_b.x - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_spring_system_creation() {
        let system = SpringSystem::new();
        assert_eq!(system.spring_count(), 0);
    }

    #[test]
    fn test_spring_system_add() {
        let mut system = SpringSystem::new();
        system.add_spring(Spring::new(0, 1, 1.0, 100.0));

        assert_eq!(system.spring_count(), 1);
    }

    #[test]
    fn test_spring_system_connect() {
        let mut system = SpringSystem::new();
        system.connect(0, 1, 1.0, 100.0);

        assert_eq!(system.spring_count(), 1);
        assert_eq!(system.springs[0].particle_a, 0);
        assert_eq!(system.springs[0].particle_b, 1);
    }

    #[test]
    fn test_spring_with_damping() {
        let spring = Spring::new(0, 1, 1.0, 100.0).with_damping(0.5);
        assert_eq!(spring.damping, 0.5);
    }
}
