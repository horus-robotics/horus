//! Record/Replay System for HORUS
//!
//! Enables node-level granular recording and replay for debugging,
//! testing, and analysis. Features include:
//! - Record individual nodes or entire system
//! - Replay with tick-perfect determinism
//! - Mix recordings from different runs
//! - Time travel to specific ticks

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::time::SystemTime;

/// Directory for storing recordings
const RECORDINGS_DIR: &str = ".horus/recordings";

/// Recording file extension
const RECORDING_EXT: &str = "horus";

/// Maximum recording size (100MB per node by default)
const MAX_RECORDING_SIZE: usize = 100 * 1024 * 1024;

/// Recording configuration
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    /// Session name
    pub session_name: String,
    /// Base directory for recordings
    pub base_dir: PathBuf,
    /// Maximum recording size per node
    pub max_size: usize,
    /// Whether to compress recordings
    pub compress: bool,
    /// Record interval (record every N ticks, 1 = every tick)
    pub interval: u64,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(RECORDINGS_DIR);

        Self {
            session_name: format!(
                "recording_{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ),
            base_dir,
            max_size: MAX_RECORDING_SIZE,
            compress: true,
            interval: 1,
        }
    }
}

impl RecordingConfig {
    /// Create config with a named session
    pub fn with_name(name: &str) -> Self {
        Self {
            session_name: name.to_string(),
            ..Default::default()
        }
    }

    /// Get the session directory
    pub fn session_dir(&self) -> PathBuf {
        self.base_dir.join(&self.session_name)
    }

    /// Get the path for a node recording
    pub fn node_path(&self, node_name: &str, node_id: &str) -> PathBuf {
        self.session_dir()
            .join(format!("{}@{}.{}", node_name, node_id, RECORDING_EXT))
    }

    /// Get the path for the scheduler recording
    pub fn scheduler_path(&self, scheduler_id: &str) -> PathBuf {
        self.session_dir()
            .join(format!("scheduler@{}.{}", scheduler_id, RECORDING_EXT))
    }
}

/// A snapshot of a node's state at a specific tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTickSnapshot {
    /// Tick number
    pub tick: u64,
    /// Timestamp (microseconds since epoch)
    pub timestamp_us: u64,
    /// Inputs received this tick (topic -> serialized data)
    pub inputs: HashMap<String, Vec<u8>>,
    /// Outputs produced this tick (topic -> serialized data)
    pub outputs: HashMap<String, Vec<u8>>,
    /// Internal state snapshot (optional)
    pub state: Option<Vec<u8>>,
    /// Execution duration (nanoseconds)
    pub duration_ns: u64,
}

impl NodeTickSnapshot {
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            timestamp_us: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            state: None,
            duration_ns: 0,
        }
    }

    pub fn with_input(mut self, topic: &str, data: Vec<u8>) -> Self {
        self.inputs.insert(topic.to_string(), data);
        self
    }

    pub fn with_output(mut self, topic: &str, data: Vec<u8>) -> Self {
        self.outputs.insert(topic.to_string(), data);
        self
    }

    pub fn with_state(mut self, state: Vec<u8>) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_duration(mut self, duration_ns: u64) -> Self {
        self.duration_ns = duration_ns;
        self
    }
}

/// Recording of a single node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecording {
    /// Node ID (unique identifier)
    pub node_id: String,
    /// Node name
    pub node_name: String,
    /// Recording session name
    pub session_name: String,
    /// When recording started
    pub started_at: u64,
    /// When recording ended
    pub ended_at: Option<u64>,
    /// First tick recorded
    pub first_tick: u64,
    /// Last tick recorded
    pub last_tick: u64,
    /// All recorded tick snapshots
    pub snapshots: Vec<NodeTickSnapshot>,
    /// Node configuration at recording time
    pub config: Option<String>,
}

