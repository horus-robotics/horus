pub mod maze;
pub mod terrain;

use bevy::prelude::*;

/// Procedural generation plugin
pub struct ProceduralGenerationPlugin;

impl Plugin for ProceduralGenerationPlugin {
    fn build(&self, app: &mut App) {
        // Add procedural generation resources and systems
        info!("Procedural generation plugin loaded");
    }
}
