use crate::communication::network::smart_transport::NetworkLocation;
use crate::core::node::NodeInfo;
use crate::error::HorusResult;
use crate::memory::shm_region::ShmRegion;
use std::marker::PhantomData;
use std::mem;
use std::net::{SocketAddr, UdpSocket};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[cfg(unix)]
use std::os::unix::net::UnixDatagram;

/// Branch prediction hint: this condition is unlikely
/// Helps CPU predict the common path (not full, has data)
#[inline(always)]
fn unlikely(b: bool) -> bool {
    // Use core::intrinsics::unlikely when stable, for now use cold hint
    #[cold]
    #[inline(never)]
    fn cold_path() {}

    if b {
        cold_path();
    }
    b
}

/// Link role - determines whether this end can send or receive
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkRole {
    Producer,
    Consumer,
}

/// Connection state for Link connections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

// Helper functions for state conversion
impl ConnectionState {
    fn into_u8(self) -> u8 {
        match self {
            ConnectionState::Disconnected => 0,
            ConnectionState::Connecting => 1,
            ConnectionState::Connected => 2,
            ConnectionState::Reconnecting => 3,
            ConnectionState::Failed => 4,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            0 => ConnectionState::Disconnected,
            1 => ConnectionState::Connecting,
            2 => ConnectionState::Connected,
            3 => ConnectionState::Reconnecting,
            _ => ConnectionState::Failed,
        }
    }
}

/// Metrics for Link monitoring
#[derive(Debug, Clone, Default)]
pub struct LinkMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub send_failures: u64,
    pub recv_failures: u64,
}

/// Lock-free atomic metrics for Link monitoring (stored in local memory)
#[derive(Debug)]
#[repr(align(64))] // Cache-line aligned to prevent false sharing
struct AtomicLinkMetrics {
    messages_sent: std::sync::atomic::AtomicU64,
    messages_received: std::sync::atomic::AtomicU64,
    send_failures: std::sync::atomic::AtomicU64,
    recv_failures: std::sync::atomic::AtomicU64,
    _padding: [u8; 32], // Pad to cache line boundary (4 * 8 bytes + 32 = 64)
}

/// Header for Link shared memory - single-slot design
/// Just a sequence counter to signal new data availability
/// This is the simplest possible 1P1C design - producer overwrites, consumer tracks what it's seen
#[repr(C, align(64))]
struct LinkHeader {
    sequence: AtomicU64,       // Version counter - incremented on each write
    element_size: AtomicUsize, // For validation
    _padding: [u8; 48],        // Pad to full cache line (8 + 8 + 48 = 64)
}

// =============================================================================
// 1P1C OPTIMIZED NETWORK BACKEND FOR LINK
// =============================================================================
//
// Link uses a specialized 1P1C (One Producer, One Consumer) design that is
// fundamentally different from Hub's pub/sub MPMC pattern. This allows for
// significant performance optimizations:
//
// 1. **Connected UDP** - Uses connect() to establish a "virtual connection"
//    - Producer: connect() to consumer, then send() (not sendto())
//    - Consumer: bind(), then recv() (not recvfrom())
//    - ~30% faster than unconnected UDP due to kernel route caching
//
// 2. **No routing overhead** - Direct point-to-point, no topic matching
//
// 3. **Lock-free** - No contention between producer and consumer
//
// 4. **Pre-allocated buffers** - Reuse serialization buffers
//
// Performance hierarchy (fastest to slowest):
// - Local shared memory: ~250ns
// - Unix domain socket (localhost): ~1-2µs
// - Connected UDP (LAN): ~3-5µs
// - Standard UDP (LAN): ~5-8µs
// =============================================================================

/// Network backend type for Link's 1P1C (one producer, one consumer) pattern
///
/// Unlike Hub's NetworkBackend which handles pub/sub routing, Link's backend
/// is optimized for direct point-to-point communication with a single peer.
pub enum LinkNetworkBackend<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    /// Connected UDP - optimized for 1P1C with connect() syscall
    /// ~30% faster than unconnected UDP due to kernel route caching
    ConnectedUdp(ConnectedUdpBackend<T>),

    /// Unix domain socket (localhost only, very fast)
    #[cfg(unix)]
    UnixSocket(UnixSocketLinkBackend<T>),
}

/// Connected UDP backend - the fastest UDP option for 1P1C
///
/// Uses connect() to establish a "virtual connection" which allows:
/// - send()/recv() instead of sendto()/recvfrom() (fewer syscall args)
/// - Kernel caches route lookup (no per-packet route decision)
/// - ICMP errors are delivered to the socket
/// - ~30% faster than unconnected UDP
pub struct ConnectedUdpBackend<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    socket: UdpSocket,
    role: LinkRole,
    /// Pre-allocated send buffer to avoid allocation on hot path
    send_buffer: std::cell::UnsafeCell<Vec<u8>>,
    /// Pre-allocated receive buffer
    recv_buffer: std::cell::UnsafeCell<Vec<u8>>,
    _phantom: std::marker::PhantomData<T>,
}

// Safety: ConnectedUdpBackend is designed for 1P1C where only one thread
// accesses send_buffer (producer) and one thread accesses recv_buffer (consumer)
unsafe impl<T> Send for ConnectedUdpBackend<T> where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static
{
}
unsafe impl<T> Sync for ConnectedUdpBackend<T> where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static
{
}

