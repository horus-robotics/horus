pub mod composition;
pub mod loader;
pub mod sdf_importer;
pub mod spawner;
pub mod validation;

// Re-export SDF types for external use
pub use sdf_importer::{
    SDFAttenuation, SDFAxis, SDFCollision, SDFImporter, SDFInertia, SDFInertial, SDFJoint,
    SDFLight, SDFLink, SDFMaterial, SDFModel, SDFPhysics, SDFPose, SDFVisual, SDFWorld,
};

// Re-export scene loader types
pub use loader::{
    DirectionalLightConfig, LoadedScene, SceneBuilder, SceneDefinition, SceneLighting, SceneLoader,
    SceneObject, SceneRobot, SceneShape,
};

// Re-export scene spawner types
pub use spawner::{
    despawn_all_objects_system, ObjectSpawnConfig, ObjectSpawner, SpawnShape, SpawnedObjects,
};

// Re-export scene composition types
pub use composition::{ComposableScene, SceneComposer, SceneInclude};
