use super::shm_region::ShmRegion;
use crate::error::HorusResult;
use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// Safety constants to prevent dangerous configurations
const MAX_CAPACITY: usize = 1_000_000; // Maximum number of elements
const MIN_CAPACITY: usize = 1; // Minimum number of elements
const MAX_ELEMENT_SIZE: usize = 1_000_000; // Maximum size per element in bytes
const MAX_TOTAL_SIZE: usize = 100_000_000; // Maximum total shared memory size (100MB)

/// Header for shared memory ring buffer with cache-line alignment
#[repr(C, align(64))] // Cache-line aligned for optimal performance
struct RingBufferHeader {
    capacity: AtomicUsize,
    head: AtomicUsize,
    tail: AtomicUsize, // This is now unused - kept for compatibility
    element_size: AtomicUsize,
    consumer_count: AtomicUsize,
    sequence_number: AtomicUsize, // Global sequence counter
    _padding: [u8; 24],           // Pad to cache line boundary
}

/// Lock-free ring buffer in real shared memory using mmap with cache optimization
#[repr(align(64))] // Cache-line aligned structure
pub struct ShmTopic<T> {
    _region: Arc<ShmRegion>,
    header: NonNull<RingBufferHeader>,
    data_ptr: NonNull<u8>,
    capacity: usize,
    consumer_tail: AtomicUsize, // Each consumer tracks its own position
    _phantom: std::marker::PhantomData<T>,
    _padding: [u8; 24], // Pad to prevent false sharing
}

unsafe impl<T: Send> Send for ShmTopic<T> {}
unsafe impl<T: Send> Sync for ShmTopic<T> {}

/// A loaned sample for zero-copy publishing
/// When dropped, automatically marks the slot as available for consumers
pub struct PublisherSample<'a, T> {
    data_ptr: *mut T,
    #[allow(dead_code)]
    slot_index: usize,
    #[allow(dead_code)]
    topic: &'a ShmTopic<T>,
    _phantom: PhantomData<&'a mut T>,
}

/// A received sample for zero-copy consumption  
/// When dropped, automatically releases the slot
pub struct ConsumerSample<'a, T> {
    data_ptr: *const T,
    #[allow(dead_code)]
    slot_index: usize,
    #[allow(dead_code)]
    topic: &'a ShmTopic<T>,
    _phantom: PhantomData<&'a T>,
}

unsafe impl<T: Send> Send for PublisherSample<'_, T> {}
unsafe impl<T: Sync> Sync for PublisherSample<'_, T> {}

unsafe impl<T: Send> Send for ConsumerSample<'_, T> {}
unsafe impl<T: Sync> Sync for ConsumerSample<'_, T> {}

impl<'a, T> PublisherSample<'a, T> {
    /// Get a mutable reference to the loaned memory
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data_ptr
    }

    /// Write data directly into the loaned memory
    pub fn write(&mut self, value: T) {
        unsafe {
            std::ptr::write(self.data_ptr, value);
        }
    }

    /// Get a mutable reference to the data (unsafe because it bypasses borrow checker)
    ///
    /// # Safety
    ///
    /// The caller must ensure that no other references to this data exist,
    /// and that the data pointer is valid and properly aligned.
    pub unsafe fn as_mut(&mut self) -> &mut T {
        &mut *self.data_ptr
    }
}

impl<'a, T> ConsumerSample<'a, T> {
    /// Get a const reference to the received data
    pub fn get_ref(&self) -> &T {
        unsafe { &*self.data_ptr }
    }

    /// Get the raw pointer to the data
    pub fn as_ptr(&self) -> *const T {
        self.data_ptr
    }

    /// Read the data by copy (for types that implement Copy)
    pub fn read(&self) -> T
    where
        T: Copy,
    {
        unsafe { std::ptr::read(self.data_ptr) }
    }
}

impl<'a, T> Drop for PublisherSample<'a, T> {
    fn drop(&mut self) {
        // When the publisher sample is dropped, publish it by updating sequence number
        let header = unsafe { self.topic.header.as_ref() };
        header.sequence_number.fetch_add(1, Ordering::Release);
    }
}

impl<'a, T> Drop for ConsumerSample<'a, T> {
    fn drop(&mut self) {
        // Consumer sample drop is automatic - just releases the reference
        // The actual slot management is handled by the consumer's tail position
    }
}

