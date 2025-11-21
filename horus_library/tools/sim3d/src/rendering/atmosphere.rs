use bevy::prelude::*;

/// Fog rendering mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FogMode {
    /// Linear fog (simple, cheap)
    Linear,
    /// Exponential fog (more natural)
    Exponential,
    /// Exponential squared (very dense)
    ExponentialSquared,
    /// Atmospheric scattering (realistic, expensive)
    Atmospheric,
    Disabled,
}

/// Fog configuration
#[derive(Resource, Clone, Debug)]
pub struct FogConfig {
    pub enabled: bool,
    pub mode: FogMode,
    pub color: Color,
    pub density: f32,
    pub start_distance: f32,
    pub end_distance: f32,
    pub height_falloff: f32,
    pub height_offset: f32,
}

impl Default for FogConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

impl FogConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            mode: FogMode::Disabled,
            color: Color::WHITE,
            density: 0.0,
            start_distance: 0.0,
            end_distance: 100.0,
            height_falloff: 0.0,
            height_offset: 0.0,
        }
    }

    /// Light linear fog
    pub fn linear_light() -> Self {
        Self {
            enabled: true,
            mode: FogMode::Linear,
            color: Color::srgb(0.8, 0.85, 0.9),
            density: 0.02,
            start_distance: 20.0,
            end_distance: 100.0,
            height_falloff: 0.0,
            height_offset: 0.0,
        }
    }

    /// Dense linear fog
    pub fn linear_dense() -> Self {
        Self {
            enabled: true,
            mode: FogMode::Linear,
            color: Color::srgb(0.7, 0.75, 0.8),
            density: 0.05,
            start_distance: 10.0,
            end_distance: 50.0,
            height_falloff: 0.0,
            height_offset: 0.0,
        }
    }

    /// Exponential fog
    pub fn exponential() -> Self {
        Self {
            enabled: true,
            mode: FogMode::Exponential,
            color: Color::srgb(0.75, 0.8, 0.85),
            density: 0.015,
            start_distance: 0.0,
            end_distance: 100.0,
            height_falloff: 0.0,
            height_offset: 0.0,
        }
    }

    /// Very dense fog
    pub fn exponential_dense() -> Self {
        Self {
            enabled: true,
            mode: FogMode::ExponentialSquared,
            color: Color::srgb(0.7, 0.7, 0.75),
            density: 0.03,
            start_distance: 0.0,
            end_distance: 50.0,
            height_falloff: 0.0,
            height_offset: 0.0,
        }
    }

    /// Atmospheric scattering fog
    pub fn atmospheric() -> Self {
        Self {
            enabled: true,
            mode: FogMode::Atmospheric,
            color: Color::srgb(0.6, 0.7, 0.9),
            density: 0.01,
            start_distance: 0.0,
            end_distance: 200.0,
            height_falloff: 0.05,
            height_offset: 0.0,
        }
    }

    /// Height fog (valleys, low-lying areas)
    pub fn height_fog() -> Self {
        Self {
            enabled: true,
            mode: FogMode::Exponential,
            color: Color::srgb(0.8, 0.85, 0.9),
            density: 0.02,
            start_distance: 0.0,
            end_distance: 100.0,
            height_falloff: 0.1,
            height_offset: 2.0,
        }
    }

    /// Calculate fog factor for a given distance
    pub fn calculate_fog_factor(&self, distance: f32, height: f32) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        let base_factor = match self.mode {
            FogMode::Disabled => 0.0,
            FogMode::Linear => {
                if distance < self.start_distance {
                    0.0
                } else if distance > self.end_distance {
                    1.0
                } else {
                    (distance - self.start_distance) / (self.end_distance - self.start_distance)
                }
            }
            FogMode::Exponential => 1.0 - (-self.density * distance).exp(),
            FogMode::ExponentialSquared => 1.0 - (-self.density * distance * distance).exp(),
            FogMode::Atmospheric => {
                // Simplified atmospheric scattering
                let rayleigh = 1.0 - (-self.density * distance).exp();
                rayleigh.clamp(0.0, 1.0)
            }
        };

        // Apply height falloff
        if self.height_falloff > 0.0 {
            let height_factor =
                (-self.height_falloff * (height - self.height_offset).max(0.0)).exp();
            base_factor * height_factor
        } else {
            base_factor
        }
    }
}