impl NodeRecording {
    pub fn new(node_name: &str, node_id: &str, session_name: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            node_name: node_name.to_string(),
            session_name: session_name.to_string(),
            started_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            ended_at: None,
            first_tick: 0,
            last_tick: 0,
            snapshots: Vec::new(),
            config: None,
        }
    }

    /// Add a tick snapshot
    pub fn add_snapshot(&mut self, snapshot: NodeTickSnapshot) {
        if self.snapshots.is_empty() {
            self.first_tick = snapshot.tick;
        }
        self.last_tick = snapshot.tick;
        self.snapshots.push(snapshot);
    }

    /// Get snapshot for a specific tick
    pub fn get_snapshot(&self, tick: u64) -> Option<&NodeTickSnapshot> {
        self.snapshots.iter().find(|s| s.tick == tick)
    }

    /// Get snapshots in a tick range
    pub fn get_snapshots_range(&self, start_tick: u64, end_tick: u64) -> Vec<&NodeTickSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.tick >= start_tick && s.tick <= end_tick)
            .collect()
    }

    /// Mark recording as ended
    pub fn finish(&mut self) {
        self.ended_at = Some(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        );
    }

    /// Get total number of snapshots
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Get estimated size in bytes
    pub fn estimated_size(&self) -> usize {
        self.snapshots
            .iter()
            .map(|s| {
                s.inputs.values().map(|v| v.len()).sum::<usize>()
                    + s.outputs.values().map(|v| v.len()).sum::<usize>()
                    + s.state.as_ref().map(|v| v.len()).unwrap_or(0)
                    + 100 // Overhead estimate
            })
            .sum()
    }

    /// Save to file
    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        // Use bincode for efficient serialization
        bincode::serialize_into(writer, self).map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(())
    }

    /// Load from file
    pub fn load(path: &PathBuf) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        bincode::deserialize_from(reader).map_err(|e| std::io::Error::other(e.to_string()))
    }
}

/// Recording of the entire scheduler/system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerRecording {
    /// Scheduler ID
    pub scheduler_id: String,
    /// Session name
    pub session_name: String,
    /// When recording started
    pub started_at: u64,
    /// When recording ended
    pub ended_at: Option<u64>,
    /// Total ticks recorded
    pub total_ticks: u64,
    /// Node recordings (node_id -> file path relative to session dir)
    pub node_recordings: HashMap<String, String>,
    /// Execution order per tick (for determinism)
    pub execution_order: Vec<Vec<String>>,
    /// Scheduler configuration at recording time
    pub config: Option<String>,
}

impl SchedulerRecording {
    pub fn new(scheduler_id: &str, session_name: &str) -> Self {
        Self {
            scheduler_id: scheduler_id.to_string(),
            session_name: session_name.to_string(),
            started_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            ended_at: None,
            total_ticks: 0,
            node_recordings: HashMap::new(),
            execution_order: Vec::new(),
            config: None,
        }
    }

    /// Register a node recording
    pub fn add_node_recording(&mut self, node_id: &str, relative_path: &str) {
        self.node_recordings
            .insert(node_id.to_string(), relative_path.to_string());
    }

    /// Record execution order for a tick
    pub fn record_execution_order(&mut self, order: Vec<String>) {
        self.execution_order.push(order);
        self.total_ticks += 1;
    }

    /// Mark recording as ended
    pub fn finish(&mut self) {
        self.ended_at = Some(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        );
    }

    /// Save to file
    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        bincode::serialize_into(writer, self).map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(())
    }

    /// Load from file
    pub fn load(path: &PathBuf) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        bincode::deserialize_from(reader).map_err(|e| std::io::Error::other(e.to_string()))
    }
}

/// Active recorder for a node
pub struct NodeRecorder {
    recording: NodeRecording,
    config: RecordingConfig,
    current_snapshot: Option<NodeTickSnapshot>,
    enabled: bool,
}

impl NodeRecorder {
    pub fn new(node_name: &str, node_id: &str, config: RecordingConfig) -> Self {
        Self {
            recording: NodeRecording::new(node_name, node_id, &config.session_name),
            config,
            current_snapshot: None,
            enabled: true,
        }
    }

    /// Start recording a new tick
    pub fn begin_tick(&mut self, tick: u64) {
        if !self.enabled {
            return;
        }

        // Check recording interval
        if tick % self.config.interval != 0 {
            self.current_snapshot = None;
            return;
        }

        self.current_snapshot = Some(NodeTickSnapshot::new(tick));
    }

    /// Record an input received
    pub fn record_input(&mut self, topic: &str, data: Vec<u8>) {
        if let Some(ref mut snapshot) = self.current_snapshot {
            snapshot.inputs.insert(topic.to_string(), data);
        }
    }

