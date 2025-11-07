use crate::core::node::NodeInfo;
use crate::error::HorusResult;
use crate::memory::shm_topic::ShmTopic;
use std::sync::Arc;
use std::time::Instant;

/// Connection state for Hub connections
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Lock-free atomic metrics for Hub monitoring with cache optimization
#[derive(Debug)]
#[repr(align(64))] // Cache-line aligned to prevent false sharing
pub struct AtomicHubMetrics {
    pub messages_sent: std::sync::atomic::AtomicU64,
    pub messages_received: std::sync::atomic::AtomicU64,
    pub send_failures: std::sync::atomic::AtomicU64,
    pub recv_failures: std::sync::atomic::AtomicU64,
    _padding: [u8; 32], // Pad to cache line boundary
}

impl Default for AtomicHubMetrics {
    fn default() -> Self {
        Self {
            messages_sent: std::sync::atomic::AtomicU64::new(0),
            messages_received: std::sync::atomic::AtomicU64::new(0),
            send_failures: std::sync::atomic::AtomicU64::new(0),
            recv_failures: std::sync::atomic::AtomicU64::new(0),
            _padding: [0; 32],
        }
    }
}

impl AtomicHubMetrics {
    /// Get current metrics snapshot (for monitoring/debugging)
    pub fn snapshot(&self) -> HubMetrics {
        HubMetrics {
            messages_sent: self
                .messages_sent
                .load(std::sync::atomic::Ordering::Relaxed),
            messages_received: self
                .messages_received
                .load(std::sync::atomic::Ordering::Relaxed),
            send_failures: self
                .send_failures
                .load(std::sync::atomic::Ordering::Relaxed),
            recv_failures: self
                .recv_failures
                .load(std::sync::atomic::Ordering::Relaxed),
            last_activity: None, // Eliminated to remove Instant::now() overhead
        }
    }
}

/// Simple metrics for Hub monitoring (for backwards compatibility)
#[derive(Debug, Clone, Default)]
pub struct HubMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub send_failures: u64,
    pub recv_failures: u64,
    pub last_activity: Option<Instant>,
}

/// Optimized Hub for pub/sub messaging with cache-aligned lock-free hot paths
#[repr(align(64))] // Cache-line aligned structure
pub struct Hub<T> {
    shm_topic: Arc<ShmTopic<T>>,
    topic_name: String,
    state: std::sync::atomic::AtomicU8, // Lock-free state using atomic u8
    metrics: Arc<AtomicHubMetrics>,     // Lock-free atomic metrics
    _padding: [u8; 15],                 // Pad to prevent false sharing
}

// Manual Clone implementation since AtomicU8 doesn't implement Clone
impl<T> Clone for Hub<T> {
    fn clone(&self) -> Self {
        Self {
            shm_topic: self.shm_topic.clone(),
            topic_name: self.topic_name.clone(),
            state: std::sync::atomic::AtomicU8::new(
                self.state.load(std::sync::atomic::Ordering::Relaxed),
            ),
            metrics: self.metrics.clone(),
            _padding: [0; 15],
        }
    }
}

// Manual Debug implementation to avoid ShmTopic Debug requirement
impl<T> std::fmt::Debug for Hub<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hub")
            .field("topic_name", &self.topic_name)
            .field(
                "state",
                &self.state.load(std::sync::atomic::Ordering::Relaxed),
            )
            .finish_non_exhaustive()
    }
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

