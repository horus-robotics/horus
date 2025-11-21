use bevy::core_pipeline::bloom::{Bloom, BloomCompositeMode, BloomPrefilter, BloomSettings};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;

/// Bloom quality preset
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BloomQuality {
    Low,
    Medium,
    High,
    Ultra,
}

/// Bloom and HDR configuration
#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct BloomConfig {
    pub enabled: bool,
    pub intensity: f32,
    pub low_frequency_boost: f32,
    pub low_frequency_boost_curvature: f32,
    pub high_pass_frequency: f32,
    pub composite_mode: BloomCompositeMode,
    pub quality: BloomQuality,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self::medium()
    }
}

impl BloomConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            intensity: 0.0,
            low_frequency_boost: 0.0,
            low_frequency_boost_curvature: 0.0,
            high_pass_frequency: 1.0,
            composite_mode: BloomCompositeMode::Additive,
            quality: BloomQuality::Medium,
        }
    }

    pub fn low() -> Self {
        Self {
            enabled: true,
            intensity: 0.15,
            low_frequency_boost: 0.5,
            low_frequency_boost_curvature: 0.3,
            high_pass_frequency: 1.0,
            composite_mode: BloomCompositeMode::Additive,
            quality: BloomQuality::Low,
        }
    }

    pub fn medium() -> Self {
        Self {
            enabled: true,
            intensity: 0.3,
            low_frequency_boost: 0.7,
            low_frequency_boost_curvature: 0.4,
            high_pass_frequency: 1.0,
            composite_mode: BloomCompositeMode::Additive,
            quality: BloomQuality::Medium,
        }
    }

    pub fn high() -> Self {
        Self {
            enabled: true,
            intensity: 0.4,
            low_frequency_boost: 0.9,
            low_frequency_boost_curvature: 0.5,
            high_pass_frequency: 0.9,
            composite_mode: BloomCompositeMode::Additive,
            quality: BloomQuality::High,
        }
    }

    pub fn ultra() -> Self {
        Self {
            enabled: true,
            intensity: 0.5,
            low_frequency_boost: 1.0,
            low_frequency_boost_curvature: 0.6,
            high_pass_frequency: 0.8,
            composite_mode: BloomCompositeMode::EnergyConserving,
            quality: BloomQuality::Ultra,
        }
    }

    /// Convert to Bevy BloomSettings
    pub fn to_bloom_settings(&self) -> BloomSettings {
        BloomSettings {
            intensity: self.intensity,
            low_frequency_boost: self.low_frequency_boost,
            low_frequency_boost_curvature: self.low_frequency_boost_curvature,
            high_pass_frequency: self.high_pass_frequency,
            prefilter: BloomPrefilter::default(),
            composite_mode: self.composite_mode,
            max_mip_dimension: 512,
            uv_offset: Default::default(),
        }
    }
}

/// HDR and tonemapping configuration
#[derive(Resource, Clone, Debug)]
pub struct HDRConfig {
    pub enabled: bool,
    pub tonemapping: Tonemapping,
    pub exposure: f32,
    pub white_point: f32,
    pub deband_dither: DebandDither,
}

impl Default for HDRConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tonemapping: Tonemapping::TonyMcMapface,
            exposure: 1.0,
            white_point: 11.2,
            deband_dither: DebandDither::Enabled,
        }
    }
}

impl HDRConfig {
    /// ACES filmic tonemapping (cinematic)
    pub fn aces_filmic() -> Self {
        Self {
            enabled: true,
            tonemapping: Tonemapping::AcesFitted,
            exposure: 1.0,
            white_point: 11.2,
            deband_dither: DebandDither::Enabled,
        }
    }

    /// Reinhard tonemapping (classic)
    pub fn reinhard() -> Self {
        Self {
            enabled: true,
            tonemapping: Tonemapping::Reinhard,
            exposure: 1.0,
            white_point: 11.2,
            deband_dither: DebandDither::Enabled,
        }
    }

    /// AgX tonemapping (modern, balanced)
    pub fn agx() -> Self {
        Self {
            enabled: true,
            tonemapping: Tonemapping::AgX,
            exposure: 1.0,
            white_point: 11.2,
            deband_dither: DebandDither::Enabled,
        }
    }

    /// TonyMcMapface (Bevy default)
    pub fn tony_mc_mapface() -> Self {
        Self {
            enabled: true,
            tonemapping: Tonemapping::TonyMcMapface,
            exposure: 1.0,
            white_point: 11.2,
            deband_dither: DebandDither::Enabled,
        }
    }
}

/// Color grading configuration
#[derive(Resource, Clone, Debug)]
pub struct ColorGrading {
    pub enabled: bool,
    pub temperature: f32, // -1.0 to 1.0 (blue to orange)
    pub tint: f32,        // -1.0 to 1.0 (green to magenta)
    pub contrast: f32,    // 0.0 to 2.0
    pub saturation: f32,  // 0.0 to 2.0
    pub brightness: f32,  // -1.0 to 1.0
    pub gamma: f32,       // 0.5 to 2.5
}

impl Default for ColorGrading {
    fn default() -> Self {
        Self {
            enabled: false,
            temperature: 0.0,
            tint: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            brightness: 0.0,
            gamma: 1.0,
        }
    }
}

impl ColorGrading {
    /// Warm color grading
    pub fn warm() -> Self {
        Self {
            enabled: true,
            temperature: 0.3,
            tint: 0.0,
            contrast: 1.1,
            saturation: 1.1,
            brightness: 0.05,
            gamma: 1.0,
        }
    }

