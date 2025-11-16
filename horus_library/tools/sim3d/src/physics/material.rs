/// Physics material properties for Rapier3D simulation
#[derive(Clone, Copy, Debug)]
pub struct MaterialPreset {
    /// Friction coefficient (0.0 = frictionless, 1.0 = high friction)
    pub friction: f32,
    /// Restitution/bounciness (0.0 = no bounce, 1.0 = perfect bounce)
    pub restitution: f32,
    /// Density in kg/m³ (optional, for automatic mass calculation)
    pub density: Option<f32>,
}

impl MaterialPreset {
    /// Create a custom material preset
    pub fn new(friction: f32, restitution: f32) -> Self {
        Self {
            friction,
            restitution,
            density: None,
        }
    }

    /// Create a custom material with density
    pub fn with_density(friction: f32, restitution: f32, density: f32) -> Self {
        Self {
            friction,
            restitution,
            density: Some(density),
        }
    }

    /// Concrete - rough, no bounce
    pub fn concrete() -> Self {
        Self::with_density(0.9, 0.0, 2400.0)
    }

    /// Wood - medium friction, low bounce
    pub fn wood() -> Self {
        Self::with_density(0.6, 0.1, 700.0)
    }

    /// Plastic - smooth, some bounce
    pub fn plastic() -> Self {
        Self::with_density(0.4, 0.3, 1200.0)
    }

    /// Steel - low friction, low bounce
    pub fn steel() -> Self {
        Self::with_density(0.3, 0.05, 7850.0)
    }

    /// Aluminum - low friction, low bounce, lighter than steel
    pub fn aluminum() -> Self {
        Self::with_density(0.35, 0.05, 2700.0)
    }

    /// Ice - very low friction, low bounce
    pub fn ice() -> Self {
        Self::with_density(0.05, 0.1, 917.0)
    }

    /// Rubber - high friction, high bounce
    pub fn rubber() -> Self {
        Self::with_density(0.9, 0.8, 1100.0)
    }

    /// Glass - low friction, medium bounce
    pub fn glass() -> Self {
        Self::with_density(0.2, 0.4, 2500.0)
    }

    /// Stone - rough, no bounce
    pub fn stone() -> Self {
        Self::with_density(0.8, 0.0, 2500.0)
    }

    /// Metal (generic) - medium friction, low bounce
    pub fn metal() -> Self {
        Self::with_density(0.4, 0.05, 7000.0)
    }
}

impl Default for MaterialPreset {
    fn default() -> Self {
        Self::concrete()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_presets() {
        let concrete = MaterialPreset::concrete();
        assert_eq!(concrete.friction, 0.9);
        assert_eq!(concrete.restitution, 0.0);
        assert_eq!(concrete.density, Some(2400.0));

        let ice = MaterialPreset::ice();
        assert!(ice.friction < 0.1); // Very low friction
        assert!(ice.restitution > 0.0);

        let rubber = MaterialPreset::rubber();
        assert!(rubber.friction > 0.8); // High friction
        assert!(rubber.restitution > 0.7); // High bounce
    }

    #[test]
    fn test_custom_material() {
        let custom = MaterialPreset::new(0.5, 0.2);
        assert_eq!(custom.friction, 0.5);
        assert_eq!(custom.restitution, 0.2);
        assert_eq!(custom.density, None);

        let custom_dense = MaterialPreset::with_density(0.5, 0.2, 1000.0);
        assert_eq!(custom_dense.density, Some(1000.0));
    }

    #[test]
    fn test_default() {
        let default = MaterialPreset::default();
        assert_eq!(default.friction, MaterialPreset::concrete().friction);
    }
}
