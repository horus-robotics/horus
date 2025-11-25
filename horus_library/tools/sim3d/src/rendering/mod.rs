pub mod ambient_occlusion;
pub mod area_lights;
pub mod atmosphere;
pub mod camera_controller;
pub mod environment;
pub mod gizmos;
pub mod materials;
pub mod pbr_extended;
pub mod post_processing;
pub mod setup;
pub mod shadows;

// Re-export key rendering types and presets
pub use post_processing::{
    BloomConfig, BloomQuality, HDRConfig, ColorGrading,
    Vignette, ChromaticAberration, FilmGrain, PostProcessingPlugin,
};
pub use shadows::{ShadowConfig, ShadowQuality, ShadowsPlugin};
pub use atmosphere::{FogConfig, FogMode, VolumetricLightingConfig, AtmosphericScattering, AtmosphericParticles, ParticleType, AtmospherePlugin};
pub use ambient_occlusion::{AOTechnique, AmbientOcclusionConfig, AmbientOcclusionOverride, AOSamplingPattern, AdvancedAOSettings, AOStats, AmbientOcclusionPlugin, AOUtils};
pub use materials::{PBRMaterialProperties, MaterialPresets, MaterialLibrary, MaterialUtils, ColorPalette, MaterialPlugin};
pub use camera_controller::OrbitCamera;
pub use environment::{EnvironmentConfig, EnvironmentPlugin};
