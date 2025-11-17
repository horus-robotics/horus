# HORUS Logging System Research Report

## Executive Summary

The HORUS logging system uses a **dual-path architecture** where console output (via `println!()`, `print!()`, and `io::stdout()`) is completely separate from the LogEntry buffer (written to shared memory). However, **console I/O is unbuffered and synchronously flushed after every log operation**, creating potential performance bottlenecks in high-frequency publish/subscribe workloads.

---

## 1. Logging Architecture

### 1.1 How `enable_logging` Flag Works

**Location**: `horus_core/src/core/node.rs:247-248`

```rust
pub struct NodeConfig {
    pub enable_logging: bool,      // Master switch for console output
    pub log_level: String,         // "DEBUG", "INFO", "QUIET" (also supports custom)
    // ... other fields ...
}

// Default:
impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            enable_logging: true,           // ⚠️  Enabled by default
            log_level: "INFO".to_string(),  // ⚠️  Development default
            // ...
        }
    }
}
```

**Control Flow**:
1. `enable_logging` controls **BOTH console output AND LogEntry buffer population** simultaneously
2. When `false`: Completely disables all logging (console AND buffer)
3. When `true`: Enables based on `log_level` (e.g., "QUIET" disables metric logging but not pub/sub)

### 1.2 Logging Flow Diagram

```
Message Published/Subscribed
         |
         ├─→ [1] log_pub_summary() / log_sub_summary()
         |       - ALWAYS called with ipc_ns measurement
         |
         ├─→ [2] if enable_logging
         |       ├─→ [2a] Format color-coded string
         |       ├─→ [2b] print!() or println!()
         |       └─→ [2c] io::stdout().flush() ⚠️  SYNCHRONOUS FLUSH
         |
         └─→ [3] publish_log(LogEntry)
                 - UNCONDITIONAL (always writes to shared memory buffer)
                 - Independent of enable_logging
```

### 1.3 Console vs LogEntry Buffer Separation

**YES - They are separate:**

Console Output Path:
- Uses `print!()`, `println!()`, or `io::stdout().write_all()`
- Color-coded ANSI escape sequences
- Immediately flushed to stdout
- Gated by `enable_logging` flag

LogEntry Buffer Path (`horus_core/src/core/log_buffer.rs`):
- Written to `/dev/shm/horus_logs` (shared memory)
- Ring buffer: max 5,000 entries × 512 bytes = ~2.56MB
- **INDEPENDENT** of `enable_logging` - always populated
- Used by dashboard (separate process reads this)

**Key Insight**: Dashboard does NOT need console output. It reads LogEntry buffer directly via:
```rust
pub fn publish_log(entry: LogEntry) {
    GLOBAL_LOG_BUFFER.push(entry);  // Line 204, log_buffer.rs
}
```

---

## 2. All Console Output Locations

### 2.1 In logging path (horus_core/src/core/node.rs)

| Location | Function | Output Type | Details |
|----------|----------|-------------|---------|
| Line 535 | `log_pub_summary()` | `print!()` | Publish message with color codes, includes ANSI newlines `\r\n` |
| Line 542 | `log_pub_summary()` | `io::stdout().flush()` | **SYNCHRONOUS** flush - blocks until written |
| Line 575 | `log_sub_summary()` | `println!()` | Subscribe message, automatic newline + color codes |
| Line 582 | `log_sub_summary()` | `io::stdout().flush()` | **SYNCHRONOUS** flush |
| Line 613 | `log_info()` | `eprintln!()` | Goes to stderr, not stdout |
| Line 643-649 | `log_warning()` | `io::stdout().write_all()` + `flush()` | Manual buffer write + flush |
| Line 683-689 | `log_error()` | `io::stdout().write_all()` + `flush()` | Manual buffer write + flush |
| Line 723-729 | `log_debug()` | `io::stdout().write_all()` + `flush()` | Manual buffer write + flush (only if level == "DEBUG") |
| Line 754 | `log_metrics_summary()` | `println!()` | Only if enable_logging && log_level != "QUIET" |

### 2.2 Console Output Characteristics

**Color Code Usage**:
```rust
// From line 535 (log_pub_summary):
print!("\r\n\x1b[36m[{}]\x1b[0m \x1b[32m[IPC: {}ns | Tick: {}μs]\x1b[0m \x1b[34m[#{}]\x1b[0m \x1b[33m{}\x1b[0m \x1b[1;32m--PUB-->\x1b[0m \x1b[35m'{}'\x1b[0m = {}\r\n",
    now.format("%H:%M:%S%.3f"),
    ipc_ns,
    current_tick_us,
    self.metrics.total_ticks,
    self.name, topic, summary);

// Colors:
// Cyan (\x1b[36m)  - timestamp
// Green (\x1b[32m) - metrics (IPC ns, tick duration)
// Blue (\x1b[34m)  - tick number
// Yellow (\x1b[33m) - node name
// Bold Green (\x1b[1;32m) - PUB arrow
// Bold Blue (\x1b[1;34m) - SUB arrow
// Magenta (\x1b[35m) - topic
```

