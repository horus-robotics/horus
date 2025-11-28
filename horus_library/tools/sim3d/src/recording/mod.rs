pub mod dataset_export;
pub mod manager;
pub mod sensor_data;
pub mod time_control;
pub mod trajectory;
pub mod video_export;

use bevy::prelude::*;

// Re-export key types

// Re-export recording manager and presets

/// Recording and playback plugin
pub struct RecordingPlugin;

impl Plugin for RecordingPlugin {
    fn build(&self, app: &mut App) {
        // Trajectory recording
        app.init_resource::<trajectory::RecordingSession>()
            .add_systems(Update, trajectory::record_trajectories_system)
            .add_systems(Update, trajectory::playback_trajectories_system);

        // Time control
        app.init_resource::<time_control::TimeControl>()
            .add_systems(Update, time_control::apply_time_control_system);

        // Video recording
        app.insert_resource(video_export::VideoRecorder::new(
            video_export::VideoRecordingConfig::default(),
        ));

        // Sensor data recording - initialized lazily when user starts recording
        app.insert_resource(sensor_data::SensorBagRecorder::new(
            "sim3d_recording".to_string(),
        ));

        // Note: SensorBagPlayback and DatasetRecorder are initialized on-demand
        // when the user loads a bag file or starts RL training

        info!("Recording plugin loaded with video, trajectory, and sensor recording");
    }
}
