use crate::core::{Node, NodeInfo, NodeState, NodeHeartbeat};
use crate::error::HorusResult;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Enhanced node registration info with lifecycle tracking
struct RegisteredNode {
    node: Box<dyn Node>,
    priority: u32,
    logging_enabled: bool,
    initialized: bool,
    context: Option<NodeInfo>,
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

    /// Register a node under numeric `priority` (0 = highest).
    /// If users only use register(node, priority) then logging defaults to false
    pub fn register(&mut self, node: Box<dyn Node>, priority: u32, logging_enabled: Option<bool>) -> &mut Self {
        let node_name = node.name().to_string();
        let logging_enabled = logging_enabled.unwrap_or(false);
        
        let context = NodeInfo::new(node_name.clone(), logging_enabled);
        
        self.nodes.push(RegisteredNode {
            node,
            priority,
            logging_enabled,
            initialized: false,
            context: Some(context),
        });
        
        println!("Registered node '{}' with priority {} (logging: {})", 
                 node_name, priority, logging_enabled);
        
        self
    }
    
    /// Set the scheduler name (chainable)
    pub fn name(mut self, name: &str) -> Self {
        self.scheduler_name = name.to_string();
        self
    }
    
    /// Tick specific nodes by name (runs continuously with the specified nodes)
    pub fn tick_node(&mut self, node_names: &[&str]) -> HorusResult<()> {
        // Use the same pattern as tick_all() but with node filtering
        self.run_with_filter(Some(node_names))
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

    /// Main loop with automatic signal handling and cleanup
    pub fn tick_all(&mut self) -> HorusResult<()> {
        self.run_with_filter(None)
    }

    /// Internal method to run scheduler with optional node filtering
    fn run_with_filter(&mut self, node_filter: Option<&[&str]>) -> HorusResult<()> {
        // Create tokio runtime for nodes that need async
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        
        rt.block_on(async {
            // Set up signal handling
            let running = self.running.clone();
            ctrlc::set_handler(move || {
                eprintln!("\n🛑 Ctrl+C received! Shutting down HORUS scheduler...");
                if let Ok(mut r) = running.lock() {
                    *r = false;
                }
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    eprintln!("🚪 Force terminating application...");
                    std::process::exit(0);
                });
            }).expect("Error setting HORUS signal handler");
            
            // Initialize nodes
            for registered in self.nodes.iter_mut() {
                let node_name = registered.node.name();
                let should_run = node_filter.map_or(true, |filter| filter.contains(&node_name));
                
                if should_run && !registered.initialized {
                    if let Some(ref mut ctx) = registered.context {
                        match registered.node.init(ctx) {
                            Ok(()) => {
                                registered.initialized = true;
                                println!("Initialized node '{}'", node_name);
                            },
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
                let now = Instant::now();
                self.last_instant = now;
                
                // Process nodes in priority order
                self.nodes.sort_by_key(|r| r.priority);
                for registered in self.nodes.iter_mut() {
                    let node_name = registered.node.name();
                    let should_run = node_filter.map_or(true, |filter| filter.contains(&node_name));
                    
                    if should_run && registered.initialized {
                        if let Some(ref mut context) = registered.context {
                            context.start_tick();
                            registered.node.tick(Some(context));
                            context.record_tick();

                            // Write heartbeat after each tick
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
                let should_run = node_filter.map_or(true, |filter| filter.contains(&node_name));
                
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
        self.nodes.iter()
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
                info.insert("logging_enabled".to_string(), registered.logging_enabled.to_string());
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
        self.nodes.iter()
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
        let dir = PathBuf::from("/dev/shm/horus/heartbeats");
        let _ = fs::create_dir_all(&dir);
    }

    /// Write heartbeat for a node
    fn write_heartbeat(node_name: &str, context: &NodeInfo) {
        let heartbeat = NodeHeartbeat::from_metrics(
            context.state().clone(),
            context.metrics()
        );

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
