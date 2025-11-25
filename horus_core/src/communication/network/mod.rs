/// Network communication backends for HORUS
///
/// This module provides network-based communication in addition to the local
/// shared memory backend. It includes:
/// - Endpoint parsing for network addresses
/// - Binary protocol for efficient serialization
/// - UDP direct connections (no discovery)
/// - Unix domain sockets (localhost optimization)
/// - Multicast discovery
/// - Message batching for efficiency
/// - Congestion control with drop policies
/// - Compression (LZ4/Zstd)
/// - Query/Response patterns
/// - Topic caching with TTL
pub mod backend;
pub mod batching;
pub mod caching;
pub mod compression;
pub mod congestion;
pub mod direct;
pub mod discovery;
pub mod endpoint;
pub mod fragmentation;
pub mod protocol;
pub mod queryable;
pub mod reconnect;
pub mod router;
pub mod udp_direct;
pub mod udp_multicast;

// Unix domain sockets are only available on Unix-like systems
#[cfg(unix)]
pub mod unix_socket;

#[cfg(feature = "tls")]
pub mod tls;

// io_uring is only available on Linux
#[cfg(target_os = "linux")]
pub mod io_uring;

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

// Re-export new modules
pub use batching::{BatchConfig, BatchReceiver, MessageBatch, MessageBatcher, SharedBatcher};
pub use caching::{CacheConfig, CacheStats, SharedCache, TopicCache};
pub use compression::{CompressedData, CompressedPacket, CompressionAlgo, CompressionConfig, Compressor};
pub use congestion::{CongestionConfig, CongestionController, CongestionResult, DropPolicy, SharedCongestionController};
pub use queryable::{QueryClient, QueryConfig, QueryError, QueryHandler, QueryRequest, QueryResponse, QueryServer, ResponseStatus};

#[cfg(unix)]
pub use unix_socket::UnixSocketBackend;

#[cfg(feature = "tls")]
pub use tls::{TlsCertConfig, TlsStream};

#[cfg(target_os = "linux")]
pub use io_uring::{IoUringBackend, IoUringConfig, IoUringStats, is_io_uring_available};
