//! Zenoh Publisher implementation for HORUS

#[cfg(feature = "zenoh")]
pub use self::implementation::*;

#[cfg(feature = "zenoh")]
mod implementation {
    use zenoh::*;
    use std::sync::Arc;
    use super::super::ZenohError;

    /// HORUS wrapper around Zenoh Publisher
    #[derive(Debug)]
    pub struct Publisher<T> {
        session: Arc<zenoh::Session>,
        key_expr: String,
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> Publisher<T> 
    where 
        T: Send + Sync + Clone + serde::Serialize + 'static
    {
        pub(in super::super) fn new(session: Arc<zenoh::Session>, key_expr: &str) -> Result<Self, ZenohError> {
            Ok(Self {
                session,
                key_expr: key_expr.to_string(),
                _phantom: std::marker::PhantomData,
            })
        }
        
        /// Send a message using Zenoh's network-aware publishing
        pub fn send(&self, msg: T) -> std::result::Result<(), ZenohError> {
            // Serialize the message using serde
            let payload = serde_json::to_vec(&msg)
                .map_err(|e| ZenohError::SerializationFailed(format!("Failed to serialize message: {:?}", e)))?;
            
            // Send over Zenoh network - using blocking version for trait compatibility
            let rt = tokio::runtime::Handle::try_current()
                .map_err(|_| ZenohError::SendFailed("No Tokio runtime available".into()))?;
            
            rt.block_on(async {
                self.session
                    .put(&self.key_expr, payload)
                    .await
                    .map_err(|e| ZenohError::SendFailed(format!("Failed to send message: {:?}", e)))
            })
        }
        
        /// Send a message asynchronously
        pub async fn send_async(&self, msg: T) -> std::result::Result<(), ZenohError> {
            let payload = serde_json::to_vec(&msg)
                .map_err(|e| ZenohError::SerializationFailed(format!("Failed to serialize message: {:?}", e)))?;
            
            self.session
                .put(&self.key_expr, payload)
                .await
                .map_err(|e| ZenohError::SendFailed(format!("Failed to send message: {:?}", e)))
        }

        /// Send raw bytes (for advanced usage)
        pub async fn send_bytes(&self, data: Vec<u8>) -> std::result::Result<(), ZenohError> {
            self.session
                .put(&self.key_expr, data)
                .await
                .map_err(|e| ZenohError::SendFailed(format!("Failed to send raw data: {:?}", e)))
        }

        /// Get the key expression this publisher uses
        pub fn key_expr(&self) -> &str {
            &self.key_expr
        }
    }

    impl<T> Clone for Publisher<T> 
    where
        T: Send + Sync + Clone + serde::Serialize + 'static
    {
        fn clone(&self) -> Self {
            Self {
                session: self.session.clone(),
                key_expr: self.key_expr.clone(),
                _phantom: std::marker::PhantomData,
            }
        }
    }

    unsafe impl<T> Send for Publisher<T> where T: Send + Sync + serde::Serialize {}
    unsafe impl<T> Sync for Publisher<T> where T: Send + Sync + serde::Serialize {}
}

#[cfg(not(feature = "zenoh"))]
mod stub {
    /// Stub Publisher when zenoh backend is not enabled
    #[derive(Debug)]
    pub struct Publisher<T>(std::marker::PhantomData<T>);
    
    impl<T> Clone for Publisher<T> {
        fn clone(&self) -> Self {
            Publisher(std::marker::PhantomData)
        }
    }
}

#[cfg(not(feature = "zenoh"))]
pub use stub::*;