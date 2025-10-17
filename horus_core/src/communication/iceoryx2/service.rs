//! iceoryx2 service management for HORUS integration

#[cfg(feature = "iceoryx2")]
use iceoryx2::prelude::*;
#[cfg(feature = "iceoryx2")]
use std::sync::Arc;
#[cfg(feature = "iceoryx2")]
use super::IceoryxError;

#[cfg(feature = "iceoryx2")]
/// Wrapper around iceoryx2 service for HORUS integration
pub struct Service<T> {
    service: iceoryx2::service::Service<ipc::Service, T, ()>,
    node: Arc<iceoryx2::node::Node<ipc::Service>>,
}

impl<T> Service<T> 
where 
    T: Send + Sync + 'static
{
    /// Create a new iceoryx2 service
    pub fn new(service_name: &str) -> Result<Self, IceoryxError> {
        let node = NodeBuilder::new()
            .create::<ipc::Service>()
            .map_err(|e| IceoryxError::ServiceCreation(format!("Node creation failed: {:?}", e)))?;
        
        let service_name = ServiceName::new(service_name)
            .map_err(|e| IceoryxError::ServiceCreation(format!("Invalid service name: {:?}", e)))?;
        
        let service = Arc::new(node)
            .service_builder(&service_name)
            .publish_subscribe::<T>()
            .create()
            .map_err(|e| IceoryxError::ServiceCreation(format!("Service creation failed: {:?}", e)))?;
        
        Ok(Self {
            service,
            node: Arc::new(node),
        })
    }
    
    /// Create a publisher for this service
    pub fn create_publisher(&self) -> Result<super::Publisher<T>, IceoryxError> {
        let publisher = self.service
            .publisher_builder()
            .create()
            .map_err(|e| IceoryxError::PublisherCreation(format!("Publisher creation failed: {:?}", e)))?;
        
        Ok(super::Publisher::new(publisher))
    }
    
    /// Create a subscriber for this service
    pub fn create_subscriber(&self) -> Result<super::Subscriber<T>, IceoryxError> {
        let subscriber = self.service
            .subscriber_builder()
            .create()
            .map_err(|e| IceoryxError::SubscriberCreation(format!("Subscriber creation failed: {:?}", e)))?;
        
        Ok(super::Subscriber::new(subscriber))
    }
}

impl<T> Clone for Service<T> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            node: self.node.clone(),
        }
    }
}

unsafe impl<T> Send for Service<T> where T: Send + Sync {}
unsafe impl<T> Sync for Service<T> where T: Send + Sync {}