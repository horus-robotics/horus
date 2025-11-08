use crate::core::node::NodeInfo;
use crate::error::HorusResult;
use crate::memory::shm_region::ShmRegion;
use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

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

/// Metrics for Link monitoring
#[derive(Debug, Clone, Default)]
pub struct LinkMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub send_failures: u64,
}

/// Lock-free atomic metrics for Link monitoring (stored in local memory)
#[derive(Debug)]
#[repr(align(64))] // Cache-line aligned to prevent false sharing
struct AtomicLinkMetrics {
    messages_sent: std::sync::atomic::AtomicU64,
    messages_received: std::sync::atomic::AtomicU64,
    send_failures: std::sync::atomic::AtomicU64,
    _padding: [u8; 40], // Pad to cache line boundary
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

/// SPSC (Single Producer Single Consumer) direct link with shared memory IPC
/// Single-slot design: always returns the LATEST value, perfect for sensors/control
/// Producer overwrites old data, consumer tracks what it's already read via sequence number
#[derive(Debug)]
#[repr(align(64))]
pub struct Link<T> {
    shm_region: Arc<ShmRegion>,
    topic_name: String,
    producer_node: String,
    consumer_node: String,
    role: LinkRole,
    header: NonNull<LinkHeader>,
    data_ptr: NonNull<u8>,
    last_seen_sequence: AtomicU64, // Consumer tracks what it's read (local memory)
    metrics: Arc<AtomicLinkMetrics>,
    _phantom: PhantomData<T>,
    _padding: [u8; 8],
}

impl<T: crate::core::LogSummary> Link<T> {
    // ====== PRIMARY API (recommended) ======

    /// Create a Link as a producer (sender)
    ///
    /// The producer can send messages but cannot receive.
    /// Single-slot design: always overwrites with latest value.
    ///
    /// # Example
    /// ```rust,ignore
    /// let output: Link<f32> = Link::producer("sensor_data")?;
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
    /// # Example
    /// ```rust,ignore
    /// let input: Link<f32> = Link::consumer("sensor_data")?;
    /// if let Some(value) = input.recv(None) {
    ///     println!("Received: {}", value);
    /// }
    /// ```
    pub fn consumer(topic: &str) -> HorusResult<Self> {
        Self::with_role(topic, LinkRole::Consumer)
    }

    // ====== INTERNAL IMPLEMENTATION ======

    /// Internal method to create Link with explicit role
    fn with_role(topic: &str, role: LinkRole) -> HorusResult<Self> {
        let element_size = mem::size_of::<T>();
        let element_align = mem::align_of::<T>();
        let header_size = mem::size_of::<LinkHeader>();

        if element_size == 0 {
            return Err("Cannot create Link for zero-sized types".into());
        }

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
            _padding: [0; 40],
        });

        Ok(Link {
            shm_region,
            topic_name: topic_name.to_string(),
            producer_node: producer_node.to_string(),
            consumer_node: consumer_node.to_string(),
            role,
            header,
            data_ptr,
            last_seen_sequence: AtomicU64::new(0),
            metrics,
            _phantom: PhantomData,
            _padding: [0; 8],
        })
    }

    /// Ultra-fast send with inline zero-copy - optimized for minimum latency
    /// Single-slot design: always overwrites with latest value
    /// Automatically logs if context is provided
    ///
    /// Optimizations applied:
    /// - Single atomic operation (sequence increment)
    /// - No buffer full checks (always succeeds)
    /// - Relaxed atomics for metrics
    #[inline(always)]
    pub fn send(&self, msg: T, ctx: Option<&mut NodeInfo>) -> Result<(), T>
    where
        T: std::fmt::Debug + Clone,
    {
        let header = unsafe { self.header.as_ref() };

        // Write message to the single slot
        unsafe {
            let slot = self.data_ptr.as_ptr() as *mut T;
            std::ptr::write(slot, msg);
        }

        // Increment sequence with Release to publish (this is the only sync point!)
        header.sequence.fetch_add(1, Ordering::Release);

        // Update local metrics
        self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);

        // Zero-cost logging
        if unlikely(ctx.is_some()) {
            if let Some(ctx) = ctx {
                let slot = unsafe { &*(self.data_ptr.as_ptr() as *const T) };
                ctx.log_pub(&self.topic_name, slot, 0);
            }
        }

        Ok(())
    }

    /// Ultra-fast receive with inline - optimized for minimum latency
    /// Single-slot design: reads latest value if new, returns None if already seen
    /// Automatically logs if context is provided
    ///
    /// Optimizations applied:
    /// - Single atomic load with Acquire (syncs with producer's Release)
    /// - Local sequence tracking (no atomic stores to shared memory)
    /// - Relaxed atomics for metrics
    #[inline(always)]
    pub fn recv(&self, ctx: Option<&mut NodeInfo>) -> Option<T>
    where
        T: std::fmt::Debug + Clone,
    {
        let header = unsafe { self.header.as_ref() };

        // Read sequence with Acquire to synchronize with producer's Release
        let current_seq = header.sequence.load(Ordering::Acquire);
        let last_seen = self.last_seen_sequence.load(Ordering::Relaxed);

        // If we've already seen this sequence, return None (no new data)
        if current_seq <= last_seen {
            return None;
        }

        // Read the message
        let msg = unsafe {
            let slot = self.data_ptr.as_ptr() as *const T;
            std::ptr::read(slot)
        };

        // Update what we've seen (local memory, Relaxed is fine)
        self.last_seen_sequence
            .store(current_seq, Ordering::Relaxed);

        // Update local metrics
        self.metrics
            .messages_received
            .fetch_add(1, Ordering::Relaxed);

        // Zero-cost logging
        if unlikely(ctx.is_some()) {
            if let Some(ctx) = ctx {
                ctx.log_sub(&self.topic_name, &msg, 0);
            }
        }

        Some(msg)
    }

    /// Check if link has messages available (new data since last read)
    pub fn has_messages(&self) -> bool {
        let header = unsafe { self.header.as_ref() };
        let current_seq = header.sequence.load(Ordering::Acquire);
        let last_seen = self.last_seen_sequence.load(Ordering::Relaxed);
        current_seq > last_seen
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

    /// Get performance metrics snapshot (lock-free)
    ///
    /// Returns current counts of messages sent, received, and send failures.
    /// These metrics are stored in local memory for zero-overhead tracking.
    pub fn get_metrics(&self) -> LinkMetrics {
        LinkMetrics {
            messages_sent: self.metrics.messages_sent.load(Ordering::Relaxed),
            messages_received: self.metrics.messages_received.load(Ordering::Relaxed),
            send_failures: self.metrics.send_failures.load(Ordering::Relaxed),
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
            header: self.header,
            data_ptr: self.data_ptr,
            last_seen_sequence: AtomicU64::new(self.last_seen_sequence.load(Ordering::Relaxed)),
            metrics: self.metrics.clone(),
            _phantom: PhantomData,
            _padding: [0; 8],
        }
    }
}

unsafe impl<T: Send> Send for Link<T> {}
unsafe impl<T: Send> Sync for Link<T> {}
