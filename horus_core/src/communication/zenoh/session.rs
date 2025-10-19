//! Zenoh session management for HORUS integration

#[cfg(feature = "zenoh")]
pub use self::implementation::*;

#[cfg(feature = "zenoh")]
mod implementation {
    use super::super::ZenohError;
    use std::sync::Arc;
    use zenoh::*;

    /// Wrapper around Zenoh session for HORUS integration
    pub struct Session {
        session: Arc<zenoh::Session>,
    }

    impl Session {
        /// Create a new Zenoh session with default configuration
        pub async fn new() -> Result<Self, ZenohError> {
            let session = zenoh::open(Config::default()).await.map_err(|e| {
                ZenohError::SessionCreation(format!("Failed to open Zenoh session: {:?}", e))
            })?;

            Ok(Self {
                session: Arc::new(session),
            })
        }

        /// Create a new Zenoh session with custom configuration
        pub async fn with_config(config: Config) -> Result<Self, ZenohError> {
            let session = zenoh::open(config).await.map_err(|e| {
                ZenohError::SessionCreation(format!(
                    "Failed to open Zenoh session with config: {:?}",
                    e
                ))
            })?;

            Ok(Self {
                session: Arc::new(session),
            })
        }

        /// Create a publisher for the given key expression
        pub fn create_publisher<T>(
            &self,
            key_expr: &str,
        ) -> std::result::Result<super::super::Publisher<T>, ZenohError>
        where
            T: Send + Sync + Clone + serde::Serialize + 'static,
        {
            super::super::Publisher::new(self.session.clone(), key_expr)
        }

        /// Create a subscriber for the given key expression  
        pub fn create_subscriber<T>(
            &self,
            key_expr: &str,
        ) -> std::result::Result<super::super::Subscriber<T>, ZenohError>
        where
            T: Send + Sync + Clone + serde::de::DeserializeOwned + 'static,
        {
            super::super::Subscriber::new(self.session.clone(), key_expr)
        }

        /// Get the underlying Zenoh session
        pub fn inner(&self) -> Arc<zenoh::Session> {
            self.session.clone()
        }
    }

    impl Clone for Session {
        fn clone(&self) -> Self {
            Self {
                session: self.session.clone(),
            }
        }
    }

    unsafe impl Send for Session {}
    unsafe impl Sync for Session {}
}

#[cfg(not(feature = "zenoh"))]
mod stub {
    use super::super::ZenohError;

    /// Stub Session when zenoh backend is not enabled
    #[derive(Debug)]
    pub struct Session;

    impl Session {
        pub async fn new() -> Result<Self, ZenohError> {
            Err(ZenohError::SessionCreation(
                "Zenoh backend not enabled".into(),
            ))
        }
    }

    impl Clone for Session {
        fn clone(&self) -> Self {
            Session
        }
    }
}

#[cfg(not(feature = "zenoh"))]
pub use stub::*;
