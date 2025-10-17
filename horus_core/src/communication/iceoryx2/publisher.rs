//! iceoryx2 Publisher implementation for HORUS

#[cfg(feature = "iceoryx2")]
pub use self::implementation::*;

#[cfg(feature = "iceoryx2")]
mod implementation {
    use iceoryx2::prelude::*;
    use super::super::IceoryxError;

    /// HORUS wrapper around iceoryx2 Publisher
    #[derive(Debug)]
    pub struct Publisher<T> {
        publisher: iceoryx2::service::publisher::Publisher<ipc::Service, T, ()>,
    }

    impl<T> Publisher<T> 
    where 
        T: Send + Sync + 'static
    {
        pub(in super::super) fn new(publisher: iceoryx2::service::publisher::Publisher<ipc::Service, T, ()>) -> Self {
            Self { publisher }
        }
        
        /// Send a message using iceoryx2's loan-based zero-copy mechanism
        pub fn send(&self, msg: T) -> Result<(), IceoryxError> {
            loop {
                match self.publisher.loan_uninit() {
                    Ok(sample) => {
                        let sample = sample.write_payload(msg);
                        sample.send()
                            .map_err(|e| IceoryxError::SendFailed(format!("Failed to send sample: {:?}", e)))?;
                        return Ok(());
                    }
                    Err(_) => {
                        // Retry if no slots available - this maintains HORUS Hub-like behavior
                        std::hint::spin_loop();
                    }
                }
            }
        }
        
        /// Try to send without spinning (non-blocking)
        pub fn try_send(&self, msg: T) -> Result<(), IceoryxError> {
            match self.publisher.loan_uninit() {
                Ok(sample) => {
                    let sample = sample.write_payload(msg);
                    sample.send()
                        .map_err(|e| IceoryxError::SendFailed(format!("Failed to send sample: {:?}", e)))?;
                    Ok(())
                }
                Err(_) => Err(IceoryxError::SendFailed("No slots available".into())),
            }
        }
        
        /// Loan memory for zero-copy writing (advanced API)
        pub fn loan(&self) -> Result<iceoryx2::service::publisher::publisher::SampleMutUninit<ipc::Service, T, ()>, IceoryxError> {
            self.publisher.loan_uninit()
                .map_err(|e| IceoryxError::SendFailed(format!("Failed to loan sample: {:?}", e)))
        }
    }

    impl<T> Clone for Publisher<T> {
        fn clone(&self) -> Self {
            Self {
                publisher: self.publisher.clone(),
            }
        }
    }

    unsafe impl<T> Send for Publisher<T> where T: Send + Sync {}
    unsafe impl<T> Sync for Publisher<T> where T: Send + Sync {}
}

#[cfg(not(feature = "iceoryx2"))]
mod stub {
    /// Stub Publisher when iceoryx2 backend is not enabled
    #[derive(Debug)]
    pub struct Publisher<T>(std::marker::PhantomData<T>);
    
    impl<T> Clone for Publisher<T> {
        fn clone(&self) -> Self {
            Publisher(std::marker::PhantomData)
        }
    }
}

#[cfg(not(feature = "iceoryx2"))]
pub use stub::*;