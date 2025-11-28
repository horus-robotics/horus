//! Domain randomization for improving RL generalization
//!
//! Randomizes physical and visual properties of the environment
//! to train more robust policies.

use bevy::prelude::*;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

/// Configuration for domain randomization
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DomainRandomizationConfig {
    /// Randomize physics parameters
    pub physics: PhysicsRandomization,
    /// Randomize visual parameters
    pub visual: VisualRandomization,
    /// Randomize environment parameters
    pub environment: EnvironmentRandomization,
    /// Random seed (None for random)
    pub seed: Option<u64>,
}

/// Physics randomization parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhysicsRandomization {
    /// Randomize mass
    pub mass_range: Option<(f32, f32)>,
    /// Randomize friction coefficients
    pub friction_range: Option<(f32, f32)>,
    /// Randomize restitution (bounciness)
    pub restitution_range: Option<(f32, f32)>,
    /// Randomize gravity
    pub gravity_range: Option<(Vec3, Vec3)>,
    /// Randomize joint damping
    pub joint_damping_range: Option<(f32, f32)>,
    /// Randomize joint stiffness
    pub joint_stiffness_range: Option<(f32, f32)>,
    /// Randomize center of mass offset
    pub com_offset_range: Option<(Vec3, Vec3)>,
}

impl Default for PhysicsRandomization {
    fn default() -> Self {
        Self {
            mass_range: Some((0.8, 1.2)),        // ±20%
            friction_range: Some((0.3, 0.9)),    // 0.3-0.9
            restitution_range: Some((0.0, 0.3)), // 0.0-0.3
            gravity_range: Some((Vec3::new(0.0, -8.0, 0.0), Vec3::new(0.0, -12.0, 0.0))),
            joint_damping_range: Some((0.1, 1.0)),
            joint_stiffness_range: Some((50.0, 200.0)),
            com_offset_range: Some((Vec3::new(-0.05, -0.05, -0.05), Vec3::new(0.05, 0.05, 0.05))),
        }
    }
}

/// Visual randomization parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualRandomization {
    /// Randomize lighting
    pub light_intensity_range: Option<(f32, f32)>,
    pub light_color_temp_range: Option<(f32, f32)>, // Color temperature in K
    pub light_direction_range: Option<(Vec3, Vec3)>,
    /// Randomize material colors
    pub material_color_range: Option<(Color, Color)>,
    /// Randomize textures (if available)
    pub randomize_textures: bool,
    /// Camera pose noise
    pub camera_position_noise: Option<(Vec3, Vec3)>,
    pub camera_rotation_noise: Option<(f32, f32)>, // In radians
}

impl Default for VisualRandomization {
    fn default() -> Self {
        Self {
            light_intensity_range: Some((500.0, 2000.0)),
            light_color_temp_range: Some((3000.0, 7000.0)),
            light_direction_range: Some((Vec3::new(-1.0, 0.5, -1.0), Vec3::new(1.0, 1.0, 1.0))),
            material_color_range: Some((Color::srgb(0.5, 0.5, 0.5), Color::srgb(1.0, 1.0, 1.0))),
            randomize_textures: false,
            camera_position_noise: Some((Vec3::new(-0.1, -0.1, -0.1), Vec3::new(0.1, 0.1, 0.1))),
            camera_rotation_noise: Some((-0.1, 0.1)),
        }
    }
}

/// Environment randomization parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentRandomization {
    /// Randomize object initial positions
    pub object_position_noise: Option<(Vec3, Vec3)>,
    /// Randomize object initial rotations
    pub object_rotation_noise: Option<(f32, f32)>,
    /// Randomize target positions
    pub target_position_noise: Option<(Vec3, Vec3)>,
    /// Randomize object scales
    pub object_scale_range: Option<(Vec3, Vec3)>,
    /// Number of distractor objects to spawn
    pub num_distractors_range: Option<(usize, usize)>,
}

impl Default for EnvironmentRandomization {
    fn default() -> Self {
        Self {
            object_position_noise: Some((Vec3::new(-0.2, 0.0, -0.2), Vec3::new(0.2, 0.1, 0.2))),
            object_rotation_noise: Some((-std::f32::consts::PI / 4.0, std::f32::consts::PI / 4.0)),
            target_position_noise: Some((Vec3::new(-0.1, -0.1, -0.1), Vec3::new(0.1, 0.1, 0.1))),
            object_scale_range: Some((Vec3::new(0.9, 0.9, 0.9), Vec3::new(1.1, 1.1, 1.1))),
            num_distractors_range: Some((0, 3)),
        }
    }
}