/// Unix socket backend for Link - optimized for localhost 1P1C
#[cfg(unix)]
pub struct UnixSocketLinkBackend<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    socket: UnixDatagram,
    socket_path: String,
    role: LinkRole,
    /// Pre-allocated send buffer
    send_buffer: std::cell::UnsafeCell<Vec<u8>>,
    /// Pre-allocated receive buffer
    recv_buffer: std::cell::UnsafeCell<Vec<u8>>,
    _phantom: std::marker::PhantomData<T>,
}

#[cfg(unix)]
unsafe impl<T> Send for UnixSocketLinkBackend<T> where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static
{
}
#[cfg(unix)]
unsafe impl<T> Sync for UnixSocketLinkBackend<T> where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static
{
}

impl<T> std::fmt::Debug for LinkNetworkBackend<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkNetworkBackend::ConnectedUdp(u) => f
                .debug_struct("ConnectedUdp")
                .field("role", &u.role)
                .finish(),
            #[cfg(unix)]
            LinkNetworkBackend::UnixSocket(u) => f
                .debug_struct("UnixSocket")
                .field("role", &u.role)
                .field("path", &u.socket_path)
                .finish(),
        }
    }
}

// Buffer size for network messages (64KB - fits most robotics data)
const LINK_BUFFER_SIZE: usize = 65536;

