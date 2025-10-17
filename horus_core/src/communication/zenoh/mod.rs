//! Zenoh integration for HORUS
//! 
//! Distributed IPC using Zenoh for network-based and cross-platform scenarios:
//! - Publisher: Network-aware message publishing with automatic discovery
//! - Subscriber: Network-aware message consumption with filtering
//! - Session: Zenoh session management and configuration

#[cfg(feature = "zenoh")]
use zenoh::*;
use std::sync::Arc;

pub mod session;
pub mod publisher;
pub mod subscriber;

pub use session::Session;
pub use publisher::Publisher;
pub use subscriber::Subscriber;

use crate::communication::traits::{Publisher as PublisherTrait, Subscriber as SubscriberTrait};

/// Error types for Zenoh operations
#[derive(Debug, thiserror::Error)]
pub enum ZenohError {
    #[error("Session creation failed: {0}")]
    SessionCreation(String),
    #[error("Publisher creation failed: {0}")]
    PublisherCreation(String),
    #[error("Subscriber creation failed: {0}")]
    SubscriberCreation(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

#[cfg(feature = "zenoh")]
mod implementations {
    use super::*;
    
    // Implement common traits for Zenoh Publisher
    impl<T> PublisherTrait<T> for Publisher<T>
    where
        T: Send + Sync + Clone + std::fmt::Debug + serde::Serialize + 'static
    {
        fn send(&self, msg: T) -> crate::error::HorusResult<()> {
            self.send(msg).map_err(|e| crate::error::HorusError::Backend {
                backend: "zenoh".to_string(),
                message: e.to_string()
            })
        }
    }

    // Implement common traits for Zenoh Subscriber  
    impl<T> SubscriberTrait<T> for Subscriber<T>
    where
        T: Send + Sync + Clone + std::fmt::Debug + serde::de::DeserializeOwned + 'static
    {
        fn try_recv(&self) -> Option<T> {
            self.recv()
        }
    }
}