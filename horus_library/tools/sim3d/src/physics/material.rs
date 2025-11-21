use bevy::prelude::*;
use std::collections::HashMap;

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

/// Advanced material properties with detailed friction and contact models
#[derive(Clone, Debug)]
pub struct AdvancedMaterial {
    /// Basic friction coefficient (Coulomb model)
    pub friction: f32,
    /// Restitution coefficient
    pub restitution: f32,
    /// Material density in kg/m³
    pub density: f32,

    // Advanced friction properties
    /// Friction model to use
    pub friction_model: FrictionModel,
    /// Static friction coefficient
    pub static_friction: f32,
    /// Dynamic/kinetic friction coefficient
    pub dynamic_friction: f32,
    /// Rolling resistance coefficient
    pub rolling_friction: f32,
    /// Spinning friction coefficient
    pub spinning_friction: f32,

    // Contact properties
    /// Contact stiffness (N/m)
    pub contact_stiffness: f32,
    /// Contact damping (N·s/m)
    pub contact_damping: f32,
    /// Compliance (inverse of stiffness)
    pub compliance: f32,

    // Surface properties
    /// Surface roughness (0.0 = smooth, 1.0 = rough)
    pub roughness: f32,
    /// Material hardness (normalized 0-1)
    pub hardness: f32,

    // Anisotropic friction (optional)
    pub anisotropic: Option<AnisotropicFriction>,
}

/// Friction model types
#[derive(Clone, Debug)]
pub enum FrictionModel {
    /// Simple Coulomb friction (velocity-independent)
    Coulomb,

    /// Stribeck friction model (velocity-dependent)
    Stribeck {
        v_stribeck: f32, // Stribeck velocity
        mu_c: f32,       // Coulomb friction
        mu_s: f32,       // Static friction
        delta_mu: f32,   // Friction drop
    },

    /// LuGre friction model (bristle model for stick-slip)
    LuGre {
        sigma_0: f32, // Bristle stiffness
        sigma_1: f32, // Bristle damping
        sigma_2: f32, // Viscous friction
        v_s: f32,     // Stribeck velocity
        mu_c: f32,    // Coulomb friction
        mu_s: f32,    // Static friction
    },

    /// Box friction (velocity thresholds)
    Box {
        static_threshold: f32,
        dynamic_threshold: f32,
    },
}

impl Default for FrictionModel {
    fn default() -> Self {
        Self::Coulomb
    }
}

/// Anisotropic friction (direction-dependent friction)
#[derive(Clone, Debug)]
pub struct AnisotropicFriction {
    /// Friction along primary axis
    pub friction_x: f32,
    /// Friction along secondary axis
    pub friction_y: f32,
    /// Primary friction direction (local frame)
    pub direction: Vec3,
}

impl AdvancedMaterial {
    /// Create material with advanced properties
    pub fn new(name: &str) -> Self {
        Self {
            friction: 0.5,
            restitution: 0.0,
            density: 1000.0,
            friction_model: FrictionModel::Coulomb,
            static_friction: 0.6,
            dynamic_friction: 0.5,
            rolling_friction: 0.01,
            spinning_friction: 0.01,
            contact_stiffness: 1e6,
            contact_damping: 1e3,
            compliance: 1e-6,
            roughness: 0.5,
            hardness: 0.5,
            anisotropic: None,
        }
    }

    /// Convert to Rapier material (basic friction + restitution)
    pub fn to_rapier(&self) -> (f32, f32) {
        (self.friction, self.restitution)
    }

    /// Get effective friction at a given relative velocity
    pub fn effective_friction(&self, relative_velocity: f32) -> f32 {
        match &self.friction_model {
            FrictionModel::Coulomb => self.friction,
            FrictionModel::Stribeck {
                v_stribeck,
                mu_c,
                mu_s,
                delta_mu,
            } => {
                // Stribeck curve: mu = mu_c + (mu_s - mu_c) * exp(-(v/v_s)^2) - delta_mu * (v/v_s)
                let v_ratio = relative_velocity / v_stribeck;
                mu_c + (mu_s - mu_c) * (-v_ratio * v_ratio).exp() - delta_mu * v_ratio
            }
            FrictionModel::LuGre {
                mu_c, mu_s, v_s, ..
            } => {
                // Simplified LuGre steady-state friction
                let v_ratio = relative_velocity / v_s;
                mu_c + (mu_s - mu_c) * (-v_ratio.abs()).exp()
            }
            FrictionModel::Box {
                static_threshold,
                dynamic_threshold,
            } => {
                if relative_velocity.abs() < *static_threshold {
                    self.static_friction
                } else if relative_velocity.abs() < *dynamic_threshold {
                    self.dynamic_friction
                } else {
                    self.friction
                }
            }
        }
    }

