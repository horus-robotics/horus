use super::endpoint::Endpoint;
use super::router::RouterBackend;
use super::udp_direct::UdpDirectBackend;
use super::udp_multicast::UdpMulticastBackend;
#[cfg(unix)]
use super::unix_socket::UnixSocketBackend;
/// Network backend for Hub communication
///
/// Provides actual network implementations:
/// - UDP direct connections
/// - Unix domain sockets (localhost, Unix only)
/// - Multicast discovery (future)
use crate::error::HorusResult;

/// Network backend enum wrapping different transport types
pub enum NetworkBackend<T> {
    /// Unix domain socket (localhost, Unix only)
    #[cfg(unix)]
    UnixSocket(UnixSocketBackend<T>),

    /// Direct UDP connection
    UdpDirect(UdpDirectBackend<T>),

    /// Multicast discovery
    Multicast(UdpMulticastBackend<T>),

    /// Router (central message broker)
    Router(RouterBackend<T>),
}

impl<T> NetworkBackend<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    /// Create a new network backend from an endpoint
    pub fn new(endpoint: Endpoint) -> HorusResult<Self> {
        match endpoint {
            Endpoint::Local { .. } => Err(crate::error::HorusError::Communication(
                "Local endpoint should use shared memory, not network backend".to_string(),
            )),

            Endpoint::Localhost { topic, .. } => {
                // Unix socket backend (Unix only), fallback to UDP on Windows
                #[cfg(unix)]
                {
                    let unix_backend = UnixSocketBackend::new_subscriber(&topic)?;
                    Ok(NetworkBackend::UnixSocket(unix_backend))
                }
                #[cfg(not(unix))]
                {
                    // On Windows, fallback to localhost UDP
                    let udp_backend = UdpDirectBackend::new(&topic, "127.0.0.1".parse().unwrap(), 0)?;
                    Ok(NetworkBackend::UdpDirect(udp_backend))
                }
            }

            Endpoint::Direct { topic, host, port } => {
                // Create UDP direct backend
                let udp_backend = UdpDirectBackend::new(&topic, host, port)?;
                Ok(NetworkBackend::UdpDirect(udp_backend))
            }

            Endpoint::Multicast { topic } => {
                // Create multicast backend with discovery
                let multicast_backend = UdpMulticastBackend::new(&topic)?;
                Ok(NetworkBackend::Multicast(multicast_backend))
            }

            Endpoint::Router { topic, host, port } => {
                // Create router backend
                let router_backend = if let Some(h) = host {
                    let p = port.unwrap_or(7777);
                    RouterBackend::new_with_addr(&topic, h, p)?
                } else {
                    // Use default localhost router
                    let h = "127.0.0.1".parse().unwrap();
                    let p = port.unwrap_or(7777);
                    RouterBackend::new_with_addr(&topic, h, p)?
                };
                Ok(NetworkBackend::Router(router_backend))
            }
        }
    }

    /// Send a message over the network
    pub fn send(&self, msg: &T) -> HorusResult<()> {
        match self {
            #[cfg(unix)]
            NetworkBackend::UnixSocket(backend) => backend.send(msg),
            NetworkBackend::UdpDirect(backend) => backend.send(msg),
            NetworkBackend::Multicast(backend) => backend.send(msg),
            NetworkBackend::Router(backend) => backend.send(msg),
        }
    }

    /// Receive a message from the network
    pub fn recv(&mut self) -> Option<T> {
        match self {
            #[cfg(unix)]
            NetworkBackend::UnixSocket(backend) => backend.recv(),
            NetworkBackend::UdpDirect(backend) => backend.recv(),
            NetworkBackend::Multicast(backend) => backend.recv(),
            NetworkBackend::Router(backend) => backend.recv(),
        }
    }
}

impl<T> std::fmt::Debug for NetworkBackend<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(unix)]
            NetworkBackend::UnixSocket(backend) => f
                .debug_struct("NetworkBackend::UnixSocket")
                .field("backend", backend)
                .finish(),
            NetworkBackend::UdpDirect(backend) => f
                .debug_struct("NetworkBackend::UdpDirect")
                .field("backend", backend)
                .finish(),
            NetworkBackend::Multicast(backend) => f
                .debug_struct("NetworkBackend::Multicast")
                .field("backend", backend)
                .finish(),
            NetworkBackend::Router(backend) => f
                .debug_struct("NetworkBackend::Router")
                .field("backend", backend)
                .finish(),
        }
    }
}
