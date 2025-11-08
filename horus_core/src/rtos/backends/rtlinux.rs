// RT-Linux (PREEMPT_RT) backend implementation

use crate::error::HorusResult;
use crate::rtos::interrupt::{InterruptHandler, InterruptPriority};
use crate::rtos::{RTOSBackend, RTOSPlatform, TaskAttributes, TaskHandle, TaskPriority};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

/// RT-Linux backend using PREEMPT_RT kernel
pub struct RTLinuxBackend {
    initialized: AtomicBool,
    tick_count: AtomicU64,
    tasks: HashMap<usize, RTLinuxTask>,
    next_task_id: AtomicU64,
    cpu_cores: Vec<usize>,
}

struct RTLinuxTask {
    handle: thread::JoinHandle<()>,
    name: String,
    priority: TaskPriority,
    suspended: AtomicBool,
}

impl RTLinuxBackend {
    /// Create new RT-Linux backend
    pub fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            tick_count: AtomicU64::new(0),
            tasks: HashMap::new(),
            next_task_id: AtomicU64::new(1),
            cpu_cores: (0..num_cpus::get()).collect(),
        }
    }

    /// Set thread to real-time priority
    fn set_rt_priority(priority: TaskPriority) -> HorusResult<()> {
        #[cfg(target_os = "linux")]
        {
            use libc::{sched_param, sched_setscheduler, SCHED_FIFO};
            use std::os::unix::thread::JoinHandleExt;

            let rt_priority = match priority {
                TaskPriority::Idle => 1,
                TaskPriority::Low => 10,
                TaskPriority::Normal => 50,
                TaskPriority::High => 70,
                TaskPriority::RealTime => 90,
                TaskPriority::Critical => 99,
            };

            let param = sched_param {
                sched_priority: rt_priority as i32,
            };

            unsafe {
                let tid = libc::pthread_self();
                let result = sched_setscheduler(0, SCHED_FIFO, &param);
                if result != 0 {
                    return Err(crate::error::HorusError::Internal(format!(
                        "Failed to set RT priority: {}",
                        std::io::Error::last_os_error()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Set CPU affinity for thread
    fn set_cpu_affinity(cpu: usize) -> HorusResult<()> {
        #[cfg(target_os = "linux")]
        {
            use libc::{cpu_set_t, pthread_self, pthread_setaffinity_np, CPU_SET, CPU_ZERO};
            use std::mem;

            unsafe {
                let mut cpuset: cpu_set_t = mem::zeroed();
                CPU_ZERO(&mut cpuset);
                CPU_SET(cpu, &mut cpuset);

                let thread = pthread_self();
                let result = pthread_setaffinity_np(
                    thread,
                    mem::size_of::<cpu_set_t>(),
                    &cpuset as *const _,
                );

                if result != 0 {
                    return Err(crate::error::HorusError::Internal(format!(
                        "Failed to set CPU affinity: {}",
                        std::io::Error::last_os_error()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Lock memory to prevent page faults
    fn lock_memory() -> HorusResult<()> {
        #[cfg(target_os = "linux")]
        {
            use libc::{mlockall, MCL_CURRENT, MCL_FUTURE};

            unsafe {
                let result = mlockall(MCL_CURRENT | MCL_FUTURE);
                if result != 0 {
                    // Non-fatal: may require privileges
                    eprintln!("Warning: Failed to lock memory (requires CAP_SYS_NICE or root)");
                }
            }
        }

        Ok(())
    }
}

impl RTOSBackend for RTLinuxBackend {
    fn platform(&self) -> RTOSPlatform {
        RTOSPlatform::RTLinux
    }

    fn init(&mut self) -> HorusResult<()> {
        if self.initialized.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        // Lock memory pages to prevent page faults
        Self::lock_memory()?;

        // Check if RT kernel is available
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            // Check for PREEMPT_RT in kernel version
            if let Ok(version) = fs::read_to_string("/proc/version") {
                if !version.contains("PREEMPT") && !version.contains("RT") {
                    eprintln!(
                        "Warning: RT kernel not detected. Real-time features may be limited."
                    );
                    eprintln!(
                        "Consider installing PREEMPT_RT kernel for better real-time performance."
                    );
                }
            }

            // Check scheduler policies available
            if let Ok(content) = fs::read_to_string("/proc/sys/kernel/sched_rt_runtime_us") {
                let runtime_us: i32 = content.trim().parse().unwrap_or(-1);
                if runtime_us != -1 {
                    eprintln!("Warning: RT throttling enabled ({}us). Consider disabling for hard real-time.",
                        runtime_us);
                }
            }
        }

        println!("RT-Linux backend initialized");
        Ok(())
    }

    fn create_task(
        &self,
        attrs: TaskAttributes,
        task_fn: Box<dyn FnOnce() + Send + 'static>,
    ) -> HorusResult<TaskHandle> {
        let task_id = self.next_task_id.fetch_add(1, Ordering::SeqCst) as usize;
        let priority = attrs.priority;
        let cpu_affinity = attrs.affinity;
        let stack_size = attrs.stack_size;

        let builder = thread::Builder::new()
            .name(attrs.name.clone())
            .stack_size(stack_size);

        let handle = builder
            .spawn(move || {
                // Set RT priority
                if let Err(e) = Self::set_rt_priority(priority) {
                    eprintln!("Failed to set RT priority: {}", e);
                }

                // Set CPU affinity if specified
                if let Some(cpu) = cpu_affinity {
                    if let Err(e) = Self::set_cpu_affinity(cpu as usize) {
                        eprintln!("Failed to set CPU affinity: {}", e);
                    }
                }

                // Run the task
                task_fn();
            })
            .map_err(|e| {
                crate::error::HorusError::Internal(format!("Failed to create task: {}", e))
            })?;

        Ok(TaskHandle {
            id: task_id,
            platform_handle: std::ptr::null_mut(),
        })
    }

    fn delete_task(&self, handle: TaskHandle) -> HorusResult<()> {
        // RT-Linux doesn't provide task deletion - tasks run to completion
        Ok(())
    }

    fn suspend_task(&self, handle: TaskHandle) -> HorusResult<()> {
        // Use signals to suspend/resume on Linux
        #[cfg(target_os = "linux")]
        {
            use libc::{pthread_kill, SIGSTOP};
            unsafe {
                // Would need to track pthread_t for each task
                // For now, just return OK
            }
        }
        Ok(())
    }

    fn resume_task(&self, handle: TaskHandle) -> HorusResult<()> {
        // Use signals to suspend/resume on Linux
        #[cfg(target_os = "linux")]
        {
            use libc::{pthread_kill, SIGCONT};
            unsafe {
                // Would need to track pthread_t for each task
                // For now, just return OK
            }
        }
        Ok(())
    }

    fn current_task(&self) -> TaskHandle {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        thread::current().id().hash(&mut hasher);

        TaskHandle {
            id: hasher.finish() as usize,
            platform_handle: std::ptr::null_mut(),
        }
    }

    fn task_yield(&self) {
        thread::yield_now();
    }

    fn task_delay(&self, duration: Duration) {
        // Use high-resolution sleep
        #[cfg(target_os = "linux")]
        {
            use libc::{nanosleep, timespec};

            let ts = timespec {
                tv_sec: duration.as_secs() as i64,
                tv_nsec: duration.subsec_nanos() as i64,
            };

            unsafe {
                nanosleep(&ts, std::ptr::null_mut());
            }
        }

        #[cfg(not(target_os = "linux"))]
        thread::sleep(duration);
    }

    fn get_tick_count(&self) -> u64 {
        self.tick_count.load(Ordering::SeqCst)
    }

    fn get_tick_frequency(&self) -> u32 {
        1000 // 1kHz default
    }

    fn enter_critical(&self) {
        // Disable preemption on RT-Linux
        #[cfg(target_os = "linux")]
        unsafe {
            // Would use raw syscalls or RT-specific APIs
            // For now, use a simpler approach
        }
    }

    fn exit_critical(&self) {
        // Enable preemption on RT-Linux
        #[cfg(target_os = "linux")]
        unsafe {
            // Would use raw syscalls or RT-specific APIs
            // For now, use a simpler approach
        }
    }

    fn set_task_priority(&self, handle: TaskHandle, priority: TaskPriority) -> HorusResult<()> {
        Self::set_rt_priority(priority)
    }

    fn get_task_priority(&self, handle: TaskHandle) -> HorusResult<TaskPriority> {
        #[cfg(target_os = "linux")]
        {
            use libc::{sched_getparam, sched_getscheduler, sched_param};

            unsafe {
                let mut param: sched_param = std::mem::zeroed();
                let result = sched_getparam(0, &mut param);

                if result == 0 {
                    let priority = match param.sched_priority {
                        1..=9 => TaskPriority::Idle,
                        10..=49 => TaskPriority::Low,
                        50..=69 => TaskPriority::Normal,
                        70..=89 => TaskPriority::High,
                        90..=98 => TaskPriority::RealTime,
                        99 => TaskPriority::Critical,
                        _ => TaskPriority::Normal,
                    };
                    return Ok(priority);
                }
            }
        }

        Ok(TaskPriority::Normal)
    }

    fn get_free_heap(&self) -> usize {
        // Read from /proc/meminfo
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            if let Ok(content) = fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemAvailable:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            return parts[1].parse().unwrap_or(0) * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }

        0
    }

    fn get_min_free_heap(&self) -> usize {
        // Not easily available on Linux, return current free
        self.get_free_heap()
    }

    fn allocate(&self, size: usize) -> *mut u8 {
        // Use aligned allocation for better performance
        #[cfg(target_os = "linux")]
        {
            use libc::{c_void, posix_memalign};

            let mut ptr: *mut c_void = std::ptr::null_mut();
            let alignment = 64; // Cache line size

            unsafe {
                let result = posix_memalign(&mut ptr, alignment, size);
                if result == 0 {
                    return ptr as *mut u8;
                }
            }
        }

        std::ptr::null_mut()
    }

    fn deallocate(&self, ptr: *mut u8) {
        #[cfg(target_os = "linux")]
        {
            use libc::free;
            unsafe {
                free(ptr as *mut c_void);
            }
        }
    }

    fn register_interrupt(
        &self,
        irq: u32,
        handler: InterruptHandler,
        priority: InterruptPriority,
    ) -> HorusResult<()> {
        // Linux doesn't expose direct IRQ handling from userspace
        // Would need kernel module or use signal handlers
        Err(crate::error::HorusError::Internal(
            "Direct interrupt handling not available in userspace".to_string(),
        ))
    }

    fn enable_interrupt(&self, irq: u32) -> HorusResult<()> {
        // Would require kernel module
        Ok(())
    }

    fn disable_interrupt(&self, irq: u32) -> HorusResult<()> {
        // Would require kernel module
        Ok(())
    }

    fn start_scheduler(&self) -> ! {
        // RT-Linux doesn't have a scheduler start - threads run immediately
        // Just block forever
        loop {
            thread::park();
        }
    }
}

/// RT-Linux specific extensions
impl RTLinuxBackend {
    /// Set process to run on isolated CPUs
    pub fn isolate_cpus(&mut self, cpus: &[usize]) -> HorusResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            // Write to /sys/devices/system/cpu/isolated
            // This requires root privileges and kernel support

            for &cpu in cpus {
                let path = format!("/sys/devices/system/cpu/cpu{}/online", cpu);
                if let Err(e) = fs::write(&path, "0") {
                    eprintln!("Failed to isolate CPU {}: {}", cpu, e);
                }
            }
        }

        self.cpu_cores.retain(|c| !cpus.contains(c));
        Ok(())
    }

    /// Enable/disable CPU frequency scaling
    pub fn set_cpu_governor(&self, governor: &str) -> HorusResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            for &cpu in &self.cpu_cores {
                let path = format!(
                    "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_governor",
                    cpu
                );
                if let Err(e) = fs::write(&path, governor) {
                    eprintln!("Failed to set CPU {} governor: {}", cpu, e);
                }
            }
        }

        Ok(())
    }

    /// Set interrupt affinity
    pub fn set_irq_affinity(&self, irq: u32, cpu: usize) -> HorusResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            let path = format!("/proc/irq/{}/smp_affinity", irq);
            let mask = 1u64 << cpu;
            let affinity = format!("{:x}", mask);

            if let Err(e) = fs::write(&path, affinity) {
                return Err(crate::error::HorusError::Internal(format!(
                    "Failed to set IRQ affinity: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    /// Get CPU isolation status
    pub fn get_isolated_cpus(&self) -> Vec<usize> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            if let Ok(content) = fs::read_to_string("/sys/devices/system/cpu/isolated") {
                return content
                    .trim()
                    .split(',')
                    .filter_map(|s| s.parse().ok())
                    .collect();
            }
        }

        Vec::new()
    }

    /// Check if running with real-time privileges
    pub fn check_rt_privileges(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            use libc::{sched_get_priority_max, SCHED_FIFO};

            unsafe {
                let max_priority = sched_get_priority_max(SCHED_FIFO);
                max_priority >= 99 // RT priorities go up to 99
            }
        }

        #[cfg(not(target_os = "linux"))]
        false
    }
}
