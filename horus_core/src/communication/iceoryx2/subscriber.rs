//! iceoryx2 Subscriber implementation for HORUS

use iceoryx2::prelude::*;

/// HORUS wrapper around iceoryx2 Subscriber
#[derive(Debug)]
pub struct Subscriber<T> {
    subscriber: iceoryx2::service::subscriber::Subscriber<ipc::Service, T, ()>,
}

impl<T> Subscriber<T>
where 
    T: Send + Sync + Clone + 'static
{
    pub(super) fn new(subscriber: iceoryx2::service::subscriber::Subscriber<ipc::Service, T, ()>) -> Self {
        Self { subscriber }
    }
    
    /// Receive a message (non-blocking)
    pub fn recv(&self) -> Option<T> {
        match self.subscriber.receive() {
            Ok(Some(sample)) => {
                // Clone the data from the sample - this maintains HORUS API compatibility
                Some(sample.payload().clone())
            }
            Ok(None) | Err(_) => None,
        }
    }

    /// Alias for recv() for backward compatibility
    #[deprecated(since = "0.1.0", note = "Use recv() instead")]
    pub fn try_recv(&self) -> Option<T> {
        self.recv()
    }
    
    /// Check if messages are available
    pub fn has_messages(&self) -> bool {
        // iceoryx2 doesn't have a direct equivalent, so we use a quick peek
        matches!(self.subscriber.receive(), Ok(Some(_)))
    }
    
    /// Receive with zero-copy (advanced API) - returns a reference to shared memory
    pub fn try_recv_zero_copy(&self) -> Option<iceoryx2::service::subscriber::subscriber::Sample<ipc::Service, T, ()>> {
        match self.subscriber.receive() {
            Ok(sample) => sample,
            Err(_) => None,
        }
    }
}

impl<T> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        Self {
            subscriber: self.subscriber.clone(),
        }
    }
}

unsafe impl<T> Send for Subscriber<T> where T: Send + Sync {}
unsafe impl<T> Sync for Subscriber<T> where T: Send + Sync {}