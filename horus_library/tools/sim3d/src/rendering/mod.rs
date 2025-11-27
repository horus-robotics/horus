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
pub use ambient_occlusion::{
    AOSamplingPattern, AOStats, AOTechnique, AOUtils, AdvancedAOSettings, AmbientOcclusionConfig,
    AmbientOcclusionOverride, AmbientOcclusionPlugin,
};
pub use atmosphere::{
    AtmospherePlugin, AtmosphericParticles, AtmosphericScattering, FogConfig, FogMode,
    ParticleType, VolumetricLightingConfig,
};
pub use camera_controller::OrbitCamera;
pub use environment::{EnvironmentConfig, EnvironmentPlugin};
pub use materials::{
    ColorPalette, MaterialLibrary, MaterialPlugin, MaterialPresets, MaterialUtils,
    PBRMaterialProperties,
};
pub use post_processing::{
    BloomConfig, BloomQuality, ChromaticAberration, ColorGrading, FilmGrain, HDRConfig,
    PostProcessingPlugin, Vignette,
};
pub use shadows::{ShadowConfig, ShadowQuality, ShadowsPlugin};
