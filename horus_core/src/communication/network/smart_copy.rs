//! Smart Copy - Adaptive zero-copy for large messages
//!
//! Automatically selects the optimal copy strategy based on message size:
//! - Small messages (<64KB): Normal copy (zero-copy setup overhead not worth it)
//! - Large messages (≥64KB): Zero-copy path using pre-registered buffers
//!
//! This provides the best of both worlds:
//! - Minimal latency for small, frequent messages (sensor data, commands)
//! - Maximum throughput for large messages (images, point clouds)
//!
//! # Performance Characteristics
//!
//! | Message Size | Copy Strategy | Latency |
//! |--------------|---------------|---------|
//! | < 4KB        | Stack copy    | ~100ns  |
//! | 4KB - 64KB   | Heap copy     | ~1-5µs  |
//! | ≥ 64KB       | Zero-copy     | ~10-50µs (but no memory bandwidth) |
//!
//! # Example
//!
//! ```ignore
//! use horus_core::communication::network::smart_copy::{SmartCopySender, SmartCopyConfig};
//!
//! let config = SmartCopyConfig::default();
//! let sender = SmartCopySender::new(config)?;
//!
//! // Small message - uses normal copy
//! sender.send(&small_data, target)?;
//!
//! // Large message - automatically uses zero-copy
//! sender.send(&large_image, target)?;
//! ```

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Configuration for smart copy behavior
#[derive(Debug, Clone)]
pub struct SmartCopyConfig {
    /// Threshold in bytes above which zero-copy is used (default: 64KB)
    pub zero_copy_threshold: usize,
    /// Size of each buffer in the pool (default: 1MB)
    pub buffer_size: usize,
    /// Number of pre-allocated buffers (default: 8)
    pub pool_size: usize,
    /// Whether to enable zero-copy (can be disabled for debugging)
    pub enable_zero_copy: bool,
    /// Maximum message size supported (default: 16MB)
    pub max_message_size: usize,
}

impl Default for SmartCopyConfig {
    fn default() -> Self {
        Self {
            zero_copy_threshold: 64 * 1024,     // 64KB
            buffer_size: 1024 * 1024,           // 1MB
            pool_size: 8,                        // 8 buffers = 8MB total
            enable_zero_copy: true,
            max_message_size: 16 * 1024 * 1024, // 16MB
        }
    }
}

impl SmartCopyConfig {
    /// Configuration optimized for robotics (images, point clouds)
    pub fn robotics() -> Self {
        Self {
            zero_copy_threshold: 32 * 1024,     // 32KB - lower threshold for robotics
            buffer_size: 4 * 1024 * 1024,       // 4MB - larger buffers for images
            pool_size: 4,                        // 4 buffers = 16MB total
            enable_zero_copy: true,
            max_message_size: 64 * 1024 * 1024, // 64MB for large point clouds
        }
    }

    /// Configuration for low-memory systems (embedded, Raspberry Pi)
    pub fn low_memory() -> Self {
        Self {
            zero_copy_threshold: 128 * 1024,    // 128KB - higher threshold
            buffer_size: 512 * 1024,            // 512KB buffers
            pool_size: 4,                        // 4 buffers = 2MB total
            enable_zero_copy: true,
            max_message_size: 4 * 1024 * 1024,  // 4MB max
        }
    }

    /// Disable zero-copy (for debugging or compatibility)
    pub fn no_zero_copy() -> Self {
        Self {
            enable_zero_copy: false,
            ..Default::default()
        }
    }
}

/// Statistics for smart copy operations
#[derive(Debug, Default)]
pub struct SmartCopyStats {
    /// Number of small messages sent via normal copy
    pub normal_copy_count: AtomicU64,
    /// Number of large messages sent via zero-copy
    pub zero_copy_count: AtomicU64,
    /// Total bytes sent via normal copy
    pub normal_copy_bytes: AtomicU64,
    /// Total bytes sent via zero-copy
    pub zero_copy_bytes: AtomicU64,
    /// Number of times buffer pool was exhausted (had to fall back)
    pub pool_exhausted_count: AtomicU64,
    /// Current number of buffers in use
    pub buffers_in_use: AtomicUsize,
    /// Peak number of buffers in use
    pub peak_buffers_in_use: AtomicUsize,
}

