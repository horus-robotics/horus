use bevy::prelude::*;
use std::path::PathBuf;

use super::{
    dataset_export::DatasetRecorder, sensor_data::SensorBagRecorder, trajectory::RecordingSession,
    video_export::VideoRecorder,
};

/// Recording manager - unified interface for all recording features
#[derive(Resource)]
pub struct RecordingManager {
    pub session_name: String,
    pub output_directory: PathBuf,
    pub recording_active: bool,
    pub start_time: f64,

    // Feature flags
    pub record_trajectories: bool,
    pub record_sensors: bool,
    pub record_video: bool,
    pub record_dataset: bool,
}

impl RecordingManager {
    pub fn new(session_name: String, output_directory: PathBuf) -> Self {
        Self {
            session_name,
            output_directory,
            recording_active: false,
            start_time: 0.0,
            record_trajectories: true,
            record_sensors: true,
            record_video: false,
            record_dataset: false,
        }
    }

    /// Start all enabled recording features
    pub fn start_recording(
        &mut self,
        current_time: f64,
        trajectory_session: Option<&mut RecordingSession>,
        sensor_recorder: Option<&mut SensorBagRecorder>,
        video_recorder: Option<&mut VideoRecorder>,
        dataset_recorder: Option<&mut DatasetRecorder>,
    ) {
        self.recording_active = true;
        self.start_time = current_time;

        if self.record_trajectories {
            if let Some(session) = trajectory_session {
                session.start(current_time);
            }
        }

        if self.record_sensors {
            if let Some(recorder) = sensor_recorder {
                recorder.start_recording();
            }
        }

        if self.record_video {
            if let Some(recorder) = video_recorder {
                recorder.start_recording(current_time);
            }
        }

        if self.record_dataset {
            if let Some(recorder) = dataset_recorder {
                recorder.start_recording();
            }
        }

        info!("Recording started: {}", self.session_name);
    }

    /// Stop all active recording features
    pub fn stop_recording(
        &mut self,
        trajectory_session: Option<&mut RecordingSession>,
        sensor_recorder: Option<&mut SensorBagRecorder>,
        video_recorder: Option<&mut VideoRecorder>,
        dataset_recorder: Option<&mut DatasetRecorder>,
    ) {
        self.recording_active = false;

        if let Some(session) = trajectory_session {
            session.stop();
        }

        if let Some(recorder) = sensor_recorder {
            recorder.stop_recording();
        }

        if let Some(recorder) = video_recorder {
            recorder.stop_recording();
        }

        if let Some(recorder) = dataset_recorder {
            recorder.stop_recording();
        }

        info!("Recording stopped: {}", self.session_name);
    }

    /// Export all recorded data to disk
    pub fn export_all(
        &self,
        trajectory_session: Option<&RecordingSession>,
        sensor_recorder: Option<&mut SensorBagRecorder>,
        video_recorder: Option<&VideoRecorder>,
        dataset_recorder: Option<&mut DatasetRecorder>,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.output_directory)?;

        // Export trajectories
        if self.record_trajectories {
            if let Some(session) = trajectory_session {
                let path = self
                    .output_directory
                    .join(format!("{}_trajectories.json", self.session_name));
                session.save_to_file(&path)?;
                info!("Trajectories exported to: {:?}", path);
            }
        }

        // Export sensor data
        if self.record_sensors {
            if let Some(recorder) = sensor_recorder {
                let path = self
                    .output_directory
                    .join(format!("{}_sensors.bag", self.session_name));
                recorder.bag.save_to_file(&path)?;
                info!("Sensor data exported to: {:?}", path);
            }
        }

        // Export video
        if self.record_video {
            if let Some(recorder) = video_recorder {
                recorder.export_frames()?;
                info!("Video frames exported");
            }
        }

        // Export dataset
        if self.record_dataset {
            if let Some(recorder) = dataset_recorder {
                let path = self
                    .output_directory
                    .join(format!("{}_dataset.json.gz", self.session_name));
                recorder.export(&path, super::dataset_export::DatasetFormat::JSON)?;
                info!("Dataset exported to: {:?}", path);
            }
        }