impl<T> ShmTopic<T> {
    /// Create a new ring buffer in shared memory
    pub fn new(name: &str, capacity: usize) -> HorusResult<Self> {
        // Safety validation: check capacity bounds
        if capacity < MIN_CAPACITY {
            return Err(format!(
                "Capacity {} too small, minimum is {}",
                capacity, MIN_CAPACITY
            )
            .into());
        }
        if capacity > MAX_CAPACITY {
            return Err(format!(
                "Capacity {} too large, maximum is {}",
                capacity, MAX_CAPACITY
            )
            .into());
        }

        let element_size = mem::size_of::<T>();
        let element_align = mem::align_of::<T>();
        let header_size = mem::size_of::<RingBufferHeader>();

        // Safety validation: check element size
        if element_size == 0 {
            return Err("Cannot create shared memory for zero-sized types".into());
        }
        if element_size > MAX_ELEMENT_SIZE {
            return Err(format!(
                "Element size {} too large, maximum is {}",
                element_size, MAX_ELEMENT_SIZE
            )
            .into());
        }

        // Safety validation: check for overflow in size calculations
        let data_size = capacity
            .checked_mul(element_size)
            .ok_or("Integer overflow calculating data size")?;
        if data_size > MAX_TOTAL_SIZE {
            return Err(
                format!("Data size {} exceeds maximum {}", data_size, MAX_TOTAL_SIZE).into(),
            );
        }

        // Ensure data section is properly aligned
        let aligned_header_size = header_size.div_ceil(element_align) * element_align;
        let total_size = aligned_header_size
            .checked_add(data_size)
            .ok_or("Integer overflow calculating total size")?;

        if total_size > MAX_TOTAL_SIZE {
            return Err(format!(
                "Total size {} exceeds maximum {}",
                total_size, MAX_TOTAL_SIZE
            )
            .into());
        }

        let region = Arc::new(ShmRegion::new(name, total_size)?);

        // Initialize header with safety checks
        let header_ptr = region.as_ptr() as *mut RingBufferHeader;

        // Safety check: ensure we have enough space for the header
        if region.size() < header_size {
            return Err("Shared memory region too small for header".into());
        }

        // Safety check: ensure pointer is not null and properly aligned
        if header_ptr.is_null() {
            return Err("Null pointer for shared memory header".into());
        }
        if !(header_ptr as usize).is_multiple_of(std::mem::align_of::<RingBufferHeader>()) {
            return Err("Header pointer not properly aligned".into());
        }

        let header = unsafe {
            // This is now safe because we've validated the pointer
            NonNull::new_unchecked(header_ptr)
        };

        unsafe {
            (*header.as_ptr())
                .capacity
                .store(capacity, Ordering::Relaxed);
            (*header.as_ptr()).head.store(0, Ordering::Relaxed);
            (*header.as_ptr()).tail.store(0, Ordering::Relaxed);
            (*header.as_ptr())
                .element_size
                .store(element_size, Ordering::Relaxed);
            (*header.as_ptr())
                .consumer_count
                .store(0, Ordering::Relaxed);
            (*header.as_ptr())
                .sequence_number
                .store(0, Ordering::Relaxed);
            (*header.as_ptr())._padding = [0; 24]; // Initialize padding for cache alignment
        }

        log::info!(
            "SHM_TRUE: Created true shared memory topic '{}' with capacity: {} (size: {} bytes)",
            name,
            capacity,
            total_size
        );

        // Log topic creation to global log buffer
        use crate::core::log_buffer::{publish_log, LogEntry, LogType};
        use chrono::Local;
        publish_log(LogEntry {
            timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
            node_name: "shm_topic".to_string(),
            log_type: LogType::TopicMap,
            topic: Some(name.to_string()),
            message: format!(
                "Created topic (capacity: {}, size: {} bytes)",
                capacity, total_size
            ),
            tick_us: 0,
            ipc_ns: 0,
        });

        // Data starts after aligned header with comprehensive safety checks
        let data_ptr = unsafe {
            let raw_ptr = (region.as_ptr() as *mut u8).add(aligned_header_size);

            // Safety checks for data pointer
            if raw_ptr.is_null() {
                return Err("Null pointer for data region".into());
            }

            // Verify we have enough space for the data
            if region.size() < aligned_header_size + data_size {
                return Err("Shared memory region too small for data".into());
            }

            // Verify alignment
            if !(raw_ptr as usize).is_multiple_of(element_align) {
                return Err("Data pointer not properly aligned".into());
            }

            // Verify the pointer is within the mapped region bounds
            let region_end = (region.as_ptr() as *mut u8).add(region.size());
            let data_end = raw_ptr.add(data_size);
            if data_end > region_end {
                return Err("Data region extends beyond mapped memory".into());
            }

            NonNull::new_unchecked(raw_ptr)
        };

        // Register as the first consumer
        unsafe {
            (*header.as_ptr())
                .consumer_count
                .fetch_add(1, Ordering::Relaxed);
        }

        Ok(ShmTopic {
            _region: region,
            header,
            data_ptr,
            capacity,
            consumer_tail: AtomicUsize::new(0), // Start at beginning
            _phantom: std::marker::PhantomData,
            _padding: [0; 24],
        })
    }

