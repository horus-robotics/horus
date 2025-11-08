// Memory management for RTOS integration

use crate::error::HorusResult;
use std::alloc::{GlobalAlloc, Layout};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

/// RTOS memory pool for fixed-size allocations
pub struct RTOSMemoryPool {
    base_addr: *mut u8,
    block_size: usize,
    num_blocks: usize,
    free_list: Vec<usize>,
    allocated: AtomicUsize,
}

impl RTOSMemoryPool {
    /// Create memory pool
    pub fn new(base_addr: *mut u8, block_size: usize, num_blocks: usize) -> Self {
        let mut free_list = Vec::with_capacity(num_blocks);
        for i in 0..num_blocks {
            free_list.push(i);
        }

        Self {
            base_addr,
            block_size,
            num_blocks,
            free_list,
            allocated: AtomicUsize::new(0),
        }
    }

    /// Allocate block from pool
    pub fn allocate(&mut self) -> Option<*mut u8> {
        if let Some(block_idx) = self.free_list.pop() {
            let offset = block_idx * self.block_size;
            let ptr = unsafe { self.base_addr.add(offset) };
            self.allocated.fetch_add(1, Ordering::SeqCst);
            Some(ptr)
        } else {
            None
        }
    }

    /// Free block back to pool
    pub fn free(&mut self, ptr: *mut u8) -> HorusResult<()> {
        let offset = ptr as usize - self.base_addr as usize;
        if offset % self.block_size != 0 {
            return Err(crate::error::HorusError::Internal(
                "Invalid pointer for memory pool".to_string(),
            ));
        }

        let block_idx = offset / self.block_size;
        if block_idx >= self.num_blocks {
            return Err(crate::error::HorusError::Internal(
                "Pointer outside memory pool".to_string(),
            ));
        }

        self.free_list.push(block_idx);
        self.allocated.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }

    /// Get number of allocated blocks
    pub fn allocated_count(&self) -> usize {
        self.allocated.load(Ordering::SeqCst)
    }

    /// Get number of free blocks
    pub fn free_count(&self) -> usize {
        self.num_blocks - self.allocated_count()
    }

    /// Get total pool size
    pub fn total_size(&self) -> usize {
        self.block_size * self.num_blocks
    }

    /// Get block size
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Check if pool is full
    pub fn is_full(&self) -> bool {
        self.free_list.is_empty()
    }

    /// Check if pool is empty
    pub fn is_empty(&self) -> bool {
        self.allocated_count() == 0
    }
}

/// Static allocator for embedded systems without heap
pub struct StaticAllocator {
    heap_start: *mut u8,
    heap_end: *mut u8,
    current: AtomicUsize,
    peak_usage: AtomicUsize,
}

impl StaticAllocator {
    /// Create static allocator with fixed memory region
    pub const fn new(heap_start: *mut u8, heap_size: usize) -> Self {
        Self {
            heap_start,
            heap_end: unsafe { heap_start.add(heap_size) },
            current: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
        }
    }

    /// Allocate memory
    pub fn allocate(&self, size: usize, align: usize) -> *mut u8 {
        let current = self.current.load(Ordering::SeqCst);
        let heap_start = self.heap_start as usize;

        // Calculate aligned address
        let addr = heap_start + current;
        let aligned_addr = (addr + align - 1) & !(align - 1);
        let offset = aligned_addr - heap_start;
        let new_current = offset + size;

        // Check if we have enough space
        if (heap_start + new_current) > self.heap_end as usize {
            return ptr::null_mut();
        }

        // Try to update current position atomically
        match self.current.compare_exchange(
            current,
            new_current,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => {
                // Update peak usage
                let mut peak = self.peak_usage.load(Ordering::SeqCst);
                while new_current > peak {
                    match self.peak_usage.compare_exchange_weak(
                        peak,
                        new_current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(p) => peak = p,
                    }
                }

                aligned_addr as *mut u8
            }
            Err(_) => ptr::null_mut(),
        }
    }

    /// Get current heap usage
    pub fn used(&self) -> usize {
        self.current.load(Ordering::SeqCst)
    }

    /// Get peak heap usage
    pub fn peak_usage(&self) -> usize {
        self.peak_usage.load(Ordering::SeqCst)
    }

    /// Get remaining heap space
    pub fn available(&self) -> usize {
        let heap_size = self.heap_end as usize - self.heap_start as usize;
        heap_size - self.used()
    }

