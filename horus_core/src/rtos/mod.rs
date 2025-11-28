// RTOS Integration Layer for HORUS
//
// This module provides abstraction for running HORUS on various
// Real-Time Operating Systems (RTOS) including:
// - FreeRTOS
// - Zephyr
// - RT-Linux (PREEMPT_RT)
// - QNX Neutrino
// - VxWorks
// - NuttX
// - ThreadX

use crate::error::HorusResult;
use std::ffi::c_void;
use std::time::Duration;

pub mod backends;
pub mod hal;
pub mod interrupt;
pub mod memory;
pub mod sync;

pub use backends::RTLinuxBackend;
pub use hal::{HardwareAbstractionLayer, HardwareTimer, PlatformInfo};
pub use interrupt::{InterruptHandler, InterruptPriority};
pub use memory::{RTOSMemoryPool, StaticAllocator};
pub use sync::{RTOSCondVar, RTOSMutex, RTOSSemaphore};

/// RTOS platform identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RTOSPlatform {
    FreeRTOS,
    Zephyr,
    RTLinux,
    QNX,
    VxWorks,
    NuttX,
    ThreadX,
    Bare, // Bare metal, no RTOS
}

impl RTOSPlatform {
    /// Detect the current RTOS platform at compile time
    pub fn detect() -> Self {
        #[cfg(feature = "freertos")]
        {
            return RTOSPlatform::FreeRTOS;
        }

        #[cfg(all(feature = "zephyr", not(feature = "freertos")))]
        {
            return RTOSPlatform::Zephyr;
        }

        #[cfg(all(
            feature = "rt-linux",
            not(any(feature = "freertos", feature = "zephyr"))
        ))]
        {
            return RTOSPlatform::RTLinux;
        }

        #[cfg(all(
            feature = "qnx",
            not(any(feature = "freertos", feature = "zephyr", feature = "rt-linux"))
        ))]
        {
            return RTOSPlatform::QNX;
        }

        #[cfg(all(
            feature = "bare-metal",
            not(any(
                feature = "freertos",
                feature = "zephyr",
                feature = "rt-linux",
                feature = "qnx"
            ))
        ))]
        {
            return RTOSPlatform::Bare;
        }

        // Default to RT-Linux if no specific RTOS is selected
        #[cfg(not(any(
            feature = "freertos",
            feature = "zephyr",
            feature = "rt-linux",
            feature = "qnx",
            feature = "bare-metal"
        )))]
        RTOSPlatform::RTLinux
    }

    pub fn name(&self) -> &'static str {
        match self {
            RTOSPlatform::FreeRTOS => "FreeRTOS",
            RTOSPlatform::Zephyr => "Zephyr",
            RTOSPlatform::RTLinux => "RT-Linux",
            RTOSPlatform::QNX => "QNX Neutrino",
            RTOSPlatform::VxWorks => "VxWorks",
            RTOSPlatform::NuttX => "NuttX",
            RTOSPlatform::ThreadX => "ThreadX",
            RTOSPlatform::Bare => "Bare Metal",
        }
    }
}

/// Task priority levels for RTOS scheduling
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Idle = 0,
    Low = 10,
    Normal = 50,
    High = 70,
    RealTime = 90,
    Critical = 99,
}

/// Task handle for RTOS tasks
#[derive(Debug, Clone, Copy)]
pub struct TaskHandle {
    pub id: usize,
    pub platform_handle: *mut c_void,
}

unsafe impl Send for TaskHandle {}
unsafe impl Sync for TaskHandle {}

/// Task attributes for creating RTOS tasks
#[derive(Debug, Clone)]
pub struct TaskAttributes {
    pub name: String,
    pub priority: TaskPriority,
    pub stack_size: usize,
    pub affinity: Option<u32>, // CPU core affinity
    pub floating_point: bool,  // Enable FPU context save
}

impl Default for TaskAttributes {
    fn default() -> Self {
        Self {
            name: "horus_task".to_string(),
            priority: TaskPriority::Normal,
            stack_size: 4096, // 4KB default
            affinity: None,
            floating_point: false,
        }
    }
}

/// Core RTOS abstraction trait
pub trait RTOSBackend: Send + Sync {
    /// Get platform identifier
    fn platform(&self) -> RTOSPlatform;

    /// Initialize the RTOS backend
    fn init(&mut self) -> HorusResult<()>;

    /// Create a new task
    fn create_task(
        &self,
        attrs: TaskAttributes,
        task_fn: Box<dyn FnOnce() + Send + 'static>,
    ) -> HorusResult<TaskHandle>;

    /// Delete a task
    fn delete_task(&self, handle: TaskHandle) -> HorusResult<()>;

    /// Suspend a task
    fn suspend_task(&self, handle: TaskHandle) -> HorusResult<()>;

    /// Resume a suspended task
    fn resume_task(&self, handle: TaskHandle) -> HorusResult<()>;

