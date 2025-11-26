//! Smart Transport Selector for automatic backend selection
//!
//! Automatically chooses the best transport based on:
//! - Target location (local vs LAN vs WAN)
//! - Available system features (io_uring, QUIC)
//! - User configuration
//! - Runtime performance feedback
//!
//! Priority order:
//! 1. Shared Memory (same machine) - 200-500ns
//! 2. io_uring (Linux, LAN) - 3-5µs
//! 3. Batch UDP (LAN) - 10-20µs
//! 4. QUIC (WAN, reliable) - variable

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Available transport types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    /// Local shared memory (fastest, same machine only)
    SharedMemory,
    /// io_uring zero-copy (Linux only, ~2-3µs)
    IoUring,
    /// Batch UDP with sendmmsg/recvmmsg (fast, LAN)
    BatchUdp,
    /// Standard UDP (fallback)
    Udp,
    /// TCP with optimizations
    Tcp,
    /// QUIC (reliable, WAN-friendly)
    Quic,
    /// Unix domain socket (local, cross-process)
    UnixSocket,
}

impl TransportType {
    /// Get expected latency range in microseconds
    pub fn expected_latency_us(&self) -> (u32, u32) {
        match self {
            TransportType::SharedMemory => (0, 1),      // 200-500ns
            TransportType::IoUring => (2, 5),           // 2-5µs
            TransportType::UnixSocket => (1, 3),        // 1-3µs
            TransportType::BatchUdp => (5, 15),         // 5-15µs
            TransportType::Udp => (5, 20),              // 5-20µs
            TransportType::Tcp => (10, 50),             // 10-50µs
            TransportType::Quic => (20, 100),           // 20-100µs (includes crypto)
        }
    }

    /// Check if this transport is available on the current system
    pub fn is_available(&self) -> bool {
        match self {
            TransportType::SharedMemory => true,
            TransportType::IoUring => {
                #[cfg(all(target_os = "linux", feature = "io-uring-net"))]
                {
                    super::io_uring::is_real_io_uring_available()
                }
                #[cfg(not(all(target_os = "linux", feature = "io-uring-net")))]
                {
                    false
                }
            }
            TransportType::UnixSocket => cfg!(unix),
            TransportType::BatchUdp => cfg!(target_os = "linux"),
            TransportType::Udp => true,
            TransportType::Tcp => true,
            TransportType::Quic => cfg!(feature = "quic"),
        }
    }

    /// Get priority for transport selection (higher = better)
    pub fn priority(&self) -> u8 {
        match self {
            TransportType::SharedMemory => 100,  // Best for local
            TransportType::IoUring => 95,        // Best for network (Linux)
            TransportType::UnixSocket => 85,    // Good for localhost
            TransportType::BatchUdp => 70,       // Good for LAN
            TransportType::Udp => 50,            // Universal fallback
            TransportType::Tcp => 40,            // Reliable but slower
            TransportType::Quic => 60,           // Good for WAN
        }
    }
}

/// Network location classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkLocation {
    /// Same process (can use shared memory)
    SameProcess,
    /// Same machine, different process
    SameMachine,
    /// Local network (LAN)
    LocalNetwork,
    /// Wide area network (internet)
    WideArea,
    /// Unknown (treat as WAN for safety)
    Unknown,
}

impl NetworkLocation {
    /// Determine location from socket address
    pub fn from_addr(addr: &SocketAddr) -> Self {
        match addr.ip() {
            IpAddr::V4(ip) => {
                if ip.is_loopback() {
                    NetworkLocation::SameMachine
                } else if ip.is_private() {
                    // 10.x.x.x, 172.16-31.x.x, 192.168.x.x
                    NetworkLocation::LocalNetwork
                } else if ip.is_link_local() {
                    // 169.254.x.x
                    NetworkLocation::LocalNetwork
                } else if ip.is_unspecified() {
                    NetworkLocation::SameMachine
                } else {
                    NetworkLocation::WideArea
                }
            }
            IpAddr::V6(ip) => {
                if ip.is_loopback() {
                    NetworkLocation::SameMachine
                } else if is_ipv6_unique_local(&ip) {
                    // fc00::/7 - Unique local addresses
                    NetworkLocation::LocalNetwork
                } else if is_ipv6_link_local(&ip) {
                    // fe80::/10 - Link-local
                    NetworkLocation::LocalNetwork
                } else if ip.is_unspecified() {
                    NetworkLocation::SameMachine
                } else {
                    NetworkLocation::WideArea
                }
            }
        }
    }
}