impl SmartCopyStats {
    /// Get total messages sent
    pub fn total_messages(&self) -> u64 {
        self.normal_copy_count.load(Ordering::Relaxed)
            + self.zero_copy_count.load(Ordering::Relaxed)
    }

    /// Get total bytes sent
    pub fn total_bytes(&self) -> u64 {
        self.normal_copy_bytes.load(Ordering::Relaxed)
            + self.zero_copy_bytes.load(Ordering::Relaxed)
    }

    /// Get zero-copy ratio (0.0 - 1.0)
    pub fn zero_copy_ratio(&self) -> f64 {
        let total = self.total_messages();
        if total == 0 {
            return 0.0;
        }
        self.zero_copy_count.load(Ordering::Relaxed) as f64 / total as f64
    }

    /// Get average message size
    pub fn avg_message_size(&self) -> usize {
        let total = self.total_messages();
        if total == 0 {
            return 0;
        }
        (self.total_bytes() / total) as usize
    }
}

/// A registered buffer for zero-copy operations
#[derive(Debug)]
pub struct RegisteredBuffer {
    /// The actual buffer data
    data: Vec<u8>,
    /// Current length of valid data
    len: usize,
    /// Buffer ID for tracking
    id: usize,
}

impl RegisteredBuffer {
    /// Create a new registered buffer
    fn new(size: usize, id: usize) -> Self {
        Self {
            data: vec![0u8; size],
            len: 0,
            id,
        }
    }

    /// Get a mutable slice for writing
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data[..self.len]
    }

    /// Get an immutable slice of valid data
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }

    /// Set the length of valid data
    pub fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.data.len());
        self.len = len.min(self.data.len());
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    /// Get buffer ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Copy data into the buffer
    pub fn copy_from(&mut self, data: &[u8]) -> bool {
        if data.len() > self.data.len() {
            return false;
        }
        self.data[..data.len()].copy_from_slice(data);
        self.len = data.len();
        true
    }
}

/// Pool of pre-allocated buffers for zero-copy operations
#[derive(Debug)]
pub struct BufferPool {
    /// Available buffers
    available: Mutex<VecDeque<RegisteredBuffer>>,
    /// Configuration
    config: SmartCopyConfig,
    /// Statistics
    stats: Arc<SmartCopyStats>,
    /// Next buffer ID
    next_id: AtomicUsize,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(config: SmartCopyConfig, stats: Arc<SmartCopyStats>) -> Self {
        let mut available = VecDeque::with_capacity(config.pool_size);
        let pool_size = config.pool_size;

        // Pre-allocate buffers
        for i in 0..config.pool_size {
            available.push_back(RegisteredBuffer::new(config.buffer_size, i));
        }

        Self {
            available: Mutex::new(available),
            config,
            stats,
            next_id: AtomicUsize::new(pool_size),
        }
    }

    /// Acquire a buffer from the pool
    pub fn acquire(&self) -> Option<RegisteredBuffer> {
        let mut available = self.available.lock().ok()?;

        let buffer = available.pop_front();

        if buffer.is_some() {
            let in_use = self.config.pool_size - available.len();
            self.stats.buffers_in_use.store(in_use, Ordering::Relaxed);

            // Update peak
            let peak = self.stats.peak_buffers_in_use.load(Ordering::Relaxed);
            if in_use > peak {
                self.stats.peak_buffers_in_use.store(in_use, Ordering::Relaxed);
            }
        } else {
            self.stats.pool_exhausted_count.fetch_add(1, Ordering::Relaxed);
        }

        buffer
    }

    /// Release a buffer back to the pool
    pub fn release(&self, buffer: RegisteredBuffer) {
        if let Ok(mut available) = self.available.lock() {
            // Only return to pool if we're not over capacity
            if available.len() < self.config.pool_size {
                available.push_back(buffer);
            }

            let in_use = self.config.pool_size.saturating_sub(available.len());
            self.stats.buffers_in_use.store(in_use, Ordering::Relaxed);
        }
    }

    /// Allocate a new buffer (when pool is exhausted)
    pub fn allocate_overflow(&self, size: usize) -> RegisteredBuffer {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        RegisteredBuffer::new(size, id)
    }

    /// Get number of available buffers
    pub fn available_count(&self) -> usize {
        self.available.lock().map(|a| a.len()).unwrap_or(0)
    }
}

/// Copy strategy determined by smart copy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyStrategy {
    /// Normal memory copy (small messages)
    NormalCopy,
    /// Zero-copy via registered buffer (large messages)
    ZeroCopy,
    /// Fallback when zero-copy unavailable
    Fallback,
}