    /// Advanced material presets
    pub fn steel() -> Self {
        Self {
            friction: 0.3,
            static_friction: 0.35,
            dynamic_friction: 0.3,
            rolling_friction: 0.002,
            spinning_friction: 0.001,
            contact_stiffness: 2e8,
            contact_damping: 1e5,
            density: 7850.0,
            hardness: 0.9,
            roughness: 0.2,
            restitution: 0.05,
            friction_model: FrictionModel::Coulomb,
            compliance: 5e-9,
            anisotropic: None,
        }
    }

    pub fn rubber() -> Self {
        Self {
            friction: 0.9,
            static_friction: 1.0,
            dynamic_friction: 0.85,
            rolling_friction: 0.05,
            spinning_friction: 0.03,
            contact_stiffness: 1e5,
            contact_damping: 5e3,
            density: 1100.0,
            hardness: 0.3,
            roughness: 0.7,
            restitution: 0.8,
            friction_model: FrictionModel::Stribeck {
                v_stribeck: 0.01,
                mu_c: 0.85,
                mu_s: 1.0,
                delta_mu: 0.1,
            },
            compliance: 1e-5,
            anisotropic: None,
        }
    }

    pub fn ice() -> Self {
        Self {
            friction: 0.05,
            static_friction: 0.08,
            dynamic_friction: 0.04,
            rolling_friction: 0.001,
            spinning_friction: 0.0005,
            contact_stiffness: 1e7,
            contact_damping: 1e4,
            density: 917.0,
            hardness: 0.4,
            roughness: 0.1,
            restitution: 0.1,
            friction_model: FrictionModel::Coulomb,
            compliance: 1e-7,
            anisotropic: None,
        }
    }

    pub fn wood() -> Self {
        Self {
            friction: 0.6,
            static_friction: 0.7,
            dynamic_friction: 0.55,
            rolling_friction: 0.03,
            spinning_friction: 0.02,
            contact_stiffness: 5e6,
            contact_damping: 5e3,
            density: 600.0,
            hardness: 0.4,
            roughness: 0.5,
            restitution: 0.1,
            friction_model: FrictionModel::Coulomb,
            compliance: 2e-7,
            anisotropic: None,
        }
    }

    pub fn concrete() -> Self {
        Self {
            friction: 0.9,
            static_friction: 1.0,
            dynamic_friction: 0.85,
            rolling_friction: 0.015,
            spinning_friction: 0.01,
            contact_stiffness: 3e7,
            contact_damping: 2e4,
            density: 2400.0,
            hardness: 0.7,
            roughness: 0.8,
            restitution: 0.0,
            friction_model: FrictionModel::Coulomb,
            compliance: 3e-8,
            anisotropic: None,
        }
    }

    pub fn plastic() -> Self {
        Self {
            friction: 0.4,
            static_friction: 0.45,
            dynamic_friction: 0.4,
            rolling_friction: 0.02,
            spinning_friction: 0.015,
            contact_stiffness: 1e6,
            contact_damping: 1e3,
            density: 1200.0,
            hardness: 0.5,
            roughness: 0.3,
            restitution: 0.3,
            friction_model: FrictionModel::Coulomb,
            compliance: 1e-6,
            anisotropic: None,
        }
    }

    pub fn cloth() -> Self {
        Self {
            friction: 0.7,
            static_friction: 0.8,
            dynamic_friction: 0.65,
            rolling_friction: 0.1,
            spinning_friction: 0.08,
            contact_stiffness: 5e4,
            contact_damping: 2e3,
            density: 200.0,
            hardness: 0.1,
            roughness: 0.9,
            restitution: 0.0,
            friction_model: FrictionModel::Coulomb,
            compliance: 2e-5,
            anisotropic: None,
        }
    }

    pub fn foam() -> Self {
        Self {
            friction: 0.8,
            static_friction: 0.85,
            dynamic_friction: 0.75,
            rolling_friction: 0.15,
            spinning_friction: 0.12,
            contact_stiffness: 2e4,
            contact_damping: 5e2,
            density: 25.0,
            hardness: 0.05,
            roughness: 0.6,
            restitution: 0.1,
            friction_model: FrictionModel::Coulomb,
            compliance: 5e-5,
            anisotropic: None,
        }
    }