/// Check if IPv6 address is unique local (fc00::/7)
fn is_ipv6_unique_local(ip: &std::net::Ipv6Addr) -> bool {
    let segments = ip.segments();
    (segments[0] & 0xfe00) == 0xfc00
}

/// Check if IPv6 address is link-local (fe80::/10)
fn is_ipv6_link_local(ip: &std::net::Ipv6Addr) -> bool {
    let segments = ip.segments();
    (segments[0] & 0xffc0) == 0xfe80
}

/// Transport selection preferences
#[derive(Debug, Clone)]
pub struct TransportPreferences {
    /// Prefer lower latency over throughput
    pub prefer_low_latency: bool,
    /// Prefer reliability over speed
    pub prefer_reliability: bool,
    /// Allow unencrypted transports
    pub allow_unencrypted: bool,
    /// Force a specific transport type
    pub force_transport: Option<TransportType>,
    /// Enable automatic fallback
    pub enable_fallback: bool,
}

impl Default for TransportPreferences {
    fn default() -> Self {
        Self {
            prefer_low_latency: true,
            prefer_reliability: false,
            allow_unencrypted: true,
            force_transport: None,
            enable_fallback: true,
        }
    }
}

impl TransportPreferences {
    /// Prefer reliability (for critical data)
    pub fn reliable() -> Self {
        Self {
            prefer_low_latency: false,
            prefer_reliability: true,
            allow_unencrypted: true,
            force_transport: None,
            enable_fallback: true,
        }
    }

    /// Prefer security (encrypted only)
    pub fn secure() -> Self {
        Self {
            prefer_low_latency: false,
            prefer_reliability: true,
            allow_unencrypted: false,
            force_transport: None,
            enable_fallback: true,
        }
    }

    /// Maximum performance (may lose packets)
    pub fn max_performance() -> Self {
        Self {
            prefer_low_latency: true,
            prefer_reliability: false,
            allow_unencrypted: true,
            force_transport: None,
            enable_fallback: false,
        }
    }
}

/// Statistics for transport selection decisions
#[derive(Debug, Default)]
pub struct TransportSelectorStats {
    pub shm_selections: AtomicU64,
    pub io_uring_selections: AtomicU64,
    pub batch_udp_selections: AtomicU64,
    pub udp_selections: AtomicU64,
    pub tcp_selections: AtomicU64,
    pub quic_selections: AtomicU64,
    pub unix_socket_selections: AtomicU64,
    pub fallback_events: AtomicU64,
}

/// Smart transport selector
pub struct TransportSelector {
    preferences: TransportPreferences,
    stats: Arc<TransportSelectorStats>,
}

impl TransportSelector {
    /// Create a new transport selector with default preferences
    pub fn new() -> Self {
        Self {
            preferences: TransportPreferences::default(),
            stats: Arc::new(TransportSelectorStats::default()),
        }
    }

    /// Create with custom preferences
    pub fn with_preferences(preferences: TransportPreferences) -> Self {
        Self {
            preferences,
            stats: Arc::new(TransportSelectorStats::default()),
        }
    }

    /// Select the best transport for a given target
    pub fn select(&self, target: &SocketAddr) -> TransportType {
        // Check for forced transport
        if let Some(transport) = self.preferences.force_transport {
            if transport.is_available() {
                return transport;
            }
            // Fall through to selection if forced transport not available
        }

        let location = NetworkLocation::from_addr(target);
        let transport = self.select_for_location(location);

        // Record selection
        self.record_selection(transport);

        transport
    }

    /// Select transport based on network location
    fn select_for_location(&self, location: NetworkLocation) -> TransportType {
        match location {
            NetworkLocation::SameProcess | NetworkLocation::SameMachine => {
                self.select_local_transport()
            }
            NetworkLocation::LocalNetwork => {
                self.select_lan_transport()
            }
            NetworkLocation::WideArea | NetworkLocation::Unknown => {
                self.select_wan_transport()
            }
        }
    }