---

## 3. Performance Configuration Options

### 3.1 NodeConfig Options

```rust
pub struct NodeConfig {
    pub enable_logging: bool,         // Master on/off
    pub log_level: String,            // "DEBUG", "INFO", "QUIET"
    // Performance/Lifecycle options:
    pub max_tick_duration_ms: Option<u64>,
    pub restart_on_failure: bool,
    pub max_restart_attempts: u32,
    pub restart_delay_ms: u64,
    pub custom_params: HashMap<String, String>,
}
```

### 3.2 Log Levels

| Level | Effect |
|-------|--------|
| "DEBUG" | Calls `log_debug()` enabled; all info/warn/error enabled; pub/sub logging enabled |
| "INFO" | (Default) Info logging enabled; pub/sub logging enabled; debug disabled |
| "QUIET" | `log_metrics_summary()` disabled; pub/sub logging STILL enabled |
| (any other) | Treated as >= "INFO" |

**IMPORTANT**: Pub/sub logging is controlled only by `enable_logging`, NOT by `log_level`:
```rust
// Line 532 (log_pub_summary):
if self.config.enable_logging {  // <- Only this controls pub/sub
    // ... print to console
}
```

### 3.3 Scheduler Configuration (horus_core/src/scheduling/config.rs)

The Scheduler has **NO direct logging configuration**, but provides presets:

```rust
pub struct SchedulerConfig {
    pub monitoring: MonitoringConfig {
        pub profiling_enabled: bool,      // Runtime profiling
        pub tracing_enabled: bool,        // Distributed tracing
        pub metrics_interval_ms: u64,     // How often to log metrics
        pub telemetry_endpoint: Option<String>,
        pub black_box_enabled: bool,      // Record all events
        pub black_box_size_mb: usize,
    },
    // ... other configs ...
}

// Presets available:
// - SchedulerConfig::standard()        // logging enabled
// - SchedulerConfig::safety_critical() // profiling_enabled: false
// - SchedulerConfig::high_performance()// profiling_enabled: false
// - SchedulerConfig::hard_realtime()   // profiling_enabled: false
```

### 3.4 Per-Node Registration

```rust
// Line 424-428 (scheduler.rs):
pub fn add(
    &mut self,
    node: Box<dyn Node>,
    priority: u32,
    logging_enabled: Option<bool>,  // <- Can control per-node!
) -> &mut Self {
    let logging_enabled = logging_enabled.unwrap_or(false);  // Defaults to FALSE
    let context = NodeInfo::new(node_name.clone(), logging_enabled);
    // ...
}
```

**Key Finding**: 
- Default is `false` when registering with scheduler (Line 435)
- But `NodeConfig::default()` has `enable_logging: true`
- Actual value depends on how `NodeInfo::new()` is called

---

## 4. Release vs Debug Build Impact

### 4.1 Debug Build
- ANSI color codes enabled (full terminal output)
- `formal_verification: cfg!(debug_assertions)` is TRUE
- `profiling_enabled: true` by default
- All logging functions execute normally

### 4.2 Release Build
```bash
cargo build --release
```
- ANSI color codes still enabled (compile-time strings)
- `formal_verification: cfg!(debug_assertions)` is FALSE (in safety_critical preset)
- Compiler optimizations may inline/reorder console writes
- **IMPORTANT**: Even in release, console I/O is synchronously flushed

### 4.3 Key Release Optimization

```rust
// Line 250 (config.rs - hard_realtime preset):
formal_verification: cfg!(debug_assertions),  // Only in debug builds
profiling_enabled: false,                     // Disabled for production
```

---

## 5. Console I/O Buffering Analysis

### 5.1 Buffering Mode: UNBUFFERED + SYNCHRONOUS FLUSH

**Current Implementation**:
```rust
// Example from log_pub_summary (line 535-542):
print!("\r\n...");  // Line buffered (automatic flush on \n)
let _ = io::stdout().flush();  // <- SYNCHRONOUS EXPLICIT FLUSH
```

**Buffering Characteristics**:
- `print!()` uses line-buffering by default on Unix
- BUT explicit `io::stdout().flush()` forces SYNCHRONOUS write
- Each log call = system call (context switch + kernel I/O)
- No batching or deferred writes