/// Domain randomization resource
#[derive(Resource)]
pub struct DomainRandomizer {
    pub config: DomainRandomizationConfig,
    pub enabled: bool,
}

impl DomainRandomizer {
    pub fn new(config: DomainRandomizationConfig) -> Self {
        Self {
            config,
            enabled: true,
        }
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Sample random mass multiplier
    pub fn sample_mass(&self) -> f32 {
        if !self.enabled {
            return 1.0;
        }

        if let Some((min, max)) = self.config.physics.mass_range {
            thread_rng().gen_range(min..max)
        } else {
            1.0
        }
    }

    /// Sample random friction coefficient
    pub fn sample_friction(&self) -> f32 {
        if !self.enabled {
            return 0.6;
        }

        if let Some((min, max)) = self.config.physics.friction_range {
            thread_rng().gen_range(min..max)
        } else {
            0.6
        }
    }

    /// Sample random restitution
    pub fn sample_restitution(&self) -> f32 {
        if !self.enabled {
            return 0.1;
        }

        if let Some((min, max)) = self.config.physics.restitution_range {
            thread_rng().gen_range(min..max)
        } else {
            0.1
        }
    }

    /// Sample random gravity
    pub fn sample_gravity(&self) -> Vec3 {
        if !self.enabled {
            return Vec3::new(0.0, -9.81, 0.0);
        }

        if let Some((min, max)) = self.config.physics.gravity_range {
            Vec3::new(
                thread_rng().gen_range(min.x..max.x),
                thread_rng().gen_range(min.y..max.y),
                thread_rng().gen_range(min.z..max.z),
            )
        } else {
            Vec3::new(0.0, -9.81, 0.0)
        }
    }

    /// Sample random joint damping
    pub fn sample_joint_damping(&self) -> f32 {
        if !self.enabled {
            return 0.5;
        }

        if let Some((min, max)) = self.config.physics.joint_damping_range {
            thread_rng().gen_range(min..max)
        } else {
            0.5
        }
    }

    /// Sample random joint stiffness
    pub fn sample_joint_stiffness(&self) -> f32 {
        if !self.enabled {
            return 100.0;
        }

        if let Some((min, max)) = self.config.physics.joint_stiffness_range {
            thread_rng().gen_range(min..max)
        } else {
            100.0
        }
    }

    /// Sample random center of mass offset
    pub fn sample_com_offset(&self) -> Vec3 {
        if !self.enabled {
            return Vec3::ZERO;
        }

        if let Some((min, max)) = self.config.physics.com_offset_range {
            Vec3::new(
                thread_rng().gen_range(min.x..max.x),
                thread_rng().gen_range(min.y..max.y),
                thread_rng().gen_range(min.z..max.z),
            )
        } else {
            Vec3::ZERO
        }
    }

    /// Sample random light intensity
    pub fn sample_light_intensity(&self) -> f32 {
        if !self.enabled {
            return 1000.0;
        }

        if let Some((min, max)) = self.config.visual.light_intensity_range {
            thread_rng().gen_range(min..max)
        } else {
            1000.0
        }
    }

    /// Sample random light color from temperature
    pub fn sample_light_color(&self) -> Color {
        if !self.enabled {
            return Color::WHITE;
        }

        if let Some((min_temp, max_temp)) = self.config.visual.light_color_temp_range {
            let temp = thread_rng().gen_range(min_temp..max_temp);
            // Simplified color temperature to RGB conversion
            temperature_to_rgb(temp)
        } else {
            Color::WHITE
        }
    }

    /// Sample random light direction
    pub fn sample_light_direction(&self) -> Vec3 {
        if !self.enabled {
            return Vec3::new(0.0, 1.0, 0.0);
        }

        if let Some((min, max)) = self.config.visual.light_direction_range {
            Vec3::new(
                thread_rng().gen_range(min.x..max.x),
                thread_rng().gen_range(min.y..max.y),
                thread_rng().gen_range(min.z..max.z),
            )
            .normalize()
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        }
    }

    /// Sample random material color
    pub fn sample_material_color(&self) -> Color {
        if !self.enabled {
            return Color::srgb(0.8, 0.8, 0.8);
        }

        if let Some((min_color, max_color)) = self.config.visual.material_color_range {
            let r = thread_rng().gen_range(min_color.to_srgba().red..max_color.to_srgba().red);
            let g = thread_rng().gen_range(min_color.to_srgba().green..max_color.to_srgba().green);
            let b = thread_rng().gen_range(min_color.to_srgba().blue..max_color.to_srgba().blue);
            Color::srgb(r, g, b)
        } else {
            Color::srgb(0.8, 0.8, 0.8)
        }
    }

    /// Sample random object position noise
    pub fn sample_position_noise(&self) -> Vec3 {
        if !self.enabled {
            return Vec3::ZERO;
        }

        if let Some((min, max)) = self.config.environment.object_position_noise {
            Vec3::new(
                thread_rng().gen_range(min.x..max.x),
                thread_rng().gen_range(min.y..max.y),
                thread_rng().gen_range(min.z..max.z),
            )
        } else {
            Vec3::ZERO
        }
    }

    /// Sample random object rotation noise
    pub fn sample_rotation_noise(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if let Some((min, max)) = self.config.environment.object_rotation_noise {
            thread_rng().gen_range(min..max)
        } else {
            0.0
        }
    }

    /// Sample random object scale
    pub fn sample_scale(&self) -> Vec3 {
        if !self.enabled {
            return Vec3::ONE;
        }

        if let Some((min, max)) = self.config.environment.object_scale_range {
            Vec3::new(
                thread_rng().gen_range(min.x..max.x),
                thread_rng().gen_range(min.y..max.y),
                thread_rng().gen_range(min.z..max.z),
            )
        } else {
            Vec3::ONE
        }
    }

    /// Sample number of distractor objects
    pub fn sample_num_distractors(&self) -> usize {
        if !self.enabled {
            return 0;
        }

        if let Some((min, max)) = self.config.environment.num_distractors_range {
            thread_rng().gen_range(min..=max)
        } else {
            0
        }
    }
}

/// Convert color temperature (Kelvin) to RGB
fn temperature_to_rgb(kelvin: f32) -> Color {
    let temp = kelvin / 100.0;

    // Red
    let red = if temp <= 66.0 {
        255.0
    } else {
        329.698_73 * (temp - 60.0).powf(-0.133_204_76)
    };

    // Green
    let green = if temp <= 66.0 {
        99.470_8 * temp.ln() - 161.119_57
    } else {
        288.122_16 * (temp - 60.0).powf(-0.075_514_846)
    };

    // Blue
    let blue = if temp >= 66.0 {
        255.0
    } else if temp <= 19.0 {
        0.0
    } else {
        138.517_73 * (temp - 10.0).ln() - 305.044_8
    };

    Color::srgb(
        (red / 255.0).clamp(0.0, 1.0),
        (green / 255.0).clamp(0.0, 1.0),
        (blue / 255.0).clamp(0.0, 1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_randomizer_creation() {
        let config = DomainRandomizationConfig::default();
        let randomizer = DomainRandomizer::new(config);
        assert!(randomizer.enabled);
    }

    #[test]
    fn test_disable_randomization() {
        let config = DomainRandomizationConfig::default();
        let mut randomizer = DomainRandomizer::new(config);

        randomizer.disable();
        assert!(!randomizer.enabled);

        // Should return default values when disabled
        assert_eq!(randomizer.sample_mass(), 1.0);
        assert_eq!(randomizer.sample_position_noise(), Vec3::ZERO);
    }

    #[test]
    fn test_sample_mass() {
        let config = DomainRandomizationConfig::default();
        let randomizer = DomainRandomizer::new(config);

        // Sample multiple times and verify range
        for _ in 0..100 {
            let mass = randomizer.sample_mass();
            assert!(mass >= 0.8 && mass <= 1.2);
        }
    }

    #[test]
    fn test_sample_friction() {
        let config = DomainRandomizationConfig::default();
        let randomizer = DomainRandomizer::new(config);

        for _ in 0..100 {
            let friction = randomizer.sample_friction();
            assert!(friction >= 0.3 && friction <= 0.9);
        }
    }

    #[test]
    fn test_temperature_to_rgb() {
        // Test warm light (3000K - reddish)
        let warm = temperature_to_rgb(3000.0);
        assert!(warm.to_srgba().red > 0.9);

        // Test cool light (6500K - bluish)
        let cool = temperature_to_rgb(6500.0);
        assert!(cool.to_srgba().blue > 0.8);
    }
}