    /// Select transport for local communication
    fn select_local_transport(&self) -> TransportType {
        // Prefer shared memory for same machine
        if TransportType::SharedMemory.is_available() {
            return TransportType::SharedMemory;
        }

        // Unix socket for cross-process on Unix
        if TransportType::UnixSocket.is_available() {
            return TransportType::UnixSocket;
        }

        // io_uring if available
        if TransportType::IoUring.is_available() {
            return TransportType::IoUring;
        }

        // Fallback to batch UDP
        if TransportType::BatchUdp.is_available() {
            return TransportType::BatchUdp;
        }

        TransportType::Tcp
    }

    /// Select transport for LAN communication
    fn select_lan_transport(&self) -> TransportType {
        if self.preferences.prefer_reliability {
            // Reliable: prefer QUIC or TCP
            if !self.preferences.allow_unencrypted && TransportType::Quic.is_available() {
                return TransportType::Quic;
            }
            return TransportType::Tcp;
        }

        // Low latency path - use the fastest available
        if self.preferences.prefer_low_latency {
            // io_uring is fastest (~2-3µs)
            if TransportType::IoUring.is_available() {
                return TransportType::IoUring;
            }

            // Batch UDP is good (~5-15µs)
            if TransportType::BatchUdp.is_available() {
                return TransportType::BatchUdp;
            }
        }

        // Standard UDP
        TransportType::Udp
    }

    /// Select transport for WAN communication
    fn select_wan_transport(&self) -> TransportType {
        // QUIC is ideal for WAN
        if TransportType::Quic.is_available() {
            return TransportType::Quic;
        }

        // TCP for reliability
        if self.preferences.prefer_reliability {
            return TransportType::Tcp;
        }

        // UDP if low latency is more important
        TransportType::Udp
    }