    /// Reset allocator (dangerous!)
    pub unsafe fn reset(&self) {
        self.current.store(0, Ordering::SeqCst);
    }
}

unsafe impl GlobalAlloc for StaticAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocate(layout.size(), layout.align())
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Static allocator doesn't support deallocation
        // Memory is only freed when entire allocator is reset
    }
}

/// DMA-safe memory allocator
pub struct DMAAllocator {
    pool: RTOSMemoryPool,
    alignment: usize,
    cached: bool,
}

impl DMAAllocator {
    /// Create DMA allocator with specific alignment
    pub fn new(base_addr: *mut u8, size: usize, alignment: usize) -> Self {
        // Ensure base address is aligned
        let aligned_base =
            ((base_addr as usize + alignment - 1) / alignment * alignment) as *mut u8;
        let adjusted_size = size - (aligned_base as usize - base_addr as usize);
        let block_size = alignment;
        let num_blocks = adjusted_size / block_size;

        Self {
            pool: RTOSMemoryPool::new(aligned_base, block_size, num_blocks),
            alignment,
            cached: false,
        }
    }

    /// Allocate DMA-safe buffer
    pub fn allocate(&mut self, size: usize) -> Option<*mut u8> {
        // Round up to alignment
        let blocks_needed = (size + self.alignment - 1) / self.alignment;

        // For simplicity, allocate contiguous blocks
        // In real implementation, would need better algorithm
        if blocks_needed == 1 {
            self.pool.allocate()
        } else {
            // Would need to allocate multiple contiguous blocks
            None
        }
    }

    /// Free DMA buffer
    pub fn free(&mut self, ptr: *mut u8) -> HorusResult<()> {
        self.pool.free(ptr)
    }

    /// Flush cache for DMA region (if cached)
    pub fn flush_cache(&self, ptr: *mut u8, size: usize) {
        if self.cached {
            // Platform-specific cache flush
            // Would call HAL function
        }
    }

    /// Invalidate cache for DMA region (if cached)
    pub fn invalidate_cache(&self, ptr: *mut u8, size: usize) {
        if self.cached {
            // Platform-specific cache invalidate
            // Would call HAL function
        }
    }
}

/// Memory region for MPU (Memory Protection Unit) configuration
#[derive(Debug, Clone, Copy)]
pub struct MPURegion {
    pub region_number: u8,
    pub base_address: usize,
    pub size: usize,
    pub permissions: MPUPermissions,
    pub attributes: MPUAttributes,
    pub enabled: bool,
}

/// MPU permissions
#[derive(Debug, Clone, Copy)]
pub struct MPUPermissions {
    pub privileged_read: bool,
    pub privileged_write: bool,
    pub privileged_execute: bool,
    pub unprivileged_read: bool,
    pub unprivileged_write: bool,
    pub unprivileged_execute: bool,
}

impl MPUPermissions {
    /// Read-only for all
    pub const fn read_only() -> Self {
        Self {
            privileged_read: true,
            privileged_write: false,
            privileged_execute: false,
            unprivileged_read: true,
            unprivileged_write: false,
            unprivileged_execute: false,
        }
    }

    /// Read-write for privileged, read-only for unprivileged
    pub const fn privileged_write() -> Self {
        Self {
            privileged_read: true,
            privileged_write: true,
            privileged_execute: false,
            unprivileged_read: true,
            unprivileged_write: false,
            unprivileged_execute: false,
        }
    }

    /// Execute only
    pub const fn execute_only() -> Self {
        Self {
            privileged_read: false,
            privileged_write: false,
            privileged_execute: true,
            unprivileged_read: false,
            unprivileged_write: false,
            unprivileged_execute: true,
        }
    }

    /// Full access
    pub const fn full_access() -> Self {
        Self {
            privileged_read: true,
            privileged_write: true,
            privileged_execute: true,
            unprivileged_read: true,
            unprivileged_write: true,
            unprivileged_execute: true,
        }
    }

    /// No access
    pub const fn no_access() -> Self {
        Self {
            privileged_read: false,
            privileged_write: false,
            privileged_execute: false,
            unprivileged_read: false,
            unprivileged_write: false,
            unprivileged_execute: false,
        }
    }
}

/// MPU attributes
#[derive(Debug, Clone, Copy)]
pub struct MPUAttributes {
    pub cacheable: bool,
    pub bufferable: bool,
    pub shareable: bool,
    pub memory_type: MemoryType,
}