impl<T: Send + Sync + 'static + Clone + std::fmt::Debug> Hub<T> {
    /// Create a new Hub
    pub fn new(topic_name: &str) -> HorusResult<Self> {
        Self::new_with_capacity(topic_name, 1024)
    }

    /// Create a new Hub with custom capacity
    pub fn new_with_capacity(topic_name: &str, capacity: usize) -> HorusResult<Self> {
        let shm_topic = Arc::new(ShmTopic::new(topic_name, capacity)?);

        Ok(Hub {
            shm_topic,
            topic_name: topic_name.to_string(),
            state: std::sync::atomic::AtomicU8::new(ConnectionState::Connected.into_u8()),
            metrics: Arc::new(AtomicHubMetrics::default()),
            _padding: [0; 15],
        })
    }

    /// High-performance send using zero-copy loan pattern internally
    /// This method now uses the loan() backend for optimal performance (~200ns latency)
    /// The API remains simple while delivering the best possible performance
    #[inline(always)]
    pub fn send(&self, msg: T, ctx: Option<&mut NodeInfo>) -> Result<(), T>
    where
        T: crate::core::LogSummary,
    {
        match self.shm_topic.loan() {
            Ok(mut sample) => {
                // Fast path: when ctx is None (benchmarks), bypass logging completely
                if let Some(ctx) = ctx {
                    // Logging enabled: get lightweight summary BEFORE moving msg
                    let summary = msg.log_summary();
                    let ipc_start = Instant::now();

                    sample.write(msg);
                    drop(sample);

                    self.metrics
                        .messages_sent
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    self.state.store(
                        ConnectionState::Connected.into_u8(),
                        std::sync::atomic::Ordering::Relaxed,
                    );

                    // Record pub/sub metadata for graph visualization
                    self.record_pubsub_activity(ctx.name(), "pub");

                    let ipc_ns = ipc_start.elapsed().as_nanos() as u64;
                    ctx.log_pub_summary(&self.topic_name, &summary, ipc_ns);
                } else {
                    // No logging: zero overhead path for benchmarks
                    sample.write(msg);
                    drop(sample);

                    self.metrics
                        .messages_sent
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    self.state.store(
                        ConnectionState::Connected.into_u8(),
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }

                Ok(())
            }
            Err(_) => {
                self.metrics
                    .send_failures
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.state.store(
                    ConnectionState::Failed.into_u8(),
                    std::sync::atomic::Ordering::Relaxed,
                );
                Err(msg)
            }
        }
    }
    /// Receive a message from the topic
    #[inline(always)]
    pub fn recv(&self, ctx: Option<&mut NodeInfo>) -> Option<T>
    where
        T: crate::core::LogSummary,
    {
        match self.shm_topic.pop() {
            Some(msg) => {
                // Fast path: when ctx is None, bypass logging completely (benchmarks + production)
                if let Some(ctx) = ctx {
                    // Logging enabled: get summary and measure IPC timing
                    let summary = msg.log_summary();
                    let ipc_start = Instant::now();
                    let ipc_ns = ipc_start.elapsed().as_nanos() as u64;
                    ctx.log_sub_summary(&self.topic_name, &summary, ipc_ns);

                    // Record pub/sub metadata for graph visualization
                    self.record_pubsub_activity(ctx.name(), "sub");
                }

                // Lock-free atomic increment for success metrics
                self.metrics
                    .messages_received
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                Some(msg)
            }
            None => {
                // Lock-free atomic increment for failure metrics
                self.metrics
                    .recv_failures
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                None
            }
        }
    }
    /// Get current connection state (lock-free)
    pub fn get_connection_state(&self) -> ConnectionState {
        let state_u8 = self.state.load(std::sync::atomic::Ordering::Relaxed);
        ConnectionState::from_u8(state_u8)
    }

    /// Get current metrics snapshot (lock-free)
    pub fn get_metrics(&self) -> HubMetrics {
        self.metrics.snapshot()
    }

    /// Get the topic name for this Hub
    pub fn get_topic_name(&self) -> &str {
        &self.topic_name
    }

    /// Record pub/sub activity for graph visualization discovery
    /// Writes lightweight metadata to /dev/shm/horus/pubsub_metadata/
    fn record_pubsub_activity(&self, node_name: &str, direction: &str) {
        use std::fs;
        use std::path::PathBuf;

        // Create pubsub metadata directory if it doesn't exist
        let metadata_dir = PathBuf::from("/dev/shm/horus/pubsub_metadata");
        let _ = fs::create_dir_all(&metadata_dir);

        // File naming: node_name_topic_name_direction
        // e.g., MyControlNode_sensor_data_sub
        let safe_node_name = node_name.replace('/', "_").replace(' ', "_");
        let safe_topic_name = self.topic_name.replace('/', "_").replace(' ', "_");
        let filename = format!("{}_{}_{}",  safe_node_name, safe_topic_name, direction);
        let filepath = metadata_dir.join(filename);

        // Write minimal metadata (just timestamp to show activity)
        // File existence is enough to know the relationship exists
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Only write once every 5 seconds to reduce I/O
        // Check if file exists and is recent
        if let Ok(metadata) = fs::metadata(&filepath) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    if elapsed.as_secs() < 5 {
                        return; // File is recent, skip write
                    }
                }
            }
        }

        // Write timestamp (lightweight operation)
        let _ = fs::write(&filepath, timestamp.to_string());
    }
}
