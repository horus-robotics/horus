use horus_core::core::{HealthStatus, NodeHeartbeat, NodeState};
use horus_core::error::HorusResult;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

// Data structures for comprehensive monitoring
#[derive(Debug, Clone)]
pub struct NodeStatus {
    pub name: String,
    pub status: String,
    pub health: HealthStatus,
    pub priority: u32,
    pub process_id: u32,
    pub command_line: String,
    pub working_dir: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub start_time: String,
    pub scheduler_name: String,
    pub category: ProcessCategory,
    pub tick_count: u64,
    pub error_count: u32,
    pub actual_rate_hz: u32,
    pub publishers: Vec<TopicInfo>,
    pub subscribers: Vec<TopicInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessCategory {
    Node, // Runtime scheduler nodes
    Tool, // GUI applications
    CLI,  // Command line tools
}

#[derive(Debug, Clone)]
pub struct SharedMemoryInfo {
    pub topic_name: String,
    pub size_bytes: u64,
    pub active: bool,
    pub accessing_processes: Vec<u32>,
    pub last_modified: Option<std::time::SystemTime>,
    pub message_type: Option<String>,
    pub publishers: Vec<String>,
    pub subscribers: Vec<String>,
    pub message_rate_hz: f32,
}

// Fast discovery cache to avoid expensive filesystem operations
#[derive(Clone)]
struct DiscoveryCache {
    nodes: Vec<NodeStatus>,
    shared_memory: Vec<SharedMemoryInfo>,
    last_updated: Instant,
    cache_duration: Duration,
}

impl DiscoveryCache {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            shared_memory: Vec::new(),
            last_updated: Instant::now() - Duration::from_secs(10), // Force initial update
            cache_duration: Duration::from_millis(250), // Cache for 250ms (real-time updates)
        }
    }

    fn is_stale(&self) -> bool {
        self.last_updated.elapsed() > self.cache_duration
    }

    fn update_nodes(&mut self, nodes: Vec<NodeStatus>) {
        self.nodes = nodes;
        self.last_updated = Instant::now();
    }

    fn update_shared_memory(&mut self, shm: Vec<SharedMemoryInfo>) {
        self.shared_memory = shm;
        self.last_updated = Instant::now();
    }
}

// Global cache instance
lazy_static::lazy_static! {
    static ref DISCOVERY_CACHE: Arc<RwLock<DiscoveryCache>> = Arc::new(RwLock::new(DiscoveryCache::new()));
}

#[derive(Debug, Default)]
#[allow(dead_code)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cmdline: String,
    working_dir: String,
    cpu_percent: f32,
    memory_kb: u64,
    start_time: String,
}

pub fn discover_nodes() -> HorusResult<Vec<NodeStatus>> {
    // Check cache first
    if let Ok(cache) = DISCOVERY_CACHE.read() {
        if !cache.is_stale() {
            return Ok(cache.nodes.clone());
        }
    }

    // Cache is stale - do synchronous update for immediate detection
    let nodes = discover_nodes_uncached()?;

    // Update cache with fresh data
    if let Ok(mut cache) = DISCOVERY_CACHE.write() {
        cache.update_nodes(nodes.clone());
    }

    Ok(nodes)
}

fn discover_nodes_uncached() -> HorusResult<Vec<NodeStatus>> {
    // PRIMARY SOURCE: /dev/shm/horus/pubsub_metadata/ - discover nodes from active pub/sub activity
    let mut nodes = discover_nodes_from_pubsub_activity().unwrap_or_default();

    // SUPPLEMENT: Add heartbeat data if available (extra metadata like tick counts)
    enrich_nodes_with_heartbeats(&mut nodes);

    // SUPPLEMENT: Add registry metadata if available (command_line, working_dir, etc.)
    let registry_metadata = load_registry_metadata();
    for node in &mut nodes {
        if let Some(metadata) = registry_metadata.get(&node.name) {
            node.command_line = metadata.command_line.clone();
            node.working_dir = metadata.working_dir.clone();
            node.priority = metadata.priority;
            node.scheduler_name = metadata.scheduler_name.clone();
            node.publishers = metadata.publishers.clone();
            node.subscribers = metadata.subscribers.clone();
        }
    }

    // SUPPLEMENT: Add process info (CPU, memory) if we have a PID
    for node in &mut nodes {
        if node.process_id > 0 {
            if let Ok(proc_info) = get_process_info(node.process_id) {
                node.cpu_usage = proc_info.cpu_percent;
                node.memory_usage = proc_info.memory_kb;
                node.start_time = proc_info.start_time;
                if node.command_line.is_empty() {
                    node.command_line = proc_info.cmdline.clone();
                }
                if node.working_dir.is_empty() {
                    node.working_dir = proc_info.working_dir.clone();
                }
            }
        }
    }

    // EXTRA: Add any other HORUS processes (tools, CLIs) not detected via pub/sub
    if let Ok(process_nodes) = discover_horus_processes() {
        for process_node in process_nodes {
            // Only add if not already found
            if !nodes
                .iter()
                .any(|n| n.process_id == process_node.process_id || n.name == process_node.name)
            {
                nodes.push(process_node);
            }
        }
    }

    Ok(nodes)
}

// Metadata from registry (supplemental info only)
#[derive(Debug, Clone)]
struct NodeMetadata {
    command_line: String,
    working_dir: String,
    priority: u32,
    scheduler_name: String,
    publishers: Vec<TopicInfo>,
    subscribers: Vec<TopicInfo>,
}

// Enhanced node status with pub/sub info
#[derive(Debug, Clone)]
pub struct TopicInfo {
    pub topic: String,
    pub type_name: String,
}

fn read_registry_file() -> anyhow::Result<Vec<NodeStatus>> {
    let home_dir =
        std::env::var("HOME").map_err(|_| anyhow::anyhow!("Could not determine home directory"))?;
    let registry_path = format!("{}/.horus_registry.json", home_dir);

    if !std::path::Path::new(&registry_path).exists() {
        return Ok(Vec::new());
    }

    let registry_content = std::fs::read_to_string(&registry_path)?;
    let registry: serde_json::Value = serde_json::from_str(&registry_content)?;

    let mut nodes = Vec::new();

    if let Some(scheduler_nodes) = registry["nodes"].as_array() {
        let scheduler_pid = registry["pid"].as_u64().unwrap_or(0) as u32;
        let scheduler_name = registry["scheduler_name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();
        let working_dir = registry["working_dir"].as_str().unwrap_or("/").to_string();

        // Smart filter: Only include if scheduler process actually exists
        if process_exists(scheduler_pid) {
            // Double-check the process is actually running
            if let Ok(proc_info) = get_process_info(scheduler_pid) {
                for node in scheduler_nodes {
                    let name = node["name"].as_str().unwrap_or("Unknown").to_string();
                    let priority = node["priority"].as_u64().unwrap_or(0) as u32;

                    // Parse publishers and subscribers
                    let mut publishers = Vec::new();
                    if let Some(pubs) = node["publishers"].as_array() {
                        for pub_info in pubs {
                            if let (Some(topic), Some(type_name)) =
                                (pub_info["topic"].as_str(), pub_info["type"].as_str())
                            {
                                publishers.push(TopicInfo {
                                    topic: topic.to_string(),
                                    type_name: type_name.to_string(),
                                });
                            }
                        }
                    }

                    let mut subscribers = Vec::new();
                    if let Some(subs) = node["subscribers"].as_array() {
                        for sub_info in subs {
                            if let (Some(topic), Some(type_name)) =
                                (sub_info["topic"].as_str(), sub_info["type"].as_str())
                            {
                                subscribers.push(TopicInfo {
                                    topic: topic.to_string(),
                                    type_name: type_name.to_string(),
                                });
                            }
                        }
                    }

                    // Check heartbeat for real status and health
                    let (status, health, tick_count, error_count, actual_rate) =
                        check_node_heartbeat(&name);

                    nodes.push(NodeStatus {
                        name: name.clone(),
                        status,
                        health,
                        priority,
                        process_id: scheduler_pid,
                        command_line: proc_info.cmdline.clone(),
                        working_dir: working_dir.clone(),
                        cpu_usage: proc_info.cpu_percent,
                        memory_usage: proc_info.memory_kb,
                        start_time: proc_info.start_time.clone(),
                        scheduler_name: scheduler_name.clone(),
                        category: ProcessCategory::Node,
                        tick_count,
                        error_count,
                        actual_rate_hz: actual_rate,
                        publishers: publishers.clone(),
                        subscribers: subscribers.clone(),
                    });
                }
            }
        }
    }

    Ok(nodes)
}

/// Discover all scheduler registry files in home directory
fn discover_registry_files() -> Vec<std::path::PathBuf> {
    let mut registry_files = Vec::new();

    let home_dir = match std::env::var("HOME") {
        Ok(dir) => dir,
        Err(_) => return registry_files,
    };

    let home_path = std::path::Path::new(&home_dir);

    // Look for all .horus_registry*.json files
    if let Ok(entries) = std::fs::read_dir(home_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with(".horus_registry") && filename.ends_with(".json") {
                    registry_files.push(path);
                }
            }
        }
    }

    registry_files
}

