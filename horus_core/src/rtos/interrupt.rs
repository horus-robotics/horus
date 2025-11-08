// Interrupt handling for RTOS integration

use crate::error::HorusResult;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Interrupt priority levels
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterruptPriority {
    Lowest = 255,
    Low = 192,
    Normal = 128,
    High = 64,
    Critical = 16,
    Highest = 0,
}

impl InterruptPriority {
    /// Convert to numeric value (lower = higher priority)
    pub fn value(&self) -> u8 {
        *self as u8
    }
}

/// Interrupt handler function type
pub type InterruptHandler = fn(irq: u32);

/// Interrupt vector table
pub struct InterruptVectorTable {
    handlers: HashMap<u32, InterruptHandler>,
    priorities: HashMap<u32, InterruptPriority>,
    enabled: HashMap<u32, bool>,
    pending: HashMap<u32, AtomicBool>,
    stats: InterruptStatistics,
}

impl InterruptVectorTable {
    /// Create interrupt vector table
    pub fn new(max_interrupts: u32) -> Self {
        Self {
            handlers: HashMap::with_capacity(max_interrupts as usize),
            priorities: HashMap::with_capacity(max_interrupts as usize),
            enabled: HashMap::with_capacity(max_interrupts as usize),
            pending: HashMap::with_capacity(max_interrupts as usize),
            stats: InterruptStatistics::new(),
        }
    }

    /// Register interrupt handler
    pub fn register(
        &mut self,
        irq: u32,
        handler: InterruptHandler,
        priority: InterruptPriority,
    ) -> HorusResult<()> {
        if self.handlers.contains_key(&irq) {
            return Err(crate::error::HorusError::Internal(format!(
                "Interrupt {} already registered",
                irq
            )));
        }

        self.handlers.insert(irq, handler);
        self.priorities.insert(irq, priority);
        self.enabled.insert(irq, false);
        self.pending.insert(irq, AtomicBool::new(false));

        Ok(())
    }

    /// Unregister interrupt handler
    pub fn unregister(&mut self, irq: u32) -> HorusResult<()> {
        if !self.handlers.contains_key(&irq) {
            return Err(crate::error::HorusError::Internal(format!(
                "Interrupt {} not registered",
                irq
            )));
        }

        self.handlers.remove(&irq);
        self.priorities.remove(&irq);
        self.enabled.remove(&irq);
        self.pending.remove(&irq);

        Ok(())
    }

    /// Enable interrupt
    pub fn enable(&mut self, irq: u32) -> HorusResult<()> {
        if let Some(enabled) = self.enabled.get_mut(&irq) {
            *enabled = true;
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(format!(
                "Interrupt {} not registered",
                irq
            )))
        }
    }

    /// Disable interrupt
    pub fn disable(&mut self, irq: u32) -> HorusResult<()> {
        if let Some(enabled) = self.enabled.get_mut(&irq) {
            *enabled = false;
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(format!(
                "Interrupt {} not registered",
                irq
            )))
        }
    }

    /// Check if interrupt is enabled
    pub fn is_enabled(&self, irq: u32) -> bool {
        self.enabled.get(&irq).copied().unwrap_or(false)
    }

    /// Set interrupt pending
    pub fn set_pending(&self, irq: u32) -> HorusResult<()> {
        if let Some(pending) = self.pending.get(&irq) {
            pending.store(true, Ordering::SeqCst);
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(format!(
                "Interrupt {} not registered",
                irq
            )))
        }
    }

    /// Clear interrupt pending
    pub fn clear_pending(&self, irq: u32) -> HorusResult<()> {
        if let Some(pending) = self.pending.get(&irq) {
            pending.store(false, Ordering::SeqCst);
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(format!(
                "Interrupt {} not registered",
                irq
            )))
        }
    }

    /// Check if interrupt is pending
    pub fn is_pending(&self, irq: u32) -> bool {
        self.pending
            .get(&irq)
            .map(|p| p.load(Ordering::SeqCst))
            .unwrap_or(false)
    }

    /// Handle interrupt (called from ISR)
    pub fn handle(&mut self, irq: u32) {
        // Clear pending flag
        if let Some(pending) = self.pending.get(&irq) {
            pending.store(false, Ordering::SeqCst);
        }

        // Check if enabled
        if !self.is_enabled(irq) {
            self.stats.record_spurious(irq);
            return;
        }

        // Call handler
        if let Some(handler) = self.handlers.get(&irq) {
            let start = std::time::Instant::now();
            handler(irq);
            let duration = start.elapsed();

            self.stats.record_interrupt(irq, duration);
        } else {
            self.stats.record_unhandled(irq);
        }
    }

    /// Get interrupt priority
    pub fn get_priority(&self, irq: u32) -> Option<InterruptPriority> {
        self.priorities.get(&irq).copied()
    }

    /// Set interrupt priority
    pub fn set_priority(&mut self, irq: u32, priority: InterruptPriority) -> HorusResult<()> {
        if let Some(p) = self.priorities.get_mut(&irq) {
            *p = priority;
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(format!(
                "Interrupt {} not registered",
                irq
            )))
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &InterruptStatistics {
        &self.stats
    }
}

