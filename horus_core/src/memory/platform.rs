// Cross-platform shared memory - each platform uses its optimal mechanism
//
// Linux: /dev/shm/horus (tmpfs - RAM-backed, fastest) via file mmap
// macOS: POSIX shm_open() (Mach shared memory - RAM-backed) - NOT file-based!
// Windows: CreateFileMappingW (pagefile-backed - optimized for IPC) - NOT temp files!
//
// Note: macOS and Windows no longer use filesystem paths for shared memory.
// The path functions below are kept for Linux and backward compatibility only.

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
    shm_base_dir()
        .join("sessions")
        .join(session_id)
        .join("topics")
}

/// Get the heartbeats directory for node health monitoring
pub fn shm_heartbeats_dir() -> PathBuf {
    shm_base_dir().join("heartbeats")
}

/// Get the global shared memory directory
pub fn shm_global_dir() -> PathBuf {
    shm_base_dir().join("global")
}

/// Get the network status directory for transport monitoring
pub fn shm_network_dir() -> PathBuf {
    shm_base_dir().join("network")
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

/// Write the current process PID to session directory for liveness tracking
/// This enables dashboards to detect dead sessions instantly (like rqt in ROS)
/// Supports multi-process sessions by storing PIDs in separate files
pub fn write_session_pid(session_id: &str) -> std::io::Result<()> {
    let session_dir = shm_session_dir(session_id);
    let pids_dir = session_dir.join("pids");
    std::fs::create_dir_all(&pids_dir)?;
    // Create a PID file named by our process ID (supports multiple processes per session)
    let pid_file = pids_dir.join(std::process::id().to_string());
    std::fs::write(pid_file, "")
}

/// Read all PIDs from a session directory (supports multi-process sessions)
pub fn read_session_pids(session_id: &str) -> Vec<u32> {
    let pids_dir = shm_session_dir(session_id).join("pids");
    if !pids_dir.exists() {
        // Legacy: check single pid file
        let legacy_pid = shm_session_dir(session_id).join("pid");
        if let Ok(content) = std::fs::read_to_string(legacy_pid) {
            if let Ok(pid) = content.trim().parse() {
                return vec![pid];
            }
        }
        return vec![];
    }

    std::fs::read_dir(pids_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().to_str()?.parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Check if a session is still alive by verifying ANY of its PIDs exist
/// Returns true if at least one PID is running, false if all dead
pub fn is_session_alive(session_id: &str) -> bool {
    let pids = read_session_pids(session_id);
    if pids.is_empty() {
        return false; // No PIDs = assume dead
    }

    // Session is alive if ANY process is still running
    let alive = pids.iter().any(|&pid| is_process_running(pid));

    // Clean up dead PID files while we're checking
    if alive {
        let pids_dir = shm_session_dir(session_id).join("pids");
        for pid in &pids {
            if !is_process_running(*pid) {
                let _ = std::fs::remove_file(pids_dir.join(pid.to_string()));
            }
        }
    }

    alive
}

/// Check if a process with given PID is running
pub fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // On Unix, kill(pid, 0) checks if process exists without sending signal
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(windows)]
    {
        // On Windows, try to open the process using windows-sys
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 {
                false
            } else {
                CloseHandle(handle);
                true
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        // Fallback: check /proc on Linux-like systems
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
}

/// Check if we're running on a platform with true RAM-backed shared memory
///
/// All major platforms now use optimal shared memory:
/// - Linux: /dev/shm (tmpfs - RAM)
/// - macOS: shm_open() (Mach shared memory - RAM)
/// - Windows: CreateFileMapping (pagefile-backed - OS optimized)
pub fn has_native_shm() -> bool {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        true
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        false // BSD and others still use file-based fallback
    }
}

/// Get platform name for logging/diagnostics
pub fn platform_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "Linux"
    }

    #[cfg(target_os = "macos")]
    {
        "macOS"
    }

    #[cfg(target_os = "windows")]
    {
        "Windows"
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        "Unix"
    }
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
