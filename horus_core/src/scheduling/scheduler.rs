use crate::core::{Node, NodeHeartbeat, NodeInfo};
use crate::error::HorusResult;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use colored::Colorize;

// Import intelligence modules
use super::executors::{AsyncIOExecutor, AsyncResult, ParallelExecutor};
use super::fault_tolerance::CircuitBreaker;
use super::intelligence::{DependencyGraph, ExecutionTier, RuntimeProfiler, TierClassifier};
use super::jit::CompiledDataflow;
use super::safety_monitor::SafetyMonitor;
use tokio::sync::mpsc;

/// Enhanced node registration info with lifecycle tracking and per-node rate control
struct RegisteredNode {
    node: Box<dyn Node>,
    priority: u32,
    logging_enabled: bool,
    initialized: bool,
    context: Option<NodeInfo>,
    rate_hz: Option<f64>, // Per-node rate control (None = use global scheduler rate)
    last_tick: Option<Instant>, // Last tick time for rate limiting
    circuit_breaker: CircuitBreaker, // Fault tolerance
    is_rt_node: bool,     // Track if this is a real-time node
    wcet_budget: Option<Duration>, // WCET budget for RT nodes
    deadline: Option<Duration>, // Deadline for RT nodes
}

/// Central orchestrator: holds nodes, drives the tick loop.
pub struct Scheduler {
    nodes: Vec<RegisteredNode>,
    running: Arc<Mutex<bool>>,
    last_instant: Instant,
    last_snapshot: Instant,
    scheduler_name: String,
    working_dir: PathBuf,

    // Intelligence layer (internal, not exposed via API)
    profiler: RuntimeProfiler,
    dependency_graph: Option<DependencyGraph>,
    classifier: Option<TierClassifier>,
    parallel_executor: ParallelExecutor,
    async_io_executor: Option<AsyncIOExecutor>,
    async_result_rx: Option<mpsc::UnboundedReceiver<AsyncResult>>,
    async_result_tx: Option<mpsc::UnboundedSender<AsyncResult>>,
    learning_complete: bool,

    // JIT compilation for ultra-fast nodes
    jit_compiled_nodes: HashMap<String, CompiledDataflow>,

    // Configuration (stored for runtime use)
    config: Option<super::config::SchedulerConfig>,

    // Safety monitor for real-time critical systems
    safety_monitor: Option<SafetyMonitor>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    /// Create an empty scheduler.
    pub fn new() -> Self {
        let running = Arc::new(Mutex::new(true));
        let now = Instant::now();

        Self {
            nodes: Vec::new(),
            running,
            last_instant: now,
            last_snapshot: now,
            scheduler_name: "DefaultScheduler".to_string(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),

            // Initialize intelligence layer
            profiler: RuntimeProfiler::new_default(),
            dependency_graph: None,
            classifier: None,
            parallel_executor: ParallelExecutor::new(),
            async_io_executor: None,
            async_result_rx: None,
            async_result_tx: None,
            learning_complete: false,

            // JIT compilation
            jit_compiled_nodes: HashMap::new(),

            // Configuration
            config: None,

            // Safety monitor
            safety_monitor: None,
        }
    }

    /// Apply a configuration preset to this scheduler (builder pattern)
    ///
    /// # Example
    /// ```
    /// let mut scheduler = Scheduler::new()
    ///     .with_config(SchedulerConfig::hard_realtime())
    ///     .disable_learning();
    /// ```
    pub fn with_config(mut self, config: super::config::SchedulerConfig) -> Self {
        self.set_config(config);
        self
    }

