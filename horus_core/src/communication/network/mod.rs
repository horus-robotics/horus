/// Network communication backends for HORUS
///
/// This module provides network-based communication in addition to the local
/// shared memory backend. It includes:
/// - Endpoint parsing for network addresses
/// - Binary protocol for efficient serialization
/// - UDP direct connections (no discovery)
/// - Unix domain sockets (localhost optimization)
/// - Multicast discovery (future)
pub mod backend;
pub mod direct;
pub mod discovery;
pub mod endpoint;
pub mod fragmentation;
pub mod protocol;
pub mod reconnect;
pub mod router;
pub mod udp_direct;
pub mod udp_multicast;
pub mod unix_socket;

#[cfg(feature = "tls")]
pub mod tls;

// Re-export commonly used types
pub use backend::NetworkBackend;
pub use direct::{DirectBackend, DirectRole};
pub use discovery::{DiscoveryService, PeerInfo};
pub use endpoint::{parse_endpoint, Endpoint, DEFAULT_PORT, MULTICAST_ADDR, MULTICAST_PORT};
pub use fragmentation::{Fragment, FragmentManager};
pub use protocol::{HorusPacket, MessageType};
pub use reconnect::{ConnectionHealth, ReconnectContext, ReconnectStrategy};
pub use router::RouterBackend;
pub use udp_direct::UdpDirectBackend;
pub use udp_multicast::UdpMulticastBackend;
pub use unix_socket::UnixSocketBackend;

#[cfg(feature = "tls")]
pub use tls::{TlsCertConfig, TlsStream};