### 5.2 Syscall Overhead

Per log operation:
1. `print!()` - internal buffer write (~10-50ns)
2. Line buffering detects newline (`\r\n`) - triggers implicit flush
3. `io::stdout().flush()` - explicit syscall (Linux `write(2)`)
   - Context switch: ~1-5μs
   - Actual I/O: 100-1000ns
   - Total per call: **~2-10μs per flush** (pipe buffering)

### 5.3 Performance Impact Estimate

For a 60Hz robot with 10 active nodes each logging pub/sub:
- 60 ticks/sec × 10 nodes × 1 pub/sub = 600 log operations/sec
- 600 × 5μs = **3ms additional latency per second** (0.3% overhead)

But for high-frequency sensor loops (200Hz × 20 nodes):
- 200 × 20 × 2 = 8,000 log operations/sec
- 8,000 × 5μs = **40ms additional latency per second** (4% overhead)

Plus stdout lock contention under high concurrency.

---

## 6. Current Implementation Details

### 6.1 LogEntry Structure

```rust
// log_buffer.rs line 8-18:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,      // "HH:MM:SS.fff"
    pub tick_number: u64,       // Deterministic tick counter
    pub node_name: String,
    pub log_type: LogType,      // Publish, Subscribe, Info, Warning, etc.
    pub topic: Option<String>,  // For pub/sub logs
    pub message: String,        // Summary string
    pub tick_us: u64,          // Tick elapsed time
    pub ipc_ns: u64,           // IPC nanoseconds (for pub/sub)
}
```

### 6.2 Shared Memory Ring Buffer

```rust
// log_buffer.rs line 39-41:
const MAX_LOG_ENTRIES: usize = 5000;
const LOG_ENTRY_SIZE: usize = 512;  // Fixed size per entry
const HEADER_SIZE: usize = 64;      // Metadata (write index)
// Total size: 64 + (5000 × 512) = 2,560,064 bytes ≈ 2.56MB

// Ring buffer location: /dev/shm/horus_logs
// Lock-free append (CAS on write_idx in mmap header)
```

### 6.3 LogEntry Writing (Independent of Console)

```rust
// node.rs line 545-556 (log_pub_summary):
use crate::core::log_buffer::{publish_log, LogEntry, LogType};
publish_log(LogEntry {
    timestamp: now.format("%H:%M:%S%.3f").to_string(),
    tick_number: self.metrics.total_ticks,
    node_name: self.name.clone(),
    log_type: LogType::Publish,
    topic: Some(topic.to_string()),
    message: summary.to_string(),
    tick_us: current_tick_us,
    ipc_ns,  // IPC latency measurement
});
// This happens REGARDLESS of enable_logging!
```

---

## 7. Recommended Production Configuration

### 7.1 Disable Console Output (Keep Buffer for Dashboard)

```rust
// Create scheduler with high-performance preset:
let mut scheduler = Scheduler::new();

// Option A: Use preset
let mut config = SchedulerConfig::high_performance();
config.monitoring.profiling_enabled = false;

// Option B: Disable logging per node
for node in nodes {
    scheduler.add(Box::new(node), priority, Some(false));  // <- disable logging
}
```

### 7.2 Eliminate I/O Overhead

Currently impossible with codebase, but would require:
1. Deferred console writes (batch multiple logs, flush periodically)
2. Non-blocking I/O (pipe to separate logging thread)
3. Or complete removal of console output (use only LogEntry buffer)

**Workaround**: Redirect stdout/stderr to /dev/null
```bash
cargo run --release 2>/dev/null 1>/dev/null
```

### 7.3 Dashboard-Only Logging

Since LogEntry buffer is **independent of console output**, you can:
1. Set `enable_logging: false` (disables ALL console output)
2. Dashboard still reads from `/dev/shm/horus_logs` (unaffected)
3. Zero console I/O overhead

---

## 8. Environment Variables and Runtime Flags

**Currently: NO environment variables for logging control**

The only way to control logging at runtime:
1. Via `NodeConfig::enable_logging` (compile-time or setup)
2. Via `Scheduler::add(..., Some(bool))` (at registration)
3. Redirect stdout/stderr (post-startup)

**Missing**: 
- `HORUS_LOG_LEVEL` env var
- `HORUS_LOGGING_ENABLED` env var  
- `--log-level` CLI argument in horus_manager

---

## 9. Benchmarks and Performance Data

### 9.1 From test_logging.rs (benchmarks/src/bin/test_logging.rs)

Performance test (line 197-270):
```rust
// Test: Send 10,000 messages
// With logging enabled (default):
// Expected: >10,000 msg/s for PASS
// Actual: Not disclosed in benchmark
```