    /// Cool color grading
    pub fn cool() -> Self {
        Self {
            enabled: true,
            temperature: -0.3,
            tint: 0.0,
            contrast: 1.1,
            saturation: 1.0,
            brightness: 0.0,
            gamma: 1.0,
        }
    }

    /// Cinematic color grading
    pub fn cinematic() -> Self {
        Self {
            enabled: true,
            temperature: 0.1,
            tint: -0.05,
            contrast: 1.2,
            saturation: 0.9,
            brightness: -0.05,
            gamma: 1.1,
        }
    }

    /// Desaturated/noir look
    pub fn desaturated() -> Self {
        Self {
            enabled: true,
            temperature: 0.0,
            tint: 0.0,
            contrast: 1.3,
            saturation: 0.3,
            brightness: 0.0,
            gamma: 1.0,
        }
    }
}

/// Vignette effect
#[derive(Resource, Clone, Debug)]
pub struct Vignette {
    pub enabled: bool,
    pub intensity: f32,
    pub smoothness: f32,
    pub color: Color,
}

impl Default for Vignette {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.5,
            smoothness: 0.5,
            color: Color::BLACK,
        }
    }
}

/// Chromatic aberration effect
#[derive(Resource, Clone, Debug)]
pub struct ChromaticAberration {
    pub enabled: bool,
    pub intensity: f32,
    pub samples: u32,
}

impl Default for ChromaticAberration {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.01,
            samples: 3,
        }
    }
}

/// Film grain effect
#[derive(Resource, Clone, Debug)]
pub struct FilmGrain {
    pub enabled: bool,
    pub intensity: f32,
    pub size: f32,
}

impl Default for FilmGrain {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.05,
            size: 1.0,
        }
    }
}

/// Post-processing plugin
pub struct PostProcessingPlugin {
    pub bloom: BloomConfig,
    pub hdr: HDRConfig,
}

impl Default for PostProcessingPlugin {
    fn default() -> Self {
        Self {
            bloom: BloomConfig::medium(),
            hdr: HDRConfig::default(),
        }
    }
}

impl Plugin for PostProcessingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.bloom.clone())
            .insert_resource(self.hdr.clone())
            .insert_resource(ColorGrading::default())
            .insert_resource(Vignette::default())
            .insert_resource(ChromaticAberration::default())
            .insert_resource(FilmGrain::default())
            .add_systems(Update, apply_bloom_to_cameras);
    }
}

/// System to apply bloom settings to cameras
fn apply_bloom_to_cameras(
    bloom_config: Res<BloomConfig>,
    mut cameras: Query<(Entity, Option<&mut Bloom>), With<Camera>>,
    mut commands: Commands,
) {
    if !bloom_config.is_changed() {
        return;
    }

    for (entity, bloom) in cameras.iter_mut() {
        if bloom_config.enabled {
            let settings = bloom_config.to_bloom_settings();
            if let Some(mut existing_bloom) = bloom {
                *existing_bloom = Bloom {
                    intensity: settings.intensity,
                    low_frequency_boost: settings.low_frequency_boost,
                    low_frequency_boost_curvature: settings.low_frequency_boost_curvature,
                    high_pass_frequency: settings.high_pass_frequency,
                    prefilter: settings.prefilter,
                    composite_mode: settings.composite_mode,
                    max_mip_dimension: settings.max_mip_dimension,
                    uv_offset: settings.uv_offset,
                };
            } else {
                commands.entity(entity).insert(Bloom {
                    intensity: settings.intensity,
                    low_frequency_boost: settings.low_frequency_boost,
                    low_frequency_boost_curvature: settings.low_frequency_boost_curvature,
                    high_pass_frequency: settings.high_pass_frequency,
                    prefilter: settings.prefilter,
                    composite_mode: settings.composite_mode,
                    max_mip_dimension: settings.max_mip_dimension,
                    uv_offset: settings.uv_offset,
                });
            }
        } else if bloom.is_some() {
            commands.entity(entity).remove::<Bloom>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_presets() {
        let low = BloomConfig::low();
        assert!(low.enabled);
        assert!(low.intensity > 0.0);

        let ultra = BloomConfig::ultra();
        assert!(ultra.intensity > low.intensity);
    }

    #[test]
    fn test_hdr_presets() {
        let aces = HDRConfig::aces_filmic();
        assert!(aces.enabled);
        assert!(matches!(aces.tonemapping, Tonemapping::AcesFitted));

        let reinhard = HDRConfig::reinhard();
        assert!(matches!(reinhard.tonemapping, Tonemapping::Reinhard));
    }

    #[test]
    fn test_color_grading_presets() {
        let warm = ColorGrading::warm();
        assert!(warm.enabled);
        assert!(warm.temperature > 0.0);

        let cool = ColorGrading::cool();
        assert!(cool.temperature < 0.0);
    }

    #[test]
    fn test_vignette() {
        let vignette = Vignette::default();
        assert!(!vignette.enabled);
        assert_eq!(vignette.color, Color::BLACK);
    }

    #[test]
    fn test_chromatic_aberration() {
        let ca = ChromaticAberration::default();
        assert!(!ca.enabled);
        assert!(ca.intensity > 0.0);
        assert!(ca.samples > 0);
    }

    #[test]
    fn test_film_grain() {
        let grain = FilmGrain::default();
        assert!(!grain.enabled);
        assert!(grain.intensity > 0.0);
    }
}
