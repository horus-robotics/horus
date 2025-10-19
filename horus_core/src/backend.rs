//! Backend selection and configuration for HORUS
//!
//! This module handles runtime backend selection with clear precedence:
//! 1. Environment variable (HORUS_BACKEND)
//! 2. Configuration file (Cargo.toml metadata)
//! 3. Compile-time features

use std::env;
use std::fmt;

/// Available IPC backends for HORUS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Native HORUS shared memory implementation (fastest, local only)
    Horus,
    /// IceOryx2 zero-copy IPC (medium speed, cross-process)
    #[cfg(feature = "iceoryx2")]
    IceOryx2,
    /// Zenoh distributed IPC (slowest, network capable)
    #[cfg(feature = "zenoh")]
    Zenoh,
}

impl Backend {
    /// Get the currently selected backend
    ///
    /// Selection precedence:
    /// 1. HORUS_BACKEND environment variable
    /// 2. Configuration file (if provided)
    /// 3. First available compiled backend
    pub fn current() -> Self {
        // Check environment variable first
        if let Ok(backend_str) = env::var("HORUS_BACKEND") {
            if let Ok(backend) = backend_str.parse() {
                return backend;
            }
        }

        // Default to native HORUS backend
        Backend::Horus
    }

    /// Check if a specific backend is available (compiled in)
    pub fn is_available(&self) -> bool {
        match self {
            Backend::Horus => true,
            #[cfg(feature = "iceoryx2")]
            Backend::IceOryx2 => true,
            #[cfg(feature = "zenoh")]
            Backend::Zenoh => true,
        }
    }

    /// Get all available backends
    pub fn available_backends() -> Vec<Backend> {
        vec![
            Backend::Horus,
            #[cfg(feature = "iceoryx2")]
            Backend::IceOryx2,
            #[cfg(feature = "zenoh")]
            Backend::Zenoh,
        ]
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Backend::Horus => write!(f, "horus"),
            #[cfg(feature = "iceoryx2")]
            Backend::IceOryx2 => write!(f, "iceoryx2"),
            #[cfg(feature = "zenoh")]
            Backend::Zenoh => write!(f, "zenoh"),
        }
    }
}

impl std::str::FromStr for Backend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "horus" | "native" => Ok(Backend::Horus),
            #[cfg(feature = "iceoryx2")]
            "iceoryx2" | "iceoryx" => Ok(Backend::IceOryx2),
            #[cfg(feature = "zenoh")]
            "zenoh" => Ok(Backend::Zenoh),
            _ => Err(format!(
                "Unknown backend: {}. Available: {:?}",
                s,
                Backend::available_backends()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_parsing() {
        assert_eq!("horus".parse::<Backend>().unwrap(), Backend::Horus);
        assert_eq!("native".parse::<Backend>().unwrap(), Backend::Horus);

        #[cfg(feature = "iceoryx2")]
        {
            assert_eq!("iceoryx2".parse::<Backend>().unwrap(), Backend::IceOryx2);
            assert_eq!("iceoryx".parse::<Backend>().unwrap(), Backend::IceOryx2);
        }

        #[cfg(feature = "zenoh")]
        assert_eq!("zenoh".parse::<Backend>().unwrap(), Backend::Zenoh);

        assert!("invalid".parse::<Backend>().is_err());
    }

    #[test]
    fn test_backend_display() {
        assert_eq!(Backend::Horus.to_string(), "horus");

        #[cfg(feature = "iceoryx2")]
        assert_eq!(Backend::IceOryx2.to_string(), "iceoryx2");

        #[cfg(feature = "zenoh")]
        assert_eq!(Backend::Zenoh.to_string(), "zenoh");
    }

    #[test]
    fn test_backend_availability() {
        assert!(Backend::Horus.is_available());

        let available = Backend::available_backends();
        assert!(available.contains(&Backend::Horus));
    }
}
