pub mod dataset_export;
pub mod manager;
pub mod sensor_data;
pub mod time_control;
pub mod trajectory;
pub mod video_export;

use bevy::prelude::*;

/// Recording and playback plugin
pub struct RecordingPlugin;

impl Plugin for RecordingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<trajectory::RecordingSession>()
            .init_resource::<time_control::TimeControl>()
            .add_systems(Update, trajectory::record_trajectories_system)
            .add_systems(Update, trajectory::playback_trajectories_system)
            .add_systems(Update, time_control::apply_time_control_system);

        info!("Recording plugin loaded");
    }
}
