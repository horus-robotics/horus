/// Message batching for network efficiency
///
/// Automatically batches multiple small messages into single network packets
/// to reduce overhead and improve throughput for high-frequency data.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Default batch size (number of messages)
const DEFAULT_BATCH_SIZE: usize = 100;
/// Default flush timeout
const DEFAULT_FLUSH_TIMEOUT: Duration = Duration::from_millis(5);
/// Maximum batch payload size (64KB to fit in UDP)
const MAX_BATCH_BYTES: usize = 60000;

/// Batching configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum messages per batch
    pub max_messages: usize,
    /// Maximum bytes per batch
    pub max_bytes: usize,
    /// Flush timeout (send incomplete batch after this duration)
    pub flush_timeout: Duration,
    /// Whether batching is enabled
    pub enabled: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_messages: DEFAULT_BATCH_SIZE,
            max_bytes: MAX_BATCH_BYTES,
            flush_timeout: DEFAULT_FLUSH_TIMEOUT,
            enabled: true,
        }
    }
}

impl BatchConfig {
    /// Create config optimized for low latency (smaller batches, shorter timeout)
    pub fn low_latency() -> Self {
        Self {
            max_messages: 10,
            max_bytes: MAX_BATCH_BYTES,
            flush_timeout: Duration::from_micros(500),
            enabled: true,
        }
    }

    /// Create config optimized for throughput (larger batches, longer timeout)
    pub fn high_throughput() -> Self {
        Self {
            max_messages: 500,
            max_bytes: MAX_BATCH_BYTES,
            flush_timeout: Duration::from_millis(20),
            enabled: true,
        }
    }

    /// Disable batching (immediate send)
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }
}

/// A batch of serialized messages
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageBatch {
    /// Number of messages in batch
    pub count: u32,
    /// Topic name
    pub topic: String,
    /// Serialized messages (length-prefixed)
    pub payloads: Vec<Vec<u8>>,
    /// Batch sequence number
    pub sequence: u64,
    /// Timestamp when batch was created
    pub created_at_us: u64,
}

impl MessageBatch {
    /// Create a new empty batch
    pub fn new(topic: &str, sequence: u64) -> Self {
        Self {
            count: 0,
            topic: topic.to_string(),
            payloads: Vec::new(),
            sequence,
            created_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        }
    }

    /// Add a serialized message to the batch
    pub fn add(&mut self, payload: Vec<u8>) {
        self.count += 1;
        self.payloads.push(payload);
    }

    /// Get total byte size of batch
    pub fn byte_size(&self) -> usize {
        self.payloads.iter().map(|p| p.len() + 4).sum::<usize>() // +4 for length prefix
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Encode batch to bytes
    pub fn encode(&self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(self)
    }

    /// Decode batch from bytes
    pub fn decode(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(data)
    }
}

/// Message batcher that accumulates messages and flushes as batches
pub struct MessageBatcher {
    config: BatchConfig,
    current_batch: MessageBatch,
    batch_start_time: Instant,
    sequence: u64,
    topic: String,
    /// Accumulated byte size
    current_bytes: usize,
}

impl MessageBatcher {
    /// Create a new batcher for a topic
    pub fn new(topic: &str, config: BatchConfig) -> Self {
        Self {
            current_batch: MessageBatch::new(topic, 0),
            batch_start_time: Instant::now(),
            sequence: 0,
            topic: topic.to_string(),
            current_bytes: 0,
            config,
        }
    }

    /// Add a message to the batch
    /// Returns Some(batch) if batch should be flushed
    pub fn add(&mut self, payload: Vec<u8>) -> Option<MessageBatch> {
        if !self.config.enabled {
            // Batching disabled - return immediately as single-message batch
            let mut batch = MessageBatch::new(&self.topic, self.sequence);
            self.sequence += 1;
            batch.add(payload);
            return Some(batch);
        }

        let payload_size = payload.len();

        // Check if this message would overflow the batch
        let would_overflow = self.current_batch.count as usize >= self.config.max_messages
            || self.current_bytes + payload_size + 4 > self.config.max_bytes;

        let result = if would_overflow && !self.current_batch.is_empty() {
            // Flush current batch and start new one
            let batch = std::mem::replace(
                &mut self.current_batch,
                MessageBatch::new(&self.topic, self.sequence + 1),
            );
            self.sequence += 1;
            self.batch_start_time = Instant::now();
            self.current_bytes = 0;
            Some(batch)
        } else {
            None
        };

        // Add message to current batch
        self.current_bytes += payload_size + 4;
        self.current_batch.add(payload);

        result
    }

    /// Check if batch should be flushed due to timeout
    /// Returns Some(batch) if timeout exceeded
    pub fn check_timeout(&mut self) -> Option<MessageBatch> {
        if !self.config.enabled || self.current_batch.is_empty() {
            return None;
        }

        if self.batch_start_time.elapsed() >= self.config.flush_timeout {
            self.flush()
        } else {
            None
        }
    }

    /// Force flush the current batch
    pub fn flush(&mut self) -> Option<MessageBatch> {
        if self.current_batch.is_empty() {
            return None;
        }

        let batch = std::mem::replace(
            &mut self.current_batch,
            MessageBatch::new(&self.topic, self.sequence + 1),
        );
        self.sequence += 1;
        self.batch_start_time = Instant::now();
        self.current_bytes = 0;
        Some(batch)
    }