        Ok(())
    }

    pub fn get_duration(&self, current_time: f64) -> f64 {
        if self.recording_active {
            current_time - self.start_time
        } else {
            0.0
        }
    }

    pub fn enable_trajectory_recording(&mut self) {
        self.record_trajectories = true;
    }

    pub fn disable_trajectory_recording(&mut self) {
        self.record_trajectories = false;
    }

    pub fn enable_sensor_recording(&mut self) {
        self.record_sensors = true;
    }

    pub fn disable_sensor_recording(&mut self) {
        self.record_sensors = false;
    }

    pub fn enable_video_recording(&mut self) {
        self.record_video = true;
    }

    pub fn disable_video_recording(&mut self) {
        self.record_video = false;
    }

    pub fn enable_dataset_recording(&mut self) {
        self.record_dataset = true;
    }

    pub fn disable_dataset_recording(&mut self) {
        self.record_dataset = false;
    }
}

/// Recording preset configurations
pub struct RecordingPresets;

impl RecordingPresets {
    /// Minimal recording (trajectories only)
    pub fn minimal(session_name: String, output_dir: PathBuf) -> RecordingManager {
        let mut manager = RecordingManager::new(session_name, output_dir);
        manager.record_trajectories = true;
        manager.record_sensors = false;
        manager.record_video = false;
        manager.record_dataset = false;
        manager
    }

    /// Standard recording (trajectories + sensors)
    pub fn standard(session_name: String, output_dir: PathBuf) -> RecordingManager {
        let mut manager = RecordingManager::new(session_name, output_dir);
        manager.record_trajectories = true;
        manager.record_sensors = true;
        manager.record_video = false;
        manager.record_dataset = false;
        manager
    }

    /// Full recording (everything)
    pub fn full(session_name: String, output_dir: PathBuf) -> RecordingManager {
        let mut manager = RecordingManager::new(session_name, output_dir);
        manager.record_trajectories = true;
        manager.record_sensors = true;
        manager.record_video = true;
        manager.record_dataset = true;
        manager
    }

    /// RL training recording (dataset + trajectories)
    pub fn rl_training(session_name: String, output_dir: PathBuf) -> RecordingManager {
        let mut manager = RecordingManager::new(session_name, output_dir);
        manager.record_trajectories = true;
        manager.record_sensors = false;
        manager.record_video = false;
        manager.record_dataset = true;
        manager
    }

    /// Video demonstration (video + trajectories)
    pub fn video_demo(session_name: String, output_dir: PathBuf) -> RecordingManager {
        let mut manager = RecordingManager::new(session_name, output_dir);
        manager.record_trajectories = true;
        manager.record_sensors = false;
        manager.record_video = true;
        manager.record_dataset = false;
        manager
    }
}

/// Recording statistics
#[derive(Clone, Debug)]
pub struct RecordingStats {
    pub duration: f64,
    pub trajectory_count: usize,
    pub sensor_message_count: usize,
    pub video_frame_count: u32,
    pub dataset_episode_count: usize,
    pub total_size_mb: f64,
}

impl RecordingStats {
    pub fn gather(
        manager: &RecordingManager,
        current_time: f64,
        trajectory_session: Option<&RecordingSession>,
        sensor_recorder: Option<&SensorBagRecorder>,
        video_recorder: Option<&VideoRecorder>,
        dataset_recorder: Option<&DatasetRecorder>,
    ) -> Self {
        let duration = manager.get_duration(current_time);

        let trajectory_count = trajectory_session
            .map(|s| s.trajectories.values().map(|t| t.points.len()).sum())
            .unwrap_or(0);

        let sensor_message_count = sensor_recorder.map(|r| r.bag.messages.len()).unwrap_or(0);

        let video_frame_count = video_recorder.map(|r| r.frame_count).unwrap_or(0);

        let dataset_episode_count = dataset_recorder
            .map(|r| r.dataset.episodes.len())
            .unwrap_or(0);

        // Rough size estimation
        let mut total_size_mb = 0.0;

        if let Some(recorder) = video_recorder {
            total_size_mb += recorder.get_estimated_size_mb();
        }

        // Estimate sensor data size (rough)
        total_size_mb += (sensor_message_count * 1024) as f64 / (1024.0 * 1024.0);

        // Estimate trajectory size (rough)
        total_size_mb += (trajectory_count * 64) as f64 / (1024.0 * 1024.0);

        Self {
            duration,
            trajectory_count,
            sensor_message_count,
            video_frame_count,
            dataset_episode_count,
            total_size_mb,
        }
    }

