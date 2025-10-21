use crate::core::node::NodeInfo;
use crate::error::HorusResult;
use crate::memory::shm_region::ShmRegion;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

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

/// RAII wrapper for zero-copy writing into Link shared memory
/// Automatically publishes when dropped
pub struct LinkSample<'a, T> {
    link: &'a Link<T>,
    slot_ptr: *mut T,
    index: usize,
    published: bool,
}

impl<'a, T> LinkSample<'a, T> {
    /// Write data directly into shared memory slot (zero-copy)
    pub fn write(self, msg: T) -> Self {
        unsafe {
            std::ptr::write(self.slot_ptr, msg);
        }
        self
    }

    /// Get mutable reference to write into (for in-place construction)
    pub fn payload_mut(&mut self) -> &mut MaybeUninit<T> {
        unsafe { &mut *(self.slot_ptr as *mut MaybeUninit<T>) }
    }

    /// Manually publish (usually auto-published on drop)
    pub fn send(mut self) {
        self.publish();
    }

    fn publish(&mut self) {
        if !self.published {
            let header = unsafe { self.link.header.as_ref() };
            let next_head = (self.index + 1) & (self.link.capacity - 1);
            header.head.store(next_head, Ordering::Release);
            self.published = true;
        }
    }
}

impl<'a, T> Drop for LinkSample<'a, T> {
    fn drop(&mut self) {
        self.publish();
    }
}

/// Metrics for Link monitoring
#[derive(Debug, Clone, Default)]
pub struct LinkMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub send_failures: u64,
}

/// Header for Link shared memory ring buffer
#[repr(C, align(64))]
struct LinkHeader {
    head: AtomicUsize, // Producer writes here
    tail: AtomicUsize, // Consumer reads here
    capacity: AtomicUsize,
    element_size: AtomicUsize,
    // Metrics (shared between producer and consumer)
    messages_sent: AtomicUsize,
    messages_received: AtomicUsize,
    send_failures: AtomicUsize,
    _padding: [u8; 8],
}

/// SPSC (Single Producer Single Consumer) direct link with shared memory IPC
/// Provides ultra-low latency point-to-point communication between processes
#[derive(Debug)]
#[repr(align(64))]
pub struct Link<T> {
    shm_region: Arc<ShmRegion>,
    topic_name: String,
    producer_node: String,
    consumer_node: String,
    role: LinkRole,
    capacity: usize,
    header: NonNull<LinkHeader>,
    data_ptr: NonNull<u8>,
    _phantom: PhantomData<T>,
    _padding: [u8; 8],
}

impl<T> Link<T> {
    // ====== PRIMARY API (recommended) ======

    /// Create a Link as a producer (sender)
    ///
    /// The producer can send messages but cannot receive.
    ///
    /// # Example
    /// ```rust,ignore
    /// let output: Link<f32> = Link::producer("sensor_data")?;
    /// output.send(42.0, None)?;
    /// ```
    pub fn producer(topic: &str) -> HorusResult<Self> {
        Self::with_role(topic, LinkRole::Producer, 1024)
    }

    /// Create a Link as a consumer (receiver)
    ///
    /// The consumer can receive messages but cannot send.
    ///
    /// # Example
    /// ```rust,ignore
    /// let input: Link<f32> = Link::consumer("sensor_data")?;
    /// if let Some(value) = input.recv(None) {
    ///     println!("Received: {}", value);
    /// }
    /// ```
    pub fn consumer(topic: &str) -> HorusResult<Self> {
        Self::with_role(topic, LinkRole::Consumer, 1024)
    }

    /// Create a producer with custom capacity
    ///
    /// # Example
    /// ```rust,ignore
    /// let output: Link<f32> = Link::producer_with_capacity("fast_data", 4096)?;
    /// ```
    pub fn producer_with_capacity(
        topic: &str,
        capacity: usize,
    ) -> HorusResult<Self> {
        Self::with_role(topic, LinkRole::Producer, capacity)
    }

    /// Create a consumer with custom capacity
    ///
    /// # Example
    /// ```rust,ignore
    /// let input: Link<f32> = Link::consumer_with_capacity("fast_data", 4096)?;
    /// ```
    pub fn consumer_with_capacity(
        topic: &str,
        capacity: usize,
    ) -> HorusResult<Self> {
        Self::with_role(topic, LinkRole::Consumer, capacity)
    }

    // ====== INTERNAL IMPLEMENTATION ======

