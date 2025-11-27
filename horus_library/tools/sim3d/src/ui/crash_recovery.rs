//! Crash Recovery and Auto-Save Module for sim3d
//!
//! Provides automatic save functionality and crash recovery capabilities:
//! - Periodic auto-save with configurable intervals
//! - Rotating backup files to prevent data loss
//! - Startup recovery detection and restoration
//! - Unsaved changes tracking

use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};

// ============================================================================
// Recovery State
// ============================================================================

/// State of a recovery file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RecoveryState {
    /// Recovery file exists and is pending user decision
    #[default]
    Pending,
    /// Recovery was applied
    Recovered,
    /// Recovery was dismissed by user
    Dismissed,
    /// Recovery file is corrupted
    Corrupted,
}

impl RecoveryState {
    /// Returns whether this state represents an actionable recovery
    pub fn is_actionable(&self) -> bool {
        *self == RecoveryState::Pending
    }

    /// Returns a human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            RecoveryState::Pending => "Pending",
            RecoveryState::Recovered => "Recovered",
            RecoveryState::Dismissed => "Dismissed",
            RecoveryState::Corrupted => "Corrupted",
        }
    }
}

// ============================================================================
// Recovery File
// ============================================================================

/// Represents a recovery/auto-save file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryFile {
    /// Path to the recovery file
    pub path: PathBuf,
    /// Original scene path that was being edited
    pub original_path: Option<PathBuf>,
    /// Unix timestamp when the recovery file was created
    pub timestamp: u64,
    /// Hash of the scene content for change detection
    pub scene_hash: u64,
    /// Current state of the recovery
    pub state: RecoveryState,
    /// Size of the recovery file in bytes
    pub size_bytes: u64,
    /// Whether the file is compressed
    pub compressed: bool,
    /// Optional description/metadata
    pub description: Option<String>,
}

impl RecoveryFile {
    /// Creates a new RecoveryFile entry
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let size_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            path,
            original_path: None,
            timestamp,
            scene_hash: 0,
            state: RecoveryState::Pending,
            size_bytes,
            compressed: false,
            description: None,
        }
    }

    /// Sets the original path
    pub fn with_original_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.original_path = Some(path.into());
        self
    }

    /// Sets the scene hash
    pub fn with_hash(mut self, hash: u64) -> Self {
        self.scene_hash = hash;
        self
    }

    /// Sets compression flag
    pub fn compressed(mut self) -> Self {
        self.compressed = true;
        self
    }

    /// Sets description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Returns a formatted timestamp string
    pub fn formatted_time(&self) -> String {
        use chrono::{DateTime, Local, TimeZone};

        if let Some(dt) = Local.timestamp_opt(self.timestamp as i64, 0).single() {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            "Unknown".to_string()
        }
    }

    /// Returns age of the recovery file as a duration
    pub fn age(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Duration::from_secs(now.saturating_sub(self.timestamp))
    }

    /// Returns a human-readable age string
    pub fn age_string(&self) -> String {
        let age = self.age();
        let secs = age.as_secs();

        if secs < 60 {
            "Just now".to_string()
        } else if secs < 3600 {
            format!("{} minutes ago", secs / 60)
        } else if secs < 86400 {
            format!("{} hours ago", secs / 3600)
        } else {
            format!("{} days ago", secs / 86400)
        }
    }

    /// Returns the display name (original filename or recovery filename)
    pub fn display_name(&self) -> String {
        if let Some(original) = &self.original_path {
            original
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        } else {
            self.path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Recovery")
                .to_string()
        }
    }

    /// Checks if the recovery file still exists on disk
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Returns formatted size string
    pub fn formatted_size(&self) -> String {
        if self.size_bytes < 1024 {
            format!("{} B", self.size_bytes)
        } else if self.size_bytes < 1024 * 1024 {
            format!("{:.1} KB", self.size_bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", self.size_bytes as f64 / (1024.0 * 1024.0))
        }
    }
}

// ============================================================================
// Auto-Save Configuration
// ============================================================================

/// Configuration for the auto-save system
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct AutoSaveConfig {
    /// Whether auto-save is enabled
    pub enabled: bool,
    /// Interval between auto-saves
    pub interval: Duration,
    /// Maximum number of auto-save files to keep (rotating)
    pub max_files: usize,
    /// Directory for auto-save files
    pub directory: PathBuf,
    /// Whether to save on application exit
    pub save_on_exit: bool,
    /// Whether to compress auto-save files
    pub compress: bool,
    /// Whether to show notification on auto-save
    pub show_notification: bool,
    /// Minimum time since last change before auto-saving
    pub idle_threshold: Duration,
    /// File extension for auto-save files
    pub file_extension: String,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(300), // 5 minutes
            max_files: 5,
            directory: Self::default_directory(),
            save_on_exit: true,
            compress: true,
            show_notification: true,
            idle_threshold: Duration::from_secs(2),
            file_extension: "autosave".to_string(),
        }
    }
}

impl AutoSaveConfig {
    /// Creates a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the default auto-save directory
    pub fn default_directory() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sim3d")
            .join("autosave")
    }

    /// Sets whether auto-save is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the auto-save interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Sets the interval in seconds
    pub fn with_interval_secs(mut self, secs: u64) -> Self {
        self.interval = Duration::from_secs(secs);
        self
    }

    /// Sets the maximum number of auto-save files
    pub fn with_max_files(mut self, max: usize) -> Self {
        self.max_files = max.max(1);
        self
    }

    /// Sets the auto-save directory
    pub fn with_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.directory = dir.into();
        self
    }

    /// Sets save on exit behavior
    pub fn with_save_on_exit(mut self, enabled: bool) -> Self {
        self.save_on_exit = enabled;
        self
    }

    /// Sets compression behavior
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compress = enabled;
        self
    }

    /// Generates the path for an auto-save file with the given index
    pub fn auto_save_path(&self, index: usize) -> PathBuf {
        let ext = if self.compress {
            format!("{}.gz", self.file_extension)
        } else {
            self.file_extension.clone()
        };

        self.directory.join(format!("autosave_{}.{}", index, ext))
    }

    /// Generates the path for a crash recovery metadata file
    pub fn recovery_metadata_path(&self) -> PathBuf {
        self.directory.join("recovery.json")
    }
}