    pub fn print_summary(&self) {
        println!("Recording Statistics:");
        println!("  Duration: {:.2}s", self.duration);
        println!("  Trajectory Points: {}", self.trajectory_count);
        println!("  Sensor Messages: {}", self.sensor_message_count);
        println!("  Video Frames: {}", self.video_frame_count);
        println!("  Dataset Episodes: {}", self.dataset_episode_count);
        println!("  Estimated Size: {:.2} MB", self.total_size_mb);
    }
}

/// Quick-start commands for common recording scenarios
pub struct RecordingCommands;

impl RecordingCommands {
    /// Create a manager and start recording with preset
    pub fn start_with_preset(
        preset_name: &str,
        session_name: String,
        output_dir: PathBuf,
    ) -> RecordingManager {
        match preset_name {
            "minimal" => RecordingPresets::minimal(session_name, output_dir),
            "standard" => RecordingPresets::standard(session_name, output_dir),
            "full" => RecordingPresets::full(session_name, output_dir),
            "rl_training" => RecordingPresets::rl_training(session_name, output_dir),
            "video_demo" => RecordingPresets::video_demo(session_name, output_dir),
            _ => RecordingPresets::standard(session_name, output_dir),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = RecordingManager::new("test".to_string(), PathBuf::from("/tmp"));
        assert_eq!(manager.session_name, "test");
        assert!(!manager.recording_active);
        assert!(manager.record_trajectories);
        assert!(manager.record_sensors);
    }

    #[test]
    fn test_feature_toggles() {
        let mut manager = RecordingManager::new("test".to_string(), PathBuf::from("/tmp"));

        manager.disable_trajectory_recording();
        assert!(!manager.record_trajectories);

        manager.enable_trajectory_recording();
        assert!(manager.record_trajectories);
    }

    #[test]
    fn test_presets() {
        let minimal = RecordingPresets::minimal("test".to_string(), PathBuf::from("/tmp"));
        assert!(minimal.record_trajectories);
        assert!(!minimal.record_sensors);
        assert!(!minimal.record_video);
        assert!(!minimal.record_dataset);

        let full = RecordingPresets::full("test".to_string(), PathBuf::from("/tmp"));
        assert!(full.record_trajectories);
        assert!(full.record_sensors);
        assert!(full.record_video);
        assert!(full.record_dataset);

        let rl = RecordingPresets::rl_training("test".to_string(), PathBuf::from("/tmp"));
        assert!(rl.record_trajectories);
        assert!(!rl.record_sensors);
        assert!(!rl.record_video);
        assert!(rl.record_dataset);
    }

    #[test]
    fn test_duration() {
        let mut manager = RecordingManager::new("test".to_string(), PathBuf::from("/tmp"));

        assert_eq!(manager.get_duration(5.0), 0.0);

        manager.recording_active = true;
        manager.start_time = 2.0;

        assert_eq!(manager.get_duration(5.0), 3.0);
    }

    #[test]
    fn test_recording_commands() {
        let manager = RecordingCommands::start_with_preset(
            "minimal",
            "test".to_string(),
            PathBuf::from("/tmp"),
        );
        assert!(manager.record_trajectories);
        assert!(!manager.record_video);
    }

    #[test]
    fn test_stats_creation() {
        let manager = RecordingManager::new("test".to_string(), PathBuf::from("/tmp"));

        let stats = RecordingStats::gather(&manager, 0.0, None, None, None, None);

        assert_eq!(stats.trajectory_count, 0);
        assert_eq!(stats.sensor_message_count, 0);
        assert_eq!(stats.video_frame_count, 0);
        assert_eq!(stats.dataset_episode_count, 0);
    }
}