    /// Internal method to create Link with explicit role
    fn with_role(
        topic: &str,
        role: LinkRole,
        capacity: usize,
    ) -> HorusResult<Self> {
        let capacity = capacity.next_power_of_two();
        let element_size = mem::size_of::<T>();
        let element_align = mem::align_of::<T>();
        let header_size = mem::size_of::<LinkHeader>();

        if element_size == 0 {
            return Err("Cannot create Link for zero-sized types".into());
        }

        let aligned_header_size = header_size.div_ceil(element_align) * element_align;
        let data_size = capacity * element_size;
        let total_size = aligned_header_size + data_size;

        let link_name = format!("links/{}", topic);
        let shm_region = Arc::new(ShmRegion::new(&link_name, total_size)?);

        // Use role names for logging
        let (producer_node, consumer_node) = match role {
            LinkRole::Producer => ("producer", "consumer"),
            LinkRole::Consumer => ("consumer", "producer"),
        };

        Self::create_link(
            topic,
            producer_node,
            consumer_node,
            role,
            capacity,
            shm_region,
        )
    }

    /// Common link creation logic
    fn create_link(
        topic_name: &str,
        producer_node: &str,
        consumer_node: &str,
        role: LinkRole,
        capacity: usize,
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
            // Initialize header for first time
            unsafe {
                (*header.as_ptr()).head.store(0, Ordering::Relaxed);
                (*header.as_ptr()).tail.store(0, Ordering::Relaxed);
                (*header.as_ptr())
                    .capacity
                    .store(capacity, Ordering::Relaxed);
                (*header.as_ptr())
                    .element_size
                    .store(element_size, Ordering::Relaxed);
                // Initialize metrics
                (*header.as_ptr()).messages_sent.store(0, Ordering::Relaxed);
                (*header.as_ptr())
                    .messages_received
                    .store(0, Ordering::Relaxed);
                (*header.as_ptr()).send_failures.store(0, Ordering::Relaxed);
                (*header.as_ptr())._padding = [0; 8];
            }
        } else {
            // Validate existing header
            let stored_capacity = unsafe { (*header.as_ptr()).capacity.load(Ordering::Relaxed) };
            let stored_element_size =
                unsafe { (*header.as_ptr()).element_size.load(Ordering::Relaxed) };

            if stored_capacity != capacity {
                return Err(format!(
                    "Capacity mismatch: expected {}, got {}",
                    capacity, stored_capacity
                )
                .into());
            }
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

        Ok(Link {
            shm_region,
            topic_name: topic_name.to_string(),
            producer_node: producer_node.to_string(),
            consumer_node: consumer_node.to_string(),
            role,
            capacity,
            header,
            data_ptr,
            _phantom: PhantomData,
            _padding: [0; 8],
        })
    }

    /// Loan a slot in shared memory for zero-copy writing (advanced API)
    /// Returns a LinkSample that auto-publishes on drop
    /// Only works if this Link is a Producer
    pub fn loan(&self) -> Result<LinkSample<'_, T>, &'static str> {
        if self.role != LinkRole::Producer {
            return Err("Cannot loan on Consumer Link");
        }

        let header = unsafe { self.header.as_ref() };
        let head = header.head.load(Ordering::Relaxed);
        let next_head = (head + 1) & (self.capacity - 1);

        if next_head == header.tail.load(Ordering::Acquire) {
            return Err("Buffer full");
        }

        let slot_ptr = unsafe { self.data_ptr.as_ptr().add(head * mem::size_of::<T>()) as *mut T };

        Ok(LinkSample {
            link: self,
            slot_ptr,
            index: head,
            published: false,
        })
    }

