//! iceoryx2 integration for HORUS
//!
//! Zero-copy IPC using iceoryx2 for large message scenarios:
//! - Publisher: Zero-copy message publishing with loan-based API
//! - Subscriber: Zero-copy message consumption
//! - Service: iceoryx2 service management

#[cfg(feature = "iceoryx2")]
use iceoryx2::prelude::*;
use std::marker::PhantomData;
use std::sync::Arc;

pub mod publisher;
pub mod service;
pub mod subscriber;

pub use publisher::Publisher;
pub use service::Service;
pub use subscriber::Subscriber;

use crate::communication::traits::{Publisher as PublisherTrait, Subscriber as SubscriberTrait};

/// Error types for iceoryx2 operations
#[derive(Debug, thiserror::Error)]
pub enum IceoryxError {
    #[error("Service creation failed: {0}")]
    ServiceCreation(String),
    #[error("Publisher creation failed: {0}")]
    PublisherCreation(String),
    #[error("Subscriber creation failed: {0}")]
    SubscriberCreation(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
}

#[cfg(feature = "iceoryx2")]
mod implementations {
    use super::*;
    use crate::communication::traits::{
        Publisher as PublisherTrait, Subscriber as SubscriberTrait,
    };

    // Implement common traits for iceoryx2 Publisher
    impl<T> PublisherTrait<T> for Publisher<T>
    where
        T: Send + Sync + Clone + std::fmt::Debug + 'static,
    {
        fn send(&self, msg: T) -> crate::error::HorusResult<()> {
            self.send(msg)
                .map_err(|e| crate::error::HorusError::Backend {
                    backend: "iceoryx2".to_string(),
                    message: e.to_string(),
                })
        }
    }

    // Implement common traits for iceoryx2 Subscriber
    impl<T> SubscriberTrait<T> for Subscriber<T>
    where
        T: Send + Sync + Clone + std::fmt::Debug + 'static,
    {
        fn recv(&self) -> Option<T> {
            Subscriber::recv(self)
        }

        fn has_messages(&self) -> bool {
            Subscriber::has_messages(self)
        }
    }
}