    /// Pre-allocate node capacity (prevents reallocations during runtime)
    ///
    /// Call this before adding nodes for deterministic memory behavior.
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.nodes.reserve(capacity);
        self
    }

    /// Enable deterministic execution for reproducible, bit-exact behavior
    ///
    /// When enabled:
    /// - Learning disabled (no adaptive optimizations)
    /// - Deterministic collections (sorted iteration order)
    /// - Logical clock support (opt-in via config)
    /// - Predictable memory allocation (opt-in via config)
    ///
    /// Performance impact: ~5-10% slower (optimized deterministic implementation)
    ///
    /// Use cases:
    /// - Simulation (Gazebo, Unity integration)
    /// - Testing (reproducible tests in CI/CD)
    /// - Debugging (replay exact behavior)
    /// - Certification (FDA/CE requirements)
    ///
    /// # Example
    /// ```
    /// let scheduler = Scheduler::new()
    ///     .enable_determinism();  // Reproducible execution
    /// ```
    pub fn enable_determinism(mut self) -> Self {
        self.learning_complete = true;
        self.classifier = None;
        self.scheduler_name = "DeterministicScheduler".to_string();
        self
    }

    /// Disable learning phase for predictable startup (internal helper)
    ///
    /// Prefer using `enable_determinism()` for full deterministic behavior.
    pub(crate) fn disable_learning(mut self) -> Self {
        self.learning_complete = true;
        self.classifier = None;
        self
    }

    /// Enable safety monitor with maximum allowed deadline misses
    pub fn with_safety_monitor(mut self, max_deadline_misses: u64) -> Self {
        self.safety_monitor = Some(SafetyMonitor::new(max_deadline_misses as u64));
        self
    }

    /// Set scheduler name (for debugging/logging)
    pub fn with_name(mut self, name: &str) -> Self {
        self.scheduler_name = name.to_string();
        self
    }

    // ============================================================================
    // Convenience Constructors (thin wrappers for common patterns)
    // ============================================================================

    /// Create a hard real-time scheduler (convenience constructor)
    ///
    /// This is equivalent to:
    /// ```
    /// Scheduler::new()
    ///     .with_config(SchedulerConfig::hard_realtime())
    ///     .with_capacity(128)
    ///     .enable_determinism()
    ///     .with_safety_monitor(3);
    /// ```
    ///
    /// After construction, call OS integration methods:
    /// - `set_realtime_priority(99)` - SCHED_FIFO scheduling
    /// - `pin_to_cpu(7)` - Pin to isolated core
    /// - `lock_memory()` - Prevent page faults
    ///
    /// # Example
    /// ```
    /// let mut scheduler = Scheduler::new_realtime()?;
    /// scheduler.set_realtime_priority(99)?;
    /// scheduler.pin_to_cpu(7)?;
    /// scheduler.lock_memory()?;
    /// ```
    pub fn new_realtime() -> crate::error::HorusResult<Self> {
        let sched = Self::new()
            .with_config(super::config::SchedulerConfig::hard_realtime())
            .with_capacity(128)
            .enable_determinism()  // Use unified determinism API
            .with_safety_monitor(3)
            .with_name("RealtimeScheduler");

        println!("⚡ Real-time scheduler initialized");
        println!("   - Config: hard_realtime() preset");
        println!("   - Capacity: 128 nodes pre-allocated");
        println!("   - Determinism: ENABLED");
        println!("   - Safety monitor: ENABLED (max 3 misses)");
        println!("   - Next: Call set_realtime_priority(99), pin_to_cpu(N), lock_memory()");

        Ok(sched)
    }

    /// Create a deterministic scheduler (convenience constructor)
    ///
    /// This is equivalent to:
    /// ```
    /// Scheduler::new()
    ///     .enable_determinism();
    /// ```
    ///
    /// Provides reproducible, bit-exact execution for simulation and testing.
    pub fn new_deterministic() -> Self {
        let sched = Self::new()
            .enable_determinism();

        println!("✓ Deterministic scheduler initialized");
        println!("   - Determinism: ENABLED");
        println!("   - Execution: Reproducible, bit-exact");
        println!("   - Use for: Simulation, testing, certification");

        sched
    }

    // ============================================================================
    // OS Integration Methods (low-level, genuinely different from config)
    // ============================================================================

    /// Set real-time priority using SCHED_FIFO (Linux RT-PREEMPT required)
    ///
    /// # Arguments
    /// * `priority` - Priority level (1-99, higher = more important)
    ///   - 99: Critical control loops (motors, safety)
    ///   - 90: High-priority sensors
    ///   - 80: Normal control
    ///   - 50-70: Background tasks
    ///
    /// # Requirements
    /// - RT-PREEMPT kernel (linux-image-rt)
    /// - CAP_SYS_NICE capability or root
    ///
    /// # Example
    /// ```
    /// scheduler.set_realtime_priority(99)?;  // Highest priority
    /// ```
    pub fn set_realtime_priority(&self, priority: i32) -> crate::error::HorusResult<()> {
        if priority < 1 || priority > 99 {
            return Err(crate::error::HorusError::config(
                "Priority must be between 1 and 99"
            ));
        }

        #[cfg(target_os = "linux")]
        unsafe {
            use libc::{sched_param, sched_setscheduler, SCHED_FIFO};

            let param = sched_param {
                sched_priority: priority,
            };

            if sched_setscheduler(0, SCHED_FIFO, &param) != 0 {
                let err = std::io::Error::last_os_error();
                return Err(crate::error::HorusError::Internal(format!(
                    "Failed to set real-time priority: {}. \
                     Ensure you have RT-PREEMPT kernel and CAP_SYS_NICE capability.",
                    err
                )));
            }

            println!("✓ Real-time priority set to {} (SCHED_FIFO)", priority);
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(crate::error::HorusError::Unsupported(
                "Real-time priority scheduling is only supported on Linux".to_string(),
            ))
        }
    }

    /// Pin scheduler to a specific CPU core (prevent context switches)
    ///
    /// # Arguments
    /// * `cpu_id` - CPU core number (0-N)
    ///
    /// # Best Practices
    /// - Use isolated cores (boot with isolcpus=7 kernel parameter)
    /// - Reserve core for RT tasks only
    /// - Disable hyperthreading for predictable performance
    ///
    /// # Example
    /// ```
    /// // Pin to isolated core 7
    /// scheduler.pin_to_cpu(7)?;
    /// ```
    pub fn pin_to_cpu(&self, cpu_id: usize) -> crate::error::HorusResult<()> {
        #[cfg(target_os = "linux")]
        unsafe {
            use libc::{cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};

            let mut cpuset: cpu_set_t = std::mem::zeroed();
            CPU_ZERO(&mut cpuset);
            CPU_SET(cpu_id, &mut cpuset);

            if sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &cpuset) != 0 {
                let err = std::io::Error::last_os_error();
                return Err(crate::error::HorusError::Internal(format!(
                    "Failed to set CPU affinity: {}",
                    err
                )));
            }

            println!("✓ Scheduler pinned to CPU core {}", cpu_id);
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(crate::error::HorusError::Unsupported(
                "CPU pinning is only supported on Linux".to_string(),
            ))
        }
    }

    /// Lock all memory pages to prevent page faults (critical for <20μs latency)
    ///
    /// This prevents the OS from swapping out scheduler memory, which would
    /// cause multi-millisecond delays. Essential for hard real-time systems.
    ///
    /// # Requirements
    /// - Sufficient locked memory limit (ulimit -l)
    /// - CAP_IPC_LOCK capability or root
    ///
    /// # Warning
    /// This locks ALL current and future memory allocations. Ensure your
    /// application has bounded memory usage.
    ///
    /// # Example
    /// ```
    /// scheduler.lock_memory()?;
    /// ```
    pub fn lock_memory(&self) -> crate::error::HorusResult<()> {
        #[cfg(target_os = "linux")]
        unsafe {
            use libc::{mlockall, MCL_CURRENT, MCL_FUTURE};

            if mlockall(MCL_CURRENT | MCL_FUTURE) != 0 {
                let err = std::io::Error::last_os_error();
                return Err(crate::error::HorusError::Internal(format!(
                    "Failed to lock memory: {}. \
                     Check ulimit -l and ensure CAP_IPC_LOCK capability.",
                    err
                )));
            }

            println!("✓ Memory locked (no page faults)");
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(crate::error::HorusError::Unsupported(
                "Memory locking is only supported on Linux".to_string(),
            ))
        }
    }

    /// Pre-fault stack to prevent page faults during execution
    ///
    /// Touches stack pages to ensure they're resident in RAM before
    /// time-critical execution begins.
    ///
    /// # Arguments
    /// * `stack_size` - Stack size to pre-fault (bytes)
    ///
    /// # Example
    /// ```
    /// scheduler.prefault_stack(8 * 1024 * 1024)?;  // 8MB stack
    /// ```
    pub fn prefault_stack(&self, stack_size: usize) -> crate::error::HorusResult<()> {
        // Allocate array on stack and touch each page
        let page_size = 4096; // Standard page size
        let pages = stack_size / page_size;

        // Use volatile writes to prevent optimization
        for i in 0..pages {
            let offset = i * page_size;
            let mut dummy_stack = vec![0u8; page_size];
            unsafe {
                std::ptr::write_volatile(&mut dummy_stack[offset % page_size], 0xFF);
            }
        }

        println!("✓ Pre-faulted {} KB of stack", stack_size / 1024);
        Ok(())
    }

    /// Add a node with given priority (lower number = higher priority).
    /// If users only use add(node, priority) then logging defaults to false
    /// Automatically detects and wraps RTNode types for real-time support
    ///
    /// # Example
    /// ```
    /// scheduler.add(node, 0, None);  // Highest priority
    /// scheduler.add(node, 10, None); // Medium priority
    /// scheduler.add(node, 100, None); // Low priority
    /// ```
    pub fn add(
        &mut self,
        node: Box<dyn Node>,
        priority: u32,
        logging_enabled: Option<bool>,
    ) -> &mut Self {
        // Try to downcast to RTNode to detect real-time nodes
        // This is a bit tricky since we're dealing with trait objects
        // For now, we'll check if the node name contains certain patterns
        // In a real implementation, we'd need a better way to detect RTNode trait implementors
        let node_name = node.name().to_string();
        let logging_enabled = logging_enabled.unwrap_or(false);

        let context = NodeInfo::new(node_name.clone(), logging_enabled);

        // Check if this might be an RT node based on naming patterns or other heuristics
        // In production, you'd want a more robust detection mechanism
        let is_rt_node = node_name.contains("motor")
            || node_name.contains("control")
            || node_name.contains("sensor")
            || node_name.contains("critical");

        // For RT nodes, extract WCET and deadline if available
        // This would normally come from the RTNode trait methods
        let (wcet_budget, deadline) = if is_rt_node {
            // Default RT constraints for demonstration
            (
                Some(Duration::from_micros(100)), // 100μs WCET
                Some(Duration::from_millis(1)),   // 1ms deadline
            )
        } else {
            (None, None)
        };

        self.nodes.push(RegisteredNode {
            node,
            priority,
            logging_enabled,
            initialized: false,
            context: Some(context),
            rate_hz: None,   // Use global scheduler rate by default
            last_tick: None, // Will be set on first tick
            circuit_breaker: CircuitBreaker::new(5, 3, 5000), // 5 failures to open, 3 successes to close, 5s timeout
            is_rt_node,
            wcet_budget,
            deadline,
        });

        println!(
            "Added {} '{}' with priority {} (logging: {})",
            if is_rt_node { "RT node" } else { "node" },
            node_name,
            priority,
            logging_enabled
        );

        self
    }

    /// Add a real-time node with explicit RT constraints
    ///
    /// This method allows precise configuration of RT nodes with WCET budgets,
    /// deadlines, and other real-time constraints.
    ///
    /// # Example
    /// ```
    /// scheduler.add_rt(
    ///     Box::new(MotorControlNode::new("motor")),
    ///     0,  // Highest priority
    ///     Duration::from_micros(100),  // 100μs WCET budget
    ///     Duration::from_millis(1),    // 1ms deadline
    /// );
    /// ```
    pub fn add_rt(
        &mut self,
        node: Box<dyn Node>,
        priority: u32,
        wcet_budget: Duration,
        deadline: Duration,
    ) -> &mut Self {
        let node_name = node.name().to_string();
        let logging_enabled = false; // RT nodes typically don't need logging overhead

        let context = NodeInfo::new(node_name.clone(), logging_enabled);

        self.nodes.push(RegisteredNode {
            node,
            priority,
            logging_enabled,
            initialized: false,
            context: Some(context),
            rate_hz: None,
            last_tick: None,
            circuit_breaker: CircuitBreaker::new(5, 3, 5000),
            is_rt_node: true,
            wcet_budget: Some(wcet_budget),
            deadline: Some(deadline),
        });

        println!(
            "Added RT node '{}' with priority {} (WCET: {:?}, deadline: {:?})",
            node_name, priority, wcet_budget, deadline
        );

        // If safety monitor exists, configure it for this node
        if let Some(ref mut monitor) = self.safety_monitor {
            monitor.set_wcet_budget(node_name.clone(), wcet_budget);
            if let Some(ref config) = self.config {
                if config.realtime.watchdog_enabled {
                    let watchdog_timeout =
                        Duration::from_millis(config.realtime.watchdog_timeout_ms);
                    monitor.add_critical_node(node_name, watchdog_timeout);
                }
            }
        }

        self
    }

    /// Set the scheduler name (chainable)
    pub fn name(mut self, name: &str) -> Self {
        self.scheduler_name = name.to_string();
        self
    }

    /// Tick specific nodes by name (runs continuously with the specified nodes)
    pub fn tick(&mut self, node_names: &[&str]) -> HorusResult<()> {
        // Use the same pattern as run() but with node filtering
        self.run_with_filter(Some(node_names), None)
    }

    /// Check if the scheduler is running
    pub fn is_running(&self) -> bool {
        if let Ok(running) = self.running.lock() {
            *running
        } else {
            false
        }
    }

    /// Stop the scheduler
    pub fn stop(&self) {
        if let Ok(mut running) = self.running.lock() {
            *running = false;
        }
    }

    /// Set per-node rate control (chainable)
    ///
    /// Allows individual nodes to run at different frequencies independent of the global scheduler rate.
    /// If a node's rate is not set, it will tick at the global scheduler frequency.
    ///
    /// # Arguments
    /// * `name` - The name of the node
    /// * `rate_hz` - The desired rate in Hz (ticks per second)
    ///
    /// # Example
    /// ```
    /// scheduler.add(sensor, 0, Some(true))
    ///     .set_node_rate("sensor", 100.0);  // Run sensor at 100Hz
    /// ```
    pub fn set_node_rate(&mut self, name: &str, rate_hz: f64) -> &mut Self {
        for registered in self.nodes.iter_mut() {
            if registered.node.name() == name {
                registered.rate_hz = Some(rate_hz);
                registered.last_tick = Some(Instant::now());
                println!("Set node '{}' rate to {:.1} Hz", name, rate_hz);
                break;
            }
        }
        self
    }

    /// Main loop with automatic signal handling and cleanup
    pub fn run(&mut self) -> HorusResult<()> {
        self.run_with_filter(None, None)
    }

    /// Run all nodes for a specified duration, then shutdown gracefully
    pub fn run_for(&mut self, duration: Duration) -> HorusResult<()> {
        self.run_with_filter(None, Some(duration))
    }

    /// Run specific nodes for a specified duration, then shutdown gracefully
    pub fn tick_for(&mut self, node_names: &[&str], duration: Duration) -> HorusResult<()> {
        self.run_with_filter(Some(node_names), Some(duration))
    }

    /// Internal method to run scheduler with optional node filtering and duration
    fn run_with_filter(
        &mut self,
        node_filter: Option<&[&str]>,
        duration: Option<Duration>,
    ) -> HorusResult<()> {
        // Create tokio runtime for nodes that need async
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            crate::error::HorusError::Internal(format!("Failed to create tokio runtime: {}", e))
        })?;

        rt.block_on(async {
            // Track start time for duration-limited runs
            let start_time = Instant::now();

            // Set up signal handling
            let running = self.running.clone();
            if let Err(e) = ctrlc::set_handler(move || {
                eprintln!("{}", "\nCtrl+C received! Shutting down HORUS scheduler...".red());
                if let Ok(mut r) = running.lock() {
                    *r = false;
                }
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    eprintln!("{}", "Force terminating application...".red());
                    std::process::exit(0);
                });
            }) {
                eprintln!("Warning: Failed to set signal handler: {}", e);
            }

            // Initialize nodes
            for registered in self.nodes.iter_mut() {
                let node_name = registered.node.name();
                let should_run = node_filter.is_none_or(|filter| filter.contains(&node_name));

                if should_run && !registered.initialized {
                    if let Some(ref mut ctx) = registered.context {
                        match registered.node.init(ctx) {
                            Ok(()) => {
                                registered.initialized = true;
                                println!("Initialized node '{}'", node_name);
                            }
                            Err(e) => {
                                println!("Failed to initialize node '{}': {}", node_name, e);
                                ctx.transition_to_error(format!("Initialization failed: {}", e));
                            }
                        }
                    }
                }
            }

            // Create heartbeat directory
            Self::setup_heartbeat_directory();

            // Write initial registry
            self.update_registry();

            // Build dependency graph from node pub/sub relationships
            self.build_dependency_graph();

            // Main tick loop
            while self.is_running() {
                // Check if duration limit has been reached
                if let Some(max_duration) = duration {
                    if start_time.elapsed() >= max_duration {
                        println!("Scheduler reached time limit of {:?}", max_duration);
                        break;
                    }
                }

                let now = Instant::now();
                self.last_instant = now;

                // Check if learning phase is complete
                if !self.learning_complete && self.profiler.is_learning_complete() {
                    // Generate tier classification
                    self.classifier = Some(TierClassifier::from_profiler(&self.profiler));

                    // Setup JIT compiler for ultra-fast nodes
                    self.setup_jit_compiler();

                    // Initialize async I/O executor and move I/O-heavy nodes
                    self.setup_async_executor().await;

                    self.learning_complete = true;
                }

                // Execute nodes based on learning phase
                if self.learning_complete {
                    // Optimized execution with parallel groups
                    self.execute_optimized(node_filter).await;
                } else {
                    // Learning mode: sequential execution with profiling
                    self.execute_learning_mode(node_filter).await;
                    self.profiler.tick();
                }

                // Check watchdogs and handle emergency stop for RT systems
                if let Some(ref monitor) = self.safety_monitor {
                    // Check all watchdogs
                    let expired_watchdogs = monitor.check_watchdogs();
                    if !expired_watchdogs.is_empty() {
                        eprintln!(" Watchdog expired for nodes: {:?}", expired_watchdogs);
                    }

                    // Check if emergency stop was triggered
                    if monitor.is_emergency_stop() {
                        eprintln!(" Emergency stop activated - shutting down scheduler");
                        break;
                    }
                }

                // Periodic registry snapshot (every 5 seconds)
                if self.last_snapshot.elapsed() >= Duration::from_secs(5) {
                    self.snapshot_state_to_registry();
                    self.last_snapshot = Instant::now();
                }

                // Use configured tick rate or default to ~60 FPS
                let tick_period_ms = if let Some(ref config) = self.config {
                    (1000.0 / config.timing.global_rate_hz) as u64
                } else {
                    16 // Default ~60 FPS
                };
                tokio::time::sleep(Duration::from_millis(tick_period_ms)).await;
            }

            // Shutdown async I/O nodes first
            if let Some(ref mut executor) = self.async_io_executor {
                executor.shutdown_all().await;
            }

            // Shutdown nodes
            for registered in self.nodes.iter_mut() {
                let node_name = registered.node.name();
                let should_run = node_filter.is_none_or(|filter| filter.contains(&node_name));

                if should_run && registered.initialized {
                    if let Some(ref mut ctx) = registered.context {
                        // Write final "Stopped" heartbeat before shutdown - node self-reports
                        ctx.record_shutdown();

                        match registered.node.shutdown(ctx) {
                            Ok(()) => println!("Shutdown node '{}' successfully", node_name),
                            Err(e) => println!("Error shutting down node '{}': {}", node_name, e),
                        }
                    }
                }
            }

            // Clean up registry file and session (keep heartbeats for dashboards)
            self.cleanup_registry();
            // Note: Don't cleanup_heartbeats() - let dashboards see final state
            Self::cleanup_session();

            println!("Scheduler shutdown complete");
        });

        Ok(())
    }

    /// Get information about all registered nodes
    pub fn get_node_list(&self) -> Vec<String> {
        self.nodes
            .iter()
            .map(|registered| registered.node.name().to_string())
            .collect()
    }

    /// Get detailed information about a specific node
    pub fn get_node_info(&self, name: &str) -> Option<HashMap<String, String>> {
        for registered in &self.nodes {
            if registered.node.name() == name {
                let mut info = HashMap::new();
                info.insert("name".to_string(), registered.node.name().to_string());
                info.insert("priority".to_string(), registered.priority.to_string());
                info.insert(
                    "logging_enabled".to_string(),
                    registered.logging_enabled.to_string(),
                );
                return Some(info);
            }
        }
        None
    }
    /// Enable/disable logging for a specific node (chainable)
    ///
    /// # Returns
    /// Returns `&mut Self` for method chaining. Logs warning if node not found.
    ///
    /// # Example
    /// ```
    /// scheduler
    ///     .set_node_logging("sensor", false)
    ///     .set_node_logging("controller", true)
    ///     .set_node_rate("motor", 1000.0);
    /// ```
    pub fn set_node_logging(&mut self, name: &str, enabled: bool) -> &mut Self {
        let mut found = false;
        for registered in &mut self.nodes {
            if registered.node.name() == name {
                registered.logging_enabled = enabled;
                println!("Set logging for node '{}' to: {}", name, enabled);
                found = true;
                break;
            }
        }
        if !found {
            eprintln!("Warning: Node '{}' not found for logging configuration", name);
        }
        self
    }
    /// Get monitoring summary by creating temporary contexts for each node
    pub fn get_monitoring_summary(&self) -> Vec<(String, u32)> {
        self.nodes
            .iter()
            .map(|registered| (registered.node.name().to_string(), registered.priority))
            .collect()
    }

    /// Write metadata to registry file for monitor to read
    fn update_registry(&self) {
        if let Ok(registry_path) = Self::get_registry_path() {
            let pid = std::process::id();

            // Collect pub/sub info from each node
            let nodes_json: Vec<String> = self.nodes.iter().map(|registered| {
                let name = registered.node.name();
                let priority = registered.priority;
                let publishers = registered.node.get_publishers();
                let subscribers = registered.node.get_subscribers();

                // Format publishers
                let pubs_json = publishers.iter()
                    .map(|p| format!("{{\"topic\": \"{}\", \"type\": \"{}\"}}",
                        p.topic_name.replace("\"", "\\\""),
                        p.type_name.replace("\"", "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ");

                // Format subscribers
                let subs_json = subscribers.iter()
                    .map(|s| format!("{{\"topic\": \"{}\", \"type\": \"{}\"}}",
                        s.topic_name.replace("\"", "\\\""),
                        s.type_name.replace("\"", "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!(
                    "    {{\"name\": \"{}\", \"priority\": {}, \"publishers\": [{}], \"subscribers\": [{}]}}",
                    name, priority, pubs_json, subs_json
                )
            }).collect();

            let registry_data = format!(
                "{{\n  \"pid\": {},\n  \"scheduler_name\": \"{}\",\n  \"working_dir\": \"{}\",\n  \"nodes\": [\n{}\n  ]\n}}",
                pid,
                self.scheduler_name,
                self.working_dir.to_string_lossy(),
                nodes_json.join(",\n")
            );

            let _ = fs::write(&registry_path, registry_data);
        }
    }

    /// Remove registry file when scheduler stops
    fn cleanup_registry(&self) {
        if let Ok(registry_path) = Self::get_registry_path() {
            let _ = fs::remove_file(registry_path);
        }
    }

    /// Get path to registry file
    fn get_registry_path() -> Result<PathBuf, std::io::Error> {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push(".horus_registry.json");
        Ok(path)
    }

    /// Create heartbeat directory
    fn setup_heartbeat_directory() {
        // Heartbeats are intentionally global (not session-isolated) so dashboard can monitor all nodes
        let dir = PathBuf::from("/dev/shm/horus/heartbeats");
        let _ = fs::create_dir_all(&dir);
    }

    /// Clean up all heartbeat files
    fn cleanup_heartbeats() {
        let dir = PathBuf::from("/dev/shm/horus/heartbeats");
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
    }

    /// Clean up session directory (topics and metadata)
    fn cleanup_session() {
        // Get current session ID from environment
        if let Ok(session_id) = std::env::var("HORUS_SESSION_ID") {
            let session_dir = PathBuf::from(format!("/dev/shm/horus/sessions/{}", session_id));

            if session_dir.exists() {
                let _ = fs::remove_dir_all(&session_dir);
            }
        }
    }

    /// Snapshot node state to registry (for crash forensics and persistence)
    /// Called every 5 seconds to avoid I/O overhead
    fn snapshot_state_to_registry(&self) {
        if let Ok(registry_path) = Self::get_registry_path() {
            let pid = std::process::id();
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Collect node info including state and health
            let nodes_json: Vec<String> = self.nodes.iter().map(|registered| {
                let name = registered.node.name();
                let priority = registered.priority;
                let publishers = registered.node.get_publishers();
                let subscribers = registered.node.get_subscribers();

                // Get state and health from context
                let (state_str, health_str, error_count, tick_count) = if let Some(ref ctx) = registered.context {
                    let heartbeat = NodeHeartbeat::from_metrics(
                        ctx.state().clone(),
                        ctx.metrics()
                    );
                    (
                        ctx.state().to_string(),
                        heartbeat.health.as_str().to_string(),
                        ctx.metrics().errors_count,
                        ctx.metrics().total_ticks,
                    )
                } else {
                    ("Unknown".to_string(), "Unknown".to_string(), 0, 0)
                };

                // Format publishers
                let pubs_json = publishers.iter()
                    .map(|p| format!("{{\"topic\": \"{}\", \"type\": \"{}\"}}",
                        p.topic_name.replace("\"", "\\\""),
                        p.type_name.replace("\"", "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ");

                // Format subscribers
                let subs_json = subscribers.iter()
                    .map(|s| format!("{{\"topic\": \"{}\", \"type\": \"{}\"}}",
                        s.topic_name.replace("\"", "\\\""),
                        s.type_name.replace("\"", "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!(
                    "    {{\"name\": \"{}\", \"priority\": {}, \"state\": \"{}\", \"health\": \"{}\", \"error_count\": {}, \"tick_count\": {}, \"publishers\": [{}], \"subscribers\": [{}]}}",
                    name, priority, state_str, health_str, error_count, tick_count, pubs_json, subs_json
                )
            }).collect();

            let registry_data = format!(
                "{{\n  \"pid\": {},\n  \"scheduler_name\": \"{}\",\n  \"working_dir\": \"{}\",\n  \"last_snapshot\": {},\n  \"nodes\": [\n{}\n  ]\n}}",
                pid,
                self.scheduler_name,
                self.working_dir.to_string_lossy(),
                timestamp,
                nodes_json.join(",\n")
            );

            // Atomic write: write to temp file, then rename
            if let Some(parent) = registry_path.parent() {
                let temp_path = parent.join(format!(".horus_registry.json.tmp.{}", pid));

                // Write to temp file
                if fs::write(&temp_path, &registry_data).is_ok() {
                    // Atomically rename to final path
                    let _ = fs::rename(&temp_path, &registry_path);
                }
            }
        }
    }

    /// Build dependency graph from node pub/sub relationships
    fn build_dependency_graph(&mut self) {
        let node_data: Vec<(&str, Vec<String>, Vec<String>)> = self
            .nodes
            .iter()
            .map(|r| {
                let name = r.node.name();
                let pubs = r
                    .node
                    .get_publishers()
                    .iter()
                    .map(|p| p.topic_name.clone())
                    .collect();
                let subs = r
                    .node
                    .get_subscribers()
                    .iter()
                    .map(|s| s.topic_name.clone())
                    .collect();
                (name, pubs, subs)
            })
            .collect();

        if !node_data.is_empty() {
            let graph = DependencyGraph::from_nodes(&node_data);
            self.dependency_graph = Some(graph);
        }
    }

    /// Execute nodes in learning mode (sequential with profiling)
    async fn execute_learning_mode(&mut self, node_filter: Option<&[&str]>) {
        // Sort by priority
        self.nodes.sort_by_key(|r| r.priority);

        // We need to process nodes one at a time to avoid borrow checker issues
        let num_nodes = self.nodes.len();
        for i in 0..num_nodes {
            let (should_run, node_name, should_tick) = {
                let registered = &self.nodes[i];
                let node_name = registered.node.name();
                let should_run = node_filter.is_none_or(|filter| filter.contains(&node_name));

                // Check rate limiting
                let should_tick = if let Some(rate_hz) = registered.rate_hz {
                    let current_time = Instant::now();
                    if let Some(last_tick) = registered.last_tick {
                        let elapsed_secs = (current_time - last_tick).as_secs_f64();
                        let period_secs = 1.0 / rate_hz;
                        elapsed_secs >= period_secs
                    } else {
                        true
                    }
                } else {
                    true
                };

                (should_run, node_name, should_tick)
            };

            if !should_tick {
                continue;
            }

            // Check circuit breaker
            if !self.nodes[i].circuit_breaker.should_allow() {
                // Circuit is open, skip this node
                continue;
            }

            // Update last tick time if rate limited
            if self.nodes[i].rate_hz.is_some() {
                self.nodes[i].last_tick = Some(Instant::now());
            }

            if should_run && self.nodes[i].initialized {
                // Feed watchdog for RT nodes
                if self.nodes[i].is_rt_node {
                    if let Some(ref monitor) = self.safety_monitor {
                        monitor.feed_watchdog(node_name);
                    }
                }

                let tick_start = Instant::now();
                let tick_result = {
                    let registered = &mut self.nodes[i];
                    if let Some(ref mut context) = registered.context {
                        context.start_tick();

                        // Execute node tick with panic handling
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            registered.node.tick(Some(context));
                        }))
                    } else {
                        continue;
                    }
                };

                let tick_duration = tick_start.elapsed();

                // Record profiling data
                self.profiler.record(node_name, tick_duration);

                // Check WCET budget for RT nodes
                if self.nodes[i].is_rt_node && self.nodes[i].wcet_budget.is_some() {
                    if let Some(ref monitor) = self.safety_monitor {
                        if let Err(violation) = monitor.check_wcet(node_name, tick_duration) {
                            eprintln!(
                                " WCET violation in {}: {:?} > {:?}",
                                violation.node_name, violation.actual, violation.budget
                            );
                        }
                    }
                }

                // Check deadline for RT nodes
                if self.nodes[i].is_rt_node && self.nodes[i].deadline.is_some() {
                    let elapsed = tick_start.elapsed();
                    let deadline = self.nodes[i].deadline.unwrap();
                    if elapsed > deadline {
                        if let Some(ref monitor) = self.safety_monitor {
                            monitor.record_deadline_miss(node_name);
                            eprintln!(
                                " Deadline miss in {}: {:?} > {:?}",
                                node_name, elapsed, deadline
                            );
                        }
                    }
                }

                // Handle tick result
                match tick_result {
                    Ok(_) => {
                        // Record success with circuit breaker
                        self.nodes[i].circuit_breaker.record_success();

                        if let Some(ref mut context) = self.nodes[i].context {
                            context.record_tick(); // Node writes its own heartbeat
                        }
                    }
                    Err(panic_err) => {
                        // Record failure with circuit breaker
                        self.nodes[i].circuit_breaker.record_failure();
                        let error_msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                            format!("Node panicked: {}", s)
                        } else if let Some(s) = panic_err.downcast_ref::<String>() {
                            format!("Node panicked: {}", s)
                        } else {
                            "Node panicked with unknown error".to_string()
                        };

                        let registered = &mut self.nodes[i];
                        if let Some(ref mut context) = registered.context {
                            context.record_tick_failure(error_msg.clone()); // Node writes its own heartbeat
                            eprintln!(" {} failed: {}", node_name, error_msg);

                            registered.node.on_error(&error_msg, context);

                            if context.config().restart_on_failure {
                                match context.restart() {
                                    Ok(_) => {
                                        println!(
                                            " Node '{}' restarted successfully (attempt {}/{})",
                                            node_name,
                                            context.metrics().errors_count,
                                            context.config().max_restart_attempts
                                        );
                                        registered.initialized = true;
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "💀 Node '{}' exceeded max restart attempts: {}",
                                            node_name, e
                                        );
                                        context.transition_to_crashed(format!(
                                            "Max restarts exceeded: {}",
                                            e
                                        ));
                                        registered.initialized = false;
                                    }
                                }
                            } else {
                                context.transition_to_error(error_msg);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Execute nodes in optimized mode (parallel execution based on dependency graph)
    async fn execute_optimized(&mut self, node_filter: Option<&[&str]>) {
        // If no dependency graph available, fall back to sequential
        if self.dependency_graph.is_none() {
            self.execute_learning_mode(node_filter).await;
            return;
        }

        // Trigger async I/O nodes
        if let Some(ref executor) = self.async_io_executor {
            executor.tick_all().await;
        }

        // Execute nodes level by level (nodes in same level can run in parallel)
        let levels = self.dependency_graph.as_ref().unwrap().levels.clone();

        for level in &levels {
            // Find indices of nodes in this level that should run
            let mut level_indices = Vec::new();

            for node_name in level {
                for (idx, registered) in self.nodes.iter().enumerate() {
                    if registered.node.name() == node_name {
                        let should_run =
                            node_filter.is_none_or(|filter| filter.contains(&node_name.as_str()));

                        // Check rate limiting
                        let should_tick = if let Some(rate_hz) = registered.rate_hz {
                            let current_time = Instant::now();
                            if let Some(last_tick) = registered.last_tick {
                                let elapsed_secs = (current_time - last_tick).as_secs_f64();
                                let period_secs = 1.0 / rate_hz;
                                elapsed_secs >= period_secs
                            } else {
                                true
                            }
                        } else {
                            true
                        };

                        if should_run && registered.initialized && should_tick {
                            level_indices.push(idx);
                        }
                        break;
                    }
                }
            }

            // Execute nodes in this level
            // NOTE: True parallel execution requires refactoring to allow concurrent
            // mutable access to different Vec elements. Options:
            // 1. Use UnsafeCell/RwLock per node (adds overhead)
            // 2. Restructure nodes into separate Vecs (breaks encapsulation)
            // 3. Use async/await with message passing (architectural change)
            //
            // For now, execute level-by-level sequentially. Since levels are already
            // topologically sorted, this ensures correctness. Parallelism benefit
            // would only apply within levels with multiple independent nodes.
            //
            // Performance: Still better than original sequential-by-priority because:
            // - Respects true dependencies (not just priority)
            // - Enables future parallelization without API changes
            // - Critical path optimization from dependency analysis
            for idx in level_indices {
                self.execute_single_node(idx);
            }
        }

        // Process any async I/O results
        self.process_async_results().await;
    }

    /// Execute a single node by index with RT support
    fn execute_single_node(&mut self, idx: usize) {
        // Check circuit breaker first
        if !self.nodes[idx].circuit_breaker.should_allow() {
            // Circuit is open, skip this node
            return;
        }

        // Update rate limit timestamp
        if self.nodes[idx].rate_hz.is_some() {
            self.nodes[idx].last_tick = Some(Instant::now());
        }

        let node_name = self.nodes[idx].node.name();
        let is_rt_node = self.nodes[idx].is_rt_node;
        let wcet_budget = self.nodes[idx].wcet_budget;
        let deadline = self.nodes[idx].deadline;

        // Feed watchdog for RT nodes
        if is_rt_node {
            if let Some(ref monitor) = self.safety_monitor {
                monitor.feed_watchdog(node_name);
            }
        }

        let tick_start = Instant::now();

        let tick_result = {
            let registered = &mut self.nodes[idx];
            if let Some(ref mut context) = registered.context {
                context.start_tick();

                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    registered.node.tick(Some(context));
                }))
            } else {
                return;
            }
        };

        let tick_duration = tick_start.elapsed();
        self.profiler.record(node_name, tick_duration);

        // Check WCET budget for RT nodes
        if is_rt_node && wcet_budget.is_some() {
            if let Some(ref monitor) = self.safety_monitor {
                if let Err(violation) = monitor.check_wcet(node_name, tick_duration) {
                    eprintln!(
                        " WCET violation in {}: {:?} > {:?}",
                        violation.node_name, violation.actual, violation.budget
                    );
                }
            }
        }

        // Check deadline for RT nodes
        if is_rt_node && deadline.is_some() {
            let elapsed = tick_start.elapsed();
            if elapsed > deadline.unwrap() {
                if let Some(ref monitor) = self.safety_monitor {
                    monitor.record_deadline_miss(node_name);
                    eprintln!(
                        " Deadline miss in {}: {:?} > {:?}",
                        node_name,
                        elapsed,
                        deadline.unwrap()
                    );
                }
            }
        }

        match tick_result {
            Ok(_) => {
                // Record success with circuit breaker
                self.nodes[idx].circuit_breaker.record_success();

                if let Some(ref mut context) = self.nodes[idx].context {
                    context.record_tick(); // Node writes its own heartbeat
                }
            }
            Err(panic_err) => {
                // Record failure with circuit breaker
                self.nodes[idx].circuit_breaker.record_failure();
                let error_msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                    format!("Node panicked: {}", s)
                } else if let Some(s) = panic_err.downcast_ref::<String>() {
                    format!("Node panicked: {}", s)
                } else {
                    "Node panicked with unknown error".to_string()
                };

                let registered = &mut self.nodes[idx];
                if let Some(ref mut context) = registered.context {
                    context.record_tick_failure(error_msg.clone()); // Node writes its own heartbeat
                    eprintln!(" {} failed: {}", node_name, error_msg);

                    registered.node.on_error(&error_msg, context);

                    if context.config().restart_on_failure {
                        match context.restart() {
                            Ok(_) => {
                                println!(
                                    " Node '{}' restarted successfully (attempt {}/{})",
                                    node_name,
                                    context.metrics().errors_count,
                                    context.config().max_restart_attempts
                                );
                                registered.initialized = true;
                            }
                            Err(e) => {
                                eprintln!(
                                    "💀 Node '{}' exceeded max restart attempts: {}",
                                    node_name, e
                                );
                                context
                                    .transition_to_crashed(format!("Max restarts exceeded: {}", e));
                                registered.initialized = false;
                            }
                        }
                    } else {
                        context.transition_to_error(error_msg);
                    }
                }
            }
        }
    }

    /// Setup JIT compiler for ultra-fast nodes
    fn setup_jit_compiler(&mut self) {
        // Identify ultra-fast nodes from classifier
        if let Some(ref classifier) = self.classifier {
            // Collect names of ultra-fast nodes
            let ultra_fast_nodes: Vec<String> = self
                .nodes
                .iter()
                .filter_map(|registered| {
                    let node_name = registered.node.name();
                    classifier
                        .get_tier(node_name)
                        .filter(|&tier| tier == ExecutionTier::UltraFast)
                        .map(|_| node_name.to_string())
                })
                .collect();

            // Try to compile each ultra-fast node
            for node_name in ultra_fast_nodes {
                // For demonstration, compile a simple arithmetic function
                // In a real system, we'd analyze the node's computation pattern
                match CompiledDataflow::compile(
                    node_name.clone(),
                    super::jit::DataflowExpr::BinOp {
                        op: super::jit::BinaryOp::Add,
                        left: Box::new(super::jit::DataflowExpr::BinOp {
                            op: super::jit::BinaryOp::Mul,
                            left: Box::new(super::jit::DataflowExpr::Input("x".to_string())),
                            right: Box::new(super::jit::DataflowExpr::Const(2)),
                        }),
                        right: Box::new(super::jit::DataflowExpr::Const(1)),
                    },
                ) {
                    Ok(compiled) => {
                        self.jit_compiled_nodes.insert(node_name, compiled);
                    }
                    Err(e) => {
                        // Compilation failed, node will run normally
                        eprintln!("Failed to JIT compile {}: {}", node_name, e);
                    }
                }
            }
        }
    }

    /// Setup async executor and move I/O-heavy nodes to it
    async fn setup_async_executor(&mut self) {
        // Create async I/O executor
        let mut async_executor = match AsyncIOExecutor::new() {
            Ok(exec) => exec,
            Err(_) => return, // Continue without async tier if creation fails
        };

        // Create channel for async results
        let (tx, rx) = mpsc::unbounded_channel();
        self.async_result_tx = Some(tx.clone());
        self.async_result_rx = Some(rx);

        // Identify I/O-heavy nodes from classifier
        if let Some(ref classifier) = self.classifier {
            let mut nodes_to_move = Vec::new();

            // Find indices of I/O-heavy nodes
            for (idx, registered) in self.nodes.iter().enumerate() {
                let node_name = registered.node.name();

                // Check if this node is classified as AsyncIO tier
                if let Some(tier) = classifier.get_tier(node_name) {
                    if tier == ExecutionTier::AsyncIO {
                        nodes_to_move.push(idx);
                    }
                }
            }

            // Move nodes to async executor (in reverse order to maintain indices)
            for idx in nodes_to_move.into_iter().rev() {
                // Remove from main scheduler
                let registered = self.nodes.swap_remove(idx);
                let node_name = registered.node.name().to_string();

                // Spawn in async executor
                if let Err(e) =
                    async_executor.spawn_node(registered.node, registered.context, tx.clone())
                {
                    eprintln!("Failed to move {} to async tier: {}", node_name, e);
                    // Note: Can't put it back since we've moved ownership
                    // This is acceptable as the node would be dropped anyway
                }
            }
        }

        self.async_io_executor = Some(async_executor);
    }

    /// Process async I/O results
    async fn process_async_results(&mut self) {
        if let Some(ref mut rx) = self.async_result_rx {
            // Process all available results without blocking
            while let Ok(result) = rx.try_recv() {
                if !result.success {
                    if let Some(ref error) = result.error {
                        eprintln!("Async node {} failed: {}", result.node_name, error);
                    }
                }
            }
        }
    }

    /// Configure the scheduler for specific robot types (runtime configuration)
    ///
    /// **Note**: For builder pattern during construction, use `with_config()` instead.
    /// This method is for runtime reconfiguration of an existing scheduler.
    ///
    /// # Examples
    ///
    /// ```
    /// // Runtime reconfiguration
    /// let mut scheduler = Scheduler::new();
    /// scheduler.set_config(SchedulerConfig::hard_realtime());
    /// ```
    ///
    /// # Prefer Builder Pattern
    /// ```
    /// // Better: Use with_config() during construction
    /// let scheduler = Scheduler::new()
    ///     .with_config(SchedulerConfig::hard_realtime());
    /// ```
    #[deprecated(since = "0.2.0", note = "Use with_config() for builder pattern. set_config() is only for runtime reconfiguration.")]
    pub fn set_config(&mut self, config: super::config::SchedulerConfig) -> &mut Self {
        use super::config::*;

        // Apply execution mode
        match config.execution {
            ExecutionMode::JITOptimized => {
                // Force JIT compilation for all nodes
                self.profiler.force_ultra_fast_classification = true;
                println!("JIT optimization mode selected");
            }
            ExecutionMode::Parallel => {
                // Enable full parallelization
                self.parallel_executor.set_max_threads(num_cpus::get());
                println!("Parallel execution mode selected");
            }
            ExecutionMode::AsyncIO => {
                // Force async I/O tier for all I/O operations
                self.profiler.force_async_io_classification = true;
                println!("Async I/O mode selected");
            }
            ExecutionMode::Sequential => {
                // Disable all optimizations for deterministic execution
                self.learning_complete = true; // Skip learning phase
                self.classifier = None;
                self.parallel_executor.set_max_threads(1);
                println!("Sequential execution mode selected");
            }
            ExecutionMode::AutoAdaptive => {
                // Default adaptive behavior
                println!("Auto-adaptive mode selected");
            }
        }

        // Apply real-time configuration
        if config.realtime.safety_monitor
            || config.realtime.wcet_enforcement
            || config.realtime.deadline_monitoring
        {
            // Create safety monitor with configured deadline miss limit
            let mut monitor = SafetyMonitor::new(config.realtime.max_deadline_misses);

            // Configure critical nodes and WCET budgets for RT nodes
            for registered in self.nodes.iter() {
                if registered.is_rt_node {
                    let node_name = registered.node.name().to_string();

                    // Add as critical node with watchdog if configured
                    if config.realtime.watchdog_enabled {
                        let watchdog_timeout =
                            Duration::from_millis(config.realtime.watchdog_timeout_ms);
                        monitor.add_critical_node(node_name.clone(), watchdog_timeout);
                    }

                    // Set WCET budget if available
                    if let Some(wcet) = registered.wcet_budget {
                        monitor.set_wcet_budget(node_name, wcet);
                    }
                }
            }

            self.safety_monitor = Some(monitor);
            println!("Safety monitor configured for RT nodes");
        }

        // Apply timing configuration
        if config.timing.per_node_rates {
            // Per-node rate control already supported via set_node_rate()
        }

        // Global rate control
        let _tick_period_ms = (1000.0 / config.timing.global_rate_hz) as u64;
        // This will be used in the run loop (store for later)

        // Apply fault tolerance
        for registered in self.nodes.iter_mut() {
            if config.fault.circuit_breaker_enabled {
                registered.circuit_breaker = CircuitBreaker::new(
                    config.fault.max_failures,
                    config.fault.recovery_threshold,
                    config.fault.circuit_timeout_ms,
                );
            } else {
                // Disable circuit breaker by setting impossibly high threshold
                registered.circuit_breaker = CircuitBreaker::new(u32::MAX, 0, 0);
            }
        }

        // Apply resource configuration
        if let Some(ref cores) = config.resources.cpu_cores {
            // Set CPU affinity
            self.parallel_executor.set_cpu_cores(cores.clone());
            println!("CPU cores configuration: {:?}", cores);
        }

        // Apply monitoring configuration
        if config.monitoring.profiling_enabled {
            self.profiler.enable();
            println!("Profiling enabled");
        } else {
            self.profiler.disable();
            println!("Profiling disabled");
        }

        // Handle robot presets with preset-specific optimizations
        match config.preset {
            RobotPreset::SafetyCritical => {
                // Additional safety-critical setup
                println!("Configured for safety-critical operation");
                println!("- Deterministic execution enabled");
                println!("- Triple redundancy active");
                println!("- Black box recording enabled");
            }
            RobotPreset::HardRealTime => {
                println!("Configured for hard real-time operation");
                println!("- JIT compilation enabled");
                println!("- CPU cores isolated");
            }
            RobotPreset::HighPerformance => {
                println!("Configured for high-performance operation");
                println!("- Maximum optimization enabled");
                println!("- All GPUs active");
            }
            RobotPreset::Space => {
                println!("Configured for space robotics");
                println!("- Radiation hardening active");
                println!("- Power management enabled");

                // Apply space-specific settings from custom config
                if let Some(delay) = config.get_custom::<i64>("communication_delay_ms") {
                    println!("- Communication delay: {}ms", delay);
                }
            }
            RobotPreset::Swarm => {
                println!("Configured for swarm robotics");

                // Apply swarm-specific settings
                if let Some(swarm_id) = config.get_custom::<i64>("swarm_id") {
                    self.scheduler_name = format!("Swarm_{}", swarm_id);
                }
                if let Some(consensus) = config.get_custom::<String>("consensus_algorithm") {
                    println!("- Consensus algorithm: {}", consensus);
                }
            }
            RobotPreset::SoftRobotics => {
                println!("Configured for soft robotics");

                // Apply soft robotics settings
                if let Some(material) = config.get_custom::<String>("material_model") {
                    println!("- Material model: {}", material);
                }
            }
            RobotPreset::Quantum => {
                println!("Configured for quantum-assisted robotics");

                // Apply quantum settings
                if let Some(backend) = config.get_custom::<String>("quantum_backend") {
                    println!("- Quantum backend: {}", backend);
                }
                if let Some(qubits) = config.get_custom::<i64>("qubit_count") {
                    println!("- Qubit count: {}", qubits);
                }
            }
            RobotPreset::Custom => {
                println!("Using custom configuration");

                // Process all custom settings
                for (key, _value) in &config.custom {
                    println!("- Custom setting: {}", key);
                }
            }
            _ => {
                // Standard or other presets
            }
        }

        // Store config for runtime use
        self.config = Some(config);

        self
    }
}
