use super::endpoint::Endpoint;
use super::router::RouterBackend;
use super::smart_transport::{NetworkLocation, TransportSelector, TransportType};
use super::udp_direct::UdpDirectBackend;
use super::udp_multicast::UdpMulticastBackend;
#[cfg(unix)]
use super::unix_socket::UnixSocketBackend;

#[cfg(target_os = "linux")]
use super::batch_udp::{BatchUdpConfig, BatchUdpReceiver, BatchUdpSender};

use crate::error::HorusResult;
use std::net::SocketAddr;

/// Network backend for Hub communication
///
/// Provides actual network implementations with automatic selection:
/// - Shared memory (local, fastest)
/// - Unix domain sockets (localhost, Unix only)
/// - Batch UDP with sendmmsg/recvmmsg (Linux, high throughput)
/// - Standard UDP (fallback)
/// - Router (central message broker)
/// - Multicast discovery
///
/// Network backend enum wrapping different transport types
#[allow(clippy::large_enum_variant)]
pub enum NetworkBackend<T> {
    /// Unix domain socket (localhost, Unix only)
    #[cfg(unix)]
    UnixSocket(UnixSocketBackend<T>),

    /// Direct UDP connection
    UdpDirect(UdpDirectBackend<T>),

    /// Batch UDP with sendmmsg/recvmmsg (Linux only, high performance)
    #[cfg(target_os = "linux")]
    BatchUdp(BatchUdpBackendWrapper<T>),

    /// Multicast discovery
    Multicast(UdpMulticastBackend<T>),

    /// Router (central message broker)
    Router(RouterBackend<T>),
}

/// Wrapper for batch UDP sender/receiver pair
#[cfg(target_os = "linux")]
pub struct BatchUdpBackendWrapper<T> {
    sender: std::sync::Mutex<BatchUdpSender>,
    receiver: std::sync::Mutex<BatchUdpReceiver>,
    topic: String,
    remote_addr: SocketAddr,
    _phantom: std::marker::PhantomData<T>,
}

// Safety: The Mutex provides interior mutability with thread-safety
#[cfg(target_os = "linux")]
unsafe impl<T: Send> Send for BatchUdpBackendWrapper<T> {}
#[cfg(target_os = "linux")]
unsafe impl<T: Send> Sync for BatchUdpBackendWrapper<T> {}