// ============================================================================
// Auto-Save State
// ============================================================================

/// Runtime state for the auto-save system
#[derive(Resource, Debug)]
pub struct AutoSaveState {
    /// Timestamp of the last auto-save
    pub last_save: Option<Instant>,
    /// Time when next auto-save is scheduled
    pub next_save: Option<Instant>,
    /// Current rotating index for auto-save files
    pub current_index: usize,
    /// Whether there are unsaved changes
    pub has_unsaved_changes: bool,
    /// Timestamp of the last change
    pub last_change: Option<Instant>,
    /// Whether a save is currently in progress
    pub save_in_progress: bool,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Number of successful auto-saves this session
    pub save_count: u64,
    /// Hash of the last saved state (for change detection)
    pub last_saved_hash: u64,
}

impl Default for AutoSaveState {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoSaveState {
    /// Creates a new auto-save state
    pub fn new() -> Self {
        Self {
            last_save: None,
            next_save: None,
            current_index: 0,
            has_unsaved_changes: false,
            last_change: None,
            save_in_progress: false,
            last_error: None,
            save_count: 0,
            last_saved_hash: 0,
        }
    }

    /// Marks that changes have been made
    pub fn mark_changed(&mut self) {
        self.has_unsaved_changes = true;
        self.last_change = Some(Instant::now());
    }

    /// Marks that the current state has been saved
    pub fn mark_saved(&mut self, hash: u64) {
        self.has_unsaved_changes = false;
        self.last_save = Some(Instant::now());
        self.save_count += 1;
        self.last_saved_hash = hash;
        self.last_error = None;
    }

    /// Records a save error
    pub fn record_error(&mut self, error: String) {
        self.last_error = Some(error);
    }

    /// Advances to the next auto-save index
    pub fn advance_index(&mut self, max_files: usize) {
        self.current_index = (self.current_index + 1) % max_files;
    }

    /// Schedules the next auto-save
    pub fn schedule_next(&mut self, interval: Duration) {
        self.next_save = Some(Instant::now() + interval);
    }

    /// Checks if it's time for an auto-save
    pub fn should_auto_save(&self, config: &AutoSaveConfig) -> bool {
        if !config.enabled || !self.has_unsaved_changes || self.save_in_progress {
            return false;
        }

        // Check if we've passed the scheduled time
        if let Some(next) = self.next_save {
            if Instant::now() < next {
                return false;
            }
        }

        // Check idle threshold - don't save during active editing
        if let Some(last_change) = self.last_change {
            if last_change.elapsed() < config.idle_threshold {
                return false;
            }
        }

        true
    }

    /// Returns time until next scheduled save
    pub fn time_until_next_save(&self) -> Option<Duration> {
        self.next_save.map(|next| {
            let now = Instant::now();
            if next > now {
                next - now
            } else {
                Duration::ZERO
            }
        })
    }

    /// Returns time since last save
    pub fn time_since_last_save(&self) -> Option<Duration> {
        self.last_save.map(|last| last.elapsed())
    }
}

// ============================================================================
// Crash Recovery Manager
// ============================================================================

/// Manages crash recovery and auto-save functionality
#[derive(Resource, Debug)]
pub struct CrashRecoveryManager {
    /// List of detected recovery files
    recovery_files: VecDeque<RecoveryFile>,
    /// Whether recovery check has been performed
    recovery_checked: bool,
    /// Current scene path being edited
    current_scene_path: Option<PathBuf>,
    /// Whether recovery dialog should be shown
    show_recovery_dialog: bool,
}

impl Default for CrashRecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CrashRecoveryManager {
    /// Creates a new crash recovery manager
    pub fn new() -> Self {
        Self {
            recovery_files: VecDeque::new(),
            recovery_checked: false,
            current_scene_path: None,
            show_recovery_dialog: false,
        }
    }

    /// Sets the current scene path
    pub fn set_current_scene(&mut self, path: Option<PathBuf>) {
        self.current_scene_path = path;
    }

    /// Gets the current scene path
    pub fn current_scene(&self) -> Option<&PathBuf> {
        self.current_scene_path.as_ref()
    }

