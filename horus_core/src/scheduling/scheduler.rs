use crate::core::{Node, NodeHeartbeat, NodeInfo};
use crate::error::HorusResult;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Enhanced node registration info with lifecycle tracking and per-node rate control
struct RegisteredNode {
    node: Box<dyn Node>,
    priority: u32,
    logging_enabled: bool,
    initialized: bool,
    context: Option<NodeInfo>,
    rate_hz: Option<f64>, // Per-node rate control (None = use global scheduler rate)
    last_tick: Option<Instant>, // Last tick time for rate limiting
}

/// Central orchestrator: holds nodes, drives the tick loop.
pub struct Scheduler {
    nodes: Vec<RegisteredNode>,
    running: Arc<Mutex<bool>>,
    last_instant: Instant,
    last_snapshot: Instant,
    scheduler_name: String,
    working_dir: PathBuf,
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
        }
    }

    /// Add a node under numeric `priority` (0 = highest).
    /// If users only use add(node, priority) then logging defaults to false
    pub fn add(
        &mut self,
        node: Box<dyn Node>,
        priority: u32,
        logging_enabled: Option<bool>,
    ) -> &mut Self {
        let node_name = node.name().to_string();
        let logging_enabled = logging_enabled.unwrap_or(false);

        let context = NodeInfo::new(node_name.clone(), logging_enabled);

        self.nodes.push(RegisteredNode {
            node,
            priority,
            logging_enabled,
            initialized: false,
            context: Some(context),
            rate_hz: None,   // Use global scheduler rate by default
            last_tick: None, // Will be set on first tick
        });

        println!(
            "Added node '{}' with priority {} (logging: {})",
            node_name, priority, logging_enabled
        );

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
                eprintln!("\n🛑 Ctrl+C received! Shutting down HORUS scheduler...");
                if let Ok(mut r) = running.lock() {
                    *r = false;
                }
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    eprintln!("🚪 Force terminating application...");
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

                // Process nodes in priority order
                self.nodes.sort_by_key(|r| r.priority);
                for registered in self.nodes.iter_mut() {
                    let node_name = registered.node.name();
                    let should_run = node_filter.is_none_or(|filter| filter.contains(&node_name));

                    // Per-node rate control: Check if enough time has elapsed for this node
                    if let Some(rate_hz) = registered.rate_hz {
                        let current_time = Instant::now();
                        if let Some(last_tick) = registered.last_tick {
                            let elapsed_secs = (current_time - last_tick).as_secs_f64();
                            let period_secs = 1.0 / rate_hz;

                            if elapsed_secs < period_secs {
                                continue;  // Skip this node - not enough time has passed
                            }
                        }
                        // Update last tick time
                        registered.last_tick = Some(current_time);
                    }

                    if should_run && registered.initialized {
                        if let Some(ref mut context) = registered.context {
                            context.start_tick();

                            // Catch panics during tick execution
                            let tick_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                registered.node.tick(Some(context));
                            }));

                            match tick_result {
                                Ok(_) => {
                                    // Successful tick
                                    context.record_tick();
                                }
                                Err(panic_err) => {
                                    // Node panicked - handle error
                                    let error_msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                                        format!("Node panicked: {}", s)
                                    } else if let Some(s) = panic_err.downcast_ref::<String>() {
                                        format!("Node panicked: {}", s)
                                    } else {
                                        "Node panicked with unknown error".to_string()
                                    };

                                    context.record_tick_failure(error_msg.clone());
                                    eprintln!(" {} failed: {}", node_name, error_msg);

                                    // Call the node's error handler
                                    registered.node.on_error(&error_msg, context);

                                    // Check if auto-restart is enabled
                                    if context.config().restart_on_failure {
                                        match context.restart() {
                                            Ok(_) => {
                                                println!(" Node '{}' restarted successfully (attempt {}/{})",
                                                    node_name,
                                                    context.metrics().errors_count,
                                                    context.config().max_restart_attempts);
                                                registered.initialized = true; // Mark as initialized after restart
                                            }
                                            Err(e) => {
                                                eprintln!("💀 Node '{}' exceeded max restart attempts: {}", node_name, e);
                                                context.transition_to_crashed(format!("Max restarts exceeded: {}", e));
                                                registered.initialized = false; // Stop running this node
                                            }
                                        }
                                    } else {
                                        // No auto-restart - transition to error state
                                        context.transition_to_error(error_msg);
                                    }
                                }
                            }

                            // Write heartbeat after each tick (success or failure)
                            Self::write_heartbeat(node_name, context);
                        }
                    }
                }

                // Periodic registry snapshot (every 5 seconds)
                if self.last_snapshot.elapsed() >= Duration::from_secs(5) {
                    self.snapshot_state_to_registry();
                    self.last_snapshot = Instant::now();
                }

                tokio::time::sleep(Duration::from_millis(16)).await; // ~60 FPS
            }

            // Shutdown nodes
            for registered in self.nodes.iter_mut() {
                let node_name = registered.node.name();
                let should_run = node_filter.is_none_or(|filter| filter.contains(&node_name));

                if should_run && registered.initialized {
                    if let Some(ref mut ctx) = registered.context {
                        match registered.node.shutdown(ctx) {
                            Ok(()) => println!("Shutdown node '{}' successfully", node_name),
                            Err(e) => println!("Error shutting down node '{}': {}", node_name, e),
                        }
                    }
                }
            }

            // Clean up registry file and heartbeats
            self.cleanup_registry();
            Self::cleanup_heartbeats();

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
    /// Enable/disable logging for a specific node
    pub fn set_node_logging(&mut self, name: &str, enabled: bool) -> bool {
        for registered in &mut self.nodes {
            if registered.node.name() == name {
                registered.logging_enabled = enabled;
                println!("Set logging for node '{}' to: {}", name, enabled);
                return true;
            }
        }
        false
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

    /// Write heartbeat for a node
    fn write_heartbeat(node_name: &str, context: &NodeInfo) {
        let heartbeat = NodeHeartbeat::from_metrics(context.state().clone(), context.metrics());

        let _ = heartbeat.write_to_file(node_name);
    }

    /// Clean up all heartbeat files
    fn cleanup_heartbeats() {
        let dir = PathBuf::from("/dev/shm/horus/heartbeats");
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
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
}