#[cfg(target_os = "linux")]
impl<T> std::fmt::Debug for BatchUdpBackendWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchUdpBackendWrapper")
            .field("topic", &self.topic)
            .field("remote_addr", &self.remote_addr)
            .finish()
    }
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
    ///
    /// Uses smart transport selection to automatically choose the best backend
    /// based on network location and available system features.
    pub fn new(endpoint: Endpoint) -> HorusResult<Self> {
        match endpoint {
            Endpoint::Local { .. } => Err(crate::error::HorusError::Communication(
                "Local endpoint should use shared memory, not network backend".to_string(),
            )),

            Endpoint::Localhost { topic, .. } => {
                // For localhost, use smart transport selection
                let localhost_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
                Self::create_for_address(&topic, localhost_addr, true)
            }

            Endpoint::Direct { topic, host, port } => {
                // Create with smart transport selection based on target address
                let addr = SocketAddr::new(host, port);
                Self::create_for_address(&topic, addr, false)
            }

            Endpoint::Multicast { topic } => {
                // Multicast doesn't use smart selection - it's a specific use case
                let multicast_backend = UdpMulticastBackend::new(&topic)?;
                Ok(NetworkBackend::Multicast(multicast_backend))
            }

            Endpoint::Router { topic, host, port } => {
                // Router backend is also a specific use case
                let router_backend = if let Some(h) = host {
                    let p = port.unwrap_or(7777);
                    RouterBackend::new_with_addr(&topic, h, p)?
                } else {
                    let h = "127.0.0.1".parse().unwrap();
                    let p = port.unwrap_or(7777);
                    RouterBackend::new_with_addr(&topic, h, p)?
                };
                Ok(NetworkBackend::Router(router_backend))
            }
        }
    }

    /// Create backend for a specific address using smart transport selection
    fn create_for_address(topic: &str, addr: SocketAddr, is_localhost: bool) -> HorusResult<Self> {
        let selector = TransportSelector::new();
        let transport_type = selector.select(&addr);

        log::debug!(
            "Smart transport selected {:?} for {} (location: {:?})",
            transport_type,
            addr,
            NetworkLocation::from_addr(&addr)
        );

        // Try the selected transport, fall back if it fails
        match Self::try_create_transport(topic, addr, transport_type, is_localhost) {
            Ok(backend) => Ok(backend),
            Err(e) => {
                // Try fallback
                if let Some(fallback) = selector.get_fallback(transport_type) {
                    log::warn!(
                        "Primary transport {:?} failed ({}), trying fallback {:?}",
                        transport_type,
                        e,
                        fallback
                    );
                    Self::try_create_transport(topic, addr, fallback, is_localhost)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Try to create a specific transport type
    fn try_create_transport(
        topic: &str,
        addr: SocketAddr,
        transport: TransportType,
        is_localhost: bool,
    ) -> HorusResult<Self> {
        match transport {
            TransportType::SharedMemory => {
                // Shared memory is handled at a higher level (Hub), not here
                // Fall through to Unix socket or UDP
                if is_localhost {
                    #[cfg(unix)]
                    {
                        let unix_backend = UnixSocketBackend::new_subscriber(topic)?;
                        return Ok(NetworkBackend::UnixSocket(unix_backend));
                    }
                }
                // Fall through to UDP
                let udp_backend = UdpDirectBackend::new(topic, addr.ip(), addr.port())?;
                Ok(NetworkBackend::UdpDirect(udp_backend))
            }

            TransportType::UnixSocket => {
                #[cfg(unix)]
                {
                    let unix_backend = UnixSocketBackend::new_subscriber(topic)?;
                    Ok(NetworkBackend::UnixSocket(unix_backend))
                }
                #[cfg(not(unix))]
                {
                    // Fall back to UDP on non-Unix
                    let udp_backend = UdpDirectBackend::new(topic, addr.ip(), addr.port())?;
                    Ok(NetworkBackend::UdpDirect(udp_backend))
                }
            }

            TransportType::BatchUdp => {
                #[cfg(target_os = "linux")]
                {
                    Self::create_batch_udp(topic, addr)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    // Fall back to standard UDP on non-Linux
                    let udp_backend = UdpDirectBackend::new(topic, addr.ip(), addr.port())?;
                    Ok(NetworkBackend::UdpDirect(udp_backend))
                }
            }

            TransportType::IoUring => {
                // io_uring requires special setup and is best used via BatchUdp wrapper
                // For now, fall back to BatchUdp or standard UDP
                #[cfg(target_os = "linux")]
                {
                    Self::create_batch_udp(topic, addr)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let udp_backend = UdpDirectBackend::new(topic, addr.ip(), addr.port())?;
                    Ok(NetworkBackend::UdpDirect(udp_backend))
                }
            }

            TransportType::Udp => {
                let udp_backend = UdpDirectBackend::new(topic, addr.ip(), addr.port())?;
                Ok(NetworkBackend::UdpDirect(udp_backend))
            }

            TransportType::Tcp => {
                // TCP is handled via Router backend
                // For direct TCP, fall back to UDP for now
                let udp_backend = UdpDirectBackend::new(topic, addr.ip(), addr.port())?;
                Ok(NetworkBackend::UdpDirect(udp_backend))
            }

            TransportType::Quic => {
                // QUIC requires async runtime - fall back to UDP for sync API
                // QUIC can be used directly via QuicTransport for async use cases
                log::debug!("QUIC transport requested but NetworkBackend is sync; falling back to UDP");
                let udp_backend = UdpDirectBackend::new(topic, addr.ip(), addr.port())?;
                Ok(NetworkBackend::UdpDirect(udp_backend))
            }
        }
    }

    /// Create batch UDP backend (Linux only)
    #[cfg(target_os = "linux")]
    fn create_batch_udp(topic: &str, addr: SocketAddr) -> HorusResult<Self> {
        let config = BatchUdpConfig::default();

        // Bind to any available port for sending
        let bind_addr: SocketAddr = if addr.is_ipv4() {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };

        let sender = BatchUdpSender::new(bind_addr, config.clone()).map_err(|e| {
            crate::error::HorusError::Communication(format!("Failed to create batch UDP sender: {}", e))
        })?;

        // Receiver binds to a specific port (use default HORUS port or dynamic)
        let recv_addr: SocketAddr = if addr.is_ipv4() {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };

        let receiver = BatchUdpReceiver::new(recv_addr, config).map_err(|e| {
            crate::error::HorusError::Communication(format!("Failed to create batch UDP receiver: {}", e))
        })?;

        Ok(NetworkBackend::BatchUdp(BatchUdpBackendWrapper {
            sender: std::sync::Mutex::new(sender),
            receiver: std::sync::Mutex::new(receiver),
            topic: topic.to_string(),
            remote_addr: addr,
            _phantom: std::marker::PhantomData,
        }))
    }

    /// Send a message over the network
    pub fn send(&self, msg: &T) -> HorusResult<()> {
        match self {
            #[cfg(unix)]
            NetworkBackend::UnixSocket(backend) => backend.send(msg),
            NetworkBackend::UdpDirect(backend) => backend.send(msg),
            #[cfg(target_os = "linux")]
            NetworkBackend::BatchUdp(backend) => {
                let data = bincode::serialize(msg).map_err(|e| {
                    crate::error::HorusError::Communication(format!("Serialization error: {}", e))
                })?;
                let mut sender = backend.sender.lock().map_err(|e| {
                    crate::error::HorusError::Communication(format!("Sender lock error: {}", e))
                })?;
                sender.send(&data, backend.remote_addr).map_err(|e| {
                    crate::error::HorusError::Communication(format!("Batch UDP send error: {}", e))
                })
            }
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
            #[cfg(target_os = "linux")]
            NetworkBackend::BatchUdp(backend) => {
                let mut receiver = match backend.receiver.lock() {
                    Ok(r) => r,
                    Err(_) => return None,
                };
                match receiver.recv_batch(1) {
                    Ok(packets) => {
                        if let Some(packet) = packets.into_iter().next() {
                            bincode::deserialize(&packet.data).ok()
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            }
            NetworkBackend::Multicast(backend) => backend.recv(),
            NetworkBackend::Router(backend) => backend.recv(),
        }
    }

    /// Get the selected transport type for diagnostics
    pub fn transport_type(&self) -> &'static str {
        match self {
            #[cfg(unix)]
            NetworkBackend::UnixSocket(_) => "unix_socket",
            NetworkBackend::UdpDirect(_) => "udp_direct",
            #[cfg(target_os = "linux")]
            NetworkBackend::BatchUdp(_) => "batch_udp",
            NetworkBackend::Multicast(_) => "multicast",
            NetworkBackend::Router(_) => "router",
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
            #[cfg(target_os = "linux")]
            NetworkBackend::BatchUdp(backend) => f
                .debug_struct("NetworkBackend::BatchUdp")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::communication::network::endpoint::parse_endpoint;

    #[test]
    fn test_transport_type_method() {
        // Test that we can get transport type string
        let endpoint = parse_endpoint("test@192.168.1.100:9870").unwrap();
        if let Ok(backend) = NetworkBackend::<Vec<u8>>::new(endpoint) {
            let transport = backend.transport_type();
            assert!(!transport.is_empty());
            println!("Selected transport: {}", transport);
        }
    }

    #[test]
    fn test_localhost_selection() {
        let endpoint = parse_endpoint("test@localhost").unwrap();
        if let Ok(backend) = NetworkBackend::<Vec<u8>>::new(endpoint) {
            let transport = backend.transport_type();
            // Should be unix_socket on Unix, or udp_direct on Windows
            #[cfg(unix)]
            assert!(transport == "unix_socket" || transport == "batch_udp");
            #[cfg(not(unix))]
            assert_eq!(transport, "udp_direct");
        }
    }
}