    /// Ultra-fast send with inline zero-copy - optimized for minimum latency
    /// Automatically logs if context is provided
    ///
    /// Optimizations applied:
    /// - Inline assembly hints for hot path
    /// - Prefetch for write optimization
    /// - Relaxed atomics where safe (SPSC guarantee)
    #[inline(always)]
    pub fn send(&self, msg: T, ctx: Option<&mut NodeInfo>) -> Result<(), T>
    where
        T: std::fmt::Debug + Clone,
    {
        // Clone message first (application overhead, not IPC)
        let msg_clone = msg.clone();

        // Inline fast path - compiler optimizes this completely
        let header = unsafe { self.header.as_ref() };

        // Load head (producer-owned, can be Relaxed)
        let head = header.head.load(Ordering::Relaxed);
        let next_head = (head + 1) & (self.capacity - 1);

        // Load tail with Acquire to see consumer's updates
        // Likely prediction: not full (optimize for fast path)
        let tail = header.tail.load(Ordering::Acquire);
        if unlikely(next_head == tail) {
            // Buffer full - increment failure counter
            header.send_failures.fetch_add(1, Ordering::Relaxed);
            return Err(msg); // Buffer full
        }

        // Prefetch the write location (helps with cache)
        let slot = unsafe {
            let ptr = self.data_ptr.as_ptr().add(head * mem::size_of::<T>()) as *mut T;

            // Prefetch hint for write (brings cache line to L1)
            #[cfg(target_arch = "x86_64")]
            core::arch::x86_64::_mm_prefetch::<3>(ptr as *const i8);

            ptr
        };

        // Measure ONLY the pure IPC operation (write + publish)
        let ipc_start = Instant::now();

        // Direct write to shared memory (zero-copy)
        unsafe {
            std::ptr::write(slot, msg_clone);
        }

        // Publish with Release ordering for consumer visibility
        // This ensures all writes above are visible before head update
        header.head.store(next_head, Ordering::Release);

        let ipc_ns = ipc_start.elapsed().as_nanos() as u64;

        // Increment successful send counter
        header.messages_sent.fetch_add(1, Ordering::Relaxed);

        // Optional logging with IPC timing (optimized out when ctx is None)
        if let Some(ctx) = ctx {
            ctx.log_pub(&self.topic_name, &msg, ipc_ns);
        }

        Ok(())
    }

    /// Ultra-fast receive with inline - optimized for minimum latency
    /// Automatically logs if context is provided
    ///
    /// Optimizations applied:
    /// - Prefetch for read optimization
    /// - Relaxed atomics where safe (SPSC guarantee)
    /// - Branch prediction hints
    #[inline(always)]
    pub fn recv(&self, ctx: Option<&mut NodeInfo>) -> Option<T>
    where
        T: std::fmt::Debug + Clone,
    {
        // Inline fast path - compiler optimizes this completely
        let header = unsafe { self.header.as_ref() };

        // Load tail (consumer-owned, can be Relaxed)
        let tail = header.tail.load(Ordering::Relaxed);

        // Load head with Acquire to see producer's updates
        let head = header.head.load(Ordering::Acquire);

        // Likely prediction: has data (optimize for fast path)
        if unlikely(tail == head) {
            return None; // Buffer empty
        }

        // Prefetch the read location (helps with cache)
        let slot = unsafe {
            let ptr = self.data_ptr.as_ptr().add(tail * mem::size_of::<T>()) as *mut T;

            // Prefetch hint for read (brings cache line to L1)
            #[cfg(target_arch = "x86_64")]
            core::arch::x86_64::_mm_prefetch::<3>(ptr as *const i8);

            ptr
        };

        // Measure ONLY the pure IPC operation (read + update tail)
        let ipc_start = Instant::now();

        // Direct read from shared memory
        let msg = unsafe { std::ptr::read(slot) };

        // Update tail with Release ordering for producer visibility
        header
            .tail
            .store((tail + 1) & (self.capacity - 1), Ordering::Release);

        let ipc_ns = ipc_start.elapsed().as_nanos() as u64;

        // Increment successful receive counter
        header.messages_received.fetch_add(1, Ordering::Relaxed);

        // Optional logging with IPC timing (optimized out when ctx is None)
        if let Some(ctx) = ctx {
            ctx.log_sub(&self.topic_name, &msg, ipc_ns);
        }

        Some(msg)
    }

    /// Check if link has messages available
    pub fn has_messages(&self) -> bool {
        let header = unsafe { self.header.as_ref() };
        header.tail.load(Ordering::Relaxed) != header.head.load(Ordering::Acquire)
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

    /// Get the buffer capacity (power of 2)
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get performance metrics snapshot (lock-free)
    ///
    /// Returns current counts of messages sent, received, and send failures.
    /// These metrics are shared between producer and consumer in shared memory.
    pub fn get_metrics(&self) -> LinkMetrics {
        let header = unsafe { self.header.as_ref() };
        LinkMetrics {
            messages_sent: header.messages_sent.load(Ordering::Relaxed) as u64,
            messages_received: header.messages_received.load(Ordering::Relaxed) as u64,
            send_failures: header.send_failures.load(Ordering::Relaxed) as u64,
        }
    }
}

impl<T> Clone for Link<T> {
    fn clone(&self) -> Self {
        Self {
            shm_region: self.shm_region.clone(),
            topic_name: self.topic_name.clone(),
            producer_node: self.producer_node.clone(),
            consumer_node: self.consumer_node.clone(),
            role: self.role,
            capacity: self.capacity,
            header: self.header,
            data_ptr: self.data_ptr,
            _phantom: PhantomData,
            _padding: [0; 8],
        }
    }
}

unsafe impl<T: Send> Send for Link<T> {}
unsafe impl<T: Send> Sync for Link<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    fn cleanup_test_links() {
        let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_test_link_ipc");
        let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_test_link_full");
    }

