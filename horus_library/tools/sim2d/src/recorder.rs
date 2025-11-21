//! Recording and playback system for sim2d
//!
//! This module provides functionality to record simulation frames and play them back,
//! as well as export recordings to video files and CSV data.

use crate::{scenario::TrajectoryPoint, RobotConfig};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single recorded frame in the simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedFrame {
    /// Frame number (0-indexed)
    pub frame_number: u64,

    /// Simulation time in seconds
    pub time: f64,

    /// Robot states at this frame
    pub robot_states: Vec<RobotFrameState>,

    /// Optional: Screenshot data (PNG encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<Vec<u8>>,
}

/// State of a robot at a specific frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotFrameState {
    /// Robot name/ID
    pub name: String,

    /// Position [x, y] in meters
    pub position: [f32; 2],

    /// Heading angle in radians
    pub heading: f32,

    /// Linear velocity in m/s
    pub linear_velocity: f32,

    /// Angular velocity in rad/s
    pub angular_velocity: f32,

    /// Optional: Sensor data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lidar_scan: Option<Vec<f32>>,
}

/// Complete recording session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    /// Recording metadata
    pub metadata: RecordingMetadata,

    /// All recorded frames
    pub frames: Vec<RecordedFrame>,

    /// Robot configurations
    pub robot_configs: Vec<RobotConfig>,
}

/// Metadata about a recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    /// Recording name
    pub name: String,

    /// Description
    pub description: String,

    /// Total duration in seconds
    pub duration: f64,

    /// Frame rate (frames per second)
    pub framerate: f32,

    /// Total number of frames
    pub frame_count: u64,

    /// Recording start time (ISO 8601)
    pub recorded_at: String,
}

impl Recording {
    /// Create a new empty recording
    pub fn new(name: impl Into<String>, description: impl Into<String>, framerate: f32) -> Self {
        Self {
            metadata: RecordingMetadata {
                name: name.into(),
                description: description.into(),
                duration: 0.0,
                framerate,
                frame_count: 0,
                recorded_at: chrono::Utc::now().to_rfc3339(),
            },
            frames: Vec::new(),
            robot_configs: Vec::new(),
        }
    }

    /// Add a frame to the recording
    pub fn add_frame(&mut self, frame: RecordedFrame) {
        self.metadata.frame_count = frame.frame_number + 1;
        self.metadata.duration = frame.time;
        self.frames.push(frame);
    }

    /// Set robot configurations
    pub fn set_robot_configs(&mut self, configs: Vec<RobotConfig>) {
        self.robot_configs = configs;
    }

    /// Save recording to a YAML file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let yaml = serde_yaml::to_string(self).context("Failed to serialize recording to YAML")?;

        std::fs::write(path, yaml).context(format!("Failed to write recording to {:?}", path))?;