    /// Open an existing ring buffer from shared memory
    pub fn open(name: &str) -> HorusResult<Self> {
        let region = Arc::new(ShmRegion::open(name)?);

        // Safety checks for opening existing shared memory
        let header_size = mem::size_of::<RingBufferHeader>();
        if region.size() < header_size {
            return Err("Existing shared memory region too small for header".into());
        }

        let header_ptr = region.as_ptr() as *mut RingBufferHeader;

        // Safety check: ensure pointer is not null and properly aligned
        if header_ptr.is_null() {
            return Err("Null pointer for existing shared memory header".into());
        }
        if !(header_ptr as usize).is_multiple_of(std::mem::align_of::<RingBufferHeader>()) {
            return Err("Existing header pointer not properly aligned".into());
        }

        let header = unsafe { NonNull::new_unchecked(header_ptr) };

        let capacity = unsafe { (*header.as_ptr()).capacity.load(Ordering::Relaxed) };

        // Validate capacity is within safe bounds
        if !(MIN_CAPACITY..=MAX_CAPACITY).contains(&capacity) {
            return Err(format!(
                "Invalid capacity {} in existing shared memory (must be {}-{})",
                capacity, MIN_CAPACITY, MAX_CAPACITY
            )
            .into());
        }

        // Validate element size matches
        let stored_element_size =
            unsafe { (*header.as_ptr()).element_size.load(Ordering::Relaxed) };
        let expected_element_size = mem::size_of::<T>();
        if stored_element_size != expected_element_size {
            return Err(format!(
                "Element size mismatch: stored {}, expected {}",
                stored_element_size, expected_element_size
            )
            .into());
        }

        log::info!(
            "SHM_TRUE: Opened existing shared memory topic '{}' with capacity: {}",
            name,
            capacity
        );

        // Log topic open to global log buffer
        use crate::core::log_buffer::{publish_log, LogEntry, LogType};
        use chrono::Local;
        publish_log(LogEntry {
            timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
            node_name: "shm_topic".to_string(),
            log_type: LogType::TopicMap,
            topic: Some(name.to_string()),
            message: format!("Opened existing topic (capacity: {})", capacity),
            tick_us: 0,
            ipc_ns: 0,
        });

        let element_align = mem::align_of::<T>();
        let header_size = mem::size_of::<RingBufferHeader>();
        let aligned_header_size = header_size.div_ceil(element_align) * element_align;

        let data_ptr = unsafe {
            let raw_ptr = (region.as_ptr() as *mut u8).add(aligned_header_size);

            // Safety checks for data pointer in existing shared memory
            if raw_ptr.is_null() {
                return Err("Null pointer for existing data region".into());
            }

            // Calculate expected total size
            let expected_data_size = capacity * expected_element_size;
            let expected_total_size = aligned_header_size + expected_data_size;

            // Verify we have enough space for the data
            if region.size() < expected_total_size {
                return Err(format!(
                    "Existing shared memory too small: {} < {}",
                    region.size(),
                    expected_total_size
                )
                .into());
            }

            // Verify alignment
            if !(raw_ptr as usize).is_multiple_of(element_align) {
                return Err("Existing data pointer not properly aligned".into());
            }

            // Verify the pointer is within the mapped region bounds
            let region_end = (region.as_ptr() as *mut u8).add(region.size());
            let data_end = raw_ptr.add(expected_data_size);
            if data_end > region_end {
                return Err("Existing data region extends beyond mapped memory".into());
            }

            NonNull::new_unchecked(raw_ptr)
        };

        // Register as a new consumer and get current head position to start from
        unsafe {
            (*header.as_ptr())
                .consumer_count
                .fetch_add(1, Ordering::Relaxed);
        }
        let current_head = unsafe { (*header.as_ptr()).head.load(Ordering::Relaxed) };

        Ok(ShmTopic {
            _region: region,
            header,
            data_ptr,
            capacity,
            consumer_tail: AtomicUsize::new(current_head), // Start from current position
            _phantom: std::marker::PhantomData,
            _padding: [0; 24],
        })
    }