    /// Checks for existing recovery files on startup
    pub fn check_for_recovery(
        &mut self,
        config: &AutoSaveConfig,
    ) -> Result<usize, CrashRecoveryError> {
        self.recovery_files.clear();
        self.recovery_checked = true;

        // Ensure directory exists
        if !config.directory.exists() {
            return Ok(0);
        }

        // Check for recovery metadata file
        let metadata_path = config.recovery_metadata_path();
        if metadata_path.exists() {
            match self.load_recovery_metadata(&metadata_path) {
                Ok(files) => {
                    for file in files {
                        if file.path.exists() && file.state == RecoveryState::Pending {
                            self.recovery_files.push_back(file);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to load recovery metadata: {}", e);
                }
            }
        }

        // Also scan for auto-save files that might not be in metadata
        self.scan_for_autosave_files(config)?;

        if !self.recovery_files.is_empty() {
            self.show_recovery_dialog = true;
        }

        Ok(self.recovery_files.len())
    }

    /// Scans the auto-save directory for recovery files
    fn scan_for_autosave_files(
        &mut self,
        config: &AutoSaveConfig,
    ) -> Result<(), CrashRecoveryError> {
        if !config.directory.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&config.directory)
            .map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;

        for entry in entries.flatten() {
            let path = entry.path();

            // Check if it's an auto-save file
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("autosave_") && !self.has_recovery_file(&path) {
                    let mut recovery = RecoveryFile::new(&path);

                    // Update timestamp from file metadata
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                                recovery.timestamp = duration.as_secs();
                            }
                        }
                        recovery.size_bytes = metadata.len();
                    }

                    recovery.compressed = name.ends_with(".gz");
                    self.recovery_files.push_back(recovery);
                }
            }
        }

        // Sort by timestamp (most recent first)
        let mut files: Vec<_> = self.recovery_files.drain(..).collect();
        files.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        self.recovery_files = files.into();

        Ok(())
    }

    /// Checks if a recovery file is already tracked
    fn has_recovery_file(&self, path: &Path) -> bool {
        self.recovery_files.iter().any(|f| f.path == path)
    }

    /// Loads recovery metadata from a file
    fn load_recovery_metadata(&self, path: &Path) -> Result<Vec<RecoveryFile>, CrashRecoveryError> {
        let json =
            fs::read_to_string(path).map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;

        let data: RecoveryMetadata = serde_json::from_str(&json)
            .map_err(|e| CrashRecoveryError::DeserializationError(e.to_string()))?;

        Ok(data.files)
    }

    /// Saves recovery metadata to a file
    fn save_recovery_metadata(&self, path: &Path) -> Result<(), CrashRecoveryError> {
        let data = RecoveryMetadata {
            version: 1,
            files: self.recovery_files.iter().cloned().collect(),
        };

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| CrashRecoveryError::SerializationError(e.to_string()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;
        }

        fs::write(path, json).map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Gets all recovery files
    pub fn get_recovery_files(&self) -> &VecDeque<RecoveryFile> {
        &self.recovery_files
    }

    /// Gets pending recovery files
    pub fn get_pending_recoveries(&self) -> Vec<&RecoveryFile> {
        self.recovery_files
            .iter()
            .filter(|f| f.state == RecoveryState::Pending)
            .collect()
    }

    /// Gets the number of pending recovery files
    pub fn pending_count(&self) -> usize {
        self.recovery_files
            .iter()
            .filter(|f| f.state == RecoveryState::Pending)
            .count()
    }

    /// Recovers from a specific recovery file
    pub fn recover_from(
        &mut self,
        recovery_path: &Path,
        config: &AutoSaveConfig,
    ) -> Result<Vec<u8>, CrashRecoveryError> {
        // Find the recovery file
        let recovery = self
            .recovery_files
            .iter_mut()
            .find(|f| f.path == recovery_path)
            .ok_or_else(|| CrashRecoveryError::FileNotFound(recovery_path.display().to_string()))?;

        // Read the file content
        let content = if recovery.compressed {
            Self::read_compressed_file(&recovery.path)?
        } else {
            fs::read(&recovery.path).map_err(|e| CrashRecoveryError::IoError(e.to_string()))?
        };

        // Mark as recovered
        recovery.state = RecoveryState::Recovered;

        // Update metadata
        self.save_recovery_metadata(&config.recovery_metadata_path())?;

        Ok(content)
    }

    /// Reads a gzip-compressed file
    fn read_compressed_file(path: &Path) -> Result<Vec<u8>, CrashRecoveryError> {
        let file = fs::File::open(path).map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;

        let mut decoder = GzDecoder::new(file);
        let mut content = Vec::new();
        decoder
            .read_to_end(&mut content)
            .map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;

        Ok(content)
    }

    /// Dismisses a recovery file
    pub fn dismiss_recovery(
        &mut self,
        recovery_path: &Path,
        delete_file: bool,
        config: &AutoSaveConfig,
    ) -> Result<(), CrashRecoveryError> {
        // Find and update the recovery file
        if let Some(recovery) = self
            .recovery_files
            .iter_mut()
            .find(|f| f.path == recovery_path)
        {
            recovery.state = RecoveryState::Dismissed;

            if delete_file {
                let _ = fs::remove_file(&recovery.path);
            }
        }

        // Update metadata
        self.save_recovery_metadata(&config.recovery_metadata_path())?;

        // Update dialog visibility
        if self.pending_count() == 0 {
            self.show_recovery_dialog = false;
        }

        Ok(())
    }

    /// Dismisses all recovery files
    pub fn dismiss_all(
        &mut self,
        delete_files: bool,
        config: &AutoSaveConfig,
    ) -> Result<(), CrashRecoveryError> {
        for recovery in &mut self.recovery_files {
            if recovery.state == RecoveryState::Pending {
                recovery.state = RecoveryState::Dismissed;

                if delete_files {
                    let _ = fs::remove_file(&recovery.path);
                }
            }
        }

        self.show_recovery_dialog = false;

        // Update metadata
        self.save_recovery_metadata(&config.recovery_metadata_path())?;

        Ok(())
    }

    /// Triggers a manual auto-save
    pub fn trigger_auto_save(
        &mut self,
        content: &[u8],
        config: &AutoSaveConfig,
        state: &mut AutoSaveState,
    ) -> Result<PathBuf, CrashRecoveryError> {
        // Ensure directory exists
        fs::create_dir_all(&config.directory)
            .map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;

        // Get the save path
        let save_path = config.auto_save_path(state.current_index);

        // Calculate hash
        let hash = Self::calculate_hash(content);

        // Skip if content hasn't changed
        if hash == state.last_saved_hash && state.last_save.is_some() {
            return Ok(save_path);
        }

        state.save_in_progress = true;

        // Write the file
        let result = if config.compress {
            Self::write_compressed_file(&save_path, content)
        } else {
            fs::write(&save_path, content).map_err(|e| CrashRecoveryError::IoError(e.to_string()))
        };

        state.save_in_progress = false;

        match result {
            Ok(()) => {
                // Create recovery file entry
                let mut recovery = RecoveryFile::new(&save_path).with_hash(hash);

                if let Some(scene_path) = &self.current_scene_path {
                    recovery = recovery.with_original_path(scene_path.clone());
                }

                if config.compress {
                    recovery = recovery.compressed();
                }

                recovery.size_bytes = fs::metadata(&save_path).map(|m| m.len()).unwrap_or(0);

                // Update or add recovery file entry
                if let Some(existing) = self.recovery_files.iter_mut().find(|f| f.path == save_path)
                {
                    *existing = recovery;
                } else {
                    self.recovery_files.push_front(recovery);
                }

                // Advance index for next save
                state.advance_index(config.max_files);
                state.mark_saved(hash);
                state.schedule_next(config.interval);

                // Update metadata
                let _ = self.save_recovery_metadata(&config.recovery_metadata_path());

                // Clean up old files
                self.cleanup_old_files(config);

                Ok(save_path)
            }
            Err(e) => {
                state.record_error(e.to_string());
                Err(e)
            }
        }
    }

    /// Writes a gzip-compressed file
    fn write_compressed_file(path: &Path, content: &[u8]) -> Result<(), CrashRecoveryError> {
        let file =
            fs::File::create(path).map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;

        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(content)
            .map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;

        encoder
            .finish()
            .map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Calculates a hash for content (using FNV-1a)
    fn calculate_hash(content: &[u8]) -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        for byte in content {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// Marks that a save has completed (called externally)
    pub fn mark_as_saved(&mut self, state: &mut AutoSaveState) {
        state.has_unsaved_changes = false;
    }

    /// Marks that changes have been made (called externally)
    pub fn mark_as_changed(&mut self, state: &mut AutoSaveState) {
        state.mark_changed();
    }

    /// Cleans up old auto-save files beyond the max count
    fn cleanup_old_files(&mut self, config: &AutoSaveConfig) {
        // Only keep pending recovery files up to max_files
        let pending: Vec<_> = self
            .recovery_files
            .iter()
            .filter(|f| f.state == RecoveryState::Pending)
            .cloned()
            .collect();

        if pending.len() > config.max_files {
            // Remove oldest files
            let to_remove = pending.len() - config.max_files;

            for file in pending.iter().rev().take(to_remove) {
                let _ = fs::remove_file(&file.path);
            }

            // Update recovery files list
            self.recovery_files
                .retain(|f| f.state != RecoveryState::Pending || f.path.exists());
        }
    }

    /// Whether to show the recovery dialog
    pub fn should_show_recovery_dialog(&self) -> bool {
        self.show_recovery_dialog
    }

    /// Closes the recovery dialog
    pub fn close_recovery_dialog(&mut self) {
        self.show_recovery_dialog = false;
    }

    /// Whether recovery has been checked
    pub fn recovery_checked(&self) -> bool {
        self.recovery_checked
    }

    /// Cleans up all auto-save files
    pub fn cleanup_all(&mut self, config: &AutoSaveConfig) -> Result<(), CrashRecoveryError> {
        // Remove all auto-save files
        for recovery in &self.recovery_files {
            let _ = fs::remove_file(&recovery.path);
        }

        self.recovery_files.clear();

        // Remove metadata file
        let metadata_path = config.recovery_metadata_path();
        if metadata_path.exists() {
            fs::remove_file(&metadata_path)
                .map_err(|e| CrashRecoveryError::IoError(e.to_string()))?;
        }

        Ok(())
    }
}

/// Serializable metadata for recovery files
#[derive(Debug, Serialize, Deserialize)]
struct RecoveryMetadata {
    version: u32,
    files: Vec<RecoveryFile>,
}

// ============================================================================
// Errors
// ============================================================================

/// Errors that can occur in the crash recovery system
#[derive(Debug, Clone, thiserror::Error)]
pub enum CrashRecoveryError {
    #[error("I/O error: {0}")]
    IoError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Recovery file corrupted: {0}")]
    Corrupted(String),
    #[error("Auto-save in progress")]
    SaveInProgress,
}

// ============================================================================
// Events
// ============================================================================

/// Event to trigger an auto-save
#[derive(Event, Debug, Clone)]
pub struct TriggerAutoSaveEvent {
    /// Scene content to save
    pub content: Vec<u8>,
}

/// Event fired when an auto-save completes
#[derive(Event, Debug, Clone)]
pub struct AutoSaveCompletedEvent {
    /// Path where the auto-save was written
    pub path: PathBuf,
    /// Whether the save was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Event fired when recovery files are detected
#[derive(Event, Debug, Clone)]
pub struct RecoveryFilesDetectedEvent {
    /// Number of recovery files found
    pub count: usize,
}

/// Event to mark scene as changed
#[derive(Event, Debug, Clone, Default)]
pub struct SceneChangedEvent;

/// Event to mark scene as saved
#[derive(Event, Debug, Clone, Default)]
pub struct SceneSavedEvent;

/// Event fired when user selects a recovery option
#[derive(Event, Debug, Clone)]
pub struct RecoverySelectedEvent {
    /// The recovery file selected
    pub recovery_path: PathBuf,
    /// The action taken
    pub action: RecoveryAction,
}

/// Actions that can be taken on a recovery file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Recover from the file
    Recover,
    /// View details
    ViewDetails,
    /// Dismiss this file
    Dismiss,
    /// Dismiss and delete
    DismissAndDelete,
}

// ============================================================================
// Systems
// ============================================================================

/// System to check for recovery files on startup
pub fn startup_check_recovery_system(
    mut manager: ResMut<CrashRecoveryManager>,
    config: Res<AutoSaveConfig>,
    mut recovery_events: EventWriter<RecoveryFilesDetectedEvent>,
) {
    match manager.check_for_recovery(&config) {
        Ok(count) => {
            if count > 0 {
                tracing::info!("Found {} recovery file(s)", count);
                recovery_events.send(RecoveryFilesDetectedEvent { count });
            }
        }
        Err(e) => {
            tracing::warn!("Failed to check for recovery files: {}", e);
        }
    }
}

/// System to handle scene changed events
pub fn handle_scene_changed_system(
    mut events: EventReader<SceneChangedEvent>,
    mut manager: ResMut<CrashRecoveryManager>,
    mut state: ResMut<AutoSaveState>,
) {
    for _event in events.read() {
        manager.mark_as_changed(&mut state);
    }
}

/// System to handle scene saved events
pub fn handle_scene_saved_system(
    mut events: EventReader<SceneSavedEvent>,
    mut manager: ResMut<CrashRecoveryManager>,
    mut state: ResMut<AutoSaveState>,
) {
    for _event in events.read() {
        manager.mark_as_saved(&mut state);
    }
}

/// System for periodic auto-save
pub fn auto_save_timer_system(
    _manager: ResMut<CrashRecoveryManager>,
    mut state: ResMut<AutoSaveState>,
    config: Res<AutoSaveConfig>,
    _completed_events: EventWriter<AutoSaveCompletedEvent>,
) {
    if !state.should_auto_save(&config) {
        return;
    }

    // In a real implementation, this would get the actual scene content
    // For now, we'll skip if there's no mechanism to get scene data
    // The actual save should be triggered by TriggerAutoSaveEvent with content

    // Schedule next auto-save
    state.schedule_next(config.interval);
}

/// System to handle manual auto-save triggers
pub fn handle_trigger_auto_save_system(
    mut events: EventReader<TriggerAutoSaveEvent>,
    mut manager: ResMut<CrashRecoveryManager>,
    mut state: ResMut<AutoSaveState>,
    config: Res<AutoSaveConfig>,
    mut completed_events: EventWriter<AutoSaveCompletedEvent>,
) {
    for event in events.read() {
        let result = manager.trigger_auto_save(&event.content, &config, &mut state);

        match result {
            Ok(path) => {
                completed_events.send(AutoSaveCompletedEvent {
                    path,
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                completed_events.send(AutoSaveCompletedEvent {
                    path: PathBuf::new(),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }
}

/// System to initialize auto-save scheduling
pub fn init_auto_save_schedule_system(
    mut state: ResMut<AutoSaveState>,
    config: Res<AutoSaveConfig>,
) {
    if config.enabled && state.next_save.is_none() {
        state.schedule_next(config.interval);
    }
}

/// System to save on exit
pub fn on_exit_auto_save_system(
    _manager: ResMut<CrashRecoveryManager>,
    state: ResMut<AutoSaveState>,
    config: Res<AutoSaveConfig>,
) {
    if config.save_on_exit && state.has_unsaved_changes {
        // In a real implementation, this would get scene content and trigger save
        tracing::info!("Would save scene on exit (unsaved changes detected)");
    }
}

// ============================================================================
// UI Rendering (Feature-gated)
// ============================================================================

#[cfg(feature = "visual")]
use bevy_egui::{egui, EguiContexts};

#[cfg(feature = "visual")]
/// Renders the recovery dialog
pub fn render_recovery_dialog(
    ui: &mut egui::Ui,
    manager: &CrashRecoveryManager,
) -> Option<(PathBuf, RecoveryAction)> {
    let mut result: Option<(PathBuf, RecoveryAction)> = None;

    let pending = manager.get_pending_recoveries();

    if pending.is_empty() {
        ui.label("No recovery files available");
        return None;
    }

    ui.heading("Recovery Files Found");
    ui.separator();

    ui.label(format!(
        "Found {} unsaved session(s) that can be recovered:",
        pending.len()
    ));

    ui.add_space(10.0);

    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for recovery in pending {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(&recovery.display_name());
                        ui.label(format!("- {}", recovery.age_string()));
                    });

                    if let Some(original) = &recovery.original_path {
                        ui.label(format!("Original: {}", original.display()));
                    }

                    ui.label(format!("Size: {}", recovery.formatted_size()));

                    ui.horizontal(|ui| {
                        if ui.button("Recover").clicked() {
                            result = Some((recovery.path.clone(), RecoveryAction::Recover));
                        }

                        if ui.button("Details").clicked() {
                            result = Some((recovery.path.clone(), RecoveryAction::ViewDetails));
                        }

                        if ui.button("Dismiss").clicked() {
                            result = Some((recovery.path.clone(), RecoveryAction::Dismiss));
                        }
                    });
                });

                ui.add_space(5.0);
            }
        });

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Dismiss All").clicked() {
            // Return a signal to dismiss all - use empty path as marker
            result = Some((PathBuf::new(), RecoveryAction::Dismiss));
        }
    });

    result
}