        Ok(())
    }

    /// Load recording from a YAML file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let yaml = std::fs::read_to_string(path)
            .context(format!("Failed to read recording from {:?}", path))?;

        let recording: Recording =
            serde_yaml::from_str(&yaml).context("Failed to deserialize recording from YAML")?;

        Ok(recording)
    }

    /// Export trajectory data to CSV
    pub fn export_to_csv(&self, path: &Path) -> Result<()> {
        use std::io::Write;

        let mut file = std::fs::File::create(path)
            .context(format!("Failed to create CSV file at {:?}", path))?;

        // Write header
        writeln!(
            file,
            "frame,time,robot_name,pos_x,pos_y,heading,linear_vel,angular_vel"
        )?;

        // Write data rows
        for frame in &self.frames {
            for robot in &frame.robot_states {
                writeln!(
                    file,
                    "{},{},{},{},{},{},{},{}",
                    frame.frame_number,
                    frame.time,
                    robot.name,
                    robot.position[0],
                    robot.position[1],
                    robot.heading,
                    robot.linear_velocity,
                    robot.angular_velocity,
                )?;
            }
        }

        Ok(())
    }

    /// Export to video using ffmpeg (requires ffmpeg to be installed)
    pub fn export_to_video(&self, output_path: &Path, temp_dir: &Path) -> Result<()> {
        use std::process::Command;

        // Ensure temp directory exists
        std::fs::create_dir_all(temp_dir)
            .context("Failed to create temporary directory for video export")?;

        // Extract all screenshots as PNG files
        for (i, frame) in self.frames.iter().enumerate() {
            if let Some(screenshot) = &frame.screenshot {
                let frame_path = temp_dir.join(format!("frame_{:06}.png", i));
                std::fs::write(&frame_path, screenshot).context(format!(
                    "Failed to write frame {:06} to {:?}",
                    i, frame_path
                ))?;
            }
        }

        // Use ffmpeg to create video
        let output = Command::new("ffmpeg")
            .arg("-y") // Overwrite output file
            .arg("-framerate")
            .arg(format!("{}", self.metadata.framerate))
            .arg("-i")
            .arg(
                temp_dir
                    .join("frame_%06d.png")
                    .to_string_lossy()
                    .to_string(),
            )
            .arg("-c:v")
            .arg("libx264")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg(output_path.to_string_lossy().to_string())
            .output()
            .context("Failed to execute ffmpeg - is it installed?")?;

        if !output.status.success() {
            anyhow::bail!("ffmpeg failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Clean up temporary files
        std::fs::remove_dir_all(temp_dir).context("Failed to clean up temporary directory")?;

        Ok(())
    }

    /// Get frame at specific time
    pub fn get_frame_at_time(&self, time: f64) -> Option<&RecordedFrame> {
        self.frames
            .iter()
            .min_by_key(|f| ((f.time - time).abs() * 1000.0) as i64)
    }

    /// Get frame by frame number
    pub fn get_frame(&self, frame_number: u64) -> Option<&RecordedFrame> {
        self.frames.iter().find(|f| f.frame_number == frame_number)
    }

    /// Convert to trajectory points for a specific robot
    pub fn to_trajectory(&self, robot_name: &str) -> Vec<TrajectoryPoint> {
        self.frames
            .iter()
            .filter_map(|frame| {
                frame
                    .robot_states
                    .iter()
                    .find(|r| r.name == robot_name)
                    .map(|robot| TrajectoryPoint {
                        time: frame.time,
                        pose: [robot.position[0], robot.position[1], robot.heading],
                        velocity: [robot.linear_velocity, robot.angular_velocity],
                    })
            })
            .collect()
    }
}

/// Recorder for capturing simulation frames
#[derive(bevy::prelude::Resource)]
pub struct Recorder {
    /// Current recording
    recording: Option<Recording>,

    /// Whether recording is active
    is_recording: bool,

    /// Frame rate for recording
    framerate: f32,

    /// Whether to capture screenshots
    capture_screenshots: bool,

    /// Frame counter
    frame_counter: u64,
}

impl Recorder {
    /// Create a new recorder
    pub fn new(framerate: f32) -> Self {
        Self {
            recording: None,
            is_recording: false,
            framerate,
            capture_screenshots: false,
            frame_counter: 0,
        }
    }

    /// Start recording
    pub fn start_recording(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        robot_configs: Vec<RobotConfig>,
    ) {
        let mut recording = Recording::new(name, description, self.framerate);
        recording.set_robot_configs(robot_configs);

        self.recording = Some(recording);
        self.is_recording = true;
        self.frame_counter = 0;
    }

    /// Stop recording and return the recording
    pub fn stop_recording(&mut self) -> Option<Recording> {
        self.is_recording = false;
        self.recording.take()
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Enable/disable screenshot capture
    pub fn set_capture_screenshots(&mut self, enabled: bool) {
        self.capture_screenshots = enabled;
    }

    /// Record a frame
    pub fn record_frame(
        &mut self,
        time: f64,
        robot_states: Vec<RobotFrameState>,
        screenshot: Option<Vec<u8>>,
    ) {
        if !self.is_recording {
            return;
        }

        if let Some(recording) = &mut self.recording {
            let frame = RecordedFrame {
                frame_number: self.frame_counter,
                time,
                robot_states,
                screenshot: if self.capture_screenshots {
                    screenshot
                } else {
                    None
                },
            };

            recording.add_frame(frame);
            self.frame_counter += 1;
        }
    }

    /// Get current recording (if any)
    pub fn get_recording(&self) -> Option<&Recording> {
        self.recording.as_ref()
    }

    /// Get recording metadata
    pub fn get_metadata(&self) -> Option<&RecordingMetadata> {
        self.recording.as_ref().map(|r| &r.metadata)
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new(60.0) // 60 FPS by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_lifecycle() {
        let mut recorder = Recorder::new(30.0);

        // Start recording
        recorder.start_recording("Test Recording", "A test", vec![]);
        assert!(recorder.is_recording());

        // Record some frames
        for i in 0..10 {
            recorder.record_frame(
                i as f64 * 0.033,
                vec![RobotFrameState {
                    name: "robot1".to_string(),
                    position: [i as f32, i as f32 * 2.0],
                    heading: 0.0,
                    linear_velocity: 1.0,
                    angular_velocity: 0.0,
                    lidar_scan: None,
                }],
                None,
            );
        }

        // Stop recording
        let recording = recorder.stop_recording().unwrap();
        assert!(!recorder.is_recording());
        assert_eq!(recording.frames.len(), 10);
        assert_eq!(recording.metadata.frame_count, 10);
    }

    #[test]
    fn test_recording_save_load() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_recording.yaml");

        // Create a recording
        let mut recording = Recording::new("Test", "Description", 60.0);

        for i in 0..5 {
            recording.add_frame(RecordedFrame {
                frame_number: i,
                time: i as f64 * 0.016,
                robot_states: vec![RobotFrameState {
                    name: "robot1".to_string(),
                    position: [i as f32, i as f32],
                    heading: 0.0,
                    linear_velocity: 1.0,
                    angular_velocity: 0.0,
                    lidar_scan: None,
                }],
                screenshot: None,
            });
        }

        // Save
        recording.save_to_file(&test_path).unwrap();
        assert!(test_path.exists());

        // Load
        let loaded = Recording::load_from_file(&test_path).unwrap();
        assert_eq!(loaded.metadata.name, "Test");
        assert_eq!(loaded.frames.len(), 5);
        assert_eq!(loaded.frames[2].robot_states[0].position, [2.0, 2.0]);
    }

    #[test]
    fn test_csv_export() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_recording.csv");

        let mut recording = Recording::new("Test", "Description", 60.0);

        for i in 0..3 {
            recording.add_frame(RecordedFrame {
                frame_number: i,
                time: i as f64 * 0.016,
                robot_states: vec![RobotFrameState {
                    name: "robot1".to_string(),
                    position: [i as f32, i as f32 * 2.0],
                    heading: 0.5,
                    linear_velocity: 1.0,
                    angular_velocity: 0.1,
                    lidar_scan: None,
                }],
                screenshot: None,
            });
        }

        // Export to CSV
        recording.export_to_csv(&test_path).unwrap();
        assert!(test_path.exists());

        // Read and verify content
        let content = std::fs::read_to_string(&test_path).unwrap();
        assert!(content.contains("frame,time,robot_name"));
        assert!(content.contains("robot1"));
        assert!(content.contains("0,0,robot1"));
    }

    #[test]
    fn test_trajectory_conversion() {
        let mut recording = Recording::new("Test", "Description", 60.0);

        for i in 0..5 {
            recording.add_frame(RecordedFrame {
                frame_number: i,
                time: i as f64,
                robot_states: vec![RobotFrameState {
                    name: "robot1".to_string(),
                    position: [i as f32, i as f32 * 2.0],
                    heading: i as f32 * 0.1,
                    linear_velocity: 1.0,
                    angular_velocity: 0.1,
                    lidar_scan: None,
                }],
                screenshot: None,
            });
        }

        let trajectory = recording.to_trajectory("robot1");
        assert_eq!(trajectory.len(), 5);
        assert_eq!(trajectory[0].pose, [0.0, 0.0, 0.0]);
        assert_eq!(trajectory[2].pose, [2.0, 4.0, 0.2]);
        assert_eq!(trajectory[4].velocity, [1.0, 0.1]);
    }
}