    /// Get current batch size
    pub fn pending_count(&self) -> usize {
        self.current_batch.count as usize
    }

    /// Get current batch byte size
    pub fn pending_bytes(&self) -> usize {
        self.current_bytes
    }
}

/// Thread-safe batcher wrapper
pub struct SharedBatcher {
    inner: Arc<Mutex<MessageBatcher>>,
}

impl SharedBatcher {
    pub fn new(topic: &str, config: BatchConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MessageBatcher::new(topic, config))),
        }
    }

    pub fn add(&self, payload: Vec<u8>) -> Option<MessageBatch> {
        self.inner.lock().unwrap().add(payload)
    }

    pub fn check_timeout(&self) -> Option<MessageBatch> {
        self.inner.lock().unwrap().check_timeout()
    }

    pub fn flush(&self) -> Option<MessageBatch> {
        self.inner.lock().unwrap().flush()
    }
}

impl Clone for SharedBatcher {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Batch receiver/decoder
pub struct BatchReceiver {
    /// Buffer for incoming messages from decoded batches
    pending_messages: VecDeque<Vec<u8>>,
}

impl BatchReceiver {
    pub fn new() -> Self {
        Self {
            pending_messages: VecDeque::new(),
        }
    }

    /// Process a received batch and queue individual messages
    pub fn receive_batch(&mut self, batch: MessageBatch) {
        for payload in batch.payloads {
            self.pending_messages.push_back(payload);
        }
    }

    /// Get the next message from the queue
    pub fn next_message(&mut self) -> Option<Vec<u8>> {
        self.pending_messages.pop_front()
    }

    /// Check if there are pending messages
    pub fn has_pending(&self) -> bool {
        !self.pending_messages.is_empty()
    }

    /// Get count of pending messages
    pub fn pending_count(&self) -> usize {
        self.pending_messages.len()
    }
}

impl Default for BatchReceiver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_add_and_flush() {
        let mut batcher = MessageBatcher::new("test", BatchConfig::default());

        // Add messages
        for i in 0..50 {
            let payload = vec![i as u8; 100];
            let result = batcher.add(payload);
            assert!(result.is_none()); // Not full yet
        }

        assert_eq!(batcher.pending_count(), 50);

        // Force flush
        let batch = batcher.flush().unwrap();
        assert_eq!(batch.count, 50);
        assert_eq!(batch.payloads.len(), 50);
    }

    #[test]
    fn test_batch_auto_flush_on_count() {
        let config = BatchConfig {
            max_messages: 10,
            ..Default::default()
        };
        let mut batcher = MessageBatcher::new("test", config);

        // Add 10 messages - should not trigger flush yet
        for i in 0..10 {
            let result = batcher.add(vec![i as u8]);
            assert!(result.is_none());
        }

        // 11th message should trigger flush of previous 10
        let result = batcher.add(vec![10]);
        assert!(result.is_some());
        let batch = result.unwrap();
        assert_eq!(batch.count, 10);
    }

    #[test]
    fn test_batch_auto_flush_on_size() {
        let config = BatchConfig {
            max_messages: 1000,
            max_bytes: 1000,
            ..Default::default()
        };
        let mut batcher = MessageBatcher::new("test", config);

        // Add messages until we exceed byte limit
        for i in 0..5 {
            let payload = vec![i as u8; 200]; // 200 bytes each
            let result = batcher.add(payload);
            if i < 4 {
                assert!(result.is_none());
            }
        }

        // 5th message (1000+ bytes) should trigger flush
        let result = batcher.add(vec![5; 200]);
        assert!(result.is_some());
    }

    #[test]
    fn test_batch_disabled() {
        let config = BatchConfig::disabled();
        let mut batcher = MessageBatcher::new("test", config);

        // Each message should return immediately
        let result = batcher.add(vec![1, 2, 3]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().count, 1);

        let result = batcher.add(vec![4, 5, 6]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().count, 1);
    }

    #[test]
    fn test_batch_receiver() {
        let mut batcher = MessageBatcher::new("test", BatchConfig::default());
        let mut receiver = BatchReceiver::new();

        // Create a batch
        for i in 0..5 {
            batcher.add(vec![i]);
        }
        let batch = batcher.flush().unwrap();

        // Receive the batch
        receiver.receive_batch(batch);
        assert_eq!(receiver.pending_count(), 5);

        // Get individual messages
        for i in 0..5 {
            let msg = receiver.next_message().unwrap();
            assert_eq!(msg, vec![i]);
        }

        assert!(!receiver.has_pending());
    }

    #[test]
    fn test_batch_encode_decode() {
        let mut batch = MessageBatch::new("test_topic", 42);
        batch.add(vec![1, 2, 3]);
        batch.add(vec![4, 5, 6, 7]);

        let encoded = batch.encode().unwrap();
        let decoded = MessageBatch::decode(&encoded).unwrap();

        assert_eq!(decoded.count, 2);
        assert_eq!(decoded.topic, "test_topic");
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.payloads.len(), 2);
    }
}