#[cfg(feature = "visual")]
/// Renders auto-save status indicator
pub fn render_auto_save_status(ui: &mut egui::Ui, state: &AutoSaveState, config: &AutoSaveConfig) {
    if !config.enabled {
        ui.label("Auto-save: Disabled");
        return;
    }

    let status = if state.save_in_progress {
        "Saving..."
    } else if state.has_unsaved_changes {
        "Unsaved changes"
    } else {
        "All changes saved"
    };

    let color = if state.save_in_progress {
        egui::Color32::YELLOW
    } else if state.has_unsaved_changes {
        egui::Color32::LIGHT_RED
    } else {
        egui::Color32::LIGHT_GREEN
    };

    ui.horizontal(|ui| {
        ui.colored_label(color, "[*]");
        ui.label(status);

        if let Some(time) = state.time_until_next_save() {
            if time.as_secs() > 0 {
                ui.label(format!("(next save in {}s)", time.as_secs()));
            }
        }
    });

    if let Some(error) = &state.last_error {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
    }
}

// ============================================================================
// Plugin
// ============================================================================

/// Plugin for crash recovery and auto-save
pub struct CrashRecoveryPlugin {
    /// Initial configuration
    config: AutoSaveConfig,
}

impl Default for CrashRecoveryPlugin {
    fn default() -> Self {
        Self {
            config: AutoSaveConfig::default(),
        }
    }
}