    #[test]
    fn test_link_producer_consumer() {
        // Clean up before test
        let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_test_prod_cons");

        // Create explicit producer and consumer
        let producer = Link::<i32>::producer("test_prod_cons").unwrap();
        let consumer = Link::<i32>::consumer("test_prod_cons").unwrap();

        assert_eq!(producer.role(), LinkRole::Producer);
        assert_eq!(consumer.role(), LinkRole::Consumer);
        assert!(producer.is_producer());
        assert!(consumer.is_consumer());

        // Producer sends
        assert!(producer.send(42, None).is_ok());
        assert!(producer.send(43, None).is_ok());

        // Consumer receives
        assert_eq!(consumer.recv(None), Some(42));
        assert_eq!(consumer.recv(None), Some(43));
        assert_eq!(consumer.recv(None), None);

        // Cleanup
        let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_test_prod_cons");
    }

    #[test]
    fn test_link_with_capacity() {
        // Clean up before test
        let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_test_capacity");

        let producer = Link::<i32>::producer_with_capacity("test_capacity", 2048).unwrap();
        let consumer = Link::<i32>::consumer_with_capacity("test_capacity", 2048).unwrap();

        assert_eq!(producer.capacity(), 2048);
        assert_eq!(consumer.capacity(), 2048);

        // Can send up to capacity-1
        for i in 0..2047 {
            assert!(producer.send(i, None).is_ok(), "Failed to send {}", i);
        }

        // Buffer should be full now
        assert!(producer.send(9999, None).is_err());

        // Consumer can read
        assert_eq!(consumer.recv(None), Some(0));

        // Cleanup
        let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_test_capacity");
    }

    #[test]
    fn test_link_ipc() {
        // Test basic IPC functionality with new API
        // Clean up before test
        let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_test_link_ipc");

        let link_producer = Link::<i32>::producer("test_link_ipc").unwrap();

        // Verify producer role
        assert_eq!(link_producer.role(), LinkRole::Producer);
        assert!(link_producer.send(42, None).is_ok());
        assert!(link_producer.send(43, None).is_ok());

        // Create consumer
        let link_consumer = Link::<i32>::consumer("test_link_ipc").unwrap();
        assert_eq!(link_consumer.role(), LinkRole::Consumer);

        // Consumer should be able to receive
        assert_eq!(link_consumer.recv(None), Some(42));
        assert_eq!(link_consumer.recv(None), Some(43));
        assert_eq!(link_consumer.recv(None), None);

        // Cleanup
        cleanup_test_links();
    }

    #[test]
    fn test_link_metrics() {
        // Test metrics tracking
        let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_test_metrics");

        let producer = Link::<i32>::producer("test_metrics").unwrap();
        let consumer = Link::<i32>::consumer("test_metrics").unwrap();

        // Check initial metrics
        let metrics = producer.get_metrics();
        assert_eq!(metrics.messages_sent, 0);
        assert_eq!(metrics.messages_received, 0);
        assert_eq!(metrics.send_failures, 0);

        // Send 10 messages
        for i in 0..10 {
            producer.send(i, None).unwrap();
        }

        // Check producer metrics
        let metrics = producer.get_metrics();
        assert_eq!(metrics.messages_sent, 10);

        // Receive 5 messages
        for _ in 0..5 {
            consumer.recv(None);
        }

        // Check consumer metrics (both can see shared metrics)
        let metrics = consumer.get_metrics();
        assert_eq!(metrics.messages_sent, 10);
        assert_eq!(metrics.messages_received, 5);
        assert_eq!(metrics.send_failures, 0);

        // Fill buffer to cause send failure
        for i in 0..1023 {
            producer.send(i, None).ok();
        }

        // This should fail (buffer full)
        assert!(producer.send(9999, None).is_err());

        // Check failure counter
        let metrics = producer.get_metrics();
        assert!(
            metrics.send_failures > 0,
            "Should have at least one send failure"
        );

        // Cleanup
        let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_test_metrics");
    }
}