    pub fn carpet() -> Self {
        Self {
            friction: 1.2,
            static_friction: 1.3,
            dynamic_friction: 1.1,
            rolling_friction: 0.25,
            spinning_friction: 0.2,
            contact_stiffness: 3e4,
            contact_damping: 1e3,
            density: 400.0,
            hardness: 0.15,
            roughness: 0.95,
            restitution: 0.0,
            friction_model: FrictionModel::Coulomb,
            compliance: 3e-5,
            anisotropic: None,
        }
    }

    pub fn leather() -> Self {
        Self {
            friction: 0.5,
            static_friction: 0.6,
            dynamic_friction: 0.45,
            rolling_friction: 0.04,
            spinning_friction: 0.03,
            contact_stiffness: 8e5,
            contact_damping: 8e2,
            density: 900.0,
            hardness: 0.35,
            roughness: 0.4,
            restitution: 0.05,
            friction_model: FrictionModel::Coulomb,
            compliance: 1.25e-6,
            anisotropic: None,
        }
    }

    pub fn paper() -> Self {
        Self {
            friction: 0.4,
            static_friction: 0.5,
            dynamic_friction: 0.35,
            rolling_friction: 0.05,
            spinning_friction: 0.04,
            contact_stiffness: 1e5,
            contact_damping: 5e2,
            density: 700.0,
            hardness: 0.2,
            roughness: 0.6,
            restitution: 0.0,
            friction_model: FrictionModel::Coulomb,
            compliance: 1e-5,
            anisotropic: None,
        }
    }
}

/// Material interaction database for pair-wise material properties
#[derive(Resource, Default)]
pub struct MaterialInteractionDB {
    interactions: HashMap<(String, String), MaterialInteraction>,
}

/// Material interaction between two specific materials
#[derive(Clone, Debug)]
pub struct MaterialInteraction {
    /// Combined friction coefficient
    pub friction: f32,
    /// Combined restitution
    pub restitution: f32,
    /// Special interaction flags
    pub flags: InteractionFlags,
}

bitflags::bitflags! {
    /// Flags for special material interactions
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct InteractionFlags: u32 {
        const NONE      = 0b00000000;
        const ADHESIVE  = 0b00000001;  // Materials stick together
        const SLIPPERY  = 0b00000010;  // Very low friction
        const ABRASIVE  = 0b00000100;  // Causes wear/damage
        const ELASTIC   = 0b00001000;  // High restitution
    }
}

impl MaterialInteractionDB {
    pub fn new() -> Self {
        let mut db = Self {
            interactions: HashMap::new(),
        };

        // Define common interactions
        db.add(
            "rubber",
            "concrete",
            MaterialInteraction {
                friction: 1.0,
                restitution: 0.1,
                flags: InteractionFlags::NONE,
            },
        );

        db.add(
            "rubber",
            "ice",
            MaterialInteraction {
                friction: 0.15,
                restitution: 0.1,
                flags: InteractionFlags::SLIPPERY,
            },
        );

        db.add(
            "steel",
            "ice",
            MaterialInteraction {
                friction: 0.02,
                restitution: 0.05,
                flags: InteractionFlags::SLIPPERY,
            },
        );

        db.add(
            "steel",
            "steel",
            MaterialInteraction {
                friction: 0.4,
                restitution: 0.1,
                flags: InteractionFlags::NONE,
            },
        );

        db.add(
            "rubber",
            "rubber",
            MaterialInteraction {
                friction: 1.2,
                restitution: 0.9,
                flags: InteractionFlags::ELASTIC | InteractionFlags::ADHESIVE,
            },
        );

        db
    }

    /// Add a material interaction
    pub fn add(&mut self, mat1: &str, mat2: &str, interaction: MaterialInteraction) {
        let key1 = (mat1.to_string(), mat2.to_string());
        let key2 = (mat2.to_string(), mat1.to_string());
        self.interactions.insert(key1, interaction.clone());
        self.interactions.insert(key2, interaction);
    }

    /// Get interaction between two materials
    pub fn get(&self, mat1: &str, mat2: &str) -> Option<&MaterialInteraction> {
        self.interactions.get(&(mat1.to_string(), mat2.to_string()))
    }

    /// Get interaction or compute default
    pub fn get_or_default(
        &self,
        mat1: &str,
        mat2: &str,
        default_friction: f32,
        default_restitution: f32,
    ) -> MaterialInteraction {
        self.get(mat1, mat2)
            .cloned()
            .unwrap_or(MaterialInteraction {
                friction: default_friction,
                restitution: default_restitution,
                flags: InteractionFlags::NONE,
            })
    }
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