/// Interrupt statistics
#[derive(Debug)]
pub struct InterruptStatistics {
    total_interrupts: AtomicU64,
    spurious_interrupts: AtomicU64,
    unhandled_interrupts: AtomicU64,
    per_irq_count: HashMap<u32, AtomicU64>,
    per_irq_time_us: HashMap<u32, AtomicU64>,
    max_latency_us: AtomicU64,
    max_latency_irq: AtomicU64,
}

impl InterruptStatistics {
    fn new() -> Self {
        Self {
            total_interrupts: AtomicU64::new(0),
            spurious_interrupts: AtomicU64::new(0),
            unhandled_interrupts: AtomicU64::new(0),
            per_irq_count: HashMap::new(),
            per_irq_time_us: HashMap::new(),
            max_latency_us: AtomicU64::new(0),
            max_latency_irq: AtomicU64::new(0),
        }
    }

    fn record_interrupt(&mut self, irq: u32, duration: std::time::Duration) {
        self.total_interrupts.fetch_add(1, Ordering::SeqCst);

        // Update per-IRQ count
        self.per_irq_count
            .entry(irq)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::SeqCst);

        // Update per-IRQ time
        let us = duration.as_micros() as u64;
        self.per_irq_time_us
            .entry(irq)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(us, Ordering::SeqCst);

        // Update max latency
        let mut max = self.max_latency_us.load(Ordering::SeqCst);
        while us > max {
            match self.max_latency_us.compare_exchange_weak(
                max,
                us,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    self.max_latency_irq.store(irq as u64, Ordering::SeqCst);
                    break;
                }
                Err(m) => max = m,
            }
        }
    }

    fn record_spurious(&self, irq: u32) {
        self.spurious_interrupts.fetch_add(1, Ordering::SeqCst);
    }

    fn record_unhandled(&self, irq: u32) {
        self.unhandled_interrupts.fetch_add(1, Ordering::SeqCst);
    }

    /// Get total interrupt count
    pub fn total_count(&self) -> u64 {
        self.total_interrupts.load(Ordering::SeqCst)
    }

    /// Get spurious interrupt count
    pub fn spurious_count(&self) -> u64 {
        self.spurious_interrupts.load(Ordering::SeqCst)
    }

    /// Get unhandled interrupt count
    pub fn unhandled_count(&self) -> u64 {
        self.unhandled_interrupts.load(Ordering::SeqCst)
    }

    /// Get count for specific IRQ
    pub fn irq_count(&self, irq: u32) -> u64 {
        self.per_irq_count
            .get(&irq)
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(0)
    }

    /// Get average time for specific IRQ
    pub fn irq_avg_time_us(&self, irq: u32) -> f64 {
        let count = self.irq_count(irq);
        if count == 0 {
            return 0.0;
        }

        let total_us = self
            .per_irq_time_us
            .get(&irq)
            .map(|t| t.load(Ordering::SeqCst))
            .unwrap_or(0);

        total_us as f64 / count as f64
    }

    /// Get maximum latency
    pub fn max_latency(&self) -> (u64, u64) {
        (
            self.max_latency_us.load(Ordering::SeqCst),
            self.max_latency_irq.load(Ordering::SeqCst),
        )
    }
}

/// Nested interrupt controller
pub struct NestedInterruptController {
    vector_table: InterruptVectorTable,
    nesting_level: AtomicU64,
    max_nesting: u32,
    active_interrupts: Vec<u32>,
    interrupt_mask: AtomicU64,
}

impl NestedInterruptController {
    /// Create nested interrupt controller
    pub fn new(max_interrupts: u32, max_nesting: u32) -> Self {
        Self {
            vector_table: InterruptVectorTable::new(max_interrupts),
            nesting_level: AtomicU64::new(0),
            max_nesting,
            active_interrupts: Vec::with_capacity(max_nesting as usize),
            interrupt_mask: AtomicU64::new(0),
        }
    }

    /// Handle interrupt with nesting support
    pub fn handle_interrupt(&mut self, irq: u32) -> HorusResult<()> {
        let level = self.nesting_level.fetch_add(1, Ordering::SeqCst);

        if level >= self.max_nesting as u64 {
            self.nesting_level.fetch_sub(1, Ordering::SeqCst);
            return Err(crate::error::HorusError::Internal(format!(
                "Maximum interrupt nesting level {} exceeded",
                self.max_nesting
            )));
        }

        // Get interrupt priority
        let priority = self
            .vector_table
            .get_priority(irq)
            .unwrap_or(InterruptPriority::Normal);

        // Mask lower priority interrupts
        let old_mask = self.mask_lower_priority(priority);

        // Track active interrupt
        self.active_interrupts.push(irq);

        // Handle the interrupt
        self.vector_table.handle(irq);

        // Restore interrupt mask
        self.interrupt_mask.store(old_mask, Ordering::SeqCst);

        // Remove from active list
        self.active_interrupts.pop();

        self.nesting_level.fetch_sub(1, Ordering::SeqCst);

        Ok(())
    }