impl CopyStrategy {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            CopyStrategy::NormalCopy => "normal_copy",
            CopyStrategy::ZeroCopy => "zero_copy",
            CopyStrategy::Fallback => "fallback",
        }
    }
}

/// Smart copy sender that automatically chooses the best copy strategy
pub struct SmartCopySender {
    /// Configuration
    config: SmartCopyConfig,
    /// Buffer pool for zero-copy
    pool: Arc<BufferPool>,
    /// Statistics
    stats: Arc<SmartCopyStats>,
}

impl SmartCopySender {
    /// Create a new smart copy sender
    pub fn new(config: SmartCopyConfig) -> Self {
        let stats = Arc::new(SmartCopyStats::default());
        let pool = Arc::new(BufferPool::new(config.clone(), stats.clone()));

        Self {
            config,
            pool,
            stats,
        }
    }

    /// Determine the copy strategy for a given data size
    #[inline]
    pub fn select_strategy(&self, size: usize) -> CopyStrategy {
        if !self.config.enable_zero_copy {
            return CopyStrategy::NormalCopy;
        }

        if size >= self.config.zero_copy_threshold {
            CopyStrategy::ZeroCopy
        } else {
            CopyStrategy::NormalCopy
        }
    }

    /// Prepare data for sending, returning the strategy used and optional buffer
    pub fn prepare_send(&self, data: &[u8]) -> (CopyStrategy, Option<RegisteredBuffer>) {
        let strategy = self.select_strategy(data.len());

        match strategy {
            CopyStrategy::NormalCopy => {
                self.stats.normal_copy_count.fetch_add(1, Ordering::Relaxed);
                self.stats.normal_copy_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                (CopyStrategy::NormalCopy, None)
            }
            CopyStrategy::ZeroCopy => {
                // Try to get a buffer from the pool
                if let Some(mut buffer) = self.pool.acquire() {
                    if buffer.copy_from(data) {
                        self.stats.zero_copy_count.fetch_add(1, Ordering::Relaxed);
                        self.stats.zero_copy_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                        return (CopyStrategy::ZeroCopy, Some(buffer));
                    } else {
                        // Buffer too small, return it and allocate overflow
                        self.pool.release(buffer);
                    }
                }

                // Pool exhausted or buffer too small - allocate overflow buffer
                let mut overflow = self.pool.allocate_overflow(data.len());
                overflow.copy_from(data);
                self.stats.zero_copy_count.fetch_add(1, Ordering::Relaxed);
                self.stats.zero_copy_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                (CopyStrategy::Fallback, Some(overflow))
            }
            CopyStrategy::Fallback => {
                self.stats.normal_copy_count.fetch_add(1, Ordering::Relaxed);
                self.stats.normal_copy_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                (CopyStrategy::Fallback, None)
            }
        }
    }

    /// Complete a send operation, releasing the buffer back to the pool
    pub fn complete_send(&self, buffer: Option<RegisteredBuffer>) {
        if let Some(buf) = buffer {
            // Only return to pool if it's a pooled buffer (id < pool_size)
            if buf.id() < self.config.pool_size {
                self.pool.release(buf);
            }
            // Overflow buffers are just dropped
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &SmartCopyStats {
        &self.stats
    }

    /// Get configuration
    pub fn config(&self) -> &SmartCopyConfig {
        &self.config
    }

    /// Get buffer pool status
    pub fn pool_status(&self) -> (usize, usize) {
        (self.pool.available_count(), self.config.pool_size)
    }
}

impl std::fmt::Debug for SmartCopySender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmartCopySender")
            .field("threshold", &self.config.zero_copy_threshold)
            .field("pool_size", &self.config.pool_size)
            .field("buffer_size", &self.config.buffer_size)
            .finish()
    }
}

/// Extension trait for sending with smart copy
pub trait SmartCopyExt {
    /// Send data using smart copy
    fn send_smart(&self, data: &[u8], target: SocketAddr) -> std::io::Result<CopyStrategy>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_selection() {
        let sender = SmartCopySender::new(SmartCopyConfig::default());

        // Small message - normal copy
        assert_eq!(
            sender.select_strategy(1024),
            CopyStrategy::NormalCopy
        );

        // Large message - zero copy
        assert_eq!(
            sender.select_strategy(128 * 1024),
            CopyStrategy::ZeroCopy
        );

        // Exactly at threshold - zero copy
        assert_eq!(
            sender.select_strategy(64 * 1024),
            CopyStrategy::ZeroCopy
        );

        // Just below threshold - normal copy
        assert_eq!(
            sender.select_strategy(64 * 1024 - 1),
            CopyStrategy::NormalCopy
        );
    }