    /// Get current task handle
    fn current_task(&self) -> TaskHandle;

    /// Yield CPU to other tasks
    fn task_yield(&self);

    /// Delay for specified duration
    fn task_delay(&self, duration: Duration);

    /// Get system tick count
    fn get_tick_count(&self) -> u64;

    /// Get tick frequency in Hz
    fn get_tick_frequency(&self) -> u32;

    /// Enter critical section (disable interrupts)
    fn enter_critical(&self);

    /// Exit critical section (enable interrupts)
    fn exit_critical(&self);

    /// Set task priority
    fn set_task_priority(&self, handle: TaskHandle, priority: TaskPriority) -> HorusResult<()>;

    /// Get task priority
    fn get_task_priority(&self, handle: TaskHandle) -> HorusResult<TaskPriority>;

    /// Get available heap memory
    fn get_free_heap(&self) -> usize;

    /// Get minimum ever free heap (high water mark)
    fn get_min_free_heap(&self) -> usize;

    /// Allocate memory from RTOS heap
    fn allocate(&self, size: usize) -> *mut u8;

    /// Free memory back to RTOS heap
    fn deallocate(&self, ptr: *mut u8);

    /// Register interrupt handler
    fn register_interrupt(
        &self,
        irq: u32,
        handler: InterruptHandler,
        priority: InterruptPriority,
    ) -> HorusResult<()>;

    /// Enable interrupt
    fn enable_interrupt(&self, irq: u32) -> HorusResult<()>;

    /// Disable interrupt
    fn disable_interrupt(&self, irq: u32) -> HorusResult<()>;

    /// Start RTOS scheduler
    fn start_scheduler(&self) -> !;
}

/// Timer abstraction for RTOS
pub trait RTOSTimer {
    /// Create a one-shot timer
    fn create_oneshot(
        &self,
        name: &str,
        period: Duration,
        callback: fn(*mut c_void),
        context: *mut c_void,
    ) -> HorusResult<TimerHandle>;

    /// Create a periodic timer
    fn create_periodic(
        &self,
        name: &str,
        period: Duration,
        callback: fn(*mut c_void),
        context: *mut c_void,
    ) -> HorusResult<TimerHandle>;

    /// Start a timer
    fn start(&self, handle: TimerHandle) -> HorusResult<()>;

    /// Stop a timer
    fn stop(&self, handle: TimerHandle) -> HorusResult<()>;

    /// Change timer period
    fn change_period(&self, handle: TimerHandle, period: Duration) -> HorusResult<()>;

    /// Delete a timer
    fn delete(&self, handle: TimerHandle) -> HorusResult<()>;

    /// Check if timer is active
    fn is_active(&self, handle: TimerHandle) -> bool;
}

/// Timer handle
#[derive(Debug, Clone, Copy)]
pub struct TimerHandle {
    pub id: usize,
    pub platform_handle: *mut c_void,
}

unsafe impl Send for TimerHandle {}
unsafe impl Sync for TimerHandle {}

/// Queue abstraction for RTOS message passing
pub trait RTOSQueue<T: Send> {
    /// Create a queue with specified depth
    fn create(name: &str, depth: usize) -> HorusResult<QueueHandle<T>>;

    /// Send item to back of queue
    fn send(&self, item: T, timeout: Option<Duration>) -> HorusResult<()>;

    /// Send item to front of queue (priority)
    fn send_front(&self, item: T, timeout: Option<Duration>) -> HorusResult<()>;

    /// Receive item from queue
    fn receive(&self, timeout: Option<Duration>) -> HorusResult<T>;

    /// Peek at front item without removing
    fn peek(&self) -> Option<T>;

    /// Get number of items in queue
    fn len(&self) -> usize;

    /// Check if queue is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if queue is full
    fn is_full(&self) -> bool;

    /// Reset queue (clear all items)
    fn reset(&self) -> HorusResult<()>;

    /// Delete queue
    fn delete(self) -> HorusResult<()>;
}

/// Queue handle
#[derive(Debug)]
pub struct QueueHandle<T> {
    pub id: usize,
    pub platform_handle: *mut c_void,
    _phantom: std::marker::PhantomData<T>,
}

unsafe impl<T: Send> Send for QueueHandle<T> {}
unsafe impl<T: Send> Sync for QueueHandle<T> {}

/// Event group abstraction for RTOS synchronization
pub trait RTOSEventGroup {
    /// Create event group
    fn create(name: &str) -> HorusResult<EventGroupHandle>;

    /// Set event bits
    fn set_bits(&self, bits: u32) -> HorusResult<()>;

    /// Clear event bits
    fn clear_bits(&self, bits: u32) -> HorusResult<()>;

    /// Wait for all specified bits
    fn wait_all(&self, bits: u32, timeout: Option<Duration>) -> HorusResult<u32>;

    /// Wait for any specified bits
    fn wait_any(&self, bits: u32, timeout: Option<Duration>) -> HorusResult<u32>;