    /// Mask interrupts with lower priority
    fn mask_lower_priority(&self, priority: InterruptPriority) -> u64 {
        let old_mask = self.interrupt_mask.load(Ordering::SeqCst);
        let new_mask = old_mask | (1u64 << priority.value());
        self.interrupt_mask.store(new_mask, Ordering::SeqCst);
        old_mask
    }

    /// Check if interrupt can preempt current
    pub fn can_preempt(&self, irq: u32) -> bool {
        if self.active_interrupts.is_empty() {
            return true;
        }

        let new_priority = self
            .vector_table
            .get_priority(irq)
            .unwrap_or(InterruptPriority::Normal);

        // Check against currently active interrupt
        if let Some(&current_irq) = self.active_interrupts.last() {
            let current_priority = self
                .vector_table
                .get_priority(current_irq)
                .unwrap_or(InterruptPriority::Normal);

            new_priority < current_priority
        } else {
            true
        }
    }

    /// Get current nesting level
    pub fn nesting_level(&self) -> u64 {
        self.nesting_level.load(Ordering::SeqCst)
    }

    /// Check if in interrupt context
    pub fn in_interrupt(&self) -> bool {
        self.nesting_level() > 0
    }
}

/// Deferred interrupt handler (bottom half)
pub struct DeferredInterruptHandler {
    work_queue: Vec<DeferredWork>,
    enabled: AtomicBool,
}

/// Deferred work item
pub struct DeferredWork {
    pub irq: u32,
    pub handler: fn(u32),
    pub data: *mut std::ffi::c_void,
}

unsafe impl Send for DeferredWork {}

impl DeferredInterruptHandler {
    /// Create deferred handler
    pub fn new() -> Self {
        Self {
            work_queue: Vec::new(),
            enabled: AtomicBool::new(true),
        }
    }

    /// Queue work for deferred execution
    pub fn queue_work(&mut self, work: DeferredWork) {
        if self.enabled.load(Ordering::SeqCst) {
            self.work_queue.push(work);
        }
    }

    /// Process deferred work (called from lower priority context)
    pub fn process_work(&mut self) {
        while let Some(work) = self.work_queue.pop() {
            (work.handler)(work.irq);
        }
    }

    /// Enable deferred processing
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable deferred processing
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Check if work pending
    pub fn has_work(&self) -> bool {
        !self.work_queue.is_empty()
    }

    /// Clear all pending work
    pub fn clear(&mut self) {
        self.work_queue.clear();
    }
}

/// Fast interrupt handler (for critical ISRs)
#[repr(C)]
pub struct FastInterruptContext {
    /// Saved registers (platform specific)
    pub registers: [usize; 32],
    /// Stack pointer
    pub sp: *mut u8,
    /// Program counter
    pub pc: usize,
    /// Status register
    pub status: u32,
}

/// Fast interrupt handler trait
pub trait FastInterruptHandler {
    /// Handle fast interrupt (minimal overhead)
    unsafe fn handle_fast(&mut self, context: &mut FastInterruptContext);

    /// Check if can be handled as fast interrupt
    fn is_fast_capable(&self) -> bool;

    /// Maximum execution time in cycles
    fn max_cycles(&self) -> u32;
}

/// Global interrupt control
pub struct GlobalInterruptControl;

impl GlobalInterruptControl {
    /// Disable all interrupts globally
    #[inline(always)]
    pub fn disable() -> InterruptState {
        // Platform specific implementation
        // Would use assembly to disable interrupts
        InterruptState { was_enabled: true }
    }

    /// Enable all interrupts globally
    #[inline(always)]
    pub fn enable() {
        // Platform specific implementation
        // Would use assembly to enable interrupts
    }

    /// Execute closure with interrupts disabled
    pub fn critical_section<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _state = Self::disable();
        f()
    }

    /// Check if interrupts are enabled
    pub fn are_enabled() -> bool {
        // Platform specific implementation
        true
    }

    /// Get current interrupt priority mask
    pub fn get_priority_mask() -> u8 {
        // Platform specific implementation
        0
    }

    /// Set interrupt priority mask
    pub fn set_priority_mask(mask: u8) {
        // Platform specific implementation
    }
}

/// Interrupt state for restoration
pub struct InterruptState {
    was_enabled: bool,
}

impl Drop for InterruptState {
    fn drop(&mut self) {
        if self.was_enabled {
            GlobalInterruptControl::enable();
        }
    }
}