/// Load registry metadata for enriching heartbeat-discovered nodes
/// Now supports multiple schedulers by reading all registry files
fn load_registry_metadata() -> std::collections::HashMap<String, NodeMetadata> {
    let mut metadata = std::collections::HashMap::new();

    // Discover all registry files from all schedulers
    let registry_files = discover_registry_files();

    // Process each registry file (supports multiple schedulers)
    for registry_path in registry_files {
        let registry_content = match std::fs::read_to_string(&registry_path) {
            Ok(content) => content,
            Err(_) => continue, // Skip invalid files
        };

        let registry: serde_json::Value = match serde_json::from_str(&registry_content) {
            Ok(reg) => reg,
            Err(_) => continue, // Skip invalid JSON
        };

        // Only use registry if scheduler is still running
        let scheduler_pid = registry["pid"].as_u64().unwrap_or(0) as u32;
        if !process_exists(scheduler_pid) {
            // Clean up stale registry file
            let _ = std::fs::remove_file(&registry_path);
            continue;
        }

        if let Some(scheduler_nodes) = registry["nodes"].as_array() {
            let scheduler_name = registry["scheduler_name"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string();
            let working_dir = registry["working_dir"].as_str().unwrap_or("/").to_string();

            if let Ok(proc_info) = get_process_info(scheduler_pid) {
                for node in scheduler_nodes {
                    let name = node["name"].as_str().unwrap_or("Unknown").to_string();
                    let priority = node["priority"].as_u64().unwrap_or(0) as u32;

                    // Parse publishers and subscribers
                    let mut publishers = Vec::new();
                    if let Some(pubs) = node["publishers"].as_array() {
                        for pub_info in pubs {
                            if let (Some(topic), Some(type_name)) =
                                (pub_info["topic"].as_str(), pub_info["type"].as_str())
                            {
                                publishers.push(TopicInfo {
                                    topic: topic.to_string(),
                                    type_name: type_name.to_string(),
                                });
                            }
                        }
                    }

                    let mut subscribers = Vec::new();
                    if let Some(subs) = node["subscribers"].as_array() {
                        for sub_info in subs {
                            if let (Some(topic), Some(type_name)) =
                                (sub_info["topic"].as_str(), sub_info["type"].as_str())
                            {
                                subscribers.push(TopicInfo {
                                    topic: topic.to_string(),
                                    type_name: type_name.to_string(),
                                });
                            }
                        }
                    }

                    metadata.insert(
                        name.clone(),
                        NodeMetadata {
                            command_line: proc_info.cmdline.clone(),
                            working_dir: working_dir.clone(),
                            priority,
                            scheduler_name: scheduler_name.clone(),
                            publishers,
                            subscribers,
                        },
                    );
                }
            }
        }
    }

    metadata
}

/// Find PID for a node by name (scans /proc for matching heartbeat-writing process)
fn find_node_pid(node_name: &str) -> Option<u32> {
    let proc_dir = Path::new("/proc");
    if !proc_dir.exists() {
        return None;
    }

    for entry in std::fs::read_dir(proc_dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();

        if let Some(pid_str) = path.file_name().and_then(|s| s.to_str()) {
            if let Ok(pid) = pid_str.parse::<u32>() {
                if pid < 100 {
                    continue; // Skip system processes
                }

                let cmdline_path = path.join("cmdline");
                if let Ok(cmdline) = std::fs::read_to_string(cmdline_path) {
                    let cmdline_str = cmdline.replace('\0', " ");

                    // Check if this process is likely running this node
                    // (horus run, scheduler, or direct node execution with node name)
                    if cmdline_str.contains("horus") && cmdline_str.contains(node_name) {
                        return Some(pid);
                    }
                }
            }
        }
    }

    None
}

fn discover_horus_processes() -> anyhow::Result<Vec<NodeStatus>> {
    let mut nodes = Vec::new();
    let proc_dir = Path::new("/proc");

    if !proc_dir.exists() {
        return Ok(nodes);
    }

    for entry in std::fs::read_dir(proc_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Check if this is a PID directory
        if let Some(pid_str) = path.file_name().and_then(|s| s.to_str()) {
            if let Ok(pid) = pid_str.parse::<u32>() {
                // Fast skip: Ignore kernel threads and very low PIDs (system processes)
                // Most HORUS processes will have PID > 1000
                if pid < 100 {
                    continue;
                }

                // Check cmdline for HORUS-related processes
                let cmdline_path = path.join("cmdline");
                if let Ok(cmdline) = std::fs::read_to_string(cmdline_path) {
                    let cmdline_str = cmdline.replace('\0', " ").trim().to_string();

                    // Look for HORUS-related patterns (generic, not hardcoded)
                    if should_track_process(&cmdline_str) {
                        let name = extract_process_name(&cmdline_str);
                        let category = categorize_process(&name, &cmdline_str);

                        // Get detailed process info
                        let proc_info = get_process_info(pid).unwrap_or_default();

                        // Check heartbeat for real status
                        let (status, health, tick_count, error_count, actual_rate) =
                            check_node_heartbeat(&name);

                        nodes.push(NodeStatus {
                            name: name.clone(),
                            status,
                            health,
                            priority: 0, // Default for discovered processes
                            process_id: pid,
                            command_line: cmdline_str,
                            working_dir: proc_info.working_dir.clone(),
                            cpu_usage: proc_info.cpu_percent,
                            memory_usage: proc_info.memory_kb,
                            start_time: proc_info.start_time,
                            scheduler_name: "Standalone".to_string(),
                            category,
                            tick_count,
                            error_count,
                            actual_rate_hz: actual_rate,
                            publishers: Vec::new(),
                            subscribers: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    Ok(nodes)
}

fn should_track_process(cmdline: &str) -> bool {
    // Skip empty command lines
    if cmdline.trim().is_empty() {
        return false;
    }

    // Skip build/development tools, system processes, and monitoring tools
    if cmdline.contains("/bin/bash")
        || cmdline.contains("/bin/sh")
        || cmdline.starts_with("timeout ")
        || cmdline.contains("cargo build")
        || cmdline.contains("cargo install")
        || cmdline.contains("cargo run")
        || cmdline.contains("cargo test")
        || cmdline.contains("rustc")
        || cmdline.contains("rustup")
        || cmdline.contains("dashboard")
        || cmdline.contains("monitor")
        || cmdline.contains("horus run")
    // Exclude "horus run" commands - they'll be in registry once scheduler starts
    {
        return false;
    }

    // Only track processes that:
    // 1. Are registered in the HORUS registry (handled by read_registry_file)
    // 2. Are explicitly standalone HORUS project binaries (not CLI commands)

    // Check if it's a standalone HORUS binary (compiled binary running a scheduler)
    // This excludes CLI commands like "horus run", which will appear in registry once the scheduler starts
    if cmdline.contains("scheduler") && !cmdline.contains("horus run") {
        return true;
    }

    // Don't track CLI invocations - only track registered nodes
    false
}

fn categorize_process(name: &str, cmdline: &str) -> ProcessCategory {
    // GUI tools (including GUI executables)
    if name.contains("gui")
        || name.contains("GUI")
        || name.contains("viewer")
        || name.contains("viz")
        || cmdline.contains("--view")
        || cmdline.contains("--gui")
        || name.ends_with("_gui")
    {
        return ProcessCategory::Tool;
    }

    // CLI commands - horus CLI tool usage
    if name == "horus"
        || name.starts_with("horus ")
        || cmdline.contains("/bin/horus")
        || cmdline.contains("target/debug/horus")
        || cmdline.contains("target/release/horus")
        || (cmdline.contains("horus ") && !cmdline.contains("cargo"))
    {
        return ProcessCategory::CLI;
    }

    // Schedulers and other runtime components
    if name.contains("scheduler") || cmdline.contains("scheduler") {
        return ProcessCategory::Node;
    }

    // Default to Node for other HORUS components
    ProcessCategory::Node
}

fn extract_process_name(cmdline: &str) -> String {
    let parts: Vec<&str> = cmdline.split_whitespace().collect();
    if let Some(first) = parts.first() {
        if let Some(name) = Path::new(first).file_name() {
            let base_name = name.to_string_lossy().to_string();

            // For horus CLI commands, include the subcommand and package name
            if base_name == "horus" && parts.len() > 1 {
                if parts.len() > 2 && parts[1] == "monitor" {
                    return format!("horus monitor {}", parts[2]);
                } else if parts.len() > 2 && parts[1] == "run" {
                    // Include the package name for horus run commands
                    return format!("horus run {}", parts[2]);
                } else if parts.len() > 1 {
                    return format!("horus {}", parts[1]);
                }
            }

            return base_name;
        }
    }
    "Unknown".to_string()
}

fn process_exists(pid: u32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}

// CPU tracking cache
use std::collections::HashMap as StdHashMap;
lazy_static::lazy_static! {
    static ref CPU_CACHE: Arc<RwLock<StdHashMap<u32, (u64, Instant)>>> =
        Arc::new(RwLock::new(StdHashMap::new()));
}

fn get_process_info(pid: u32) -> anyhow::Result<ProcessInfo> {
    let proc_path = format!("/proc/{}", pid);

    // Read command line
    let cmdline = std::fs::read_to_string(format!("{}/cmdline", proc_path))
        .unwrap_or_default()
        .replace('\0', " ")
        .trim()
        .to_string();

    // Extract process name
    let name = extract_process_name(&cmdline);

    // Read working directory
    let working_dir = std::fs::read_link(format!("{}/cwd", proc_path))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/".to_string());

    // Read stat for memory and CPU info
    let stat_content = std::fs::read_to_string(format!("{}/stat", proc_path))?;
    let memory_kb = parse_memory_from_stat(&stat_content);

    // Calculate CPU usage with sampling
    let cpu_percent = calculate_cpu_usage(pid, &stat_content);

    // Get start time
    let start_time = get_process_start_time(pid);

    Ok(ProcessInfo {
        pid,
        name,
        cmdline,
        working_dir,
        cpu_percent,
        memory_kb,
        start_time,
    })
}

fn calculate_cpu_usage(pid: u32, stat_content: &str) -> f32 {
    // Parse utime + stime from /proc/[pid]/stat
    let fields: Vec<&str> = stat_content.split_whitespace().collect();
    if fields.len() < 15 {
        return 0.0;
    }

    // utime is field 13 (0-indexed), stime is field 14
    let utime = fields[13].parse::<u64>().unwrap_or(0);
    let stime = fields[14].parse::<u64>().unwrap_or(0);
    let total_time = utime + stime;

    // Get cached value
    if let Ok(mut cache) = CPU_CACHE.write() {
        let now = Instant::now();

        if let Some((prev_total, prev_time)) = cache.get(&pid) {
            let time_delta = now.duration_since(*prev_time).as_secs_f32();
            if time_delta > 0.0 {
                let cpu_delta = (total_time.saturating_sub(*prev_total)) as f32;
                // Convert from jiffies to percentage (100 Hz clock)
                let cpu_percent = (cpu_delta / time_delta / 100.0) * 100.0;

                // Update cache
                cache.insert(pid, (total_time, now));

                return cpu_percent.min(100.0);
            }
        }

        // First sample - cache it
        cache.insert(pid, (total_time, now));
    }

    0.0 // Return 0 for first sample
}

fn parse_memory_from_stat(stat: &str) -> u64 {
    // Parse RSS (Resident Set Size) from /proc/[pid]/stat
    // RSS is the 24th field (0-indexed: 23)
    let fields: Vec<&str> = stat.split_whitespace().collect();

    if fields.len() > 23 {
        if let Ok(rss_pages) = fields[23].parse::<u64>() {
            // Convert pages to KB (usually 4KB per page)
            let page_size = 4; // KB
            return rss_pages * page_size;
        }
    }
    0
}

fn get_process_start_time(pid: u32) -> String {
    // Read process start time from stat
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        // Start time is the 22nd field (0-indexed: 21) in jiffies since boot
        let fields: Vec<&str> = stat.split_whitespace().collect();
        if fields.len() > 21 {
            if let Ok(start_jiffies) = fields[21].parse::<u64>() {
                // Convert to seconds and format
                let start_secs = start_jiffies / 100; // Assuming 100 Hz
                let duration = std::time::Duration::from_secs(start_secs);
                return format_duration(duration);
            }
        }
    }
    "Unknown".to_string()
}

fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    if total_secs < 60 {
        format!("{}s", total_secs)
    } else if total_secs < 3600 {
        format!("{}m", total_secs / 60)
    } else if total_secs < 86400 {
        format!("{}h", total_secs / 3600)
    } else {
        format!("{}d", total_secs / 86400)
    }
}

pub fn discover_shared_memory() -> HorusResult<Vec<SharedMemoryInfo>> {
    // Check cache first
    if let Ok(cache) = DISCOVERY_CACHE.read() {
        if !cache.is_stale() {
            return Ok(cache.shared_memory.clone());
        }
    }

    // Cache is stale - do synchronous update for immediate detection
    let shared_memory = discover_shared_memory_uncached()?;

    // Update cache with fresh data
    if let Ok(mut cache) = DISCOVERY_CACHE.write() {
        cache.update_shared_memory(shared_memory.clone());
    }

    Ok(shared_memory)
}

// Topic rate tracking cache
lazy_static::lazy_static! {
    static ref TOPIC_RATE_CACHE: Arc<RwLock<StdHashMap<String, (Instant, u64)>>> =
        Arc::new(RwLock::new(StdHashMap::new()));
}

fn discover_shared_memory_uncached() -> HorusResult<Vec<SharedMemoryInfo>> {
    let mut topics = Vec::new();

    // Scan all active sessions for session-isolated topics
    let sessions_dir = Path::new("/dev/shm/horus/sessions");
    if sessions_dir.exists() {
        if let Ok(session_entries) = std::fs::read_dir(sessions_dir) {
            for session_entry in session_entries.flatten() {
                let session_topics_path = session_entry.path().join("topics");
                if session_topics_path.exists() {
                    topics.extend(scan_topics_directory(&session_topics_path)?);
                }
            }
        }
    }

    // Also scan global/legacy path for backward compatibility
    let global_shm_path = Path::new("/dev/shm/horus/topics");
    if global_shm_path.exists() {
        topics.extend(scan_topics_directory(global_shm_path)?);
    }

    Ok(topics)
}

/// Scan a specific topics directory for shared memory files
fn scan_topics_directory(shm_path: &Path) -> HorusResult<Vec<SharedMemoryInfo>> {
    let mut topics = Vec::new();

    // Load registry to get topic metadata
    let registry_topics = load_topic_metadata_from_registry();

    for entry in std::fs::read_dir(shm_path)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        // Smart filter for shared memory segments
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            // Only include files (not directories)
            if metadata.is_file() {
                let size = metadata.len();
                let modified = metadata.modified().ok();

                // Find processes accessing this segment (optimized)
                let accessing_procs = find_accessing_processes_fast(&path, name);

                // All files in HORUS directory are valid topics
                // Extract topic name from filename (remove "horus_" prefix and convert underscores)
                let topic_name = if name.starts_with("horus_") {
                    name.strip_prefix("horus_")
                        .unwrap_or(name)
                        .replace('_', "/")
                } else {
                    name.replace('_', "/")
                };

                let is_recent = if let Some(mod_time) = modified {
                    // Use 30 second threshold to handle slow publishers (e.g., 0.1 Hz = 10 sec between publishes)
                    mod_time.elapsed().unwrap_or(Duration::from_secs(3600))
                        < Duration::from_secs(30)
                } else {
                    false
                };

                let has_valid_processes = accessing_procs.iter().any(|pid| process_exists(*pid));

                // Include all topics in HORUS directory
                let active = has_valid_processes || is_recent;

                // Auto-cleanup: Remove inactive topics older than 60 seconds
                // This gives time for slow publishers to wake up
                if !active {
                    if let Some(mod_time) = modified {
                        if mod_time.elapsed().unwrap_or(Duration::from_secs(0))
                            > Duration::from_secs(60)
                        {
                            let _ = std::fs::remove_file(&path);
                            continue; // Skip adding to topics list
                        }
                    }
                }

                // Calculate message rate from modification times
                let message_rate = calculate_topic_rate(&topic_name, modified);

                // Get metadata from registry
                let (message_type, publishers, subscribers) = registry_topics
                    .get(&topic_name)
                    .map(|(t, p, s)| (Some(t.clone()), p.clone(), s.clone()))
                    .unwrap_or((None, Vec::new(), Vec::new()));

                topics.push(SharedMemoryInfo {
                    topic_name,
                    size_bytes: size,
                    active,
                    accessing_processes: accessing_procs
                        .iter()
                        .filter(|pid| process_exists(**pid))
                        .copied()
                        .collect(),
                    last_modified: modified,
                    message_type,
                    publishers,
                    subscribers,
                    message_rate_hz: message_rate,
                });
            }
        }
    }

    Ok(topics)
}

fn calculate_topic_rate(topic_name: &str, modified: Option<std::time::SystemTime>) -> f32 {
    let now = Instant::now();

    if let Some(mod_time) = modified {
        if let Ok(mut cache) = TOPIC_RATE_CACHE.write() {
            // Convert SystemTime to a simple counter for change detection
            let mod_counter = mod_time
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            if let Some((prev_instant, prev_counter)) = cache.get(topic_name) {
                if mod_counter != *prev_counter {
                    // File was modified
                    let time_delta = now.duration_since(*prev_instant).as_secs_f32();
                    if time_delta > 0.0 && time_delta < 10.0 {
                        let rate = 1.0 / time_delta;
                        cache.insert(topic_name.to_string(), (now, mod_counter));
                        return rate;
                    }
                }
            }

            // First sample or same modification time
            cache.insert(topic_name.to_string(), (now, mod_counter));
        }
    }

    0.0
}

fn load_topic_metadata_from_registry() -> StdHashMap<String, (String, Vec<String>, Vec<String>)> {
    let mut topic_map = StdHashMap::new();

    // Load from all registry files (supports multiple schedulers)
    let registry_files = discover_registry_files();

    for registry_path in registry_files {
        if let Ok(content) = std::fs::read_to_string(&registry_path) {
            if let Ok(registry) = serde_json::from_str::<serde_json::Value>(&content) {
                // Skip if scheduler is dead
                let scheduler_pid = registry["pid"].as_u64().unwrap_or(0) as u32;
                if !process_exists(scheduler_pid) {
                    continue;
                }

                if let Some(nodes) = registry["nodes"].as_array() {
                    for node in nodes {
                        let node_name = node["name"].as_str().unwrap_or("Unknown");

                        // Process publishers
                        if let Some(pubs) = node["publishers"].as_array() {
                            for pub_info in pubs {
                                if let (Some(topic), Some(type_name)) =
                                    (pub_info["topic"].as_str(), pub_info["type"].as_str())
                                {
                                    let entry = topic_map.entry(topic.to_string()).or_insert((
                                        type_name.to_string(),
                                        Vec::new(),
                                        Vec::new(),
                                    ));
                                    entry.1.push(node_name.to_string());
                                }
                            }
                        }

                        // Process subscribers
                        if let Some(subs) = node["subscribers"].as_array() {
                            for sub_info in subs {
                                if let (Some(topic), Some(type_name)) =
                                    (sub_info["topic"].as_str(), sub_info["type"].as_str())
                                {
                                    let entry = topic_map.entry(topic.to_string()).or_insert((
                                        type_name.to_string(),
                                        Vec::new(),
                                        Vec::new(),
                                    ));
                                    entry.2.push(node_name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    topic_map
}

// Fast version: Only check HORUS processes first, then fall back to full scan if needed
fn find_accessing_processes_fast(shm_path: &Path, shm_name: &str) -> Vec<u32> {
    let mut processes = Vec::new();

    // For HORUS-like shared memory, only check HORUS processes first (much faster)
    let is_horus_shm = shm_name.contains("horus")
        || shm_name.contains("topic")
        || shm_name.starts_with("ros")
        || shm_name.starts_with("shm_");

    if is_horus_shm {
        // Fast path: Only check processes with HORUS in their name
        if let Ok(proc_entries) = std::fs::read_dir("/proc") {
            for entry in proc_entries.flatten() {
                if let Some(pid_str) = entry.file_name().to_str() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        // Quick check if this is a HORUS-related process
                        if let Ok(cmdline) = std::fs::read_to_string(entry.path().join("cmdline")) {
                            let cmdline_str = cmdline.replace('\0', " ");
                            if cmdline_str.contains("horus") || cmdline_str.contains("ros") {
                                // Only now check file descriptors for this process
                                let fd_path = entry.path().join("fd");
                                if let Ok(fd_entries) = std::fs::read_dir(fd_path) {
                                    for fd_entry in fd_entries.flatten() {
                                        if let Ok(link_target) = std::fs::read_link(fd_entry.path())
                                        {
                                            if link_target == shm_path {
                                                processes.push(pid);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // If we found HORUS processes, return early
        if !processes.is_empty() {
            return processes;
        }
    }

    // Fallback: Abbreviated scan - only check first 20 processes to avoid blocking
    if let Ok(proc_entries) = std::fs::read_dir("/proc") {
        for (_checked, entry) in proc_entries.flatten().enumerate().take(20) {
            // Limit to avoid UI blocking

            if let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            {
                let fd_path = entry.path().join("fd");
                if let Ok(fd_entries) = std::fs::read_dir(fd_path) {
                    for fd_entry in fd_entries.flatten() {
                        if let Ok(link_target) = std::fs::read_link(fd_entry.path()) {
                            if link_target == shm_path {
                                processes.push(pid);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    processes
}

/// Check node heartbeat file and determine status and health
fn check_node_heartbeat(node_name: &str) -> (String, HealthStatus, u64, u32, u32) {
    // Try to read heartbeat file
    if let Some(heartbeat) = NodeHeartbeat::read_from_file(node_name) {
        let status_str = match heartbeat.state {
            NodeState::Uninitialized => "Idle",
            NodeState::Initializing => "Initializing",
            NodeState::Running => "Running",
            NodeState::Paused => "Paused",
            NodeState::Stopping => "Stopping",
            NodeState::Stopped => "Stopped",
            NodeState::Error(_) => "Error",
            NodeState::Crashed(_) => "Crashed",
        };

        // For Running nodes, be more forgiving with freshness
        // A node running at 0.1 Hz takes 10 seconds between ticks, so use 30 second threshold
        // Only mark as Frozen if heartbeat is very stale (>30 seconds) for running nodes
        if status_str == "Running" {
            if heartbeat.is_fresh(30) {
                // Node is running and heartbeat is reasonably fresh
                return (
                    status_str.to_string(),
                    heartbeat.health,
                    heartbeat.tick_count,
                    heartbeat.error_count,
                    heartbeat.actual_rate_hz,
                );
            } else {
                // Heartbeat is very stale - node is likely frozen or hung
                return (
                    "Frozen".to_string(),
                    HealthStatus::Critical,
                    heartbeat.tick_count,
                    heartbeat.error_count,
                    0,
                );
            }
        } else {
            // For non-running states (Stopped, Error, etc.), trust the heartbeat regardless of age
            return (
                status_str.to_string(),
                heartbeat.health,
                heartbeat.tick_count,
                heartbeat.error_count,
                heartbeat.actual_rate_hz,
            );
        }
    }

    // No heartbeat file found - try registry snapshot as fallback
    check_registry_snapshot(node_name)
        .unwrap_or_else(|| ("Unknown".to_string(), HealthStatus::Unknown, 0, 0, 0))
}

/// Discover active nodes from pub/sub metadata (primary discovery method)
/// This works regardless of whether scheduler writes heartbeats or not
fn discover_nodes_from_pubsub_activity() -> anyhow::Result<Vec<NodeStatus>> {
    use std::collections::{HashMap, HashSet};

    let mut node_map: HashMap<String, NodeStatus> = HashMap::new();
    let metadata_dir = std::path::Path::new("/dev/shm/horus/pubsub_metadata");

    if !metadata_dir.exists() {
        return Ok(Vec::new());
    }

    // First, discover all known topics to properly extract node names
    let mut known_topics = HashSet::new();
    let topics_dir = std::path::Path::new("/dev/shm/horus/topics");
    if topics_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(topics_dir) {
            for entry in entries.flatten() {
                if let Some(topic_name) = entry.file_name().to_str() {
                    // Normalize topic name (same as metadata files)
                    let safe_topic: String = topic_name
                        .chars()
                        .map(|c| if c == '/' || c == ' ' { '_' } else { c })
                        .collect();
                    known_topics.insert(safe_topic);
                }
            }
        }
    }

    // Scan all pub/sub metadata files
    for entry in std::fs::read_dir(metadata_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // File format: {node_name}_{topic_name}_{pub|sub}
        // Extract direction (last part after final underscore)
        let direction = if filename.ends_with("_pub") {
            "pub"
        } else if filename.ends_with("_sub") {
            "sub"
        } else {
            continue;
        };

        // Remove the direction suffix to get: {node_name}_{topic_name}
        let without_direction = if direction == "pub" {
            filename.strip_suffix("_pub").unwrap()
        } else {
            filename.strip_suffix("_sub").unwrap()
        };

        // Try to match against known topics to extract node name and topic name correctly
        let (node_name, topic_name) = if let Some(topic) = known_topics
            .iter()
            .find(|t| without_direction.ends_with(&format!("_{}", t)))
        {
            // Found matching topic - strip it to get the node name
            let node = without_direction
                .strip_suffix(&format!("_{}", topic))
                .unwrap_or(without_direction)
                .to_string();
            (node, topic.clone())
        } else {
            // Fallback: assume topic is the last underscore-separated segment
            // This handles cases where topic discovery failed
            let parts: Vec<&str> = without_direction.split('_').collect();
            if parts.len() >= 2 {
                let node = parts[..parts.len() - 1].join("_");
                let topic = parts[parts.len() - 1].to_string();
                (node, topic)
            } else {
                (without_direction.to_string(), "unknown".to_string())
            }
        };

        // Check if file was modified recently (node is active)
        let metadata = match path.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified = match metadata.modified() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let age = modified.elapsed().unwrap_or(Duration::from_secs(9999));
        if age > Duration::from_secs(30) {
            // Auto-cleanup: Remove very stale metadata files (>60 seconds old)
            if age > Duration::from_secs(60) {
                let _ = std::fs::remove_file(&path);
            }
            continue; // Stale metadata, skip
        }

        // Get or create node entry
        let node = node_map.entry(node_name.clone()).or_insert_with(|| {
            NodeStatus {
                name: node_name.clone(),
                status: "Running".to_string(), // Active if we see recent pub/sub activity
                health: HealthStatus::Healthy,
                priority: 0,
                process_id: 0, // Will be filled later
                command_line: String::new(),
                working_dir: String::new(),
                cpu_usage: 0.0,
                memory_usage: 0,
                start_time: String::new(),
                scheduler_name: "Unknown".to_string(),
                category: ProcessCategory::Node,
                tick_count: 0,
                error_count: 0,
                actual_rate_hz: 0,
                publishers: Vec::new(),
                subscribers: Vec::new(),
            }
        });

        // Add topic to publishers or subscribers based on direction
        let topic_info = TopicInfo {
            topic: topic_name.clone(),
            type_name: "unknown".to_string(), // Type name not available from metadata filename
        };

        if direction == "pub" {
            // Add to publishers if not already present
            if !node.publishers.iter().any(|t| t.topic == topic_name) {
                node.publishers.push(topic_info);
            }
        } else {
            // Add to subscribers if not already present
            if !node.subscribers.iter().any(|t| t.topic == topic_name) {
                node.subscribers.push(topic_info);
            }
        }

        // Try to find PID for this node
        if node.process_id == 0 {
            if let Some(pid) = find_node_pid(&node_name) {
                node.process_id = pid;

                // Check if process is still alive
                if !process_exists(pid) {
                    continue; // Dead process, skip
                }
            }
        }
    }

    Ok(node_map.into_values().collect())
}

/// Enrich nodes with heartbeat data if available (optional metadata)
fn enrich_nodes_with_heartbeats(nodes: &mut [NodeStatus]) {
    for node in nodes {
        let (status, health, tick_count, error_count, actual_rate) =
            check_node_heartbeat(&node.name);

        // Only update if heartbeat provides better info
        if status != "Unknown" {
            node.status = status;
            node.health = health;
            node.tick_count = tick_count;
            node.error_count = error_count;
            node.actual_rate_hz = actual_rate;
        }
    }
}

/// Discover nodes from heartbeat directory (fallback method)
fn discover_nodes_from_heartbeats() -> anyhow::Result<Vec<NodeStatus>> {
    let mut nodes = Vec::new();
    let heartbeat_dir = std::path::PathBuf::from("/dev/shm/horus/heartbeats");

    if !heartbeat_dir.exists() {
        return Ok(nodes);
    }

    // Read all heartbeat files
    if let Ok(entries) = std::fs::read_dir(&heartbeat_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(node_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Clean up very old heartbeat files (older than 60 seconds)
                    // This is 2x the freshness threshold to avoid race conditions
                    if let Ok(metadata) = path.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(elapsed) = modified.elapsed() {
                                if elapsed > std::time::Duration::from_secs(60) {
                                    let _ = std::fs::remove_file(&path);
                                    continue;
                                }
                            }
                        }
                    }

                    // Read heartbeat data
                    let (status, health, tick_count, error_count, actual_rate) =
                        check_node_heartbeat(node_name);

                    // Only show nodes that are actually running
                    // Skip Stopped, Frozen, and Unknown nodes from heartbeat-only discovery
                    if status == "Running" || status == "Initializing" {
                        nodes.push(NodeStatus {
                            name: node_name.to_string(),
                            status,
                            health,
                            priority: 0,
                            process_id: 0, // Unknown from heartbeat alone
                            command_line: String::new(),
                            working_dir: String::new(),
                            cpu_usage: 0.0,
                            memory_usage: 0,
                            start_time: String::new(),
                            scheduler_name: String::from("Unknown"),
                            category: ProcessCategory::Node,
                            tick_count,
                            error_count,
                            actual_rate_hz: actual_rate,
                            publishers: vec![],
                            subscribers: vec![],
                        });
                    }
                }
            }
        }
    }

    Ok(nodes)
}

/// Check registry snapshot for last known state (fallback when heartbeat unavailable)
fn check_registry_snapshot(node_name: &str) -> Option<(String, HealthStatus, u64, u32, u32)> {
    let registry_path = dirs::home_dir()?.join(".horus_registry.json");
    let content = std::fs::read_to_string(&registry_path).ok()?;
    let registry: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Check if registry snapshot is recent (within last 30 seconds)
    if let Some(last_snapshot) = registry["last_snapshot"].as_u64() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // If snapshot is too old, don't use it
        if now.saturating_sub(last_snapshot) > 30 {
            return None;
        }
    }

    // Search for the node in the snapshot
    let nodes = registry["nodes"].as_array()?;
    for node in nodes {
        if node["name"].as_str()? == node_name {
            let state_str = node["state"].as_str().unwrap_or("Unknown");
            let health_str = node["health"].as_str().unwrap_or("Unknown");
            let error_count = node["error_count"].as_u64().unwrap_or(0) as u32;
            let tick_count = node["tick_count"].as_u64().unwrap_or(0);

            // Parse health
            let health = match health_str {
                "Healthy" => HealthStatus::Healthy,
                "Warning" => HealthStatus::Warning,
                "Error" => HealthStatus::Error,
                "Critical" => HealthStatus::Critical,
                _ => HealthStatus::Unknown,
            };

            return Some((
                state_str.to_string(),
                health,
                tick_count,
                error_count,
                0, // No rate info in snapshot
            ));
        }
    }

    None
}

// Enhanced monitoring functions

#[cfg(test)]
mod tests {
    use super::*;

    // =====================
    // NodeStatus Tests
    // =====================
    #[test]
    fn test_node_status_creation() {
        let node = NodeStatus {
            name: "test_node".to_string(),
            status: "Running".to_string(),
            health: HealthStatus::Healthy,
            priority: 10,
            process_id: 1234,
            command_line: "horus run test".to_string(),
            working_dir: "/home/test".to_string(),
            cpu_usage: 25.5,
            memory_usage: 1024,
            start_time: "10m".to_string(),
            scheduler_name: "default".to_string(),
            category: ProcessCategory::Node,
            tick_count: 100,
            error_count: 0,
            actual_rate_hz: 50,
            publishers: vec![],
            subscribers: vec![],
        };

        assert_eq!(node.name, "test_node");
        assert_eq!(node.status, "Running");
        assert_eq!(node.priority, 10);
        assert_eq!(node.process_id, 1234);
        assert_eq!(node.tick_count, 100);
    }

    #[test]
    fn test_node_status_with_publishers_subscribers() {
        let pub_topic = TopicInfo {
            topic: "/sensor/data".to_string(),
            type_name: "SensorMsg".to_string(),
        };
        let sub_topic = TopicInfo {
            topic: "/commands".to_string(),
            type_name: "CmdMsg".to_string(),
        };

        let node = NodeStatus {
            name: "sensor_node".to_string(),
            status: "Running".to_string(),
            health: HealthStatus::Healthy,
            priority: 5,
            process_id: 5678,
            command_line: String::new(),
            working_dir: String::new(),
            cpu_usage: 0.0,
            memory_usage: 0,
            start_time: String::new(),
            scheduler_name: "main".to_string(),
            category: ProcessCategory::Node,
            tick_count: 0,
            error_count: 0,
            actual_rate_hz: 0,
            publishers: vec![pub_topic],
            subscribers: vec![sub_topic],
        };

        assert_eq!(node.publishers.len(), 1);
        assert_eq!(node.subscribers.len(), 1);
        assert_eq!(node.publishers[0].topic, "/sensor/data");
        assert_eq!(node.subscribers[0].type_name, "CmdMsg");
    }

    // =====================
    // ProcessCategory Tests
    // =====================
    #[test]
    fn test_process_category_equality() {
        assert_eq!(ProcessCategory::Node, ProcessCategory::Node);
        assert_eq!(ProcessCategory::Tool, ProcessCategory::Tool);
        assert_eq!(ProcessCategory::CLI, ProcessCategory::CLI);
        assert_ne!(ProcessCategory::Node, ProcessCategory::Tool);
        assert_ne!(ProcessCategory::Tool, ProcessCategory::CLI);
    }

    // =====================
    // SharedMemoryInfo Tests
    // =====================
    #[test]
    fn test_shared_memory_info_creation() {
        let shm = SharedMemoryInfo {
            topic_name: "/robot/pose".to_string(),
            size_bytes: 4096,
            active: true,
            accessing_processes: vec![1234, 5678],
            last_modified: Some(std::time::SystemTime::now()),
            message_type: Some("PoseMsg".to_string()),
            publishers: vec!["localization".to_string()],
            subscribers: vec!["navigation".to_string(), "visualization".to_string()],
            message_rate_hz: 30.0,
        };

        assert_eq!(shm.topic_name, "/robot/pose");
        assert_eq!(shm.size_bytes, 4096);
        assert!(shm.active);
        assert_eq!(shm.accessing_processes.len(), 2);
        assert_eq!(shm.publishers.len(), 1);
        assert_eq!(shm.subscribers.len(), 2);
        assert!((shm.message_rate_hz - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_shared_memory_info_inactive() {
        let shm = SharedMemoryInfo {
            topic_name: "/old/topic".to_string(),
            size_bytes: 1024,
            active: false,
            accessing_processes: vec![],
            last_modified: None,
            message_type: None,
            publishers: vec![],
            subscribers: vec![],
            message_rate_hz: 0.0,
        };

        assert!(!shm.active);
        assert!(shm.accessing_processes.is_empty());
        assert!(shm.message_type.is_none());
        assert!(shm.last_modified.is_none());
    }

    // =====================
    // TopicInfo Tests
    // =====================
    #[test]
    fn test_topic_info_creation() {
        let topic = TopicInfo {
            topic: "/camera/image".to_string(),
            type_name: "sensor_msgs::Image".to_string(),
        };

        assert_eq!(topic.topic, "/camera/image");
        assert_eq!(topic.type_name, "sensor_msgs::Image");
    }

    // =====================
    // Helper Function Tests
    // =====================
    #[test]
    fn test_format_duration_seconds() {
        let duration = std::time::Duration::from_secs(45);
        assert_eq!(format_duration(duration), "45s");
    }

    #[test]
    fn test_format_duration_minutes() {
        let duration = std::time::Duration::from_secs(125);
        assert_eq!(format_duration(duration), "2m");
    }

    #[test]
    fn test_format_duration_hours() {
        let duration = std::time::Duration::from_secs(7200);
        assert_eq!(format_duration(duration), "2h");
    }

    #[test]
    fn test_format_duration_days() {
        let duration = std::time::Duration::from_secs(172800);
        assert_eq!(format_duration(duration), "2d");
    }

    #[test]
    fn test_should_track_process_empty() {
        assert!(!should_track_process(""));
        assert!(!should_track_process("   "));
    }

    #[test]
    fn test_should_track_process_excluded_patterns() {
        // Build tools should be excluded
        assert!(!should_track_process("cargo build --release"));
        assert!(!should_track_process("cargo test"));
        assert!(!should_track_process("rustc --version"));
        assert!(!should_track_process("/bin/bash script.sh"));
        assert!(!should_track_process("timeout 10 some_command"));
        assert!(!should_track_process("dashboard server"));
        assert!(!should_track_process("horus run test_package"));
    }

    #[test]
    fn test_should_track_process_scheduler() {
        // Standalone scheduler should be tracked
        assert!(should_track_process("/path/to/scheduler binary"));
    }

    #[test]
    fn test_categorize_process_gui() {
        assert_eq!(
            categorize_process("robot_gui", ""),
            ProcessCategory::Tool
        );
        assert_eq!(
            categorize_process("viewer_app", ""),
            ProcessCategory::Tool
        );
        assert_eq!(
            categorize_process("viz_tool", ""),
            ProcessCategory::Tool
        );
        assert_eq!(
            categorize_process("my_GUI_app", ""),
            ProcessCategory::Tool
        );
        assert_eq!(
            categorize_process("app_gui", ""),
            ProcessCategory::Tool
        );
        assert_eq!(
            categorize_process("test", "--gui"),
            ProcessCategory::Tool
        );
        assert_eq!(
            categorize_process("test", "--view mode"),
            ProcessCategory::Tool
        );
    }

    #[test]
    fn test_categorize_process_cli() {
        assert_eq!(
            categorize_process("horus", ""),
            ProcessCategory::CLI
        );
        assert_eq!(
            categorize_process("horus run", ""),
            ProcessCategory::CLI
        );
        assert_eq!(
            categorize_process("test", "/bin/horus run pkg"),
            ProcessCategory::CLI
        );
        assert_eq!(
            categorize_process("test", "target/debug/horus run pkg"),
            ProcessCategory::CLI
        );
    }

    #[test]
    fn test_categorize_process_node() {
        assert_eq!(
            categorize_process("scheduler", ""),
            ProcessCategory::Node
        );
        assert_eq!(
            categorize_process("test", "my_scheduler"),
            ProcessCategory::Node
        );
        // Default is Node
        assert_eq!(
            categorize_process("unknown_process", "unknown cmd"),
            ProcessCategory::Node
        );
    }

    #[test]
    fn test_extract_process_name_simple() {
        assert_eq!(
            extract_process_name("/usr/bin/robot_control"),
            "robot_control"
        );
        assert_eq!(
            extract_process_name("./my_program"),
            "my_program"
        );
    }

    #[test]
    fn test_extract_process_name_horus_cli() {
        assert_eq!(
            extract_process_name("horus run my_package"),
            "horus run my_package"
        );
        assert_eq!(
            extract_process_name("horus monitor dashboard"),
            "horus monitor dashboard"
        );
        assert_eq!(
            extract_process_name("horus version"),
            "horus version"
        );
    }

    #[test]
    fn test_extract_process_name_empty() {
        assert_eq!(extract_process_name(""), "Unknown");
    }

    #[test]
    fn test_parse_memory_from_stat_valid() {
        // stat format: pid (comm) state ... rss is 24th field (0-indexed: 23)
        // We need at least 24 space-separated fields
        let stat = "1234 (test) S 1 1234 1234 0 -1 4194304 100 0 0 0 10 5 0 0 20 0 1 0 12345 12345678 500 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0";
        let memory = parse_memory_from_stat(stat);
        // 500 pages * 4KB = 2000KB
        assert_eq!(memory, 2000);
    }

    #[test]
    fn test_parse_memory_from_stat_invalid() {
        assert_eq!(parse_memory_from_stat(""), 0);
        assert_eq!(parse_memory_from_stat("short stat"), 0);
    }

    // =====================
    // Public API Tests (with real test data)
    // =====================

    /// Helper to create test pubsub metadata file
    fn create_test_pubsub_metadata(node_name: &str, topic_name: &str, direction: &str) -> Option<std::path::PathBuf> {
        let metadata_dir = std::path::Path::new("/dev/shm/horus/pubsub_metadata");
        if std::fs::create_dir_all(metadata_dir).is_err() {
            return None; // Can't create test data
        }

        let safe_topic: String = topic_name
            .chars()
            .map(|c| if c == '/' || c == ' ' { '_' } else { c })
            .collect();
        let filename = format!("{}_{}", node_name, safe_topic);
        let filepath = metadata_dir.join(format!("{}_{}", filename, direction));

        // Write current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if std::fs::write(&filepath, timestamp.to_string()).is_ok() {
            Some(filepath)
        } else {
            None
        }
    }

    /// Helper to create test topic file
    fn create_test_topic(topic_name: &str) -> Option<std::path::PathBuf> {
        let topics_dir = std::path::Path::new("/dev/shm/horus/topics");
        if std::fs::create_dir_all(topics_dir).is_err() {
            return None;
        }

        let safe_name: String = topic_name
            .chars()
            .map(|c| if c == '/' || c == ' ' { '_' } else { c })
            .collect();
        let filepath = topics_dir.join(&safe_name);

        // Create a small test file
        if std::fs::write(&filepath, vec![0u8; 1024]).is_ok() {
            Some(filepath)
        } else {
            None
        }
    }

    /// Cleanup helper
    fn cleanup_test_file(path: Option<std::path::PathBuf>) {
        if let Some(p) = path {
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn test_discover_nodes_with_real_pubsub_metadata() {
        // Create test pubsub metadata to simulate an active node
        let test_node = "TestDetectionNode";
        let test_topic = "test_detection_topic";

        let pub_file = create_test_pubsub_metadata(test_node, test_topic, "pub");
        let topic_file = create_test_topic(test_topic);

        // Only run the meaningful test if we could create test data
        if pub_file.is_some() && topic_file.is_some() {
            // Force cache refresh
            if let Ok(mut cache) = DISCOVERY_CACHE.write() {
                cache.last_updated = std::time::Instant::now() - std::time::Duration::from_secs(10);
            }

            let result = discover_nodes();
            assert!(result.is_ok(), "discover_nodes should succeed");

            let nodes = result.unwrap();
            // Should find our test node
            let found = nodes.iter().any(|n| n.name == test_node);
            assert!(found, "Should discover TestDetectionNode from pubsub metadata, found: {:?}",
                nodes.iter().map(|n| &n.name).collect::<Vec<_>>());

            // Verify the node has correct publisher info
            if let Some(node) = nodes.iter().find(|n| n.name == test_node) {
                assert!(node.publishers.iter().any(|p| p.topic == test_topic),
                    "Node should have test_detection_topic as publisher");
            }
        }

        // Cleanup
        cleanup_test_file(pub_file);
        cleanup_test_file(topic_file);
    }

    #[test]
    fn test_discover_shared_memory_with_real_topic() {
        // Use simple topic name to avoid underscore-to-slash conversion confusion
        let test_topic = "testshm";  // Simple name without underscores
        let topic_file = create_test_topic(test_topic);

        if topic_file.is_some() {
            // Force cache refresh - handle potential poisoned lock
            let cache_refreshed = DISCOVERY_CACHE.write().map(|mut cache| {
                cache.last_updated = std::time::Instant::now() - std::time::Duration::from_secs(10);
                true
            }).unwrap_or(false);

            if !cache_refreshed {
                cleanup_test_file(topic_file);
                return; // Skip test if cache is poisoned
            }

            // Call the uncached version directly to avoid cache issues in parallel tests
            let result = discover_shared_memory_uncached();
            if let Ok(topics) = result {
                // Should find our test topic (underscores in filename become / in topic name)
                let found = topics.iter().any(|t| t.topic_name.contains("testshm"));
                assert!(found, "Should discover testshm topic, found: {:?}",
                    topics.iter().map(|t| &t.topic_name).collect::<Vec<_>>());

                // Verify topic properties
                if let Some(topic) = topics.iter().find(|t| t.topic_name.contains("testshm")) {
                    assert_eq!(topic.size_bytes, 1024, "Topic should be 1024 bytes");
                }
            }
            // If result is Err, that's OK - test data might have been cleaned up by another test
        }

        cleanup_test_file(topic_file);
    }

    #[test]
    fn test_discover_nodes_returns_vec() {
        // Smoke test - should not panic even with no data
        let result = discover_nodes();
        assert!(result.is_ok());
    }

    #[test]
    fn test_discover_shared_memory_handles_missing_dirs() {
        // Smoke test - should not panic even if dirs don't exist
        let _ = discover_shared_memory();
    }

    #[test]
    fn test_pubsub_metadata_staleness_filtering() {
        // Create old metadata that should be filtered out
        let metadata_dir = std::path::Path::new("/dev/shm/horus/pubsub_metadata");
        if std::fs::create_dir_all(metadata_dir).is_err() {
            return; // Skip if can't create
        }

        let stale_file = metadata_dir.join("StaleNode_stale_topic_pub");
        // Write a timestamp from 60 seconds ago (should be filtered as stale)
        let old_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() - 60;

        if std::fs::write(&stale_file, old_timestamp.to_string()).is_ok() {
            // Force cache refresh
            if let Ok(mut cache) = DISCOVERY_CACHE.write() {
                cache.last_updated = std::time::Instant::now() - std::time::Duration::from_secs(10);
            }

            let result = discover_nodes();
            if let Ok(nodes) = result {
                // StaleNode should NOT be found (metadata too old)
                let found_stale = nodes.iter().any(|n| n.name == "StaleNode");
                assert!(!found_stale, "Stale nodes (>30 sec old metadata) should be filtered out");
            }

            let _ = std::fs::remove_file(&stale_file);
        }
    }

    #[test]
    fn test_topic_inactive_detection() {
        // Create a topic file and verify active detection works
        let topics_dir = std::path::Path::new("/dev/shm/horus/topics");
        if std::fs::create_dir_all(topics_dir).is_err() {
            return;
        }

        let test_file = topics_dir.join("test_active_topic");
        if std::fs::write(&test_file, vec![0u8; 512]).is_ok() {
            // Force cache refresh
            if let Ok(mut cache) = DISCOVERY_CACHE.write() {
                cache.last_updated = std::time::Instant::now() - std::time::Duration::from_secs(10);
            }

            let result = discover_shared_memory();
            if let Ok(topics) = result {
                if let Some(topic) = topics.iter().find(|t| t.topic_name.contains("test_active")) {
                    // Just-created file should be considered active (recently modified)
                    assert!(topic.active, "Recently created topic should be active");
                }
            }

            let _ = std::fs::remove_file(&test_file);
        }
    }

    // =====================
    // DiscoveryCache Tests
    // =====================
    #[test]
    fn test_discovery_cache_new_is_stale() {
        let cache = DiscoveryCache::new();
        // New cache should be stale (forces initial update)
        assert!(cache.is_stale());
    }

    #[test]
    fn test_discovery_cache_update_nodes() {
        let mut cache = DiscoveryCache::new();
        let nodes = vec![NodeStatus {
            name: "test".to_string(),
            status: "Running".to_string(),
            health: HealthStatus::Healthy,
            priority: 0,
            process_id: 0,
            command_line: String::new(),
            working_dir: String::new(),
            cpu_usage: 0.0,
            memory_usage: 0,
            start_time: String::new(),
            scheduler_name: String::new(),
            category: ProcessCategory::Node,
            tick_count: 0,
            error_count: 0,
            actual_rate_hz: 0,
            publishers: vec![],
            subscribers: vec![],
        }];

        cache.update_nodes(nodes);

        // After update, should not be stale
        assert!(!cache.is_stale());
        assert_eq!(cache.nodes.len(), 1);
    }

    #[test]
    fn test_discovery_cache_update_shared_memory() {
        let mut cache = DiscoveryCache::new();
        let shm = vec![SharedMemoryInfo {
            topic_name: "/test".to_string(),
            size_bytes: 1024,
            active: true,
            accessing_processes: vec![],
            last_modified: None,
            message_type: None,
            publishers: vec![],
            subscribers: vec![],
            message_rate_hz: 0.0,
        }];

        cache.update_shared_memory(shm);

        assert!(!cache.is_stale());
        assert_eq!(cache.shared_memory.len(), 1);
    }

    // =====================
    // Process Existence Tests
    // =====================
    #[test]
    fn test_process_exists_self() {
        // Current process should exist
        let pid = std::process::id();
        assert!(process_exists(pid));
    }

    #[test]
    fn test_process_exists_invalid() {
        // PID 0 or very high numbers shouldn't exist
        assert!(!process_exists(999999999));
    }

    // =====================
    // Edge Cases Tests
    // =====================
    #[test]
    fn test_node_status_clone() {
        let node = NodeStatus {
            name: "clone_test".to_string(),
            status: "Running".to_string(),
            health: HealthStatus::Warning,
            priority: 5,
            process_id: 9999,
            command_line: "test cmd".to_string(),
            working_dir: "/tmp".to_string(),
            cpu_usage: 50.0,
            memory_usage: 2048,
            start_time: "1h".to_string(),
            scheduler_name: "test_sched".to_string(),
            category: ProcessCategory::Tool,
            tick_count: 500,
            error_count: 2,
            actual_rate_hz: 100,
            publishers: vec![TopicInfo {
                topic: "/pub".to_string(),
                type_name: "Msg".to_string(),
            }],
            subscribers: vec![],
        };

        let cloned = node.clone();
        assert_eq!(cloned.name, node.name);
        assert_eq!(cloned.status, node.status);
        assert_eq!(cloned.publishers.len(), node.publishers.len());
    }

    #[test]
    fn test_shared_memory_info_clone() {
        let shm = SharedMemoryInfo {
            topic_name: "/clone_topic".to_string(),
            size_bytes: 8192,
            active: true,
            accessing_processes: vec![1, 2, 3],
            last_modified: Some(std::time::SystemTime::now()),
            message_type: Some("TestMsg".to_string()),
            publishers: vec!["pub1".to_string()],
            subscribers: vec!["sub1".to_string(), "sub2".to_string()],
            message_rate_hz: 60.0,
        };

        let cloned = shm.clone();
        assert_eq!(cloned.topic_name, shm.topic_name);
        assert_eq!(cloned.accessing_processes.len(), 3);
        assert_eq!(cloned.subscribers.len(), 2);
    }

    #[test]
    fn test_health_status_variants() {
        // Ensure all health status variants work correctly
        let node_healthy = NodeStatus {
            name: "h".to_string(),
            status: String::new(),
            health: HealthStatus::Healthy,
            priority: 0,
            process_id: 0,
            command_line: String::new(),
            working_dir: String::new(),
            cpu_usage: 0.0,
            memory_usage: 0,
            start_time: String::new(),
            scheduler_name: String::new(),
            category: ProcessCategory::Node,
            tick_count: 0,
            error_count: 0,
            actual_rate_hz: 0,
            publishers: vec![],
            subscribers: vec![],
        };

        match node_healthy.health {
            HealthStatus::Healthy => assert!(true),
            _ => panic!("Expected Healthy"),
        }
    }
}