/// Volumetric lighting configuration (god rays, light shafts)
#[derive(Resource, Clone, Debug)]
pub struct VolumetricLightingConfig {
    pub enabled: bool,
    pub num_samples: u32,
    pub scattering: f32,
    pub intensity: f32,
    pub dithering: bool,
}

impl Default for VolumetricLightingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            num_samples: 64,
            scattering: 0.5,
            intensity: 1.0,
            dithering: true,
        }
    }
}

impl VolumetricLightingConfig {
    pub fn low() -> Self {
        Self {
            enabled: true,
            num_samples: 32,
            scattering: 0.4,
            intensity: 0.8,
            dithering: true,
        }
    }

    pub fn high() -> Self {
        Self {
            enabled: true,
            num_samples: 128,
            scattering: 0.6,
            intensity: 1.2,
            dithering: true,
        }
    }
}

/// Atmospheric scattering parameters
#[derive(Resource, Clone, Debug)]
pub struct AtmosphericScattering {
    pub enabled: bool,

    // Rayleigh scattering (air molecules - blue sky)
    pub rayleigh_coefficient: Vec3,
    pub rayleigh_scale_height: f32,

    // Mie scattering (aerosols - haze)
    pub mie_coefficient: f32,
    pub mie_scale_height: f32,
    pub mie_directional_g: f32,

    // Planet parameters
    pub planet_radius: f32,
    pub atmosphere_radius: f32,

    // Sun
    pub sun_intensity: f32,
}

impl Default for AtmosphericScattering {
    fn default() -> Self {
        Self {
            enabled: false,
            // Earth-like atmosphere
            rayleigh_coefficient: Vec3::new(5.8e-6, 13.5e-6, 33.1e-6),
            rayleigh_scale_height: 8000.0,
            mie_coefficient: 21e-6,
            mie_scale_height: 1200.0,
            mie_directional_g: 0.76,
            planet_radius: 6371000.0,     // Earth radius in meters
            atmosphere_radius: 6471000.0, // +100km atmosphere
            sun_intensity: 20.0,
        }
    }
}

impl AtmosphericScattering {
    /// Earth-like atmosphere
    pub fn earth() -> Self {
        Self::default()
    }

    /// Mars-like atmosphere (thin, reddish)
    pub fn mars() -> Self {
        Self {
            enabled: true,
            rayleigh_coefficient: Vec3::new(19.0e-6, 13.0e-6, 5.8e-6), // Inverted for red tint
            rayleigh_scale_height: 11000.0,
            mie_coefficient: 5e-6,
            mie_scale_height: 2000.0,
            mie_directional_g: 0.65,
            planet_radius: 3390000.0,
            atmosphere_radius: 3490000.0,
            sun_intensity: 15.0,
        }
    }

    /// Alien atmosphere (customizable)
    pub fn alien(sky_color_bias: Vec3) -> Self {
        Self {
            enabled: true,
            rayleigh_coefficient: sky_color_bias * 10e-6,
            rayleigh_scale_height: 7000.0,
            mie_coefficient: 15e-6,
            mie_scale_height: 1500.0,
            mie_directional_g: 0.7,
            planet_radius: 6000000.0,
            atmosphere_radius: 6100000.0,
            sun_intensity: 18.0,
        }
    }
}