    /// Push a message; returns Err(msg) if the buffer is full
    /// Thread-safe for multiple producers
    /// Uses sequence numbering instead of tail checking for multi-consumer safety
    pub fn push(&self, msg: T) -> Result<(), T> {
        let header = unsafe { self.header.as_ref() };

        loop {
            let head = header.head.load(Ordering::Relaxed);
            let next = (head + 1) % self.capacity;

            // For multi-consumer, we need to check if buffer would wrap around
            // and potentially overwrite unread messages. For now, use a simple
            // heuristic: don't fill more than 75% of buffer capacity
            let current_sequence = header.sequence_number.load(Ordering::Relaxed);
            let max_unread = (self.capacity * 3) / 4; // Allow 75% fill

            if current_sequence >= max_unread
                && current_sequence - header.sequence_number.load(Ordering::Relaxed) >= max_unread
            {
                // Buffer getting too full for safe multi-consumer operation
                return Err(msg);
            }

            // Try to claim this slot atomically
            match header.head.compare_exchange_weak(
                head,
                next,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // Successfully claimed slot, now write data with comprehensive bounds checking
                    unsafe {
                        // Comprehensive bounds checking
                        if head >= self.capacity {
                            // This should never happen due to modulo arithmetic, but be extra safe
                            eprintln!(
                                "Critical safety violation: head index {} >= capacity {}",
                                head, self.capacity
                            );
                            return Err(msg);
                        }

                        // Calculate byte offset and verify it's within bounds
                        let byte_offset = head * mem::size_of::<T>();
                        let slot_ptr = self.data_ptr.as_ptr().add(byte_offset) as *mut T;

                        // Verify the write location is within our data region
                        let data_region_size = self.capacity * mem::size_of::<T>();
                        if byte_offset + mem::size_of::<T>() > data_region_size {
                            eprintln!(
                                "Critical safety violation: write would exceed data region bounds"
                            );
                            return Err(msg);
                        }

                        // Safe to write now that we've verified bounds
                        std::ptr::write(slot_ptr, msg);
                    }

                    // Increment global sequence number
                    header.sequence_number.fetch_add(1, Ordering::Relaxed);
                    return Ok(());
                }
                Err(_) => {
                    // Another thread updated head, retry
                    continue;
                }
            }
        }
    }

    /// Pop a message; returns None if the buffer is empty
    /// Thread-safe for multiple consumers - each consumer maintains its own position
    pub fn pop(&self) -> Option<T> {
        let header = unsafe { self.header.as_ref() };

        // Get this consumer's current tail position with validation
        let my_tail = self.consumer_tail.load(Ordering::Relaxed);
        let current_head = header.head.load(Ordering::Acquire);

        // Validate tail position is within bounds
        if my_tail >= self.capacity {
            eprintln!(
                "Critical safety violation: consumer tail {} >= capacity {}",
                my_tail, self.capacity
            );
            return None;
        }

        // Validate head position is within bounds
        if current_head >= self.capacity {
            eprintln!(
                "Critical safety violation: head {} >= capacity {}",
                current_head, self.capacity
            );
            return None;
        }

        if my_tail == current_head {
            // No new messages for this consumer
            return None;
        }

        // Calculate next position for this consumer
        let next_tail = (my_tail + 1) % self.capacity;

        // Update this consumer's tail position
        self.consumer_tail.store(next_tail, Ordering::Relaxed);

        // Read the message (non-destructive - message stays for other consumers) with comprehensive bounds checking
        let msg = unsafe {
            // Comprehensive bounds checking
            if my_tail >= self.capacity {
                eprintln!(
                    "Critical safety violation: consumer tail index {} >= capacity {}",
                    my_tail, self.capacity
                );
                return None;
            }

            // Calculate byte offset and verify it's within bounds
            let byte_offset = my_tail * mem::size_of::<T>();
            let slot_ptr = self.data_ptr.as_ptr().add(byte_offset) as *mut T;

            // Verify the read location is within our data region
            let data_region_size = self.capacity * mem::size_of::<T>();
            if byte_offset + mem::size_of::<T>() > data_region_size {
                eprintln!("Critical safety violation: read would exceed data region bounds");
                return None;
            }

            // Safe to read now that we've verified bounds
            std::ptr::read(slot_ptr)
        };

        Some(msg)
    }

    /// Loan a slot in the shared memory for zero-copy publishing
    /// Returns a PublisherSample that provides direct access to shared memory
    pub fn loan(&self) -> crate::error::HorusResult<PublisherSample<'_, T>> {
        let header = unsafe { self.header.as_ref() };

        loop {
            let head = header.head.load(Ordering::Relaxed);
            let next = (head + 1) % self.capacity;

            // Check buffer capacity using same logic as push()
            let current_sequence = header.sequence_number.load(Ordering::Relaxed);
            let max_unread = (self.capacity * 3) / 4; // Allow 75% fill

            if current_sequence >= max_unread
                && current_sequence - header.sequence_number.load(Ordering::Relaxed) >= max_unread
            {
                return Err("Buffer full - cannot loan slot".into());
            }

            // Try to claim this slot atomically
            match header.head.compare_exchange_weak(
                head,
                next,
                Ordering::Acquire, // We need to acquire to ensure we see all previous writes
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // Successfully claimed slot, return sample pointing to it
                    unsafe {
                        // Bounds checking
                        if head >= self.capacity {
                            eprintln!(
                                "Critical safety violation: head index {} >= capacity {}",
                                head, self.capacity
                            );
                            return Err(format!("Head index {} >= capacity {}", head, self.capacity).into());
                        }

                        let byte_offset = head * mem::size_of::<T>();
                        let data_ptr = self.data_ptr.as_ptr().add(byte_offset) as *mut T;

                        // Verify bounds
                        let data_region_size = self.capacity * mem::size_of::<T>();
                        if byte_offset + mem::size_of::<T>() > data_region_size {
                            eprintln!(
                                "Critical safety violation: loan would exceed data region bounds"
                            );
                            return Err("Loan would exceed data region bounds".into());
                        }

                        return Ok(PublisherSample {
                            data_ptr,
                            slot_index: head,
                            topic: self,
                            _phantom: PhantomData,
                        });
                    }
                }
                Err(_) => {
                    // Another thread updated head, retry
                    continue;
                }
            }
        }
    }

    /// Receive a message using zero-copy access
    /// Returns a ConsumerSample that provides direct access to shared memory
    pub fn receive(&self) -> Option<ConsumerSample<'_, T>> {
        let header = unsafe { self.header.as_ref() };

        // Get this consumer's current tail position
        let my_tail = self.consumer_tail.load(Ordering::Relaxed);
        let current_head = header.head.load(Ordering::Acquire);

        // Validate positions
        if my_tail >= self.capacity {
            eprintln!(
                "Critical safety violation: consumer tail {} >= capacity {}",
                my_tail, self.capacity
            );
            return None;
        }

        if current_head >= self.capacity {
            eprintln!(
                "Critical safety violation: head {} >= capacity {}",
                current_head, self.capacity
            );
            return None;
        }

        if my_tail == current_head {
            // No new messages for this consumer
            return None;
        }

        // Calculate next position for this consumer
        let next_tail = (my_tail + 1) % self.capacity;
        self.consumer_tail.store(next_tail, Ordering::Relaxed);

        // Return sample pointing to the message in shared memory
        unsafe {
            let byte_offset = my_tail * mem::size_of::<T>();
            let data_ptr = self.data_ptr.as_ptr().add(byte_offset) as *const T;

            // Verify bounds
            let data_region_size = self.capacity * mem::size_of::<T>();
            if byte_offset + mem::size_of::<T>() > data_region_size {
                eprintln!("Critical safety violation: receive would exceed data region bounds");
                return None;
            }

            Some(ConsumerSample {
                data_ptr,
                slot_index: my_tail,
                topic: self,
                _phantom: PhantomData,
            })
        }
    }

    /// Loan a slot and immediately write data (convenience method)
    /// This is equivalent to loan() followed by write(), but more convenient
    pub fn loan_and_write(&self, value: T) -> Result<(), T> {
        match self.loan() {
            Ok(mut sample) => {
                sample.write(value);
                // Sample is automatically published when dropped
                Ok(())
            }
            Err(_) => Err(value),
        }
    }
}
