//! Unified error handling for HORUS
//!
//! This module provides a centralized error type for the entire HORUS system,
//! ensuring consistent error handling across all components.

use thiserror::Error;

/// Main error type for HORUS operations
#[derive(Debug, Error)]
pub enum HorusError {
    /// I/O related errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration parsing or validation errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Backend-specific errors
    #[error("Backend '{backend}' error: {message}")]
    Backend {
        backend: String,
        message: String
    },

    /// Communication layer errors
    #[error("Communication error: {0}")]
    Communication(String),

    /// Node-related errors
    #[error("Node '{node}' error: {message}")]
    Node {
        node: String,
        message: String
    },

    /// Scheduling errors
    #[error("Scheduling error: {0}")]
    Scheduling(String),

    /// Memory management errors
    #[error("Memory error: {0}")]
    Memory(String),

    /// Shared memory specific errors
    #[error("Shared memory error: {0}")]
    SharedMemory(String),

    /// Parameter management errors
    #[error("Parameter error: {0}")]
    Parameter(String),

    /// Serialization/Deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Resource not found errors
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Permission/Access errors
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid input/argument errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Initialization errors
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    /// Already exists errors (for creation operations)
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// Parse errors
    #[error("Parse error: {0}")]
    ParseError(String),

    /// External command execution errors
    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    /// Feature not available errors
    #[error("Feature not available: {0}")]
    FeatureNotAvailable(String),

    /// Generic internal errors (use sparingly)
    #[error("Internal error: {0}")]
    Internal(String),

    /// Catch-all for other error types
    #[error("{0}")]
    Other(String),
}

/// Convenience type alias for Results using HorusError
pub type HorusResult<T> = Result<T, HorusError>;

// Implement conversions from common error types
impl From<serde_json::Error> for HorusError {
    fn from(err: serde_json::Error) -> Self {
        HorusError::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for HorusError {
    fn from(err: toml::de::Error) -> Self {
        HorusError::Config(format!("TOML parse error: {}", err))
    }
}

impl From<toml::ser::Error> for HorusError {
    fn from(err: toml::ser::Error) -> Self {
        HorusError::Serialization(format!("TOML serialization error: {}", err))
    }
}


impl From<std::num::ParseIntError> for HorusError {
    fn from(err: std::num::ParseIntError) -> Self {
        HorusError::ParseError(format!("Integer parse error: {}", err))
    }
}

impl From<std::num::ParseFloatError> for HorusError {
    fn from(err: std::num::ParseFloatError) -> Self {
        HorusError::ParseError(format!("Float parse error: {}", err))
    }
}

impl From<std::str::ParseBoolError> for HorusError {
    fn from(err: std::str::ParseBoolError) -> Self {
        HorusError::ParseError(format!("Boolean parse error: {}", err))
    }
}

impl From<uuid::Error> for HorusError {
    fn from(err: uuid::Error) -> Self {
        HorusError::Internal(format!("UUID error: {}", err))
    }
}

impl From<std::sync::PoisonError<std::sync::MutexGuard<'_, ()>>> for HorusError {
    fn from(_: std::sync::PoisonError<std::sync::MutexGuard<'_, ()>>) -> Self {
        HorusError::Internal("Mutex poisoned".to_string())
    }
}

// Allow conversion from Box<dyn Error> for backward compatibility
impl From<Box<dyn std::error::Error>> for HorusError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        HorusError::Other(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for HorusError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        HorusError::Other(err.to_string())
    }
}

// Convert from anyhow::Error
impl From<anyhow::Error> for HorusError {
    fn from(err: anyhow::Error) -> Self {
        HorusError::Other(err.to_string())
    }
}

// Helper methods
impl HorusError {
    /// Create a configuration error with a custom message
    pub fn config<S: Into<String>>(msg: S) -> Self {
        HorusError::Config(msg.into())
    }

    /// Create a backend error with backend name and message
    pub fn backend<S: Into<String>, T: Into<String>>(backend: S, message: T) -> Self {
        HorusError::Backend {
            backend: backend.into(),
            message: message.into(),
        }
    }

    /// Create a node error with node name and message
    pub fn node<S: Into<String>, T: Into<String>>(node: S, message: T) -> Self {
        HorusError::Node {
            node: node.into(),
            message: message.into(),
        }
    }

    /// Create a communication error
    pub fn communication<S: Into<String>>(msg: S) -> Self {
        HorusError::Communication(msg.into())
    }

    /// Create a memory error
    pub fn memory<S: Into<String>>(msg: S) -> Self {
        HorusError::Memory(msg.into())
    }

    /// Create a not found error
    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        HorusError::NotFound(resource.into())
    }

    /// Create an invalid input error
    pub fn invalid_input<S: Into<String>>(msg: S) -> Self {
        HorusError::InvalidInput(msg.into())
    }

    /// Check if this is a not found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, HorusError::NotFound(_))
    }

    /// Check if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(self, HorusError::Timeout(_))
    }

    /// Check if this is a permission error
    pub fn is_permission_denied(&self) -> bool {
        matches!(self, HorusError::PermissionDenied(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() {
        // Test IO error conversion
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let horus_err: HorusError = io_err.into();
        assert!(horus_err.to_string().contains("IO error"));

        // Test helper methods
        let config_err = HorusError::config("invalid setting");
        assert!(config_err.to_string().contains("Configuration error"));

        let backend_err = HorusError::backend("iceoryx2", "connection failed");
        assert!(backend_err.to_string().contains("Backend 'iceoryx2' error"));
    }

    #[test]
    fn test_error_checks() {
        let not_found = HorusError::not_found("resource");
        assert!(not_found.is_not_found());
        assert!(!not_found.is_timeout());

        let timeout = HorusError::Timeout("operation".to_string());
        assert!(timeout.is_timeout());
        assert!(!timeout.is_not_found());
    }
}