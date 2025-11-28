use bevy::pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel};
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;

/// Ambient occlusion technique
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AOTechnique {
    /// Screen-Space Ambient Occlusion
    SSAO,
    /// Horizon-Based Ambient Occlusion (more accurate, slightly slower)
    HBAO,
    /// Ground Truth Ambient Occlusion (ray-traced, expensive)
    GTAO,
    Disabled,
}

/// Ambient Occlusion configuration
#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct AmbientOcclusionConfig {
    /// AO technique to use
    pub technique: AOTechnique,

    /// Number of samples per pixel
    pub num_samples: u32,

    /// Sampling radius in world space
    pub radius: f32,

    /// AO intensity/strength
    pub intensity: f32,

    /// Bias to prevent self-occlusion
    pub bias: f32,

    /// Power exponent for AO darkening
    pub power: f32,

    /// Enable bilateral blur for denoising
    pub blur_enabled: bool,

    /// Blur radius (pixels)
    pub blur_radius: u32,

    /// Depth sensitivity for edge-aware blur
    pub blur_depth_sensitivity: f32,
}

impl Default for AmbientOcclusionConfig {
    fn default() -> Self {
        Self::ssao_medium()
    }
}

impl AmbientOcclusionConfig {
    /// Disabled AO
    pub fn disabled() -> Self {
        Self {
            technique: AOTechnique::Disabled,
            num_samples: 0,
            radius: 0.0,
            intensity: 0.0,
            bias: 0.0,
            power: 1.0,
            blur_enabled: false,
            blur_radius: 0,
            blur_depth_sensitivity: 0.0,
        }
    }

    /// Low quality SSAO (fast)
    pub fn ssao_low() -> Self {
        Self {
            technique: AOTechnique::SSAO,
            num_samples: 8,
            radius: 0.5,
            intensity: 1.0,
            bias: 0.025,
            power: 2.0,
            blur_enabled: true,
            blur_radius: 2,
            blur_depth_sensitivity: 1.0,
        }
    }

    /// Medium quality SSAO (balanced)
    pub fn ssao_medium() -> Self {
        Self {
            technique: AOTechnique::SSAO,
            num_samples: 16,
            radius: 1.0,
            intensity: 1.2,
            bias: 0.025,
            power: 2.0,
            blur_enabled: true,
            blur_radius: 4,
            blur_depth_sensitivity: 1.0,
        }
    }

    /// High quality SSAO (detailed)
    pub fn ssao_high() -> Self {
        Self {
            technique: AOTechnique::SSAO,
            num_samples: 32,
            radius: 1.5,
            intensity: 1.5,
            bias: 0.02,
            power: 2.2,
            blur_enabled: true,
            blur_radius: 6,
            blur_depth_sensitivity: 1.5,
        }
    }

    /// Medium quality HBAO (more accurate than SSAO)
    pub fn hbao_medium() -> Self {
        Self {
            technique: AOTechnique::HBAO,
            num_samples: 8,
            radius: 1.0,
            intensity: 1.2,
            bias: 0.015,
            power: 2.0,
            blur_enabled: true,
            blur_radius: 4,
            blur_depth_sensitivity: 1.2,
        }
    }

    /// High quality HBAO
    pub fn hbao_high() -> Self {
        Self {
            technique: AOTechnique::HBAO,
            num_samples: 16,
            radius: 1.5,
            intensity: 1.5,
            bias: 0.015,
            power: 2.2,
            blur_enabled: true,
            blur_radius: 6,
            blur_depth_sensitivity: 1.5,
        }
    }

    /// Ground-truth AO (ray-traced, very expensive)
    pub fn gtao() -> Self {
        Self {
            technique: AOTechnique::GTAO,
            num_samples: 64,
            radius: 2.0,
            intensity: 1.8,
            bias: 0.01,
            power: 2.5,
            blur_enabled: true,
            blur_radius: 8,
            blur_depth_sensitivity: 2.0,
        }
    }

    /// Check if AO is enabled
    pub fn is_enabled(&self) -> bool {
        self.technique != AOTechnique::Disabled
    }
}

/// Component for per-object AO override
#[derive(Component, Clone, Debug)]
pub struct AmbientOcclusionOverride {
    pub enabled: bool,
    pub intensity_multiplier: f32,
    pub radius_multiplier: f32,
}

impl Default for AmbientOcclusionOverride {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity_multiplier: 1.0,
            radius_multiplier: 1.0,
        }
    }
}

