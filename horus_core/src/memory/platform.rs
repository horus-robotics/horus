// Cross-platform shared memory path abstraction
//
// Linux: /dev/shm/horus (tmpfs - RAM-backed, fastest)
// macOS: /tmp/horus (regular filesystem, but still fast for IPC)
// Windows: %TEMP%\horus (uses system temp directory)

use std::path::PathBuf;

/// Get the base directory for HORUS shared memory
///
/// This returns a platform-appropriate path for shared memory:
/// - Linux: `/dev/shm/horus` (tmpfs for maximum performance)
/// - macOS: `/tmp/horus` (no /dev/shm, but /tmp is still fast)
/// - Windows: `%TEMP%\horus` (system temp directory)
pub fn shm_base_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/dev/shm/horus")
    }

    #[cfg(target_os = "macos")]
    {
        // macOS doesn't have /dev/shm, use /tmp instead
        // For better performance, could use shm_open() in the future
        PathBuf::from("/tmp/horus")
    }

    #[cfg(target_os = "windows")]
    {
        // Windows uses temp directory
        std::env::temp_dir().join("horus")
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // Fallback for other Unix-like systems (BSD, etc.)
        PathBuf::from("/tmp/horus")
    }
}

/// Get the topics directory for shared memory message passing
pub fn shm_topics_dir() -> PathBuf {
    shm_base_dir().join("topics")
}

/// Get the topics directory for a specific session
pub fn shm_session_topics_dir(session_id: &str) -> PathBuf {
    shm_base_dir().join("sessions").join(session_id).join("topics")
}

/// Get the heartbeats directory for node health monitoring
pub fn shm_heartbeats_dir() -> PathBuf {
    shm_base_dir().join("heartbeats")
}

/// Get the pubsub metadata directory
pub fn shm_pubsub_metadata_dir() -> PathBuf {
    shm_base_dir().join("pubsub_metadata")
}

/// Get the global shared memory directory
pub fn shm_global_dir() -> PathBuf {
    shm_base_dir().join("global")
}

/// Get the logs shared memory path
pub fn shm_logs_path() -> PathBuf {
    // Logs are at the same level as horus dir, not inside it
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/dev/shm/horus_logs")
    }

    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/tmp/horus_logs")
    }

    #[cfg(target_os = "windows")]
    {
        std::env::temp_dir().join("horus_logs")
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        PathBuf::from("/tmp/horus_logs")
    }
}

/// Get session directory for a specific session ID
pub fn shm_session_dir(session_id: &str) -> PathBuf {
    shm_base_dir().join("sessions").join(session_id)
}

/// Check if we're running on a platform with true shared memory (tmpfs)
pub fn has_native_shm() -> bool {
    #[cfg(target_os = "linux")]
    {
        true
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Get platform name for logging/diagnostics
pub fn platform_name() -> &'static str {
    #[cfg(target_os = "linux")]
    { "Linux" }

    #[cfg(target_os = "macos")]
    { "macOS" }

    #[cfg(target_os = "windows")]
    { "Windows" }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    { "Unix" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shm_paths_are_valid() {
        let base = shm_base_dir();
        assert!(!base.as_os_str().is_empty());

        let topics = shm_topics_dir();
        assert!(topics.starts_with(&base));

        let heartbeats = shm_heartbeats_dir();
        assert!(heartbeats.starts_with(&base));
    }

    #[test]
    fn test_session_paths() {
        let session_dir = shm_session_dir("test-session");
        assert!(session_dir.to_string_lossy().contains("test-session"));

        let session_topics = shm_session_topics_dir("test-session");
        assert!(session_topics.to_string_lossy().contains("topics"));
    }
}