/// Particle effects in atmosphere (rain, snow, dust)
#[derive(Resource, Clone, Debug)]
pub struct AtmosphericParticles {
    pub enabled: bool,
    pub particle_type: ParticleType,
    pub density: f32,
    pub wind_velocity: Vec3,
    pub turbulence: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleType {
    Rain,
    Snow,
    Dust,
    Ash,
    Leaves,
}

impl Default for AtmosphericParticles {
    fn default() -> Self {
        Self {
            enabled: false,
            particle_type: ParticleType::Rain,
            density: 0.5,
            wind_velocity: Vec3::ZERO,
            turbulence: 0.1,
        }
    }
}

impl AtmosphericParticles {
    pub fn rain(intensity: f32) -> Self {
        Self {
            enabled: true,
            particle_type: ParticleType::Rain,
            density: intensity,
            wind_velocity: Vec3::new(0.5, -10.0, 0.0),
            turbulence: 0.2,
        }
    }

    pub fn snow(intensity: f32) -> Self {
        Self {
            enabled: true,
            particle_type: ParticleType::Snow,
            density: intensity,
            wind_velocity: Vec3::new(0.2, -2.0, 0.1),
            turbulence: 0.5,
        }
    }

    pub fn dust_storm(intensity: f32) -> Self {
        Self {
            enabled: true,
            particle_type: ParticleType::Dust,
            density: intensity,
            wind_velocity: Vec3::new(5.0, 0.5, 2.0),
            turbulence: 0.8,
        }
    }
}

/// Atmosphere plugin
pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FogConfig::default())
            .insert_resource(VolumetricLightingConfig::default())
            .insert_resource(AtmosphericScattering::default())
            .insert_resource(AtmosphericParticles::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fog_presets() {
        let linear = FogConfig::linear_light();
        assert!(linear.enabled);
        assert_eq!(linear.mode, FogMode::Linear);

        let exp = FogConfig::exponential();
        assert_eq!(exp.mode, FogMode::Exponential);

        let disabled = FogConfig::disabled();
        assert!(!disabled.enabled);
    }

    #[test]
    fn test_fog_factor_calculation() {
        let linear = FogConfig::linear_light();

        let near_factor = linear.calculate_fog_factor(15.0, 0.0);
        assert!(near_factor < 0.1); // Before start distance

        let far_factor = linear.calculate_fog_factor(110.0, 0.0);
        assert!(far_factor > 0.9); // Past end distance

        let mid_factor = linear.calculate_fog_factor(60.0, 0.0);
        assert!(mid_factor > 0.3 && mid_factor < 0.7); // In between
    }

    #[test]
    fn test_height_fog() {
        let height_fog = FogConfig::height_fog();
        assert!(height_fog.height_falloff > 0.0);

        let low_height = height_fog.calculate_fog_factor(50.0, 0.0);
        let high_height = height_fog.calculate_fog_factor(50.0, 20.0);

        assert!(low_height > high_height); // More fog at lower heights
    }

    #[test]
    fn test_volumetric_lighting() {
        let vol_low = VolumetricLightingConfig::low();
        let vol_high = VolumetricLightingConfig::high();

        assert!(vol_high.num_samples > vol_low.num_samples);
        assert!(vol_low.num_samples >= 32);
    }

    #[test]
    fn test_atmospheric_scattering_presets() {
        let earth = AtmosphericScattering::earth();
        assert!(earth.planet_radius > 6_000_000.0);

        let mars = AtmosphericScattering::mars();
        assert!(mars.planet_radius < earth.planet_radius);

        let alien = AtmosphericScattering::alien(Vec3::new(1.0, 0.5, 0.2));
        assert!(alien.enabled);
    }

    #[test]
    fn test_atmospheric_particles() {
        let rain = AtmosphericParticles::rain(0.7);
        assert_eq!(rain.particle_type, ParticleType::Rain);
        assert_eq!(rain.density, 0.7);

        let snow = AtmosphericParticles::snow(0.5);
        assert_eq!(snow.particle_type, ParticleType::Snow);

        let dust = AtmosphericParticles::dust_storm(0.9);
        assert_eq!(dust.particle_type, ParticleType::Dust);
    }
}
