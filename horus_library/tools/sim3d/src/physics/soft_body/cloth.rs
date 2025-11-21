//! Cloth simulation using 2D particle grid

use super::{
    material::SoftBodyMaterial,
    particle::{Particle, ParticleSystem},
    spring::{Spring, SpringSystem},
};
use bevy::prelude::*;

/// Cloth component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Cloth {
    /// Grid width (number of particles)
    pub width: usize,
    /// Grid height (number of particles)
    pub height: usize,
    /// Physical width in meters
    pub physical_width: f32,
    /// Physical height in meters
    pub physical_height: f32,
    /// Material
    pub material: SoftBodyMaterial,
    /// Thickness
    pub thickness: f32,
}

impl Cloth {
    pub fn new(width: usize, height: usize, physical_width: f32, physical_height: f32) -> Self {
        Self {
            width,
            height,
            physical_width,
            physical_height,
            material: SoftBodyMaterial::cloth(),
            thickness: 0.001, // 1mm
        }
    }

    pub fn with_material(mut self, material: SoftBodyMaterial) -> Self {
        self.material = material;
        self
    }

    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    /// Create particle and spring systems for this cloth
    pub fn create_systems(&self, origin: Vec3, normal: Vec3) -> (ParticleSystem, SpringSystem) {
        let mut particle_system = ParticleSystem::new(Vec3::new(0.0, -9.81, 0.0));
        let mut spring_system = SpringSystem::new();

        // Calculate basis vectors for the cloth plane
        let right = if normal.abs_diff_eq(Vec3::Y, 0.001) {
            Vec3::X
        } else {
            Vec3::Y.cross(normal).normalize()
        };
        let up = normal.cross(right).normalize();

        let dx = self.physical_width / (self.width - 1) as f32;
        let dy = self.physical_height / (self.height - 1) as f32;

        // Calculate particle mass (area * thickness * density / particle_count)
        let total_volume = self.physical_width * self.physical_height * self.thickness;
        let total_mass = self.material.calculate_mass(total_volume);
        let particle_mass = total_mass / (self.width * self.height) as f32;

        // Calculate spring stiffness
        let cross_section = dx * self.thickness;
        let structural_stiffness = self.material.calculate_stiffness(cross_section, dx);
        let shear_stiffness = structural_stiffness * 0.5; // Shear springs are weaker
        let bend_stiffness = structural_stiffness * 0.2; // Bend springs resist folding

        // Create particles in grid
        for y in 0..self.height {
            for x in 0..self.width {
                let pos_x = x as f32 * dx;
                let pos_y = y as f32 * dy;
                let position = origin + right * pos_x + up * pos_y;

                let mut particle =
                    Particle::new(position, particle_mass).with_damping(self.material.damping);

                // Fix top row for hanging cloth
                if y == 0 {
                    particle = particle.with_fixed(true);
                }

                particle_system.add_particle(particle);
            }
        }

        // Helper to get particle index
        let get_index = |x: usize, y: usize| -> usize { y * self.width + x };

        // Create structural springs (horizontal and vertical)
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = get_index(x, y);

                // Horizontal spring
                if x < self.width - 1 {
                    let idx_right = get_index(x + 1, y);
                    spring_system.add_spring(
                        Spring::new(idx, idx_right, dx, structural_stiffness)
                            .with_damping(self.material.damping),
                    );
                }

                // Vertical spring
                if y < self.height - 1 {
                    let idx_down = get_index(x, y + 1);
                    spring_system.add_spring(
                        Spring::new(idx, idx_down, dy, structural_stiffness)
                            .with_damping(self.material.damping),
                    );
                }

                // Shear springs (diagonals)
                if x < self.width - 1 && y < self.height - 1 {
                    let idx_diag1 = get_index(x + 1, y + 1);
                    let diag_length = (dx * dx + dy * dy).sqrt();
                    spring_system.add_spring(
                        Spring::new(idx, idx_diag1, diag_length, shear_stiffness)
                            .with_damping(self.material.damping),
                    );
                }

                if x > 0 && y < self.height - 1 {
                    let idx_diag2 = get_index(x - 1, y + 1);
                    let diag_length = (dx * dx + dy * dy).sqrt();
                    spring_system.add_spring(
                        Spring::new(idx, idx_diag2, diag_length, shear_stiffness)
                            .with_damping(self.material.damping),
                    );
                }

                // Bend springs (skip one particle)
                if x < self.width - 2 {
                    let idx_right2 = get_index(x + 2, y);
                    spring_system.add_spring(
                        Spring::new(idx, idx_right2, dx * 2.0, bend_stiffness)
                            .with_damping(self.material.damping),
                    );
                }

                if y < self.height - 2 {
                    let idx_down2 = get_index(x, y + 2);
                    spring_system.add_spring(
                        Spring::new(idx, idx_down2, dy * 2.0, bend_stiffness)
                            .with_damping(self.material.damping),
                    );
                }
            }
        }

        (particle_system, spring_system)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloth_creation() {
        let cloth = Cloth::new(10, 10, 1.0, 1.0);
        assert_eq!(cloth.width, 10);
        assert_eq!(cloth.height, 10);
        assert_eq!(cloth.physical_width, 1.0);
        assert_eq!(cloth.physical_height, 1.0);
    }

    #[test]
    fn test_cloth_with_material() {
        let cloth = Cloth::new(5, 5, 1.0, 1.0).with_material(SoftBodyMaterial::rubber());
        assert_eq!(cloth.material.density, SoftBodyMaterial::rubber().density);
    }

    #[test]
    fn test_cloth_create_systems() {
        let cloth = Cloth::new(3, 3, 1.0, 1.0);
        let origin = Vec3::ZERO;
        let normal = Vec3::Z;

        let (particle_system, spring_system) = cloth.create_systems(origin, normal);

        // Should have width * height particles
        assert_eq!(particle_system.particle_count(), 9);

        // Top row should be fixed
        assert!(particle_system.get_particle(0).unwrap().fixed);
        assert!(particle_system.get_particle(1).unwrap().fixed);
        assert!(particle_system.get_particle(2).unwrap().fixed);

        // Other rows should not be fixed
        assert!(!particle_system.get_particle(3).unwrap().fixed);

        // Should have multiple types of springs
        // For 3x3: 2*3 horizontal + 3*2 vertical = 12 structural
        // + 2*2 diagonals * 2 = 8 shear
        // + 1*3 horizontal + 3*1 vertical = 4 bend
        // = 24 total springs
        assert!(spring_system.spring_count() > 10);
    }

    #[test]
    fn test_cloth_particle_positions() {
        let cloth = Cloth::new(3, 2, 2.0, 1.0);
        let origin = Vec3::ZERO;
        let normal = Vec3::Z;

        let (particle_system, _) = cloth.create_systems(origin, normal);

        // Check corner positions
        let p00 = particle_system.get_particle(0).unwrap().position;
        let p20 = particle_system.get_particle(2).unwrap().position;
        let p01 = particle_system.get_particle(3).unwrap().position;

        // Check spacing
        let dx = (p20 - p00).length();
        let dy = (p01 - p00).length();

        assert!((dx - 2.0).abs() < 0.01); // Width spacing
        assert!((dy - 1.0).abs() < 0.01); // Height spacing
    }

    #[test]
    fn test_cloth_with_thickness() {
        let cloth = Cloth::new(5, 5, 1.0, 1.0).with_thickness(0.002);
        assert_eq!(cloth.thickness, 0.002);
    }
}
