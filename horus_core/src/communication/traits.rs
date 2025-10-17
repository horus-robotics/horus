//! Common traits for all IPC implementations in HORUS
//! 
//! This module defines the abstraction layer that allows different IPC backends
//! (HORUS native, iceoryx2) to be used interchangeably while maintaining the
//! same API for nodes and schedulers.

use std::fmt::Debug;
use crate::error::HorusResult;

/// Common trait for publisher/sender implementations
/// Allows swapping between HORUS Hub and iceoryx2 Publisher
pub trait Publisher<T>: Send + Sync + Clone + Debug {
    /// Send a message - returns Ok on success, Err on failure
    fn send(&self, msg: T) -> HorusResult<()>;
    
    /// Try to send without blocking
    fn try_send(&self, msg: T) -> bool {
        self.send(msg).is_ok()
    }
}

/// Common trait for subscriber/receiver implementations
/// Allows swapping between HORUS Hub and iceoryx2 Subscriber
pub trait Subscriber<T>: Send + Sync + Clone + Debug {
    /// Receive a message without blocking
    fn recv(&self) -> Option<T>;

    /// Check if messages are available
    fn has_messages(&self) -> bool {
        false // Default implementation
    }
}

/// Common trait for point-to-point communication
/// Allows swapping between HORUS Link and potential iceoryx2 direct channels
pub trait Channel<T>: Publisher<T> + Subscriber<T> {
    /// Create a new channel with specified capacity
    fn new(capacity: usize) -> Self;
}

/// IPC Backend selector - compile-time choice of implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcBackend {
    /// Use HORUS native IPC (Hub/Link with shared memory)
    Horus,
    /// Use iceoryx2 for zero-copy IPC
    Iceoryx2,
    /// Use Zenoh for distributed/network IPC
    Zenoh,
}

impl Default for IpcBackend {
    fn default() -> Self {
        IpcBackend::Horus
    }
}

/// Type alias for the default Hub implementation based on backend choice
pub type DefaultHub<T> = crate::communication::horus::Hub<T>;

/// Type alias for the default Link implementation  
pub type DefaultLink<T> = crate::communication::horus::Link<T>;

// Re-export commonly used types with backend abstraction
pub use crate::communication::horus::{Hub as HorusHub, Link as HorusLink};