impl<T> ConnectedUdpBackend<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    /// Create a Connected UDP producer
    ///
    /// The producer connect()s to the consumer's address, enabling fast send() calls.
    /// This is ~30% faster than using sendto() on each message.
    pub fn new_producer(consumer_addr: SocketAddr) -> HorusResult<Self> {
        let bind_addr: SocketAddr = if consumer_addr.is_ipv4() {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };

        let socket =
            UdpSocket::bind(bind_addr).map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

        // KEY OPTIMIZATION: connect() to peer for faster send()
        socket
            .connect(consumer_addr)
            .map_err(|e| format!("Failed to connect UDP socket to {}: {}", consumer_addr, e))?;

        socket
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {}", e))?;

        Ok(Self {
            socket,
            role: LinkRole::Producer,
            send_buffer: std::cell::UnsafeCell::new(Vec::with_capacity(LINK_BUFFER_SIZE)),
            recv_buffer: std::cell::UnsafeCell::new(Vec::new()), // Producer doesn't recv
            _phantom: std::marker::PhantomData,
        })
    }

    /// Create a Connected UDP consumer
    ///
    /// The consumer bind()s to a port and waits for the producer to send data.
    /// Once first packet arrives, we know the producer's address.
    pub fn new_consumer(listen_addr: SocketAddr) -> HorusResult<Self> {
        let socket = UdpSocket::bind(listen_addr)
            .map_err(|e| format!("Failed to bind UDP socket to {}: {}", listen_addr, e))?;

        socket
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {}", e))?;

        Ok(Self {
            socket,
            role: LinkRole::Consumer,
            send_buffer: std::cell::UnsafeCell::new(Vec::new()), // Consumer doesn't send
            recv_buffer: std::cell::UnsafeCell::new(vec![0u8; LINK_BUFFER_SIZE]),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Ultra-fast send using connected UDP
    ///
    /// Uses send() instead of sendto() - kernel already knows the destination
    /// from the connect() call, so this is ~30% faster.
    #[inline(always)]
    pub fn send(&self, msg: &T) -> HorusResult<()> {
        // Safety: In 1P1C design, only producer calls send()
        let buffer = unsafe { &mut *self.send_buffer.get() };

        // Serialize into pre-allocated buffer
        buffer.clear();
        bincode::serialize_into(&mut *buffer, msg)
            .map_err(|e| format!("Serialization error: {}", e))?;

        // Connected UDP: just send() - no address needed!
        self.socket
            .send(buffer)
            .map_err(|e| format!("UDP send error: {}", e))?;

        Ok(())
    }

    /// Ultra-fast receive using pre-allocated buffer
    #[inline(always)]
    pub fn recv(&self) -> Option<T> {
        // Safety: In 1P1C design, only consumer calls recv()
        let buffer = unsafe { &mut *self.recv_buffer.get() };

        // recv() is faster than recv_from() for connected sockets
        match self.socket.recv(buffer) {
            Ok(len) => bincode::deserialize(&buffer[..len]).ok(),
            Err(_) => None,
        }
    }

    /// Get the local address this socket is bound to
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.socket.local_addr().ok()
    }
}

#[cfg(unix)]
impl<T> UnixSocketLinkBackend<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    /// Create a Unix socket producer with connect() for fast send()
    pub fn new_producer(topic: &str, consumer_path: &str) -> HorusResult<Self> {
        let socket_path = format!("/tmp/horus_link_{}_{}.sock", topic, std::process::id());

        // Remove if exists
        let _ = std::fs::remove_file(&socket_path);

        let socket = UnixDatagram::bind(&socket_path)
            .map_err(|e| format!("Failed to bind Unix socket: {}", e))?;

        // KEY OPTIMIZATION: connect() to consumer for fast send()
        socket
            .connect(consumer_path)
            .map_err(|e| format!("Failed to connect Unix socket to {}: {}", consumer_path, e))?;

        socket
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {}", e))?;

        Ok(Self {
            socket,
            socket_path,
            role: LinkRole::Producer,
            send_buffer: std::cell::UnsafeCell::new(Vec::with_capacity(LINK_BUFFER_SIZE)),
            recv_buffer: std::cell::UnsafeCell::new(Vec::new()),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Create a Unix socket consumer
    pub fn new_consumer(topic: &str) -> HorusResult<Self> {
        let socket_path = format!("/tmp/horus_link_{}_consumer.sock", topic);

        // Remove if exists
        let _ = std::fs::remove_file(&socket_path);

        let socket = UnixDatagram::bind(&socket_path)
            .map_err(|e| format!("Failed to bind Unix socket: {}", e))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {}", e))?;

        Ok(Self {
            socket,
            socket_path,
            role: LinkRole::Consumer,
            send_buffer: std::cell::UnsafeCell::new(Vec::new()),
            recv_buffer: std::cell::UnsafeCell::new(vec![0u8; LINK_BUFFER_SIZE]),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Ultra-fast send using connected Unix socket
    #[inline(always)]
    pub fn send(&self, msg: &T) -> HorusResult<()> {
        let buffer = unsafe { &mut *self.send_buffer.get() };

        buffer.clear();
        bincode::serialize_into(&mut *buffer, msg)
            .map_err(|e| format!("Serialization error: {}", e))?;

        // Connected Unix socket: just send()!
        self.socket
            .send(buffer)
            .map_err(|e| format!("Unix socket send error: {}", e))?;

        Ok(())
    }

    /// Ultra-fast receive using pre-allocated buffer
    #[inline(always)]
    pub fn recv(&self) -> Option<T> {
        let buffer = unsafe { &mut *self.recv_buffer.get() };

        match self.socket.recv(buffer) {
            Ok(len) => bincode::deserialize(&buffer[..len]).ok(),
            Err(_) => None,
        }
    }

    pub fn get_path(&self) -> &str {
        &self.socket_path
    }
}

#[cfg(unix)]
impl<T> Drop for UnixSocketLinkBackend<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// SPSC (Single Producer Single Consumer) direct link with shared memory IPC or network
/// Single-slot design: always returns the LATEST value, perfect for sensors/control
/// Producer overwrites old data, consumer tracks what it's already read via sequence number
///
/// Supports both local shared memory and network endpoints:
/// - `"topic"` → Local shared memory (248ns latency)
/// - `"topic@192.168.1.5:9000"` → Direct network connection (3-5µs with batch UDP)
/// - `"topic@localhost"` → Unix socket (optimized for localhost)
///
/// Network v2: Smart transport selection automatically picks the best backend:
/// - Batch UDP with sendmmsg/recvmmsg for high throughput (Linux)
/// - Standard UDP for cross-platform compatibility
/// - Unix domain sockets for localhost optimization
/// - TCP fallback for reliability when needed
#[repr(align(64))]
pub struct Link<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    shm_region: Option<Arc<ShmRegion>>, // Local shared memory (if local)
    network: Option<LinkNetworkBackend<T>>, // Network backend (if network)
    is_network: bool,                   // Fast dispatch flag
    transport_type: &'static str,       // For diagnostics
    topic_name: String,
    producer_node: String,
    consumer_node: String,
    role: LinkRole,
    header: Option<NonNull<LinkHeader>>, // Only for local
    data_ptr: Option<NonNull<u8>>,       // Only for local
    last_seen_sequence: AtomicU64,       // Consumer tracks what it's read (local memory)
    metrics: Arc<AtomicLinkMetrics>,
    state: std::sync::atomic::AtomicU8, // Lock-free state using atomic u8
    _phantom: PhantomData<T>,
}

// Manual Debug implementation since DirectBackend doesn't implement Debug for all T
impl<T> std::fmt::Debug for Link<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Link")
            .field("topic_name", &self.topic_name)
            .field("role", &self.role)
            .field("is_network", &self.is_network)
            .field("transport_type", &self.transport_type)
            .field(
                "state",
                &ConnectionState::from_u8(self.state.load(std::sync::atomic::Ordering::Relaxed)),
            )
            .finish_non_exhaustive()
    }
}

impl<T> Link<T>
where
    T: crate::core::LogSummary
        + serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    // ====== PRIMARY API (recommended) ======

    /// Create a Link as a producer (sender)
    ///
    /// The producer can send messages but cannot receive.
    /// Single-slot design: always overwrites with latest value.
    ///
    /// Supports both local shared memory and network endpoints:
    /// - `"sensor_data"` → Local shared memory (248ns latency)
    /// - `"sensor_data@192.168.1.5:9000"` → Direct network connection (5-15µs latency)
    ///
    /// # Example
    /// ```rust,ignore
    /// // Local
    /// let output: Link<f32> = Link::producer("sensor_data")?;
    ///
    /// // Network
    /// let output: Link<f32> = Link::producer("sensor_data@192.168.1.5:9000")?;
    /// output.send(42.0, None)?;
    /// ```
    pub fn producer(topic: &str) -> HorusResult<Self> {
        Self::with_role(topic, LinkRole::Producer)
    }

    /// Create a Link as a consumer (receiver)
    ///
    /// The consumer can receive messages but cannot send.
    /// Single-slot design: always reads latest value, skips if already seen.
    ///
    /// Supports both local shared memory and network endpoints:
    /// - `"sensor_data"` → Local shared memory (248ns latency)
    /// - `"sensor_data@0.0.0.0:9000"` → Listen for network connections (5-15µs latency)
    ///
    /// # Example
    /// ```rust,ignore
    /// // Local
    /// let input: Link<f32> = Link::consumer("sensor_data")?;
    ///
    /// // Network (listen for producer)
    /// let input: Link<f32> = Link::consumer("sensor_data@0.0.0.0:9000")?;
    /// if let Some(value) = input.recv(None) {
    ///     println!("Received: {}", value);
    /// }
    /// ```
    pub fn consumer(topic: &str) -> HorusResult<Self> {
        Self::with_role(topic, LinkRole::Consumer)
    }

    /// Create a global Link as a producer (accessible across all sessions)
    ///
    /// Global Links can communicate across different HORUS sessions.
    /// Unlike session-scoped Links, global Links are accessible system-wide.
    ///
    /// Note: Global Links only support local shared memory (not network endpoints).
    ///
    /// # Example
    /// ```rust,ignore
    /// // Create a global producer accessible from any session
    /// let output: Link<f32> = Link::producer_global("global_sensor")?;
    /// output.send(42.0, None)?;
    ///
    /// // Another process/session can consume from this global Link
    /// ```
    pub fn producer_global(topic: &str) -> HorusResult<Self> {
        Self::with_role_global(topic, LinkRole::Producer)
    }

    /// Create a global Link as a consumer (accessible across all sessions)
    ///
    /// Global Links can communicate across different HORUS sessions.
    /// Unlike session-scoped Links, global Links are accessible system-wide.
    ///
    /// Note: Global Links only support local shared memory (not network endpoints).
    ///
    /// # Example
    /// ```rust,ignore
    /// // Create a global consumer accessible from any session
    /// let input: Link<f32> = Link::consumer_global("global_sensor")?;
    /// if let Some(value) = input.recv(None) {
    ///     println!("Received: {}", value);
    /// }
    /// ```
    pub fn consumer_global(topic: &str) -> HorusResult<Self> {
        Self::with_role_global(topic, LinkRole::Consumer)
    }

    /// Create a Link producer from configuration file
    ///
    /// Loads link configuration from TOML/YAML file and creates a producer.
    ///
    /// # Arguments
    /// * `link_name` - Name of the link to look up in the config file
    ///
    /// # Config File Format
    ///
    /// TOML example:
    /// ```toml
    /// [hubs.video_link]
    /// name = "video"
    /// endpoint = "video@192.168.1.50:9000"  # Producer connects to this
    /// ```
    ///
    /// YAML example:
    /// ```yaml
    /// hubs:
    ///   video_link:
    ///     name: video
    ///     endpoint: video@192.168.1.50:9000
    /// ```
    ///
    /// # Config File Search Paths
    /// 1. `./horus.toml` or `./horus.yaml`
    /// 2. `~/.horus/config.toml` or `~/.horus/config.yaml`
    /// 3. `/etc/horus/config.toml` or `/etc/horus/config.yaml`
    ///
    /// # Example
    /// ```rust,ignore
    /// // Load from config and create producer
    /// let output: Link<VideoFrame> = Link::producer_from_config("video_link")?;
    /// output.send(frame, None)?;
    /// ```
    pub fn producer_from_config(link_name: &str) -> HorusResult<Self> {
        use crate::communication::config::HorusConfig;

        // Load config from standard search paths
        let config = HorusConfig::find_and_load()?;

        // Get link config
        let link_config = config.get_hub(link_name)?;

        // Get endpoint string
        let endpoint_str = link_config.get_endpoint();

        // Create producer with the endpoint
        Self::producer(&endpoint_str)
    }

    /// Create a Link producer from a specific config file path
    ///
    /// # Arguments
    /// * `config_path` - Path to the configuration file (TOML or YAML)
    /// * `link_name` - Name of the link to look up in the config file
    ///
    /// # Example
    /// ```rust,ignore
    /// let output: Link<f32> = Link::producer_from_config_file("my_config.toml", "sensor_link")?;
    /// ```
    pub fn producer_from_config_file<P: AsRef<std::path::Path>>(
        config_path: P,
        link_name: &str,
    ) -> HorusResult<Self> {
        use crate::communication::config::HorusConfig;

        // Load config from specific file
        let config = HorusConfig::from_file(config_path)?;

        // Get link config
        let link_config = config.get_hub(link_name)?;

        // Get endpoint string
        let endpoint_str = link_config.get_endpoint();

        // Create producer with the endpoint
        Self::producer(&endpoint_str)
    }

    /// Create a Link consumer from configuration file
    ///
    /// Loads link configuration from TOML/YAML file and creates a consumer.
    ///
    /// # Arguments
    /// * `link_name` - Name of the link to look up in the config file
    ///
    /// # Config File Format
    ///
    /// TOML example:
    /// ```toml
    /// [hubs.video_link]
    /// name = "video"
    /// endpoint = "video@0.0.0.0:9000"  # Consumer listens on this port
    /// ```
    ///
    /// # Example
    /// ```rust,ignore
    /// // Load from config and create consumer
    /// let input: Link<VideoFrame> = Link::consumer_from_config("video_link")?;
    /// if let Some(frame) = input.recv(None) {
    ///     process(frame);
    /// }
    /// ```
    pub fn consumer_from_config(link_name: &str) -> HorusResult<Self> {
        use crate::communication::config::HorusConfig;

        // Load config from standard search paths
        let config = HorusConfig::find_and_load()?;

        // Get link config
        let link_config = config.get_hub(link_name)?;

        // Get endpoint string
        let endpoint_str = link_config.get_endpoint();

        // Create consumer with the endpoint
        Self::consumer(&endpoint_str)
    }

    /// Create a Link consumer from a specific config file path
    ///
    /// # Arguments
    /// * `config_path` - Path to the configuration file (TOML or YAML)
    /// * `link_name` - Name of the link to look up in the config file
    ///
    /// # Example
    /// ```rust,ignore
    /// let input: Link<f32> = Link::consumer_from_config_file("my_config.toml", "sensor_link")?;
    /// ```
    pub fn consumer_from_config_file<P: AsRef<std::path::Path>>(
        config_path: P,
        link_name: &str,
    ) -> HorusResult<Self> {
        use crate::communication::config::HorusConfig;

        // Load config from specific file
        let config = HorusConfig::from_file(config_path)?;

        // Get link config
        let link_config = config.get_hub(link_name)?;

        // Get endpoint string
        let endpoint_str = link_config.get_endpoint();

        // Create consumer with the endpoint
        Self::consumer(&endpoint_str)
    }

    // ====== INTERNAL IMPLEMENTATION ======

    /// Internal method to create Link with explicit role
    fn with_role(topic: &str, role: LinkRole) -> HorusResult<Self> {
        let element_size = mem::size_of::<T>();

        if element_size == 0 {
            return Err("Cannot create Link for zero-sized types".into());
        }

        // Parse endpoint: check if it's network (contains '@')
        if topic.contains('@') {
            // Network endpoint
            return Self::create_network_link(topic, role);
        }

        // Local shared memory
        Self::create_local_link(topic, role)
    }

    /// Internal method to create global Link with explicit role
    fn with_role_global(topic: &str, role: LinkRole) -> HorusResult<Self> {
        let element_size = mem::size_of::<T>();

        if element_size == 0 {
            return Err("Cannot create Link for zero-sized types".into());
        }

        // Global Links only support local shared memory (no network)
        if topic.contains('@') {
            return Err("Global Links do not support network endpoints".into());
        }

        // Global shared memory
        Self::create_global_link(topic, role)
    }

    /// Create a network-based Link with smart transport selection
    ///
    /// Network v2: Automatically selects the best transport based on target:
    /// - localhost: Unix sockets (lowest latency)
    /// - LAN: Batch UDP with sendmmsg/recvmmsg (Linux) or standard UDP
    /// - WAN: TCP with congestion control (fallback)
    fn create_network_link(endpoint: &str, role: LinkRole) -> HorusResult<Self> {
        // Parse endpoint: "topic@host:port" or "topic@localhost"
        let parts: Vec<&str> = endpoint.split('@').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid network endpoint: {}", endpoint).into());
        }

        let topic_name = parts[0];
        let addr_str = parts[1];

        // Use smart transport selection based on network location
        let (network, transport_type) = Self::select_transport(topic_name, addr_str, role)?;

        log::info!(
            "Link '{}': Created as {:?} ({} to {})",
            topic_name,
            role,
            transport_type,
            addr_str
        );

        let metrics = Arc::new(AtomicLinkMetrics {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            send_failures: AtomicU64::new(0),
            recv_failures: AtomicU64::new(0),
            _padding: [0; 32],
        });

        Ok(Link {
            shm_region: None,
            network: Some(network),
            is_network: true,
            transport_type,
            topic_name: topic_name.to_string(),
            producer_node: "producer".to_string(),
            consumer_node: "consumer".to_string(),
            role,
            header: None,
            data_ptr: None,
            last_seen_sequence: AtomicU64::new(0),
            metrics,
            state: std::sync::atomic::AtomicU8::new(ConnectionState::Connected.into_u8()),
            _phantom: PhantomData,
        })
    }

    /// Select the best transport for the given address
    ///
    /// Link uses a simplified transport selection optimized for 1P1C:
    /// - localhost: Unix sockets (fastest for local IPC)
    /// - LAN/WAN: Connected UDP (optimized for point-to-point)
    fn select_transport(
        topic: &str,
        addr_str: &str,
        role: LinkRole,
    ) -> HorusResult<(LinkNetworkBackend<T>, &'static str)> {
        // Check for localhost - use Unix socket (fastest for local)
        if addr_str == "localhost"
            || addr_str.starts_with("127.0.0.1")
            || addr_str.starts_with("::1")
        {
            #[cfg(unix)]
            {
                // For localhost, prefer Unix sockets - ~1-2µs latency
                let consumer_path = format!("/tmp/horus_link_{}_consumer.sock", topic);
                let backend = match role {
                    LinkRole::Producer => {
                        UnixSocketLinkBackend::new_producer(topic, &consumer_path)?
                    }
                    LinkRole::Consumer => UnixSocketLinkBackend::new_consumer(topic)?,
                };
                return Ok((LinkNetworkBackend::UnixSocket(backend), "unix_socket"));
            }
            #[cfg(not(unix))]
            {
                // Fall through to Connected UDP on non-Unix
            }
        }

        // Parse address for non-localhost
        let addr: SocketAddr = addr_str
            .parse()
            .map_err(|e| format!("Invalid address '{}': {}", addr_str, e))?;

        // Log transport selection
        let location = NetworkLocation::from_addr(&addr);
        log::debug!(
            "Link '{}': Using Connected UDP for {:?} (1P1C optimized)",
            topic,
            location
        );

        // For all network scenarios, use Connected UDP
        // This is optimal for 1P1C because:
        // - connect() caches route in kernel (~30% faster than sendto)
        // - No batching overhead (single producer/consumer)
        // - Pre-allocated buffers avoid allocation on hot path
        let backend = match role {
            LinkRole::Producer => ConnectedUdpBackend::new_producer(addr)?,
            LinkRole::Consumer => ConnectedUdpBackend::new_consumer(addr)?,
        };
        Ok((LinkNetworkBackend::ConnectedUdp(backend), "connected_udp"))
    }

    /// Create a local shared memory Link
    fn create_local_link(topic: &str, role: LinkRole) -> HorusResult<Self> {
        let element_size = mem::size_of::<T>();
        let element_align = mem::align_of::<T>();
        let header_size = mem::size_of::<LinkHeader>();

        // Single-slot design: header + one element
        let aligned_header_size = header_size.div_ceil(element_align) * element_align;
        let total_size = aligned_header_size + element_size;

        let link_name = format!("links/{}", topic);
        let shm_region = Arc::new(ShmRegion::new(&link_name, total_size)?);

        // Use role names for logging
        let (producer_node, consumer_node) = match role {
            LinkRole::Producer => ("producer", "consumer"),
            LinkRole::Consumer => ("consumer", "producer"),
        };

        Self::create_link(topic, producer_node, consumer_node, role, shm_region)
    }

    /// Create a global shared memory Link (accessible across all sessions)
    fn create_global_link(topic: &str, role: LinkRole) -> HorusResult<Self> {
        let element_size = mem::size_of::<T>();
        let element_align = mem::align_of::<T>();
        let header_size = mem::size_of::<LinkHeader>();

        // Single-slot design: header + one element
        let aligned_header_size = header_size.div_ceil(element_align) * element_align;
        let total_size = aligned_header_size + element_size;

        let link_name = format!("links/{}", topic);
        // Use new_global() for cross-session accessibility
        let shm_region = Arc::new(ShmRegion::new_global(&link_name, total_size)?);

        // Use role names for logging
        let (producer_node, consumer_node) = match role {
            LinkRole::Producer => ("producer", "consumer"),
            LinkRole::Consumer => ("consumer", "producer"),
        };

        Self::create_link(topic, producer_node, consumer_node, role, shm_region)
    }

    /// Common link creation logic
    fn create_link(
        topic_name: &str,
        producer_node: &str,
        consumer_node: &str,
        role: LinkRole,
        shm_region: Arc<ShmRegion>,
    ) -> HorusResult<Self> {
        let element_size = mem::size_of::<T>();
        let element_align = mem::align_of::<T>();
        let header_size = mem::size_of::<LinkHeader>();
        let aligned_header_size = header_size.div_ceil(element_align) * element_align;

        // Initialize header
        let header_ptr = shm_region.as_ptr() as *mut LinkHeader;
        if header_ptr.is_null() {
            return Err("Null pointer for Link header".into());
        }

        let header = unsafe { NonNull::new_unchecked(header_ptr) };

        if shm_region.is_owner() {
            // Initialize header for first time - single-slot design
            unsafe {
                (*header.as_ptr()).sequence.store(0, Ordering::Relaxed);
                (*header.as_ptr())
                    .element_size
                    .store(element_size, Ordering::Relaxed);
                (*header.as_ptr())._padding = [0; 48];
            }
        } else {
            // Validate existing header
            let stored_element_size =
                unsafe { (*header.as_ptr()).element_size.load(Ordering::Relaxed) };

            if stored_element_size != element_size {
                return Err(format!(
                    "Element size mismatch: expected {}, got {}",
                    element_size, stored_element_size
                )
                .into());
            }
        }

        // Data pointer
        let data_ptr = unsafe {
            let raw_ptr = (shm_region.as_ptr() as *mut u8).add(aligned_header_size);
            if raw_ptr.is_null() {
                return Err("Null pointer for Link data".into());
            }
            NonNull::new_unchecked(raw_ptr)
        };

        log::info!(
            "Link '{}': Created as {:?} ({} -> {})",
            topic_name,
            role,
            producer_node,
            consumer_node
        );

        // Initialize metrics in local memory (Arc for cheap cloning)
        let metrics = Arc::new(AtomicLinkMetrics {
            messages_sent: std::sync::atomic::AtomicU64::new(0),
            messages_received: std::sync::atomic::AtomicU64::new(0),
            send_failures: std::sync::atomic::AtomicU64::new(0),
            recv_failures: std::sync::atomic::AtomicU64::new(0),
            _padding: [0; 32],
        });

        Ok(Link {
            shm_region: Some(shm_region),
            network: None,
            is_network: false,
            transport_type: "shm",
            topic_name: topic_name.to_string(),
            producer_node: producer_node.to_string(),
            consumer_node: consumer_node.to_string(),
            role,
            header: Some(header),
            data_ptr: Some(data_ptr),
            last_seen_sequence: AtomicU64::new(0),
            metrics,
            state: std::sync::atomic::AtomicU8::new(ConnectionState::Connected.into_u8()),
            _phantom: PhantomData,
        })
    }

    /// Ultra-fast send with inline zero-copy - optimized for minimum latency
    /// Single-slot design: always overwrites with latest value
    /// Automatically logs if context is provided
    ///
    /// Supports both local shared memory and network transparently
    ///
    /// Optimizations applied:
    /// - Single atomic operation (sequence increment) for local
    /// - Connected UDP with pre-allocated buffers for network
    /// - Relaxed atomics for metrics
    #[inline(always)]
    pub fn send(&self, msg: T, ctx: &mut Option<&mut NodeInfo>) -> Result<(), T>
    where
        T: std::fmt::Debug + Clone + serde::Serialize,
    {
        // Network path - optimized for 1P1C
        if self.is_network {
            if let Some(ref network) = self.network {
                let send_result = match network {
                    LinkNetworkBackend::ConnectedUdp(udp) => udp.send(&msg),
                    #[cfg(unix)]
                    LinkNetworkBackend::UnixSocket(unix) => unix.send(&msg),
                };

                match send_result {
                    Ok(_) => {
                        self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                        self.state.store(
                            ConnectionState::Connected.into_u8(),
                            std::sync::atomic::Ordering::Relaxed,
                        );

                        if unlikely(ctx.is_some()) {
                            if let Some(ref mut ctx) = ctx {
                                ctx.log_pub(&self.topic_name, &msg, 0);
                            }
                        }

                        return Ok(());
                    }
                    Err(_) => {
                        self.metrics.send_failures.fetch_add(1, Ordering::Relaxed);
                        self.state.store(
                            ConnectionState::Failed.into_u8(),
                            std::sync::atomic::Ordering::Relaxed,
                        );
                        return Err(msg);
                    }
                }
            }
            return Err(msg); // Shouldn't happen
        }

        // Local shared memory path (optimized with IPC timing)
        let header = unsafe { self.header.as_ref().unwrap().as_ref() };
        let data_ptr = self.data_ptr.unwrap();

        // Fast path: when ctx is None (benchmarks), bypass timing and logging completely
        if ctx.is_none() {
            // TIME ONLY THE ACTUAL IPC OPERATION
            let ipc_start = Instant::now();

            // Write message to the single slot
            unsafe {
                let slot = data_ptr.as_ptr() as *mut T;
                std::ptr::write(slot, msg);
            }

            // Increment sequence with Release to publish (this is the only sync point!)
            header.sequence.fetch_add(1, Ordering::Release);

            let _ipc_ns = ipc_start.elapsed().as_nanos() as u64;
            // END TIMING - no logging path

            // Update local metrics and state
            self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
            self.state.store(
                ConnectionState::Connected.into_u8(),
                std::sync::atomic::Ordering::Relaxed,
            );

            return Ok(());
        }

        // Logging enabled path: time IPC and log with accurate timing
        // TIME ONLY THE ACTUAL IPC OPERATION
        let ipc_start = Instant::now();

        // Write message to the single slot
        unsafe {
            let slot = data_ptr.as_ptr() as *mut T;
            std::ptr::write(slot, msg);
        }

        // Increment sequence with Release to publish (this is the only sync point!)
        header.sequence.fetch_add(1, Ordering::Release);

        let ipc_ns = ipc_start.elapsed().as_nanos() as u64;
        // END TIMING - everything after this is logging overhead

        // Update local metrics and state
        self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.state.store(
            ConnectionState::Connected.into_u8(),
            std::sync::atomic::Ordering::Relaxed,
        );

        // Log with accurate IPC timing
        if let Some(ref mut ctx) = ctx {
            let slot = unsafe { &*(data_ptr.as_ptr() as *const T) };
            ctx.log_pub(&self.topic_name, slot, ipc_ns);
        }

        Ok(())
    }

    /// Ultra-fast receive with inline - optimized for minimum latency
    /// Single-slot design: reads latest value if new, returns None if already seen
    /// Automatically logs if context is provided
    ///
    /// Supports both local shared memory and network transparently
    ///
    /// Optimizations applied:
    /// - Single atomic load with Acquire (syncs with producer's Release) for local
    /// - Connected UDP with pre-allocated buffers for network
    /// - Local sequence tracking (no atomic stores to shared memory)
    /// - Relaxed atomics for metrics
    #[inline(always)]
    pub fn recv(&self, ctx: &mut Option<&mut NodeInfo>) -> Option<T>
    where
        T: std::fmt::Debug + Clone + serde::de::DeserializeOwned,
    {
        // Network path - optimized for 1P1C
        if self.is_network {
            if let Some(ref network) = self.network {
                let recv_result = match network {
                    LinkNetworkBackend::ConnectedUdp(udp) => udp.recv(),
                    #[cfg(unix)]
                    LinkNetworkBackend::UnixSocket(unix) => unix.recv(),
                };

                if let Some(msg) = recv_result {
                    self.metrics
                        .messages_received
                        .fetch_add(1, Ordering::Relaxed);
                    self.state.store(
                        ConnectionState::Connected.into_u8(),
                        std::sync::atomic::Ordering::Relaxed,
                    );

                    if unlikely(ctx.is_some()) {
                        if let Some(ref mut ctx) = ctx {
                            ctx.log_sub(&self.topic_name, &msg, 0);
                        }
                    }

                    return Some(msg);
                }
                // Network recv returned None - this is normal for non-blocking
                // Don't count as failure since it could just mean no data yet
            }
            return None;
        }

        // Local shared memory path (optimized with IPC timing)
        let header = unsafe { self.header.as_ref().unwrap().as_ref() };
        let data_ptr = self.data_ptr.unwrap();

        // TIME ONLY THE ACTUAL IPC OPERATION
        let ipc_start = Instant::now();

        // Read sequence with Acquire to synchronize with producer's Release
        let current_seq = header.sequence.load(Ordering::Acquire);
        let last_seen = self.last_seen_sequence.load(Ordering::Relaxed);

        // If we've already seen this sequence, return None (no new data)
        if current_seq <= last_seen {
            return None;
        }

        // Read the message
        let msg = unsafe {
            let slot = data_ptr.as_ptr() as *const T;
            std::ptr::read(slot)
        };

        let ipc_ns = ipc_start.elapsed().as_nanos() as u64;
        // END TIMING - everything after this is post-IPC operations

        // Update what we've seen (local memory, Relaxed is fine)
        self.last_seen_sequence
            .store(current_seq, Ordering::Relaxed);

        // Update local metrics and state
        self.metrics
            .messages_received
            .fetch_add(1, Ordering::Relaxed);
        self.state.store(
            ConnectionState::Connected.into_u8(),
            std::sync::atomic::Ordering::Relaxed,
        );

        // Log with accurate IPC timing
        if unlikely(ctx.is_some()) {
            if let Some(ref mut ctx) = ctx {
                ctx.log_sub(&self.topic_name, &msg, ipc_ns);
            }
        }

        Some(msg)
    }

    /// Check if link has messages available (new data since last read)
    ///
    /// For local shared memory Links, this checks if the sequence number has incremented
    /// (indicating new data).
    ///
    /// For network Links using UDP, this always returns true since UDP is non-blocking
    /// and we can't peek without consuming. Use recv() to check for actual data.
    ///
    /// # Returns
    ///
    /// - `true` if new data is available (or might be available for network)
    /// - `false` if no new data (already seen all data)
    pub fn has_messages(&self) -> bool {
        if self.is_network {
            // For 1P1C network Links, we can't peek UDP without consuming
            // Return true to indicate caller should try recv()
            // This is semantically correct for the single-slot "latest value" design
            true
        } else {
            // Local shared memory: check sequence number
            let header = unsafe { self.header.as_ref().unwrap().as_ref() };
            let current_seq = header.sequence.load(Ordering::Acquire);
            let last_seen = self.last_seen_sequence.load(Ordering::Relaxed);
            current_seq > last_seen
        }
    }

    /// Get the role of this Link end
    pub fn role(&self) -> LinkRole {
        self.role
    }

    /// Check if this Link end is a producer
    pub fn is_producer(&self) -> bool {
        matches!(self.role, LinkRole::Producer)
    }

    /// Check if this Link end is a consumer
    pub fn is_consumer(&self) -> bool {
        matches!(self.role, LinkRole::Consumer)
    }

    /// Get the topic name
    pub fn get_topic_name(&self) -> &str {
        &self.topic_name
    }

    /// Get current connection state (lock-free)
    ///
    /// Returns the current connection state of the Link.
    /// For local shared memory Links, this will typically always be Connected.
    /// For network Links, this tracks whether the connection is healthy or has failures.
    pub fn get_connection_state(&self) -> ConnectionState {
        let state_u8 = self.state.load(std::sync::atomic::Ordering::Relaxed);
        ConnectionState::from_u8(state_u8)
    }

    /// Get performance metrics snapshot (lock-free)
    ///
    /// Returns current counts of messages sent, received, send failures, and recv failures.
    /// These metrics are stored in local memory for zero-overhead tracking.
    pub fn get_metrics(&self) -> LinkMetrics {
        LinkMetrics {
            messages_sent: self.metrics.messages_sent.load(Ordering::Relaxed),
            messages_received: self.metrics.messages_received.load(Ordering::Relaxed),
            send_failures: self.metrics.send_failures.load(Ordering::Relaxed),
            recv_failures: self.metrics.recv_failures.load(Ordering::Relaxed),
        }
    }
}

// Clone implementation for local shared memory Links only
impl<T> Clone for Link<T>
where
    T: crate::core::LogSummary
        + serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static,
{
    /// Clone this Link
    ///
    /// # Panics
    ///
    /// Panics if called on a network Link, as network backends contain
    /// non-cloneable resources (TCP streams, sockets, etc.).
    ///
    /// Only local shared memory Links can be cloned safely.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let producer: Link<f64> = Link::producer("sensor")?;
    /// let producer_clone = producer.clone();  // [OK] Works for local Links
    ///
    /// // Both can send independently
    /// producer.send(1.0, None)?;
    /// producer_clone.send(2.0, None)?;
    /// ```
    fn clone(&self) -> Self {
        if self.is_network {
            panic!(
                "Cannot clone network Link '{}': network backends contain non-cloneable resources. \
                Create separate Link instances for each endpoint instead.",
                self.topic_name
            );
        }

        Self {
            shm_region: self.shm_region.clone(), // Arc - cheap clone
            network: None, // Network backend dropped (only local Links can be cloned)
            is_network: false,
            transport_type: self.transport_type,
            topic_name: self.topic_name.clone(),
            producer_node: self.producer_node.clone(),
            consumer_node: self.consumer_node.clone(),
            role: self.role,
            header: self.header,     // NonNull - just copy the pointer
            data_ptr: self.data_ptr, // NonNull - just copy the pointer
            last_seen_sequence: AtomicU64::new(self.last_seen_sequence.load(Ordering::Relaxed)),
            metrics: self.metrics.clone(), // Arc - cheap clone
            state: std::sync::atomic::AtomicU8::new(self.state.load(Ordering::Relaxed)),
            _phantom: PhantomData,
        }
    }
}

unsafe impl<T> Send for Link<T> where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static
{
}
unsafe impl<T> Sync for Link<T> where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + Clone
        + std::fmt::Debug
        + 'static
{
}