### 9.2 IPC Benchmark Notes

From `benchmarks/src/bin/ipc_benchmark.rs`:
- Measures RDTSC-cycle-accurate timing
- 50,000 iterations (line 32)
- Does NOT specifically benchmark logging impact
- Focuses on pure IPC latency

### 9.3 Extrapolated Impact (from source analysis)

Logging overhead per pub/sub operation:
- String formatting: ~1-2μs
- Color code string building: ~0.5-1μs  
- `println!()` internal buffering: ~0.1-0.5μs
- `io::stdout().flush()` syscall: **~2-10μs** (dominant)
- LogEntry serialization/write: ~1-2μs
- **Total: ~5-16μs per message** (without network latency)

---

## 10. Best Practices for Production

### 10.1 Recommended Settings

```rust
// For production robots:
pub fn production_config() -> NodeConfig {
    NodeConfig {
        enable_logging: false,          // DISABLE console output
        log_level: "QUIET".to_string(), // But set to QUIET for safety
        max_tick_duration_ms: Some(1000),
        restart_on_failure: true,
        max_restart_attempts: 3,
        restart_delay_ms: 1000,
        custom_params: HashMap::new(),
    }
}

// Register with scheduler:
scheduler.add(Box::new(node), priority, Some(false));  // No logging
```

### 10.2 Dashboard Access

Dashboard still works because:
1. LogEntry buffer (`/dev/shm/horus_logs`) is independent
2. Dashboard reads via `SharedLogBuffer::get_all()`
3. No console output needed for dashboard

### 10.3 Monitoring Strategy

For production without console output:
1. **Dashboard** - reads LogEntry buffer (real-time web UI)
2. **Metrics endpoints** - expose via HTTP/gRPC
3. **Syslog integration** - if needed (not yet implemented)
4. **File logging** - if needed (not yet implemented)

---

## 11. Summary Table: Configuration Options

| Setting | Variable | Default | Production | Effect |
|---------|----------|---------|-----------|--------|
| Logging Master Switch | `enable_logging` | true | **false** | Disables console output + LogEntry population |
| Log Level | `log_level` | "INFO" | "QUIET" | Filters debug/metrics logs (not pub/sub) |
| Per-Node Logging | `add(..., None)` | false | false | Per-node control at registration |
| Profiling | `monitoring.profiling_enabled` | true | false | Runtime profiling overhead |
| Tracing | `monitoring.tracing_enabled` | false | false | Distributed tracing (if enabled) |
| Black Box | `monitoring.black_box_enabled` | false | true* | Record all events for debugging (*for RT systems) |
| Dashboard Access | N/A | ✓ | ✓ | Works independently of logging |

---

## 12. Key Findings & Recommendations

### Findings

1. **Dual-Path Architecture Works Well**: Console and LogEntry buffer are independent ✓
2. **Console I/O is Unbuffered**: Every log operation flushes to stdout immediately ✗
3. **Logging is Always On**: Even with `enable_logging: false`, LogEntry buffer still written ✓
4. **No Async/Batch Logging**: All I/O is synchronous (blocks on syscall) ✗
5. **Production Default Wrong**: `enable_logging: true` and `log_level: "INFO"` are development defaults ✗
6. **Dashboard Resilient**: Works even if console logging disabled ✓

### Recommendations

1. **Disable Console Output in Production**: Set `enable_logging: false`
2. **Use LogEntry Buffer Only**: Dashboard reads buffer, console output is overhead
3. **Add Async Logging**: Batch writes, defer flushing to background thread (future)
4. **Add Environment Variables**: `HORUS_LOG_LEVEL`, `HORUS_LOGGING_ENABLED` 
5. **Add CLI Flags**: `--log-level` and `--logging` to `horus manager run`
6. **Measure Impact**: Run benchmark with/without logging enabled
7. **Document**: Add "Production Configuration" section to docs

---

## Appendix: Code References

**Core Logging**:
- `horus_core/src/core/node.rs` lines 512-744: Log methods
- `horus_core/src/core/log_buffer.rs` lines 1-206: LogEntry buffer

**Configuration**:
- `horus_core/src/scheduling/config.rs`: SchedulerConfig

**Scheduler Integration**:
- `horus_core/src/scheduling/scheduler.rs` lines 424-456: Node registration

**Benchmarks**:
- `benchmarks/src/bin/test_logging.rs`: Logging tests
- `benchmarks/src/bin/ipc_benchmark.rs`: IPC latency (line 32-34: config)

**Tests**:
- `horus_core/tests/simple_test.rs`: Basic logging test (line 116-126)
