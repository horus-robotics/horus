//! Backend selection and configuration for HORUS
//!
//! This module handles backend configuration for HORUS native IPC.

use std::fmt;

/// Available IPC backends for HORUS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Native HORUS shared memory implementation (fastest, local only)
    Horus,
}

impl Backend {
    /// Get the currently selected backend (always returns Horus)
    pub fn current() -> Self {
        Backend::Horus
    }

    /// Check if a specific backend is available (always true for Horus)
    pub fn is_available(&self) -> bool {
        true
    }

    /// Get all available backends
    pub fn available_backends() -> Vec<Backend> {
        vec![Backend::Horus]
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "horus")
    }
}

impl std::str::FromStr for Backend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "horus" | "native" => Ok(Backend::Horus),
            _ => Err(format!(
                "Unknown backend: {}. Only 'horus' backend is available",
                s
            )),
        }
    }
}