impl AmbientOcclusionOverride {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Default::default()
        }
    }

    pub fn with_intensity(mut self, multiplier: f32) -> Self {
        self.intensity_multiplier = multiplier;
        self
    }

    pub fn with_radius(mut self, multiplier: f32) -> Self {
        self.radius_multiplier = multiplier;
        self
    }
}

/// Sampling pattern for AO
#[derive(Clone, Debug)]
pub enum AOSamplingPattern {
    /// Random samples (faster, more noise)
    Random,
    /// Poisson disk sampling (better distribution)
    PoissonDisk,
    /// Halton sequence (low-discrepancy)
    Halton,
    /// Blue noise (temporal stability)
    BlueNoise,
}

/// Advanced AO settings
#[derive(Resource, Clone, Debug)]
pub struct AdvancedAOSettings {
    pub sampling_pattern: AOSamplingPattern,
    pub temporal_filter: bool,
    pub temporal_blend_factor: f32,
    pub distance_falloff: bool,
    pub max_distance: f32,
}

impl Default for AdvancedAOSettings {
    fn default() -> Self {
        Self {
            sampling_pattern: AOSamplingPattern::PoissonDisk,
            temporal_filter: true,
            temporal_blend_factor: 0.9,
            distance_falloff: true,
            max_distance: 100.0,
        }
    }
}

/// AO statistics for debugging
#[derive(Resource, Clone, Debug, Default)]
pub struct AOStats {
    pub samples_per_frame: u64,
    pub average_occlusion: f32,
    pub render_time_ms: f32,
}

/// Marker component for cameras that have SSAO applied
#[derive(Component)]
struct SSAOApplied;

/// Ambient Occlusion plugin
#[derive(Default)]
pub struct AmbientOcclusionPlugin {
    pub config: AmbientOcclusionConfig,
}

impl Plugin for AmbientOcclusionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .insert_resource(AdvancedAOSettings::default())
            .insert_resource(AOStats::default())
            .add_systems(Update, (
                apply_ssao_to_cameras_system,
                update_ssao_config_system,
            ));
    }
}

/// System to apply SSAO to cameras based on AmbientOcclusionConfig
///
/// Bevy's ScreenSpaceAmbientOcclusion component enables SSAO on a camera.
/// This system automatically applies it based on our config.
fn apply_ssao_to_cameras_system(
    mut commands: Commands,
    config: Res<AmbientOcclusionConfig>,
    cameras: Query<Entity, (With<Camera3d>, Without<SSAOApplied>)>,
) {
    // Skip if AO is disabled
    if !config.is_enabled() {
        return;
    }

    for camera_entity in cameras.iter() {
        // Apply SSAO settings based on our config
        // Note: Bevy 0.15's SSAO API uses ScreenSpaceAmbientOcclusion with
        // a quality preset. We map our config to the closest preset.

        // Bevy's SSAO quality levels are based on sample count
        // We use our config to influence the quality
        let quality = match config.num_samples {
            0..=8 => ScreenSpaceAmbientOcclusionQualityLevel::Low,
            9..=24 => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            25..=48 => ScreenSpaceAmbientOcclusionQualityLevel::High,
            _ => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
        };

        commands.entity(camera_entity).insert((
            ScreenSpaceAmbientOcclusion {
                quality_level: quality,
                constant_object_thickness: config.radius * 0.5,
            },
            SSAOApplied,
        ));

        tracing::info!(
            "Applied SSAO to camera with quality {:?} (samples={}, radius={})",
            quality,
            config.num_samples,
            config.radius
        );
    }
}

/// System to update SSAO when config changes
fn update_ssao_config_system(
    mut commands: Commands,
    config: Res<AmbientOcclusionConfig>,
    mut ssao_cameras: Query<(Entity, &mut ScreenSpaceAmbientOcclusion), With<SSAOApplied>>,
) {
    if !config.is_changed() {
        return;
    }

    for (entity, mut ssao) in ssao_cameras.iter_mut() {
        if !config.is_enabled() {
            // Remove SSAO when disabled
            commands.entity(entity).remove::<ScreenSpaceAmbientOcclusion>();
            commands.entity(entity).remove::<SSAOApplied>();
            tracing::info!("Disabled SSAO on camera");
            continue;
        }

        // Update SSAO quality based on config
        let quality = match config.num_samples {
            0..=8 => ScreenSpaceAmbientOcclusionQualityLevel::Low,
            9..=24 => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            25..=48 => ScreenSpaceAmbientOcclusionQualityLevel::High,
            _ => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
        };

        ssao.quality_level = quality;
        ssao.constant_object_thickness = config.radius * 0.5;

        tracing::debug!(
            "Updated SSAO config: quality={:?}, thickness={}",
            quality,
            ssao.constant_object_thickness
        );
    }
}

