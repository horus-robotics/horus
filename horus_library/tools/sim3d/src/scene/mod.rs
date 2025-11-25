pub mod composition;
pub mod loader;
pub mod sdf_importer;
pub mod spawner;
pub mod validation;

// Re-export SDF types for external use
pub use sdf_importer::{
    SDFImporter, SDFWorld, SDFModel, SDFLink, SDFJoint, SDFAxis,
    SDFPose, SDFInertial, SDFInertia, SDFCollision, SDFVisual,
    SDFMaterial, SDFLight, SDFAttenuation, SDFPhysics,
};

// Re-export scene loader types
pub use loader::{
    SceneDefinition, SceneObject, SceneShape, SceneRobot, SceneLighting,
    DirectionalLightConfig, LoadedScene, SceneLoader, SceneBuilder,
};

// Re-export scene spawner types
pub use spawner::{
    ObjectSpawnConfig, SpawnShape, ObjectSpawner, SpawnedObjects,
    despawn_all_objects_system,
};

// Re-export scene composition types
pub use composition::{ComposableScene, SceneInclude, SceneComposer};