    /// Record an output produced
    pub fn record_output(&mut self, topic: &str, data: Vec<u8>) {
        if let Some(ref mut snapshot) = self.current_snapshot {
            snapshot.outputs.insert(topic.to_string(), data);
        }
    }

    /// Record internal state
    pub fn record_state(&mut self, state: Vec<u8>) {
        if let Some(ref mut snapshot) = self.current_snapshot {
            snapshot.state = Some(state);
        }
    }

    /// Finish recording the current tick
    pub fn end_tick(&mut self, duration_ns: u64) {
        if let Some(mut snapshot) = self.current_snapshot.take() {
            snapshot.duration_ns = duration_ns;
            self.recording.add_snapshot(snapshot);
        }
    }

    /// Check if we should stop (size limit reached)
    pub fn should_stop(&self) -> bool {
        self.recording.estimated_size() >= self.config.max_size
    }

    /// Finish and save the recording
    pub fn finish(&mut self) -> std::io::Result<PathBuf> {
        self.recording.finish();
        self.enabled = false;

        let path = self
            .config
            .node_path(&self.recording.node_name, &self.recording.node_id);
        self.recording.save(&path)?;

        Ok(path)
    }

    /// Get the current recording (for inspection)
    pub fn recording(&self) -> &NodeRecording {
        &self.recording
    }

    /// Enable/disable recording
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Replayer for a node recording
pub struct NodeReplayer {
    recording: NodeRecording,
    current_index: usize,
    current_tick: u64,
}

impl NodeReplayer {
    /// Load a recording from file
    pub fn load(path: &PathBuf) -> std::io::Result<Self> {
        let recording = NodeRecording::load(path)?;
        Ok(Self {
            recording,
            current_index: 0,
            current_tick: 0,
        })
    }

    /// Load from a recording struct
    pub fn from_recording(recording: NodeRecording) -> Self {
        Self {
            recording,
            current_index: 0,
            current_tick: 0,
        }
    }

    /// Get the snapshot for the current tick
    pub fn current_snapshot(&self) -> Option<&NodeTickSnapshot> {
        self.recording.snapshots.get(self.current_index)
    }

    /// Get outputs for the current tick
    pub fn get_outputs(&self) -> Option<&HashMap<String, Vec<u8>>> {
        self.current_snapshot().map(|s| &s.outputs)
    }

    /// Get a specific output for the current tick
    pub fn get_output(&self, topic: &str) -> Option<&Vec<u8>> {
        self.current_snapshot().and_then(|s| s.outputs.get(topic))
    }

    /// Advance to the next tick
    pub fn advance(&mut self) -> bool {
        if self.current_index + 1 < self.recording.snapshots.len() {
            self.current_index += 1;
            if let Some(snapshot) = self.recording.snapshots.get(self.current_index) {
                self.current_tick = snapshot.tick;
            }
            true
        } else {
            false
        }
    }

    /// Jump to a specific tick
    pub fn seek(&mut self, tick: u64) -> bool {
        for (i, snapshot) in self.recording.snapshots.iter().enumerate() {
            if snapshot.tick >= tick {
                self.current_index = i;
                self.current_tick = snapshot.tick;
                return true;
            }
        }
        false
    }

    /// Reset to the beginning
    pub fn reset(&mut self) {
        self.current_index = 0;
        self.current_tick = self.recording.first_tick;
    }

    /// Check if replay is finished
    pub fn is_finished(&self) -> bool {
        self.current_index >= self.recording.snapshots.len()
    }

    /// Get the recording
    pub fn recording(&self) -> &NodeRecording {
        &self.recording
    }

    /// Get current tick number
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get total ticks in recording
    pub fn total_ticks(&self) -> usize {
        self.recording.snapshots.len()
    }
}

/// Replay mode for the scheduler
#[derive(Debug, Clone)]
pub enum ReplayMode {
    /// Replay all nodes from a scheduler recording
    Full { scheduler_path: PathBuf },
    /// Replay specific nodes while others run live
    Mixed {
        replay_nodes: HashMap<String, PathBuf>,
    },
    /// Replay from specific ticks
    TimeTravel {
        scheduler_path: PathBuf,
        start_tick: u64,
        end_tick: Option<u64>,
    },
}

/// Manager for session discovery
pub struct RecordingManager {
    base_dir: PathBuf,
}

impl RecordingManager {
    pub fn new() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(RECORDINGS_DIR);

