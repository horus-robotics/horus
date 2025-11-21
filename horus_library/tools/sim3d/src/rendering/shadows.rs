use bevy::pbr::{CascadeShadowConfig, CascadeShadowConfigBuilder};
use bevy::prelude::*;

/// Shadow quality preset
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowQuality {
    Low,
    Medium,
    High,
    Ultra,
    Custom,
}

/// Shadow configuration for the scene
#[derive(Resource, Clone, Debug)]
pub struct ShadowConfig {
    /// Shadow quality preset
    pub quality: ShadowQuality,

    /// Shadow map resolution (per cascade/light)
    pub resolution: usize,

    /// Number of cascade splits for directional lights
    pub num_cascades: usize,

    /// Maximum shadow distance
    pub max_distance: f32,

    /// First cascade distance ratio
    pub first_cascade_far_bound: f32,

    /// Overlap ratio between cascades
    pub overlap_proportion: f32,

    /// Soft shadow samples (for PCF filtering)
    pub soft_shadow_samples: u32,

    /// Enable contact-hardening shadows
    pub contact_hardening: bool,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self::medium()
    }
}

impl ShadowConfig {
    /// Low quality shadows (fast, low memory)
    pub fn low() -> Self {
        Self {
            quality: ShadowQuality::Low,
            resolution: 512,
            num_cascades: 1,
            max_distance: 50.0,
            first_cascade_far_bound: 50.0,
            overlap_proportion: 0.0,
            soft_shadow_samples: 1,
            contact_hardening: false,
        }
    }

    /// Medium quality shadows (balanced)
    pub fn medium() -> Self {
        Self {
            quality: ShadowQuality::Medium,
            resolution: 1024,
            num_cascades: 3,
            max_distance: 100.0,
            first_cascade_far_bound: 10.0,
            overlap_proportion: 0.2,
            soft_shadow_samples: 4,
            contact_hardening: false,
        }
    }

    /// High quality shadows (detailed)
    pub fn high() -> Self {
        Self {
            quality: ShadowQuality::High,
            resolution: 2048,
            num_cascades: 4,
            max_distance: 150.0,
            first_cascade_far_bound: 8.0,
            overlap_proportion: 0.3,
            soft_shadow_samples: 9,
            contact_hardening: true,
        }
    }

    /// Ultra quality shadows (maximum detail)
    pub fn ultra() -> Self {
        Self {
            quality: ShadowQuality::Ultra,
            resolution: 4096,
            num_cascades: 4,
            max_distance: 200.0,
            first_cascade_far_bound: 5.0,
            overlap_proportion: 0.3,
            soft_shadow_samples: 16,
            contact_hardening: true,
        }
    }

    /// Create cascade shadow configuration for directional lights
    pub fn build_cascade_config(&self) -> CascadeShadowConfig {
        CascadeShadowConfigBuilder {
            num_cascades: self.num_cascades,
            maximum_distance: self.max_distance,
            first_cascade_far_bound: self.first_cascade_far_bound,
            overlap_proportion: self.overlap_proportion,
            ..default()
        }
        .build()
    }
}

/// Shadow caster component (marks entities that cast shadows)
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ShadowCaster {
    pub enabled: bool,
    pub bias: f32,
}

impl ShadowCaster {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            bias: 0.005,
        }
    }

    pub fn with_bias(mut self, bias: f32) -> Self {
        self.bias = bias;
        self
    }
}

/// Shadow receiver component (marks entities that receive shadows)
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ShadowReceiver {
    pub enabled: bool,
}

impl ShadowReceiver {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

/// Point light shadow configuration
#[derive(Component, Clone, Debug)]
pub struct PointLightShadows {
    pub enabled: bool,
    pub resolution: usize,
    pub radius: f32,
    pub bias: f32,
}

impl Default for PointLightShadows {
    fn default() -> Self {
        Self {
            enabled: true,
            resolution: 1024,
            radius: 10.0,
            bias: 0.005,
        }
    }
}

/// Spot light shadow configuration
#[derive(Component, Clone, Debug)]
pub struct SpotLightShadows {
    pub enabled: bool,
    pub resolution: usize,
    pub bias: f32,
}

impl Default for SpotLightShadows {
    fn default() -> Self {
        Self {
            enabled: true,
            resolution: 1024,
            bias: 0.005,
        }
    }
}

/// Shadow debug visualization
#[derive(Resource, Clone, Debug)]
pub struct ShadowDebug {
    pub visualize_cascades: bool,
    pub show_shadow_frustums: bool,
    pub cascade_colors: Vec<Color>,
}

impl Default for ShadowDebug {
    fn default() -> Self {
        Self {
            visualize_cascades: false,
            show_shadow_frustums: false,
            cascade_colors: vec![
                Color::srgb(1.0, 0.0, 0.0),
                Color::srgb(0.0, 1.0, 0.0),
                Color::srgb(0.0, 0.0, 1.0),
                Color::srgb(1.0, 1.0, 0.0),
            ],
        }
    }
}

/// System to apply shadow configuration to directional lights
pub fn apply_directional_shadows_system(
    shadow_config: Res<ShadowConfig>,
    mut lights: Query<&mut DirectionalLight, Added<DirectionalLight>>,
) {
    for mut light in lights.iter_mut() {
        light.shadows_enabled = true;
        // Additional shadow configuration would go here
        // Bevy handles most of this internally
    }
}

/// System to apply shadow caster/receiver components
pub fn apply_shadow_components_system(
    mut commands: Commands,
    entities: Query<Entity, (Without<ShadowCaster>, Without<ShadowReceiver>)>,
) {
    for entity in entities.iter() {
        commands
            .entity(entity)
            .insert(ShadowCaster::new(true))
            .insert(ShadowReceiver::new(true));
    }
}

/// Shadow quality configuration plugin
pub struct ShadowsPlugin {
    pub config: ShadowConfig,
}

impl Default for ShadowsPlugin {
    fn default() -> Self {
        Self {
            config: ShadowConfig::medium(),
        }
    }
}

impl Plugin for ShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .insert_resource(ShadowDebug::default())
            .add_systems(Update, apply_directional_shadows_system);
    }
}

