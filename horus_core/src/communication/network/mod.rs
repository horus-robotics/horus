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
///
/// Network v2 high-performance backends:
/// - Batch UDP with sendmmsg/recvmmsg (200K+ packets/sec)
/// - Real io_uring zero-copy (3-5µs latency)
/// - QUIC transport (0-RTT, reliable)
/// - Smart transport selection (auto-picks best backend)
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

// Network v2 high-performance modules
pub mod batch_udp;
pub mod smart_transport;

// Unix domain sockets are only available on Unix-like systems
#[cfg(unix)]
pub mod unix_socket;

#[cfg(feature = "tls")]
pub mod tls;

// QUIC transport (requires quic feature)
#[cfg(feature = "quic")]
pub mod quic;

// io_uring backend (real implementation using io-uring crate, Linux only)
// Requires the `io-uring-net` feature flag
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

// Network v2 re-exports
pub use batch_udp::{
    BatchUdpConfig, BatchUdpReceiver, BatchUdpSender, BatchUdpStats,
    ReceivedPacket, ScalableUdpBackend,
};
pub use smart_transport::{
    NetworkLocation, TransportBuilder, TransportPreferences,
    TransportSelector, TransportSelectorStats, TransportType,
};

// io_uring re-exports (real implementation)
#[cfg(all(target_os = "linux", feature = "io-uring-net"))]
pub use io_uring::{
    CompletionResult, RealIoUringBackend, RealIoUringConfig, RealIoUringStats,
    is_real_io_uring_available, is_sqpoll_available,
};

#[cfg(feature = "quic")]
pub use quic::{QuicConfig, QuicStats, QuicTransport, generate_self_signed_cert};
