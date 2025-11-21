//! Soft body material properties

use bevy::prelude::Reflect;

/// Soft body material properties
#[derive(Debug, Clone, Copy, Reflect)]
pub struct SoftBodyMaterial {
    /// Density (kg/m³)
    pub density: f32,
    /// Young's modulus (stiffness) in Pa
    pub youngs_modulus: f32,
    /// Poisson's ratio (lateral strain / axial strain)
    pub poissons_ratio: f32,
    /// Damping coefficient (0-1)
    pub damping: f32,
    /// Friction coefficient
    pub friction: f32,
}

impl SoftBodyMaterial {
    pub fn new(density: f32, youngs_modulus: f32) -> Self {
        Self {
            density,
            youngs_modulus,
            poissons_ratio: 0.3,
            damping: 0.1,
            friction: 0.5,
        }
    }

    /// Rubber material
    pub fn rubber() -> Self {
        Self {
            density: 1100.0,        // kg/m³
            youngs_modulus: 0.01e9, // 0.01 GPa
            poissons_ratio: 0.5,    // Nearly incompressible
            damping: 0.2,
            friction: 0.9,
        }
    }

    /// Cloth material (cotton)
    pub fn cloth() -> Self {
        Self {
            density: 1500.0,
            youngs_modulus: 0.001e9, // Very flexible
            poissons_ratio: 0.3,
            damping: 0.3,
            friction: 0.6,
        }
    }

    /// Rope/cable material (nylon)
    pub fn rope() -> Self {
        Self {
            density: 1100.0,
            youngs_modulus: 2.0e9, // 2 GPa
            poissons_ratio: 0.4,
            damping: 0.15,
            friction: 0.4,
        }
    }

    /// Foam material
    pub fn foam() -> Self {
        Self {
            density: 50.0,            // Very light
            youngs_modulus: 0.0001e9, // Very soft
            poissons_ratio: 0.2,
            damping: 0.5, // High damping
            friction: 0.7,
        }
    }

    /// Jello/gel material
    pub fn jello() -> Self {
        Self {
            density: 1000.0,
            youngs_modulus: 0.00001e9, // Extremely soft
            poissons_ratio: 0.49,      // Nearly incompressible
            damping: 0.4,
            friction: 0.3,
        }
    }

    /// Calculate spring stiffness from material properties
    pub fn calculate_stiffness(&self, cross_section_area: f32, rest_length: f32) -> f32 {
        // k = (E * A) / L
        (self.youngs_modulus * cross_section_area) / rest_length
    }

    /// Calculate particle mass from volume
    pub fn calculate_mass(&self, volume: f32) -> f32 {
        self.density * volume
    }

    /// Get shear modulus from Young's modulus and Poisson's ratio
    pub fn shear_modulus(&self) -> f32 {
        self.youngs_modulus / (2.0 * (1.0 + self.poissons_ratio))
    }

    /// Get bulk modulus from Young's modulus and Poisson's ratio
    pub fn bulk_modulus(&self) -> f32 {
        self.youngs_modulus / (3.0 * (1.0 - 2.0 * self.poissons_ratio))
    }
}

impl Default for SoftBodyMaterial {
    fn default() -> Self {
        Self::rubber()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_creation() {
        let material = SoftBodyMaterial::new(1000.0, 1e9);
        assert_eq!(material.density, 1000.0);
        assert_eq!(material.youngs_modulus, 1e9);
    }

    #[test]
    fn test_rubber_preset() {
        let rubber = SoftBodyMaterial::rubber();
        assert_eq!(rubber.density, 1100.0);
        assert!(rubber.poissons_ratio > 0.49); // Nearly incompressible
    }

    #[test]
    fn test_cloth_preset() {
        let cloth = SoftBodyMaterial::cloth();
        assert!(cloth.youngs_modulus < SoftBodyMaterial::rubber().youngs_modulus);
    }

    #[test]
    fn test_rope_preset() {
        let rope = SoftBodyMaterial::rope();
        assert!(rope.youngs_modulus > SoftBodyMaterial::cloth().youngs_modulus);
    }

    #[test]
    fn test_foam_preset() {
        let foam = SoftBodyMaterial::foam();
        assert!(foam.density < 100.0); // Very light
        assert!(foam.damping > 0.3); // High damping
    }

    #[test]
    fn test_calculate_stiffness() {
        let material = SoftBodyMaterial::new(1000.0, 1e9);
        let area = 0.01; // 1 cm²
        let length = 1.0; // 1 meter

        let stiffness = material.calculate_stiffness(area, length);
        assert_eq!(stiffness, 1e9 * 0.01 / 1.0);
    }

    #[test]
    fn test_calculate_mass() {
        let material = SoftBodyMaterial::new(1000.0, 1e9);
        let volume = 0.001; // 1 liter

        let mass = material.calculate_mass(volume);
        assert_eq!(mass, 1.0); // 1 kg
    }

    #[test]
    fn test_shear_modulus() {
        let material = SoftBodyMaterial::new(1000.0, 1e9);
        let shear = material.shear_modulus();

        // G = E / (2 * (1 + v))
        let expected = 1e9 / (2.0 * (1.0 + material.poissons_ratio));
        assert!((shear - expected).abs() < 1.0);
    }

    #[test]
    fn test_bulk_modulus() {
        let material = SoftBodyMaterial::new(1000.0, 1e9);
        let bulk = material.bulk_modulus();

        // K = E / (3 * (1 - 2*v))
        let expected = 1e9 / (3.0 * (1.0 - 2.0 * material.poissons_ratio));
        assert!((bulk - expected).abs() < 1.0);
    }

    #[test]
    fn test_default_material() {
        let material = SoftBodyMaterial::default();
        // Should be rubber
        assert_eq!(material.density, SoftBodyMaterial::rubber().density);
    }
}
