pub mod maze;
pub mod terrain;

use bevy::prelude::*;

// Re-export key types
pub use maze::MazeConfig;
pub use terrain::{TerrainConfig, VegetationConfig};

/// Procedural generation plugin
pub struct ProceduralGenerationPlugin;

impl Plugin for ProceduralGenerationPlugin {
    fn build(&self, app: &mut App) {
        // Register terrain generation resources
        app.insert_resource(TerrainConfig::default());
        app.insert_resource(VegetationConfig::default());

        // Register maze generation resource
        app.insert_resource(MazeConfig::default());

        info!("Procedural generation plugin loaded with terrain and maze support");
    }
}