impl CrashRecoveryPlugin {
    /// Creates a new plugin with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new plugin with custom configuration
    pub fn with_config(config: AutoSaveConfig) -> Self {
        Self { config }
    }
}

impl Plugin for CrashRecoveryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .init_resource::<AutoSaveState>()
            .init_resource::<CrashRecoveryManager>()
            .add_event::<TriggerAutoSaveEvent>()
            .add_event::<AutoSaveCompletedEvent>()
            .add_event::<RecoveryFilesDetectedEvent>()
            .add_event::<SceneChangedEvent>()
            .add_event::<SceneSavedEvent>()
            .add_event::<RecoverySelectedEvent>()
            .add_systems(
                Startup,
                (
                    startup_check_recovery_system,
                    init_auto_save_schedule_system,
                ),
            )
            .add_systems(
                Update,
                (
                    handle_scene_changed_system,
                    handle_scene_saved_system,
                    handle_trigger_auto_save_system,
                    auto_save_timer_system,
                ),
            );
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &TempDir) -> AutoSaveConfig {
        AutoSaveConfig::new()
            .with_directory(temp_dir.path())
            .with_max_files(3)
            .with_interval_secs(60)
    }

    #[test]
    fn test_recovery_file_creation() {
        let recovery = RecoveryFile::new("/test/autosave.dat")
            .with_original_path("/test/scene.scene")
            .with_hash(12345);

        assert_eq!(recovery.path, PathBuf::from("/test/autosave.dat"));
        assert_eq!(
            recovery.original_path,
            Some(PathBuf::from("/test/scene.scene"))
        );
        assert_eq!(recovery.scene_hash, 12345);
        assert_eq!(recovery.state, RecoveryState::Pending);
    }

    #[test]
    fn test_recovery_file_display_name() {
        let recovery_with_original =
            RecoveryFile::new("/test/autosave_0.dat").with_original_path("/scenes/my_scene.scene");

        assert_eq!(recovery_with_original.display_name(), "my_scene");

        let recovery_without_original = RecoveryFile::new("/test/autosave_0.dat");
        assert_eq!(recovery_without_original.display_name(), "autosave_0");
    }

    #[test]
    fn test_recovery_state_actionable() {
        assert!(RecoveryState::Pending.is_actionable());
        assert!(!RecoveryState::Recovered.is_actionable());
        assert!(!RecoveryState::Dismissed.is_actionable());
        assert!(!RecoveryState::Corrupted.is_actionable());
    }

    #[test]
    fn test_auto_save_config_default() {
        let config = AutoSaveConfig::default();

        assert!(config.enabled);
        assert_eq!(config.interval, Duration::from_secs(300));
        assert_eq!(config.max_files, 5);
        assert!(config.save_on_exit);
        assert!(config.compress);
    }

    #[test]
    fn test_auto_save_config_with_modifiers() {
        let config = AutoSaveConfig::new()
            .with_enabled(false)
            .with_interval_secs(120)
            .with_max_files(10)
            .with_save_on_exit(false)
            .with_compression(false);

        assert!(!config.enabled);
        assert_eq!(config.interval, Duration::from_secs(120));
        assert_eq!(config.max_files, 10);
        assert!(!config.save_on_exit);
        assert!(!config.compress);
    }

    #[test]
    fn test_auto_save_config_max_files_minimum() {
        let config = AutoSaveConfig::new().with_max_files(0);

        // Should be clamped to at least 1
        assert_eq!(config.max_files, 1);
    }

    #[test]
    fn test_auto_save_config_paths() {
        let temp_dir = TempDir::new().unwrap();
        let config = AutoSaveConfig::new().with_directory(temp_dir.path());

        let path0 = config.auto_save_path(0);
        let path1 = config.auto_save_path(1);

        assert!(path0.to_string_lossy().contains("autosave_0"));
        assert!(path1.to_string_lossy().contains("autosave_1"));
    }

    #[test]
    fn test_auto_save_state_mark_changed() {
        let mut state = AutoSaveState::new();

        assert!(!state.has_unsaved_changes);

        state.mark_changed();

        assert!(state.has_unsaved_changes);
        assert!(state.last_change.is_some());
    }

    #[test]
    fn test_auto_save_state_mark_saved() {
        let mut state = AutoSaveState::new();
        state.mark_changed();

        assert!(state.has_unsaved_changes);

        state.mark_saved(12345);

        assert!(!state.has_unsaved_changes);
        assert!(state.last_save.is_some());
        assert_eq!(state.last_saved_hash, 12345);
        assert_eq!(state.save_count, 1);
    }

    #[test]
    fn test_auto_save_state_advance_index() {
        let mut state = AutoSaveState::new();

        assert_eq!(state.current_index, 0);

        state.advance_index(3);
        assert_eq!(state.current_index, 1);

        state.advance_index(3);
        assert_eq!(state.current_index, 2);

        state.advance_index(3);
        assert_eq!(state.current_index, 0); // Wraps around
    }

    #[test]
    fn test_auto_save_state_schedule_next() {
        let mut state = AutoSaveState::new();

        assert!(state.next_save.is_none());

        state.schedule_next(Duration::from_secs(60));

        assert!(state.next_save.is_some());
        assert!(state.time_until_next_save().is_some());
    }

    #[test]
    fn test_auto_save_state_should_auto_save() {
        let mut config = AutoSaveConfig::new()
            .with_enabled(true)
            .with_interval_secs(0);
        // Set a very short idle threshold for testing
        config.idle_threshold = Duration::from_millis(5);

        let mut state = AutoSaveState::new();

        // No changes, shouldn't save
        assert!(!state.should_auto_save(&config));

        // Mark changed
        state.mark_changed();

        // Wait for idle threshold
        std::thread::sleep(Duration::from_millis(10));

        // Schedule in the past
        state.next_save = Some(Instant::now() - Duration::from_secs(1));

        // Now should save
        assert!(state.should_auto_save(&config));
    }

    #[test]
    fn test_auto_save_state_should_not_save_when_disabled() {
        let config = AutoSaveConfig::new().with_enabled(false);
        let mut state = AutoSaveState::new();
        state.mark_changed();

        assert!(!state.should_auto_save(&config));
    }

    #[test]
    fn test_auto_save_state_should_not_save_during_progress() {
        let config = AutoSaveConfig::new().with_enabled(true);
        let mut state = AutoSaveState::new();
        state.mark_changed();
        state.save_in_progress = true;

        assert!(!state.should_auto_save(&config));
    }

    #[test]
    fn test_crash_recovery_manager_new() {
        let manager = CrashRecoveryManager::new();

        assert!(manager.recovery_files.is_empty());
        assert!(!manager.recovery_checked);
        assert!(manager.current_scene_path.is_none());
    }

    #[test]
    fn test_crash_recovery_manager_set_current_scene() {
        let mut manager = CrashRecoveryManager::new();

        manager.set_current_scene(Some(PathBuf::from("/test/scene.scene")));

        assert_eq!(
            manager.current_scene(),
            Some(&PathBuf::from("/test/scene.scene"))
        );
    }

    #[test]
    fn test_crash_recovery_manager_check_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = CrashRecoveryManager::new();

        let count = manager.check_for_recovery(&config).unwrap();

        assert_eq!(count, 0);
        assert!(manager.recovery_checked);
    }

    #[test]
    fn test_crash_recovery_manager_trigger_auto_save() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir).with_compression(false);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        let content = b"test scene content";

        let path = manager
            .trigger_auto_save(content, &config, &mut state)
            .unwrap();

        assert!(path.exists());
        assert!(!state.has_unsaved_changes);
        assert!(state.last_save.is_some());
        assert_eq!(state.save_count, 1);
    }

    #[test]
    fn test_crash_recovery_manager_trigger_auto_save_compressed() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir).with_compression(true);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        let content = b"test scene content for compression";

        let path = manager
            .trigger_auto_save(content, &config, &mut state)
            .unwrap();

        assert!(path.exists());
        assert!(path.to_string_lossy().ends_with(".gz"));
    }

    #[test]
    fn test_crash_recovery_manager_skip_unchanged() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir).with_compression(false);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        let content = b"test scene content";

        // First save
        manager
            .trigger_auto_save(content, &config, &mut state)
            .unwrap();
        let first_count = state.save_count;

        // Second save with same content - should skip
        manager
            .trigger_auto_save(content, &config, &mut state)
            .unwrap();

        // Save count shouldn't increase
        assert_eq!(state.save_count, first_count);
    }

    #[test]
    fn test_crash_recovery_manager_rotating_saves() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir)
            .with_compression(false)
            .with_max_files(3);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        // Save multiple times with different content
        for i in 0..5 {
            let content = format!("content {}", i).into_bytes();
            manager
                .trigger_auto_save(&content, &config, &mut state)
                .unwrap();
        }

        // Check rotating index
        assert_eq!(state.current_index, 2); // 5 % 3 = 2
    }

    #[test]
    fn test_crash_recovery_manager_dismiss_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir).with_compression(false);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        // Create a recovery file
        let content = b"test content";
        let path = manager
            .trigger_auto_save(content, &config, &mut state)
            .unwrap();

        // Dismiss it
        manager.dismiss_recovery(&path, false, &config).unwrap();

        // Check state
        let recovery = manager.recovery_files.iter().find(|f| f.path == path);
        assert_eq!(recovery.unwrap().state, RecoveryState::Dismissed);
    }

    #[test]
    fn test_crash_recovery_manager_dismiss_and_delete() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir).with_compression(false);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        // Create a recovery file
        let content = b"test content";
        let path = manager
            .trigger_auto_save(content, &config, &mut state)
            .unwrap();

        assert!(path.exists());

        // Dismiss and delete
        manager.dismiss_recovery(&path, true, &config).unwrap();

        // File should be deleted
        assert!(!path.exists());
    }

    #[test]
    fn test_crash_recovery_manager_dismiss_all() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir).with_compression(false);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        // Create multiple recovery files
        for i in 0..3 {
            let content = format!("content {}", i).into_bytes();
            manager
                .trigger_auto_save(&content, &config, &mut state)
                .unwrap();
        }

        assert!(manager.pending_count() > 0);

        // Dismiss all
        manager.dismiss_all(false, &config).unwrap();

        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_crash_recovery_manager_pending_count() {
        let mut manager = CrashRecoveryManager::new();

        manager
            .recovery_files
            .push_back(RecoveryFile::new("/test/1.dat"));
        manager
            .recovery_files
            .push_back(RecoveryFile::new("/test/2.dat"));

        let mut dismissed = RecoveryFile::new("/test/3.dat");
        dismissed.state = RecoveryState::Dismissed;
        manager.recovery_files.push_back(dismissed);

        assert_eq!(manager.pending_count(), 2);
    }

    #[test]
    fn test_crash_recovery_manager_get_pending_recoveries() {
        let mut manager = CrashRecoveryManager::new();

        manager
            .recovery_files
            .push_back(RecoveryFile::new("/test/1.dat"));

        let mut recovered = RecoveryFile::new("/test/2.dat");
        recovered.state = RecoveryState::Recovered;
        manager.recovery_files.push_back(recovered);

        let pending = manager.get_pending_recoveries();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].path, PathBuf::from("/test/1.dat"));
    }

    #[test]
    fn test_crash_recovery_manager_recover_from() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir).with_compression(false);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        let original_content = b"original scene content";
        let path = manager
            .trigger_auto_save(original_content, &config, &mut state)
            .unwrap();

        // Recover
        let recovered_content = manager.recover_from(&path, &config).unwrap();

        assert_eq!(recovered_content, original_content);

        // Check state was updated
        let recovery = manager.recovery_files.iter().find(|f| f.path == path);
        assert_eq!(recovery.unwrap().state, RecoveryState::Recovered);
    }

    #[test]
    fn test_crash_recovery_manager_recover_compressed() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir).with_compression(true);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        let original_content = b"compressed scene content";
        let path = manager
            .trigger_auto_save(original_content, &config, &mut state)
            .unwrap();

        // Recover
        let recovered_content = manager.recover_from(&path, &config).unwrap();

        assert_eq!(recovered_content, original_content);
    }

    #[test]
    fn test_crash_recovery_manager_cleanup_all() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir).with_compression(false);
        let mut manager = CrashRecoveryManager::new();
        let mut state = AutoSaveState::new();

        // Create recovery files
        for i in 0..3 {
            let content = format!("content {}", i).into_bytes();
            manager
                .trigger_auto_save(&content, &config, &mut state)
                .unwrap();
        }

        assert!(!manager.recovery_files.is_empty());

        // Cleanup all
        manager.cleanup_all(&config).unwrap();

        assert!(manager.recovery_files.is_empty());
    }

    #[test]
    fn test_hash_calculation() {
        let content1 = b"hello world";
        let content2 = b"hello world";
        let content3 = b"different content";

        let hash1 = CrashRecoveryManager::calculate_hash(content1);
        let hash2 = CrashRecoveryManager::calculate_hash(content2);
        let hash3 = CrashRecoveryManager::calculate_hash(content3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_recovery_file_formatted_size() {
        let mut recovery = RecoveryFile::new("/test/file.dat");

        recovery.size_bytes = 500;
        assert_eq!(recovery.formatted_size(), "500 B");

        recovery.size_bytes = 2048;
        assert_eq!(recovery.formatted_size(), "2.0 KB");

        recovery.size_bytes = 1024 * 1024 * 5;
        assert_eq!(recovery.formatted_size(), "5.0 MB");
    }

    #[test]
    fn test_auto_save_state_error_recording() {
        let mut state = AutoSaveState::new();

        assert!(state.last_error.is_none());

        state.record_error("Test error".to_string());

        assert_eq!(state.last_error, Some("Test error".to_string()));

        // Save clears error
        state.mark_saved(0);
        assert!(state.last_error.is_none());
    }

    #[test]
    fn test_crash_recovery_manager_recovery_dialog_state() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let mut manager = CrashRecoveryManager::new();

        assert!(!manager.should_show_recovery_dialog());

        // Add a pending recovery
        manager
            .recovery_files
            .push_back(RecoveryFile::new("/test/recovery.dat"));
        manager.show_recovery_dialog = true;

        assert!(manager.should_show_recovery_dialog());

        manager.close_recovery_dialog();
        assert!(!manager.should_show_recovery_dialog());
    }

    #[test]
    fn test_recovery_file_age() {
        let mut recovery = RecoveryFile::new("/test/file.dat");

        // Set timestamp to now
        recovery.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let age = recovery.age();
        assert!(age.as_secs() < 2);

        // Set timestamp to 1 hour ago
        recovery.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 3600;

        let age = recovery.age();
        assert!(age.as_secs() >= 3600);
        assert!(recovery.age_string().contains("hour"));
    }

    #[test]
    fn test_trigger_auto_save_event() {
        let event = TriggerAutoSaveEvent {
            content: b"test content".to_vec(),
        };

        assert_eq!(event.content, b"test content");
    }

    #[test]
    fn test_auto_save_completed_event() {
        let success_event = AutoSaveCompletedEvent {
            path: PathBuf::from("/test/autosave.dat"),
            success: true,
            error: None,
        };

        assert!(success_event.success);
        assert!(success_event.error.is_none());

        let error_event = AutoSaveCompletedEvent {
            path: PathBuf::new(),
            success: false,
            error: Some("Failed to write".to_string()),
        };

        assert!(!error_event.success);
        assert!(error_event.error.is_some());
    }

    #[test]
    fn test_recovery_action_variants() {
        let actions = [
            RecoveryAction::Recover,
            RecoveryAction::ViewDetails,
            RecoveryAction::Dismiss,
            RecoveryAction::DismissAndDelete,
        ];

        // Just ensure all variants exist and are distinct
        assert_ne!(actions[0], actions[1]);
        assert_ne!(actions[2], actions[3]);
    }
}