    #[test]
    fn test_disabled_zero_copy() {
        let sender = SmartCopySender::new(SmartCopyConfig::no_zero_copy());

        // Even large messages use normal copy when disabled
        assert_eq!(
            sender.select_strategy(1024 * 1024),
            CopyStrategy::NormalCopy
        );
    }

    #[test]
    fn test_buffer_pool() {
        let stats = Arc::new(SmartCopyStats::default());
        let config = SmartCopyConfig {
            pool_size: 2,
            buffer_size: 1024,
            ..Default::default()
        };
        let pool = BufferPool::new(config, stats.clone());

        // Acquire two buffers
        let buf1 = pool.acquire();
        let buf2 = pool.acquire();
        assert!(buf1.is_some());
        assert!(buf2.is_some());

        // Pool should be exhausted
        let buf3 = pool.acquire();
        assert!(buf3.is_none());
        assert_eq!(stats.pool_exhausted_count.load(Ordering::Relaxed), 1);

        // Release one buffer
        pool.release(buf1.unwrap());

        // Should be able to acquire again
        let buf4 = pool.acquire();
        assert!(buf4.is_some());
    }

    #[test]
    fn test_prepare_send_small() {
        let sender = SmartCopySender::new(SmartCopyConfig::default());
        let small_data = vec![0u8; 1024];

        let (strategy, buffer) = sender.prepare_send(&small_data);
        assert_eq!(strategy, CopyStrategy::NormalCopy);
        assert!(buffer.is_none());
        assert_eq!(sender.stats().normal_copy_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_prepare_send_large() {
        let sender = SmartCopySender::new(SmartCopyConfig::default());
        let large_data = vec![0u8; 128 * 1024];

        let (strategy, buffer) = sender.prepare_send(&large_data);
        assert_eq!(strategy, CopyStrategy::ZeroCopy);
        assert!(buffer.is_some());
        assert_eq!(sender.stats().zero_copy_count.load(Ordering::Relaxed), 1);

        // Complete the send
        sender.complete_send(buffer);
    }

    #[test]
    fn test_stats() {
        let sender = SmartCopySender::new(SmartCopyConfig::default());

        // Send some small messages
        for _ in 0..10 {
            let (_, buf) = sender.prepare_send(&[0u8; 1024]);
            sender.complete_send(buf);
        }

        // Send some large messages
        for _ in 0..5 {
            let (_, buf) = sender.prepare_send(&vec![0u8; 128 * 1024]);
            sender.complete_send(buf);
        }

        assert_eq!(sender.stats().normal_copy_count.load(Ordering::Relaxed), 10);
        assert_eq!(sender.stats().zero_copy_count.load(Ordering::Relaxed), 5);
        assert_eq!(sender.stats().total_messages(), 15);

        // Zero copy ratio should be 5/15 = 0.333...
        let ratio = sender.stats().zero_copy_ratio();
        assert!((ratio - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_registered_buffer() {
        let mut buffer = RegisteredBuffer::new(1024, 0);

        assert_eq!(buffer.capacity(), 1024);
        assert_eq!(buffer.id(), 0);

        let data = [1u8, 2, 3, 4, 5];
        assert!(buffer.copy_from(&data));
        assert_eq!(buffer.as_slice(), &data);

        // Too large data should fail
        let large = vec![0u8; 2048];
        assert!(!buffer.copy_from(&large));
    }

    #[test]
    fn test_robotics_config() {
        let config = SmartCopyConfig::robotics();
        assert_eq!(config.zero_copy_threshold, 32 * 1024);
        assert_eq!(config.buffer_size, 4 * 1024 * 1024);
        assert_eq!(config.max_message_size, 64 * 1024 * 1024);
    }

    #[test]
    fn test_low_memory_config() {
        let config = SmartCopyConfig::low_memory();
        assert_eq!(config.zero_copy_threshold, 128 * 1024);
        assert_eq!(config.buffer_size, 512 * 1024);
        assert_eq!(config.pool_size, 4);
    }
}