/// Memory type for MPU
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    StronglyOrdered,
    Device,
    Normal,
    Reserved,
}

/// MPU manager for memory protection
pub struct MPUManager {
    regions: Vec<MPURegion>,
    max_regions: u8,
}

impl MPUManager {
    /// Create MPU manager
    pub fn new(max_regions: u8) -> Self {
        Self {
            regions: Vec::with_capacity(max_regions as usize),
            max_regions,
        }
    }

    /// Configure MPU region
    pub fn configure_region(&mut self, region: MPURegion) -> HorusResult<()> {
        if region.region_number >= self.max_regions {
            return Err(crate::error::HorusError::Internal(format!(
                "Region number {} exceeds max {}",
                region.region_number, self.max_regions
            )));
        }

        // Verify size is power of 2 and at least 32 bytes
        if region.size < 32 || (region.size & (region.size - 1)) != 0 {
            return Err(crate::error::HorusError::Internal(
                "MPU region size must be power of 2 and at least 32 bytes".to_string(),
            ));
        }

        // Verify base address is aligned to size
        if region.base_address & (region.size - 1) != 0 {
            return Err(crate::error::HorusError::Internal(
                "MPU region base address must be aligned to region size".to_string(),
            ));
        }

        // Add or update region
        if let Some(existing) = self
            .regions
            .iter_mut()
            .find(|r| r.region_number == region.region_number)
        {
            *existing = region;
        } else {
            self.regions.push(region);
        }

        // In real implementation, would configure hardware MPU
        Ok(())
    }

    /// Enable MPU
    pub fn enable(&self) -> HorusResult<()> {
        // Would enable hardware MPU
        Ok(())
    }

    /// Disable MPU
    pub fn disable(&self) -> HorusResult<()> {
        // Would disable hardware MPU
        Ok(())
    }

    /// Check if address is in protected region
    pub fn is_protected(&self, addr: usize) -> bool {
        for region in &self.regions {
            if !region.enabled {
                continue;
            }

            if addr >= region.base_address && addr < (region.base_address + region.size) {
                return true;
            }
        }
        false
    }

    /// Get region for address
    pub fn get_region(&self, addr: usize) -> Option<&MPURegion> {
        for region in &self.regions {
            if !region.enabled {
                continue;
            }

            if addr >= region.base_address && addr < (region.base_address + region.size) {
                return Some(region);
            }
        }
        None
    }
}

/// Stack overflow detection
pub struct StackGuard {
    stack_bottom: *mut u8,
    stack_top: *mut u8,
    guard_size: usize,
    pattern: u32,
}

impl StackGuard {
    /// Create stack guard
    pub fn new(stack_bottom: *mut u8, stack_size: usize) -> Self {
        let guard_size = 32; // 32 bytes guard region
        let pattern = 0xDEADBEEF;

        let mut guard = Self {
            stack_bottom,
            stack_top: unsafe { stack_bottom.add(stack_size) },
            guard_size,
            pattern,
        };

        // Fill guard region with pattern
        guard.fill_guard();
        guard
    }

    /// Fill guard region with pattern
    fn fill_guard(&self) {
        unsafe {
            let guard_words = self.guard_size / 4;
            let guard_ptr = self.stack_bottom as *mut u32;
            for i in 0..guard_words {
                *guard_ptr.add(i) = self.pattern;
            }
        }
    }

    /// Check for stack overflow
    pub fn check(&self) -> bool {
        unsafe {
            let guard_words = self.guard_size / 4;
            let guard_ptr = self.stack_bottom as *mut u32;
            for i in 0..guard_words {
                if *guard_ptr.add(i) != self.pattern {
                    return false; // Stack overflow detected
                }
            }
        }
        true
    }

    /// Get stack usage
    pub fn get_usage(&self) -> usize {
        let stack_size = self.stack_top as usize - self.stack_bottom as usize;

        // Scan from top to find first used byte
        unsafe {
            let mut ptr = self.stack_top;
            while ptr > self.stack_bottom {
                ptr = ptr.sub(4);
                if *(ptr as *const u32) != 0 {
                    break;
                }
            }
            stack_size - (ptr as usize - self.stack_bottom as usize)
        }
    }

    /// Get stack high water mark
    pub fn get_high_water_mark(&self) -> usize {
        self.get_usage()
    }
}
