# HORUS Core

** Internal Implementation Package - Use `horus` crate instead**

This is the internal implementation package for HORUS. Application developers should use the main `horus` crate:

```rust
//  Correct - use the main horus crate
use horus::prelude::*;

//  Wrong - don't use horus_core directly
use horus_core::prelude::*;
```

Rust-first robotics runtime: Node trait with priority scheduler, shared-memory IPC (Hub), and POSIX shared memory regions.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Core Modules](#core-modules)
- [Quick Start](#quick-start)
- [Performance](#performance)
- [Best Practices](#best-practices)

## Overview

HORUS Core provides lightweight primitives for robotics applications:

- **Nodes**: Simple `Node` trait with `init/tick/shutdown` lifecycle
- **Scheduler**: Priority-driven executor (0 = highest) with Ctrl+C handling
- **Hub**: Shared-memory pub/sub communication
- **NodeInfo**: Context for logging and metrics tracking

## Architecture

```
horus_core/
── core/                  # Core framework types
   ── node.rs           # Node trait + NodeInfo context
   ── log_buffer.rs     # Global log buffer
── communication/        # IPC primitives
   ── horus/
       ── hub.rs        # Hub pub/sub API
── memory/               # Shared memory
   ── shm_topic.rs      # Lock-free ring buffer
── scheduling/           # Task scheduling
   ── scheduler.rs      # Priority-based scheduler
── params/               # Runtime parameters
    ── mod.rs            # Parameter system
```

## Core Modules

### 1. Node Trait

From `horus_core/src/core/node.rs`:

```rust
pub trait Node: Send {
    fn name(&self) -> &'static str;
    fn init(&mut self, ctx: &mut NodeInfo) -> Result<(), String>;
    fn tick(&mut self, ctx: Option<&mut NodeInfo>);
    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<(), String>;
    fn get_publishers(&self) -> Vec<TopicMetadata> { Vec::new() }
    fn get_subscribers(&self) -> Vec<TopicMetadata> { Vec::new() }
}
```

**Key Points:**
- `init()` takes `&mut NodeInfo` (NOT Option)
- `tick()` takes `Option<&mut NodeInfo>` and returns nothing
- `shutdown()` takes `&mut NodeInfo` (NOT Option)
- All lifecycle methods use `Result<(), String>` for errors

### 2. NodeInfo Context

Provides logging and metrics tracking:

```rust
impl NodeInfo {
    pub fn log_pub<T: Debug>(&mut self, topic: &str, data: &T, ipc_ns: u64);
    pub fn log_sub<T: Debug>(&mut self, topic: &str, data: &T, ipc_ns: u64);
    pub fn log_info(&mut self, message: &str);
    pub fn log_warning(&mut self, message: &str);
    pub fn log_error(&mut self, message: &str);
    pub fn metrics(&self) -> &NodeMetrics;
}
```

**Available Metrics:**
- `total_ticks` - Total number of ticks
- `avg_tick_duration_ms` - Average tick time
- `max_tick_duration_ms` - Worst-case tick time
- `messages_sent` - Published messages
- `messages_received` - Subscribed messages
- `errors_count` - Error count
- `uptime_seconds` - Node uptime

### 3. Hub Communication

From `horus_core/src/communication/hub.rs`:

```rust
impl<T> Hub<T> {
    pub fn new(topic_name: &str) -> HorusResult<Self>;
    pub fn new_with_capacity(topic_name: &str, capacity: usize) -> HorusResult<Self>;
    pub fn send(&self, msg: T, ctx: Option<&mut NodeInfo>) -> Result<(), T>;
    pub fn recv(&self, ctx: Option<&mut NodeInfo>) -> Option<T>;
    pub fn get_topic_name(&self) -> &str;
    pub fn get_metrics(&self) -> HubMetrics;
}
```

**Hub Features:**
- Lock-free atomic operations
- Cache-line aligned (64 bytes)
- POSIX shared memory via `/dev/shm/horus/`
- Default capacity: 1024 slots per topic

**Performance (on modern x86_64 systems):**
- **Link (SPSC)**: 312ns median latency, 6M+ msg/s throughput
- **Hub (MPMC)**: 481ns median latency, flexible pub/sub
- Link is 29% faster than Hub in 1P1C scenarios
- Production-validated with 6.2M+ test messages

*Performance varies by hardware. See `benchmarks/` directory for detailed results.*

### 4. Scheduler

From `horus_core/src/scheduling/scheduler.rs`:

```rust
impl Scheduler {
    pub fn new() -> Self;
    pub fn register(&mut self, node: Box<dyn Node>, priority: u32, logging_enabled: Option<bool>) -> &mut Self;
    pub fn tick_all(&mut self) -> HorusResult<()>;
    pub fn tick_node(&mut self, node_names: &[&str]) -> HorusResult<()>;
    pub fn stop(&self);
    pub fn is_running(&self) -> bool;
}
```

**Scheduler Details:**
- Uses `tokio::runtime::Runtime` internally
- Runs at ~60 FPS (16ms sleep between ticks)
- Sorts nodes by priority each tick (0 = highest)
- Built-in Ctrl+C handling
- Writes heartbeats to `/dev/shm/horus/heartbeats/`

## Quick Start

### 1. Basic Node Implementation

```rust
use horus::prelude::*;

pub struct SensorNode {
    publisher: Hub<f64>,
    counter: u32,
}

impl SensorNode {
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new("sensor_data")?,
            counter: 0,
        })
    }
}

impl Node for SensorNode {
    fn name(&self) -> &'static str { "SensorNode" }

    // Optional: Called once at startup
    fn init(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
        ctx.log_info("SensorNode initialized");
        Ok(())
    }

    // Required: Called repeatedly by scheduler
    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        let reading = self.counter as f64 * 0.1;
        let _ = self.publisher.send(reading, ctx);
        self.counter += 1;
    }

    // Optional: Called once at shutdown
    fn shutdown(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
        ctx.log_info("SensorNode shutdown");
        Ok(())
    }
}
```

### 2. Subscriber Node

```rust
pub struct ControlNode {
    subscriber: Hub<f64>,
}

impl ControlNode {
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            subscriber: Hub::new("sensor_data")?,
        })
    }
}

impl Node for ControlNode {
    fn name(&self) -> &'static str { "ControlNode" }

    // Optional: Called once at startup
    fn init(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
        ctx.log_info("ControlNode initialized");
        Ok(())
    }

    // Required: Called repeatedly by scheduler
    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        if let Some(data) = self.subscriber.recv(ctx) {
            // Process the received data
            if let Some(ctx) = ctx {
                ctx.log_info(&format!("Received: {}", data));
            }
        }
    }

    // Optional: Called once at shutdown
    fn shutdown(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
        ctx.log_info("ControlNode shutdown");
        Ok(())
    }
}
```

### 3. Complete Application

```rust
use horus::prelude::*;

fn main() -> HorusResult<()> {
    let mut scheduler = Scheduler::new();

    scheduler
        .register(Box::new(SensorNode::new()?), 0, Some(true))
        .register(Box::new(ControlNode::new()?), 1, Some(true));

    println!("Starting scheduler...");
    scheduler.tick_all()?;

    Ok(())
}
```

## Performance

### Communication Latency

**HORUS provides two IPC mechanisms optimized for different use cases:**

**Link (SPSC) - Cross-Core:**
- Median latency: 312ns (624 cycles @ 2GHz)
- P95 latency: 444ns, P99 latency: 578ns
- Burst throughput: 6.05 MHz (6M+ msg/s)
- Bandwidth: Up to 369 MB/s
- **Best for**: Point-to-point communication, control loops

**Hub (MPMC) - Cross-Core:**
- Median latency: 481ns (962 cycles @ 2GHz)
- P95 latency: 653ns
- Flexible pub/sub architecture
- **Best for**: Multi-subscriber topics, sensor broadcasting

**Key Results:**
- Link is 29% faster than Hub in 1P1C scenarios
- Sub-microsecond latency on modern x86_64 systems
- Production-validated with 6.2M+ test messages
- Zero corruptions detected

### Memory Layout

- Lock-free ring buffers
- Cache-line aligned structures (64 bytes)
- POSIX shared memory at `/dev/shm/horus/`
- Zero-copy within shared memory

### Shared Memory Configuration

```bash
# View shared memory regions
ls -lh /dev/shm/horus/

# Check available space
df -h /dev/shm
```

**Custom capacity:**
```rust
let hub = Hub::new_with_capacity("large_topic", 2048)?;
```

## Message Safety

All shared memory messages must use fixed-size structures:

```rust
//  Good: Fixed-size types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SafeMessage {
    data: [f32; 64],       // Fixed-size array
    timestamp: u64,        // Primitive type
    counter: u32,          // Primitive type
}

//  Bad: Dynamic allocation
#[derive(Debug, Clone)]
struct UnsafeMessage {
    data: String,          // Heap pointer - causes segfaults!
    values: Vec<f64>,      // Heap pointer - causes segfaults!
}
```

## Best Practices

### 1. Node Design

```rust
impl Node for WellDesignedNode {
    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        //  Good: Non-blocking message processing
        while let Some(data) = self.input.recv(ctx) {
            let result = process_data(data);
            let _ = self.output.send(result, ctx);
        }

        //  Bad: Blocking operations in tick()
        // std::thread::sleep(Duration::from_secs(1)); // Blocks other nodes!
    }
}
```

### 2. Priority Assignment

- **0**: Critical safety nodes (emergency stop, watchdog)
- **1-5**: Control loops (motion control, stabilization)
- **6-10**: Application logic (navigation, planning)
- **11+**: Visualization, logging, non-critical tasks

### 3. Error Handling

```rust
impl Node for RobustNode {
    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Handle communication errors gracefully
        match self.publisher.send(data, ctx) {
            Ok(()) => { /* Success */ }
            Err(msg) => {
                // Log error but don't panic - keep system running
                if let Some(ctx) = ctx {
                    ctx.log_warning("Message dropped due to full buffer");
                }
            }
        }
    }
}
```

### 4. Logging

```rust
impl Node for MyNode {
    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        if let Some(ctx) = ctx {
            // Hub automatically logs via ctx.log_pub() and ctx.log_sub()
            // when you pass ctx to send/recv

            // Manual logging for important events
            ctx.log_info("Node processing data");
            ctx.log_warning("Sensor timeout");
            ctx.log_error("Critical failure");
        }
    }
}
```

## Performance Tips

1. **Use appropriate priorities**: Critical control loops should have priority 0
2. **Enable logging selectively**: Only enable logging during development/debugging
3. **Use fixed-size messages**: Avoid dynamic allocation in shared memory
4. **Batch processing**: Process multiple messages per tick when possible
5. **Custom capacity**: Use `new_with_capacity()` for high-throughput topics

## Examples

### App (Multi-Node Application)

See the SnakeSim example in `horus_library/apps/snakesim/` which demonstrates:
- Multiple nodes with different priorities
- Built-in logging for debugging message flow
- Real-time game loop execution

Example structure:
```rust
let mut scheduler = Scheduler::new();
scheduler
    .register(Box::new(KeyboardInputNode::new()?), 0, Some(true))
    .register(Box::new(SnakeControlNode::new()?), 2, Some(true))
    .register(Box::new(GUINode::new()?), 3, Some(true));
scheduler.tick_all()?;
```

## Development

### Building from Source

```bash
cd horus_core
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Benchmarks

```bash
cd benchmarks
cargo bench
```

## Contributing

See the main [HORUS README](../README.md) for guidelines.

## License

MIT License - see [LICENSE](../LICENSE) for details.