/// Helper functions for shadow management
pub struct ShadowUtils;

impl ShadowUtils {
    /// Calculate optimal shadow distance based on scene size
    pub fn calculate_shadow_distance(scene_radius: f32) -> f32 {
        (scene_radius * 2.0).max(50.0).min(500.0)
    }

    /// Calculate recommended cascade count based on distance
    pub fn recommend_cascade_count(max_distance: f32) -> usize {
        if max_distance < 50.0 {
            2
        } else if max_distance < 100.0 {
            3
        } else {
            4
        }
    }

    /// Calculate shadow bias based on surface angle
    pub fn calculate_bias(surface_normal: Vec3, light_direction: Vec3) -> f32 {
        let cos_theta = surface_normal.dot(-light_direction).abs();
        let base_bias = 0.005;
        base_bias * (1.0 - cos_theta).max(0.01)
    }

    /// Estimate memory usage for shadow maps (in MB)
    pub fn estimate_shadow_memory_mb(
        resolution: usize,
        num_cascades: usize,
        num_point_lights: usize,
        point_light_resolution: usize,
    ) -> f32 {
        // Directional light cascades (single channel depth)
        let cascade_memory = (resolution * resolution * num_cascades * 4) as f32;

        // Point lights (6 cube faces per light)
        let point_memory =
            (point_light_resolution * point_light_resolution * 6 * num_point_lights * 4) as f32;

        (cascade_memory + point_memory) / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_config_presets() {
        let low = ShadowConfig::low();
        assert_eq!(low.quality, ShadowQuality::Low);
        assert_eq!(low.resolution, 512);
        assert_eq!(low.num_cascades, 1);

        let ultra = ShadowConfig::ultra();
        assert_eq!(ultra.quality, ShadowQuality::Ultra);
        assert_eq!(ultra.resolution, 4096);
        assert_eq!(ultra.num_cascades, 4);
    }

    #[test]
    fn test_cascade_config_build() {
        let config = ShadowConfig::medium();
        let cascade = config.build_cascade_config();
        // Cascade config is built successfully (no panic)
        assert_eq!(config.num_cascades, 3);
    }

    #[test]
    fn test_shadow_caster() {
        let caster = ShadowCaster::new(true).with_bias(0.01);
        assert!(caster.enabled);
        assert_eq!(caster.bias, 0.01);
    }

    #[test]
    fn test_shadow_receiver() {
        let receiver = ShadowReceiver::new(true);
        assert!(receiver.enabled);
    }

    #[test]
    fn test_point_light_shadows() {
        let shadows = PointLightShadows::default();
        assert!(shadows.enabled);
        assert_eq!(shadows.resolution, 1024);
    }

    #[test]
    fn test_shadow_distance_calculation() {
        let distance = ShadowUtils::calculate_shadow_distance(30.0);
        assert!(distance >= 50.0);
        assert!(distance <= 500.0);

        let large_distance = ShadowUtils::calculate_shadow_distance(300.0);
        assert_eq!(large_distance, 500.0); // Capped at max
    }

    #[test]
    fn test_cascade_count_recommendation() {
        assert_eq!(ShadowUtils::recommend_cascade_count(40.0), 2);
        assert_eq!(ShadowUtils::recommend_cascade_count(80.0), 3);
        assert_eq!(ShadowUtils::recommend_cascade_count(150.0), 4);
    }

    #[test]
    fn test_bias_calculation() {
        let normal = Vec3::Y;
        let light_down = Vec3::NEG_Y;
        let bias = ShadowUtils::calculate_bias(normal, light_down);
        assert!(bias > 0.0);
        assert!(bias < 0.1);
    }

    #[test]
    fn test_shadow_memory_estimation() {
        let memory = ShadowUtils::estimate_shadow_memory_mb(1024, 3, 2, 512);
        assert!(memory > 0.0);
        assert!(memory < 100.0); // Reasonable estimate
    }

    #[test]
    fn test_shadow_debug() {
        let debug = ShadowDebug::default();
        assert!(!debug.visualize_cascades);
        assert!(!debug.show_shadow_frustums);
        assert_eq!(debug.cascade_colors.len(), 4);
    }
}