    /// Record a transport selection for statistics
    fn record_selection(&self, transport: TransportType) {
        match transport {
            TransportType::SharedMemory => {
                self.stats.shm_selections.fetch_add(1, Ordering::Relaxed);
            }
            TransportType::IoUring => {
                self.stats.io_uring_selections.fetch_add(1, Ordering::Relaxed);
            }
            TransportType::BatchUdp => {
                self.stats.batch_udp_selections.fetch_add(1, Ordering::Relaxed);
            }
            TransportType::Udp => {
                self.stats.udp_selections.fetch_add(1, Ordering::Relaxed);
            }
            TransportType::Tcp => {
                self.stats.tcp_selections.fetch_add(1, Ordering::Relaxed);
            }
            TransportType::Quic => {
                self.stats.quic_selections.fetch_add(1, Ordering::Relaxed);
            }
            TransportType::UnixSocket => {
                self.stats.unix_socket_selections.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Get fallback transport if primary fails
    pub fn get_fallback(&self, current: TransportType) -> Option<TransportType> {
        if !self.preferences.enable_fallback {
            return None;
        }

        self.stats.fallback_events.fetch_add(1, Ordering::Relaxed);

        // Fallback chain: IoUring -> BatchUdp -> Udp -> Tcp
        match current {
            TransportType::SharedMemory => Some(TransportType::UnixSocket),
            TransportType::IoUring => Some(TransportType::BatchUdp),
            TransportType::BatchUdp => Some(TransportType::Udp),
            TransportType::Udp => Some(TransportType::Tcp),
            TransportType::UnixSocket => Some(TransportType::Tcp),
            TransportType::Tcp => {
                if TransportType::Quic.is_available() {
                    Some(TransportType::Quic)
                } else {
                    None
                }
            }
            TransportType::Quic => Some(TransportType::Tcp),
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &TransportSelectorStats {
        &self.stats
    }

    /// Get current preferences
    pub fn preferences(&self) -> &TransportPreferences {
        &self.preferences
    }

    /// Update preferences
    pub fn set_preferences(&mut self, preferences: TransportPreferences) {
        self.preferences = preferences;
    }
}

impl Default for TransportSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating transports
pub struct TransportBuilder {
    selector: TransportSelector,
}

impl TransportBuilder {
    /// Create a new transport builder
    pub fn new() -> Self {
        Self {
            selector: TransportSelector::new(),
        }
    }

    /// Set transport preferences
    pub fn preferences(mut self, prefs: TransportPreferences) -> Self {
        self.selector.set_preferences(prefs);
        self
    }

    /// Force a specific transport
    pub fn force_transport(mut self, transport: TransportType) -> Self {
        self.selector.preferences.force_transport = Some(transport);
        self
    }

    /// Prefer low latency
    pub fn low_latency(mut self) -> Self {
        self.selector.preferences.prefer_low_latency = true;
        self.selector.preferences.prefer_reliability = false;
        self
    }

    /// Prefer reliability
    pub fn reliable(mut self) -> Self {
        self.selector.preferences.prefer_reliability = true;
        self.selector.preferences.prefer_low_latency = false;
        self
    }

    /// Require encryption
    pub fn encrypted(mut self) -> Self {
        self.selector.preferences.allow_unencrypted = false;
        self
    }

    /// Build the selector
    pub fn build(self) -> TransportSelector {
        self.selector
    }

    /// Select transport for a target and build appropriate backend
    pub fn select_for(&self, target: &SocketAddr) -> TransportType {
        self.selector.select(target)
    }
}

impl Default for TransportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddrV4};

    #[test]
    fn test_network_location_localhost() {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080));
        assert_eq!(NetworkLocation::from_addr(&addr), NetworkLocation::SameMachine);
    }

    #[test]
    fn test_network_location_private() {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 100), 8080));
        assert_eq!(NetworkLocation::from_addr(&addr), NetworkLocation::LocalNetwork);

        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 8080));
        assert_eq!(NetworkLocation::from_addr(&addr), NetworkLocation::LocalNetwork);

        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 16, 0, 1), 8080));
        assert_eq!(NetworkLocation::from_addr(&addr), NetworkLocation::LocalNetwork);
    }

    #[test]
    fn test_network_location_public() {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 8080));
        assert_eq!(NetworkLocation::from_addr(&addr), NetworkLocation::WideArea);
    }

    #[test]
    fn test_transport_selection_localhost() {
        let selector = TransportSelector::new();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080));

        let transport = selector.select(&addr);
        // Should prefer shared memory or unix socket for localhost
        assert!(matches!(
            transport,
            TransportType::SharedMemory | TransportType::UnixSocket | TransportType::IoUring | TransportType::BatchUdp
        ));
    }

    #[test]
    fn test_transport_selection_lan() {
        let selector = TransportSelector::new();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 100), 8080));

        let transport = selector.select(&addr);
        // Should prefer fast transports for LAN
        assert!(matches!(
            transport,
            TransportType::IoUring | TransportType::BatchUdp | TransportType::Udp
        ));
    }

    #[test]
    fn test_transport_selection_wan() {
        let selector = TransportSelector::new();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 8080));

        let transport = selector.select(&addr);
        // Should prefer reliable transport for WAN
        assert!(matches!(
            transport,
            TransportType::Quic | TransportType::Tcp | TransportType::Udp
        ));
    }

    #[test]
    fn test_forced_transport() {
        let selector = TransportSelector::with_preferences(TransportPreferences {
            force_transport: Some(TransportType::Tcp),
            ..Default::default()
        });

        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080));
        let transport = selector.select(&addr);

        assert_eq!(transport, TransportType::Tcp);
    }

    #[test]
    fn test_fallback() {
        let selector = TransportSelector::new();

        assert_eq!(
            selector.get_fallback(TransportType::IoUring),
            Some(TransportType::BatchUdp)
        );
        assert_eq!(
            selector.get_fallback(TransportType::BatchUdp),
            Some(TransportType::Udp)
        );
        assert_eq!(
            selector.get_fallback(TransportType::Udp),
            Some(TransportType::Tcp)
        );
    }

    #[test]
    fn test_transport_availability() {
        // These should always be available
        assert!(TransportType::Udp.is_available());
        assert!(TransportType::Tcp.is_available());
        assert!(TransportType::SharedMemory.is_available());

        // Platform-specific
        #[cfg(unix)]
        assert!(TransportType::UnixSocket.is_available());

        #[cfg(target_os = "linux")]
        assert!(TransportType::BatchUdp.is_available());
    }

    #[test]
    fn test_builder() {
        let selector = TransportBuilder::new()
            .low_latency()
            .build();

        assert!(selector.preferences().prefer_low_latency);
        assert!(!selector.preferences().prefer_reliability);
    }

    #[test]
    fn test_expected_latency() {
        let (min, max) = TransportType::SharedMemory.expected_latency_us();
        assert!(min < max);
        assert!(min == 0); // Sub-microsecond

        let (min, max) = TransportType::IoUring.expected_latency_us();
        assert!(min >= 3 && max <= 10);
    }
}