    /// Get current event bits
    fn get_bits(&self) -> u32;

    /// Delete event group
    fn delete(self) -> HorusResult<()>;
}

/// Event group handle
#[derive(Debug, Clone, Copy)]
pub struct EventGroupHandle {
    pub id: usize,
    pub platform_handle: *mut c_void,
}

unsafe impl Send for EventGroupHandle {}
unsafe impl Sync for EventGroupHandle {}

/// Global RTOS interface instance
static mut RTOS_INSTANCE: Option<Box<dyn RTOSBackend>> = None;

/// Initialize RTOS backend
pub fn init_rtos(backend: Box<dyn RTOSBackend>) -> HorusResult<()> {
    unsafe {
        if RTOS_INSTANCE.is_some() {
            return Err(crate::error::HorusError::Internal(
                "RTOS already initialized".to_string(),
            ));
        }
        RTOS_INSTANCE = Some(backend);
        if let Some(ref mut rtos) = RTOS_INSTANCE {
            rtos.init()?;
        }
    }
    Ok(())
}

/// Get reference to RTOS backend
pub fn rtos() -> Option<&'static dyn RTOSBackend> {
    unsafe { RTOS_INSTANCE.as_ref().map(|b| &**b) }
}

/// Get mutable reference to RTOS backend
pub fn rtos_mut() -> Option<&'static mut dyn RTOSBackend> {
    unsafe { RTOS_INSTANCE.as_mut().map(|b| &mut **b) }
}

/// Check if running on RTOS
pub fn is_rtos() -> bool {
    rtos().is_some()
}

/// Get current RTOS platform
pub fn current_platform() -> Option<RTOSPlatform> {
    rtos().map(|r| r.platform())
}

/// RTOS task entry point for HORUS nodes
pub struct RTOSNodeTask {
    node: Box<dyn crate::core::Node>,
    context: crate::core::NodeInfo,
    period: Duration,
}

impl RTOSNodeTask {
    /// Create new RTOS task for a node
    pub fn new(node: Box<dyn crate::core::Node>, period: Duration) -> Self {
        let node_name = node.name().to_string();
        let context = crate::core::NodeInfo::new(node_name, false);

        Self {
            node,
            context,
            period,
        }
    }

    /// Run the task (called by RTOS)
    pub fn run(mut self) {
        // Initialize node
        if let Err(e) = self.node.init(&mut self.context) {
            eprintln!("Node {} init failed: {}", self.node.name(), e);
            return;
        }

        // Main task loop
        loop {
            let tick_start = std::time::Instant::now();

            // Execute tick
            self.node.tick(Some(&mut self.context));

            // Calculate remaining time in period
            let elapsed = tick_start.elapsed();
            if elapsed < self.period {
                let sleep_time = self.period - elapsed;
                if let Some(rtos) = rtos() {
                    rtos.task_delay(sleep_time);
                } else {
                    std::thread::sleep(sleep_time);
                }
            }
        }
    }
}

/// RTOS-aware scheduler for HORUS
pub struct RTOSScheduler {
    platform: RTOSPlatform,
    tasks: Vec<TaskHandle>,
    backend: Box<dyn RTOSBackend>,
}

impl RTOSScheduler {
    /// Create scheduler for specific RTOS platform
    ///
    /// Currently only RT-Linux is supported. Other platforms (FreeRTOS, Zephyr, QNX)
    /// are planned for future releases.
    pub fn new(platform: RTOSPlatform) -> HorusResult<Self> {
        let backend: Box<dyn RTOSBackend> = match platform {
            RTOSPlatform::RTLinux => Box::new(backends::RTLinuxBackend::new()),
            _ => {
                return Err(crate::error::HorusError::Internal(format!(
                    "Unsupported RTOS platform: {:?}. Currently only RTLinux is supported.",
                    platform
                )));
            }
        };

        Ok(Self {
            platform,
            tasks: Vec::new(),
            backend,
        })
    }

    /// Add a node as an RTOS task
    pub fn add_node_task(
        &mut self,
        node: Box<dyn crate::core::Node>,
        priority: TaskPriority,
        period: Duration,
        stack_size: usize,
    ) -> HorusResult<()> {
        let node_name = node.name().to_string();

        let attrs = TaskAttributes {
            name: node_name.clone(),
            priority,
            stack_size,
            affinity: None,
            floating_point: true, // Most robotics nodes need FPU
        };

        let task = RTOSNodeTask::new(node, period);

        let handle = self.backend.create_task(
            attrs,
            Box::new(move || {
                task.run();
            }),
        )?;

        self.tasks.push(handle);
        println!("Created RTOS task for node: {}", node_name);

        Ok(())
    }

    /// Start the RTOS scheduler (never returns)
    pub fn run(self) -> ! {
        println!(
            "Starting {} scheduler with {} tasks",
            self.platform.name(),
            self.tasks.len()
        );
        self.backend.start_scheduler()
    }
}