        Self { base_dir }
    }

    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// List all recording sessions
    pub fn list_sessions(&self) -> std::io::Result<Vec<String>> {
        let mut sessions = Vec::new();

        if self.base_dir.exists() {
            for entry in fs::read_dir(&self.base_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        sessions.push(name.to_string());
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Get all recordings in a session
    pub fn get_session_recordings(&self, session: &str) -> std::io::Result<Vec<PathBuf>> {
        let session_dir = self.base_dir.join(session);
        let mut recordings = Vec::new();

        if session_dir.exists() {
            for entry in fs::read_dir(&session_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path
                    .extension()
                    .map(|e| e == RECORDING_EXT)
                    .unwrap_or(false)
                {
                    recordings.push(path);
                }
            }
        }

        Ok(recordings)
    }

    /// Delete a session and all its recordings
    pub fn delete_session(&self, session: &str) -> std::io::Result<()> {
        let session_dir = self.base_dir.join(session);
        if session_dir.exists() {
            fs::remove_dir_all(session_dir)?;
        }
        Ok(())
    }

    /// Get total size of recordings
    pub fn total_size(&self) -> std::io::Result<u64> {
        let mut total = 0;

        if self.base_dir.exists() {
            for session in self.list_sessions()? {
                for path in self.get_session_recordings(&session)? {
                    if let Ok(metadata) = fs::metadata(&path) {
                        total += metadata.len();
                    }
                }
            }
        }

        Ok(total)
    }
}

impl Default for RecordingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Compare two recordings for differences
pub fn diff_recordings(
    recording1: &NodeRecording,
    recording2: &NodeRecording,
) -> Vec<RecordingDiff> {
    let mut diffs = Vec::new();

    // Find common tick range
    let start = recording1.first_tick.max(recording2.first_tick);
    let end = recording1.last_tick.min(recording2.last_tick);

    for tick in start..=end {
        let snap1 = recording1.get_snapshot(tick);
        let snap2 = recording2.get_snapshot(tick);

        match (snap1, snap2) {
            (Some(s1), Some(s2)) => {
                // Compare outputs
                for (topic, data1) in &s1.outputs {
                    if let Some(data2) = s2.outputs.get(topic) {
                        if data1 != data2 {
                            diffs.push(RecordingDiff::OutputDifference {
                                tick,
                                topic: topic.clone(),
                                recording1_size: data1.len(),
                                recording2_size: data2.len(),
                            });
                        }
                    } else {
                        diffs.push(RecordingDiff::MissingOutput {
                            tick,
                            topic: topic.clone(),
                            in_recording: 1,
                        });
                    }
                }

                // Check for outputs only in recording2
                for topic in s2.outputs.keys() {
                    if !s1.outputs.contains_key(topic) {
                        diffs.push(RecordingDiff::MissingOutput {
                            tick,
                            topic: topic.clone(),
                            in_recording: 2,
                        });
                    }
                }
            }
            (Some(_), None) => {
                diffs.push(RecordingDiff::MissingTick {
                    tick,
                    in_recording: 2,
                });
            }
            (None, Some(_)) => {
                diffs.push(RecordingDiff::MissingTick {
                    tick,
                    in_recording: 1,
                });
            }
            (None, None) => {}
        }
    }

    diffs
}

/// Difference between two recordings
#[derive(Debug, Clone)]
pub enum RecordingDiff {
    /// Output data differs at this tick
    OutputDifference {
        tick: u64,
        topic: String,
        recording1_size: usize,
        recording2_size: usize,
    },
    /// Output missing in one recording
    MissingOutput {
        tick: u64,
        topic: String,
        in_recording: u8,
    },
    /// Tick missing in one recording
    MissingTick { tick: u64, in_recording: u8 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_node_recording() {
        let mut recording = NodeRecording::new("test_node", "abc123", "test_session");

        let snapshot1 = NodeTickSnapshot::new(0)
            .with_input("sensor", vec![1, 2, 3])
            .with_output("motor", vec![4, 5, 6]);

        let snapshot2 = NodeTickSnapshot::new(1)
            .with_input("sensor", vec![7, 8, 9])
            .with_output("motor", vec![10, 11, 12]);

        recording.add_snapshot(snapshot1);
        recording.add_snapshot(snapshot2);

        assert_eq!(recording.first_tick, 0);
        assert_eq!(recording.last_tick, 1);
        assert_eq!(recording.snapshot_count(), 2);

        let snap = recording.get_snapshot(1).unwrap();
        assert_eq!(snap.inputs.get("sensor").unwrap(), &vec![7, 8, 9]);
    }

    #[test]
    fn test_recording_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.horus");

        let mut recording = NodeRecording::new("test_node", "abc123", "test_session");
        recording.add_snapshot(NodeTickSnapshot::new(0).with_output("out", vec![1, 2, 3]));
        recording.finish();

        recording.save(&path).unwrap();

        let loaded = NodeRecording::load(&path).unwrap();
        assert_eq!(loaded.node_name, "test_node");
        assert_eq!(loaded.snapshot_count(), 1);
    }

    #[test]
    fn test_node_recorder() {
        let dir = tempdir().unwrap();
        let config = RecordingConfig {
            session_name: "test".to_string(),
            base_dir: dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut recorder = NodeRecorder::new("test_node", "abc123", config);

        recorder.begin_tick(0);
        recorder.record_input("sensor", vec![1, 2, 3]);
        recorder.record_output("motor", vec![4, 5, 6]);
        recorder.end_tick(1000);

        recorder.begin_tick(1);
        recorder.record_input("sensor", vec![7, 8, 9]);
        recorder.record_output("motor", vec![10, 11, 12]);
        recorder.end_tick(2000);

        assert_eq!(recorder.recording().snapshot_count(), 2);

        let path = recorder.finish().unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_node_replayer() {
        let mut recording = NodeRecording::new("test_node", "abc123", "test_session");
        recording.add_snapshot(NodeTickSnapshot::new(0).with_output("motor", vec![1, 2, 3]));
        recording.add_snapshot(NodeTickSnapshot::new(1).with_output("motor", vec![4, 5, 6]));
        recording.add_snapshot(NodeTickSnapshot::new(2).with_output("motor", vec![7, 8, 9]));

        let mut replayer = NodeReplayer::from_recording(recording);

        assert_eq!(replayer.current_tick(), 0);
        assert_eq!(replayer.get_output("motor").unwrap(), &vec![1, 2, 3]);

        replayer.advance();
        assert_eq!(replayer.current_tick(), 1);
        assert_eq!(replayer.get_output("motor").unwrap(), &vec![4, 5, 6]);

        replayer.seek(2);
        assert_eq!(replayer.current_tick(), 2);

        replayer.reset();
        assert_eq!(replayer.current_tick(), 0);
    }

    #[test]
    fn test_recording_diff() {
        let mut recording1 = NodeRecording::new("node", "1", "session");
        let mut recording2 = NodeRecording::new("node", "2", "session");

        // Same tick 0
        recording1.add_snapshot(NodeTickSnapshot::new(0).with_output("out", vec![1, 2, 3]));
        recording2.add_snapshot(NodeTickSnapshot::new(0).with_output("out", vec![1, 2, 3]));

        // Different tick 1
        recording1.add_snapshot(NodeTickSnapshot::new(1).with_output("out", vec![4, 5, 6]));
        recording2.add_snapshot(NodeTickSnapshot::new(1).with_output("out", vec![7, 8, 9]));

        let diffs = diff_recordings(&recording1, &recording2);
        assert_eq!(diffs.len(), 1);

        match &diffs[0] {
            RecordingDiff::OutputDifference { tick, topic, .. } => {
                assert_eq!(*tick, 1);
                assert_eq!(topic, "out");
            }
            _ => panic!("Expected OutputDifference"),
        }
    }

    #[test]
    fn test_recording_interval() {
        let config = RecordingConfig {
            session_name: "test".to_string(),
            base_dir: PathBuf::from("/tmp"),
            interval: 2, // Record every 2 ticks
            ..Default::default()
        };

        let mut recorder = NodeRecorder::new("test_node", "abc123", config);

        recorder.begin_tick(0);
        recorder.record_output("out", vec![1]);
        recorder.end_tick(100);

        recorder.begin_tick(1); // Should be skipped
        recorder.record_output("out", vec![2]);
        recorder.end_tick(100);

        recorder.begin_tick(2);
        recorder.record_output("out", vec![3]);
        recorder.end_tick(100);

        assert_eq!(recorder.recording().snapshot_count(), 2); // Only ticks 0 and 2
    }
}
