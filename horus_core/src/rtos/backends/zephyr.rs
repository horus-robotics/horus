// Zephyr RTOS backend implementation (stub)

use crate::error::HorusResult;
use crate::rtos::interrupt::{InterruptHandler, InterruptPriority};
use crate::rtos::{RTOSBackend, RTOSPlatform, TaskAttributes, TaskHandle, TaskPriority};
use std::time::Duration;

/// Zephyr RTOS backend
pub struct ZephyrBackend {
    initialized: bool,
}

impl ZephyrBackend {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl RTOSBackend for ZephyrBackend {
    fn platform(&self) -> RTOSPlatform {
        RTOSPlatform::Zephyr
    }

    fn init(&mut self) -> HorusResult<()> {
        self.initialized = true;
        Ok(())
    }

    fn create_task(
        &self,
        attrs: TaskAttributes,
        task_fn: Box<dyn FnOnce() + Send + 'static>,
    ) -> HorusResult<TaskHandle> {
        // Would call k_thread_create
        Ok(TaskHandle {
            id: 0,
            platform_handle: std::ptr::null_mut(),
        })
    }

    fn delete_task(&self, handle: TaskHandle) -> HorusResult<()> {
        // Would call k_thread_abort
        Ok(())
    }

    fn suspend_task(&self, handle: TaskHandle) -> HorusResult<()> {
        // Would call k_thread_suspend
        Ok(())
    }

    fn resume_task(&self, handle: TaskHandle) -> HorusResult<()> {
        // Would call k_thread_resume
        Ok(())
    }

    fn current_task(&self) -> TaskHandle {
        // Would call k_current_get
        TaskHandle {
            id: 0,
            platform_handle: std::ptr::null_mut(),
        }
    }

    fn task_yield(&self) {
        // Would call k_yield
    }

    fn task_delay(&self, duration: Duration) {
        // Would call k_msleep
    }

    fn get_tick_count(&self) -> u64 {
        // Would call k_uptime_get
        0
    }

    fn get_tick_frequency(&self) -> u32 {
        // Would return CONFIG_SYS_CLOCK_TICKS_PER_SEC
        1000
    }

    fn enter_critical(&self) {
        // Would call irq_lock
    }

    fn exit_critical(&self) {
        // Would call irq_unlock
    }

    fn set_task_priority(&self, handle: TaskHandle, priority: TaskPriority) -> HorusResult<()> {
        // Would call k_thread_priority_set
        Ok(())
    }

    fn get_task_priority(&self, handle: TaskHandle) -> HorusResult<TaskPriority> {
        // Would call k_thread_priority_get
        Ok(TaskPriority::Normal)
    }

    fn get_free_heap(&self) -> usize {
        // Would call k_mem_heap_get_stats
        0
    }

    fn get_min_free_heap(&self) -> usize {
        0
    }

    fn allocate(&self, size: usize) -> *mut u8 {
        // Would call k_malloc
        std::ptr::null_mut()
    }

    fn deallocate(&self, ptr: *mut u8) {
        // Would call k_free
    }

    fn register_interrupt(
        &self,
        irq: u32,
        handler: InterruptHandler,
        priority: InterruptPriority,
    ) -> HorusResult<()> {
        // Would call IRQ_CONNECT
        Ok(())
    }

    fn enable_interrupt(&self, irq: u32) -> HorusResult<()> {
        // Would call irq_enable
        Ok(())
    }

    fn disable_interrupt(&self, irq: u32) -> HorusResult<()> {
        // Would call irq_disable
        Ok(())
    }

    fn start_scheduler(&self) -> ! {
        // Zephyr scheduler starts automatically
        loop {
            std::thread::park();
        }
    }
}
