//! Rope simulation using particle-spring chain

use super::{
    material::SoftBodyMaterial,
    particle::{Particle, ParticleSystem},
    spring::{Spring, SpringSystem},
};
use bevy::prelude::*;

/// Rope component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Rope {
    /// Number of segments
    pub segment_count: usize,
    /// Total length
    pub length: f32,
    /// Material
    pub material: SoftBodyMaterial,
    /// Segment radius
    pub radius: f32,
}

impl Rope {
    pub fn new(segment_count: usize, length: f32, radius: f32) -> Self {
        Self {
            segment_count,
            length,
            material: SoftBodyMaterial::rope(),
            radius,
        }
    }

    pub fn with_material(mut self, material: SoftBodyMaterial) -> Self {
        self.material = material;
        self
    }

    /// Create particle and spring systems for this rope
    pub fn create_systems(&self, start: Vec3, end: Vec3) -> (ParticleSystem, SpringSystem) {
        let mut particle_system = ParticleSystem::new(Vec3::new(0.0, -9.81, 0.0));
        let mut spring_system = SpringSystem::new();

        let segment_length = self.length / self.segment_count as f32;
        let direction = (end - start).normalize();

        // Calculate particle mass (cylinder volume * density)
        let volume = std::f32::consts::PI * self.radius * self.radius * segment_length;
        let mass = self.material.calculate_mass(volume);

        // Calculate spring stiffness
        let cross_section = std::f32::consts::PI * self.radius * self.radius;
        let stiffness = self
            .material
            .calculate_stiffness(cross_section, segment_length);

        // Create particles
        for i in 0..=self.segment_count {
            let t = i as f32 / self.segment_count as f32;
            let position = start + direction * self.length * t;

            let mut particle = Particle::new(position, mass).with_damping(self.material.damping);

            // Fix first and last particles
            if i == 0 || i == self.segment_count {
                particle = particle.with_fixed(true);
            }

            particle_system.add_particle(particle);
        }

        // Create springs between adjacent particles
        for i in 0..self.segment_count {
            spring_system.add_spring(
                Spring::new(i, i + 1, segment_length, stiffness)
                    .with_damping(self.material.damping),
            );
        }

        (particle_system, spring_system)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rope_creation() {
        let rope = Rope::new(10, 5.0, 0.01);
        assert_eq!(rope.segment_count, 10);
        assert_eq!(rope.length, 5.0);
        assert_eq!(rope.radius, 0.01);
    }

    #[test]
    fn test_rope_with_material() {
        let rope = Rope::new(10, 5.0, 0.01).with_material(SoftBodyMaterial::cloth());
        assert_eq!(rope.material.density, SoftBodyMaterial::cloth().density);
    }

    #[test]
    fn test_rope_create_systems() {
        let rope = Rope::new(5, 5.0, 0.01);
        let start = Vec3::ZERO;
        let end = Vec3::new(5.0, 0.0, 0.0);

        let (particle_system, spring_system) = rope.create_systems(start, end);

        // Should have segment_count + 1 particles
        assert_eq!(particle_system.particle_count(), 6);

        // Should have segment_count springs
        assert_eq!(spring_system.spring_count(), 5);

        // First and last particles should be fixed
        assert!(particle_system.get_particle(0).unwrap().fixed);
        assert!(particle_system.get_particle(5).unwrap().fixed);
        assert!(!particle_system.get_particle(2).unwrap().fixed);
    }

    #[test]
    fn test_rope_particle_positions() {
        let rope = Rope::new(4, 4.0, 0.01);
        let start = Vec3::ZERO;
        let end = Vec3::new(4.0, 0.0, 0.0);

        let (particle_system, _) = rope.create_systems(start, end);

        // Check particle positions are evenly spaced
        assert_eq!(particle_system.get_particle(0).unwrap().position.x, 0.0);
        assert_eq!(particle_system.get_particle(1).unwrap().position.x, 1.0);
        assert_eq!(particle_system.get_particle(2).unwrap().position.x, 2.0);
        assert_eq!(particle_system.get_particle(3).unwrap().position.x, 3.0);
        assert_eq!(particle_system.get_particle(4).unwrap().position.x, 4.0);
    }

    #[test]
    fn test_rope_spring_lengths() {
        let rope = Rope::new(5, 5.0, 0.01);
        let start = Vec3::ZERO;
        let end = Vec3::new(5.0, 0.0, 0.0);

        let (_, spring_system) = rope.create_systems(start, end);

        // Each spring should have rest length of 1.0 (5.0 / 5)
        for spring in &spring_system.springs {
            assert!((spring.rest_length - 1.0).abs() < 0.001);
        }
    }
}
