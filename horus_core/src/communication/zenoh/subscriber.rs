//! Zenoh Subscriber implementation for HORUS

#[cfg(feature = "zenoh")]
pub use self::implementation::*;

#[cfg(feature = "zenoh")]
mod implementation {
    use super::super::ZenohError;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use zenoh::*;

    /// HORUS wrapper around Zenoh Subscriber
    #[derive(Debug)]
    pub struct Subscriber<T> {
        receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<T>>>,
        key_expr: String,
        _subscriber: zenoh::pubsub::Subscriber<()>,
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> Subscriber<T>
    where
        T: Send + Sync + Clone + serde::de::DeserializeOwned + 'static,
    {
        pub(in super::super) fn new(
            session: Arc<zenoh::Session>,
            key_expr: &str,
        ) -> Result<Self, ZenohError> {
            let key_expr_owned = key_expr.to_string();

            // Create channel for message passing
            let (tx, rx) = mpsc::unbounded_channel();

            // Create Zenoh subscriber with callback
            let rt = tokio::runtime::Handle::try_current()
                .map_err(|_| ZenohError::SubscriberCreation("No Tokio runtime available".into()))?;

            let subscriber = rt.block_on(async {
                session
                    .declare_subscriber(key_expr)
                    .callback(move |sample| {
                        // Deserialize the received data
                        if let Ok(msg) = serde_json::from_slice::<T>(&sample.payload().to_bytes()) {
                            let _ = tx.send(msg); // Ignore send errors (receiver may be dropped)
                        }
                    })
                    .await
                    .map_err(|e| {
                        ZenohError::SubscriberCreation(format!(
                            "Failed to create subscriber: {:?}",
                            e
                        ))
                    })
            })?;

            Ok(Self {
                receiver: Arc::new(tokio::sync::Mutex::new(rx)),
                key_expr: key_expr_owned,
                _subscriber: subscriber,
                _phantom: std::marker::PhantomData,
            })
        }

        /// Try to receive a message (non-blocking)
        pub fn try_recv(&self) -> Option<T> {
            // Use try_lock to avoid blocking in sync context
            if let Ok(mut receiver) = self.receiver.try_lock() {
                receiver.recv().ok()
            } else {
                None
            }
        }

        /// Async receive a message
        pub async fn recv(&self) -> Option<T> {
            let mut receiver = self.receiver.lock().await;
            receiver.recv().await
        }

        /// Check if messages are available
        pub fn has_messages(&self) -> bool {
            // Use try_lock to check without blocking
            if let Ok(receiver) = self.receiver.try_lock() {
                !receiver.is_empty()
            } else {
                false // Can't check, assume no messages
            }
        }

        /// Get the key expression this subscriber listens to
        pub fn key_expr(&self) -> &str {
            &self.key_expr
        }
    }

    impl<T> Clone for Subscriber<T>
    where
        T: Send + Sync + Clone + serde::de::DeserializeOwned + 'static,
    {
        fn clone(&self) -> Self {
            Self {
                receiver: self.receiver.clone(),
                key_expr: self.key_expr.clone(),
                _subscriber: self._subscriber.clone(),
                _phantom: std::marker::PhantomData,
            }
        }
    }

    unsafe impl<T> Send for Subscriber<T> where T: Send + Sync + serde::de::DeserializeOwned {}
    unsafe impl<T> Sync for Subscriber<T> where T: Send + Sync + serde::de::DeserializeOwned {}
}

#[cfg(not(feature = "zenoh"))]
mod stub {
    /// Stub Subscriber when zenoh backend is not enabled
    #[derive(Debug)]
    pub struct Subscriber<T>(std::marker::PhantomData<T>);

    impl<T> Clone for Subscriber<T> {
        fn clone(&self) -> Self {
            Subscriber(std::marker::PhantomData)
        }
    }
}

#[cfg(not(feature = "zenoh"))]
pub use stub::*;