/// Helper functions for AO calculation
pub struct AOUtils;

impl AOUtils {
    /// Calculate adaptive sample count based on distance
    pub fn adaptive_sample_count(distance: f32, base_samples: u32) -> u32 {
        let distance_factor = (1.0 - (distance / 100.0).min(1.0)).max(0.1);
        (base_samples as f32 * distance_factor) as u32
    }

    /// Calculate AO radius based on object scale
    pub fn adaptive_radius(object_scale: f32, base_radius: f32) -> f32 {
        base_radius * object_scale.sqrt()
    }

    /// Estimate performance cost (relative to baseline)
    pub fn estimate_performance_cost(config: &AmbientOcclusionConfig) -> f32 {
        match config.technique {
            AOTechnique::Disabled => 0.0,
            AOTechnique::SSAO => {
                let sample_cost = config.num_samples as f32 / 16.0;
                let blur_cost = if config.blur_enabled {
                    config.blur_radius as f32 / 4.0
                } else {
                    0.0
                };
                sample_cost + blur_cost
            }
            AOTechnique::HBAO => {
                let sample_cost = config.num_samples as f32 / 8.0;
                let blur_cost = if config.blur_enabled {
                    config.blur_radius as f32 / 4.0
                } else {
                    0.0
                };
                (sample_cost + blur_cost) * 1.3 // HBAO is ~30% more expensive
            }
            AOTechnique::GTAO => {
                let sample_cost = config.num_samples as f32 / 16.0;
                sample_cost * 4.0 // GTAO is much more expensive
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ao_config_presets() {
        let ssao_low = AmbientOcclusionConfig::ssao_low();
        assert_eq!(ssao_low.technique, AOTechnique::SSAO);
        assert_eq!(ssao_low.num_samples, 8);

        let hbao_high = AmbientOcclusionConfig::hbao_high();
        assert_eq!(hbao_high.technique, AOTechnique::HBAO);
        assert_eq!(hbao_high.num_samples, 16);

        let disabled = AmbientOcclusionConfig::disabled();
        assert_eq!(disabled.technique, AOTechnique::Disabled);
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn test_ao_enabled() {
        let enabled = AmbientOcclusionConfig::ssao_medium();
        assert!(enabled.is_enabled());

        let disabled = AmbientOcclusionConfig::disabled();
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn test_ao_override() {
        let override_config = AmbientOcclusionOverride::new(true)
            .with_intensity(1.5)
            .with_radius(2.0);

        assert!(override_config.enabled);
        assert_eq!(override_config.intensity_multiplier, 1.5);
        assert_eq!(override_config.radius_multiplier, 2.0);
    }

    #[test]
    fn test_adaptive_sample_count() {
        let close = AOUtils::adaptive_sample_count(10.0, 32);
        let far = AOUtils::adaptive_sample_count(90.0, 32);

        assert!(close > far); // Closer objects get more samples
        assert!(far >= 3); // Minimum samples
    }

    #[test]
    fn test_adaptive_radius() {
        let small_radius = AOUtils::adaptive_radius(0.5, 1.0);
        let large_radius = AOUtils::adaptive_radius(4.0, 1.0);

        assert!(small_radius < 1.0);
        assert!(large_radius > 1.0);
    }

    #[test]
    fn test_performance_estimation() {
        let disabled = AmbientOcclusionConfig::disabled();
        assert_eq!(AOUtils::estimate_performance_cost(&disabled), 0.0);

        let ssao = AmbientOcclusionConfig::ssao_medium();
        let hbao = AmbientOcclusionConfig::hbao_medium();
        let gtao = AmbientOcclusionConfig::gtao();

        let ssao_cost = AOUtils::estimate_performance_cost(&ssao);
        let hbao_cost = AOUtils::estimate_performance_cost(&hbao);
        let gtao_cost = AOUtils::estimate_performance_cost(&gtao);

        assert!(hbao_cost > ssao_cost); // HBAO more expensive than SSAO
        assert!(gtao_cost > hbao_cost); // GTAO most expensive
    }

    #[test]
    fn test_advanced_settings() {
        let settings = AdvancedAOSettings::default();
        assert!(settings.temporal_filter);
        assert_eq!(settings.temporal_blend_factor, 0.9);
        assert!(settings.distance_falloff);
    }
}
