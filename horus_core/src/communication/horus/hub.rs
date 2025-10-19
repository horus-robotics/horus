use crate::core::node::NodeInfo;
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
    pub fn new(topic_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_capacity(topic_name, 1024)
    }

    /// Create a new Hub with custom capacity
    pub fn new_with_capacity(
        topic_name: &str,
        capacity: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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
    pub fn send(&self, msg: T, ctx: Option<&mut NodeInfo>) -> Result<(), T>
    where
        T: std::fmt::Debug + Clone,
    {
        // Clone message first (application overhead, not IPC)
        let msg_clone = msg.clone();

        // Attempt to send message via shared memory using loan (faster!)
        let ipc_ns = match self.shm_topic.loan() {
            Ok(mut sample) => {
                // Measure ONLY the pure IPC operation (write + publish)
                let ipc_start = Instant::now();
                sample.write(msg_clone);
                // Sample automatically publishes when dropped (atomic pointer update)
                drop(sample); // Explicit drop for clarity
                let ipc_time = ipc_start.elapsed().as_nanos() as u64;

                // Lock-free atomic increment for success metrics (OPTIMIZED)
                self.metrics
                    .messages_sent
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                // Lock-free state update (OPTIMIZED)
                self.state.store(
                    ConnectionState::Connected.into_u8(),
                    std::sync::atomic::Ordering::Relaxed,
                );

                ipc_time
            }
            Err(_) => {
                // Lock-free atomic increment for failure metrics (OPTIMIZED)
                self.metrics
                    .send_failures
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                // Lock-free state update (OPTIMIZED)
                self.state.store(
                    ConnectionState::Failed.into_u8(),
                    std::sync::atomic::Ordering::Relaxed,
                );

                0 // Failed IPC
            }
        };

        // Log the publish event with IPC timing
        if let Some(ctx) = ctx {
            ctx.log_pub(&self.topic_name, &msg, ipc_ns);
        }

        // Return result after logging
        if ipc_ns == 0 {
            Err(msg)
        } else {
            Ok(())
        }
    }
    /// Receive a message from the topic
    pub fn recv(&self, ctx: Option<&mut NodeInfo>) -> Option<T>
    where
        T: std::fmt::Debug,
    {
        // Measure ONLY the pure IPC operation (pop from shared memory)
        let ipc_start = Instant::now();
        let result = self.shm_topic.pop();
        let ipc_ns = ipc_start.elapsed().as_nanos() as u64;

        match result {
            Some(msg) => {
                // Log the subscribe event with IPC timing
                if let Some(ctx) = ctx {
                    ctx.log_sub(&self.topic_name, &msg, ipc_ns);
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
}
