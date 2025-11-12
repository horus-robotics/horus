# HORUS Framework

<div align="center">

[![Version](https://img.shields.io/badge/version-0.1.4-blue.svg)](https://github.com/softmata/horus/releases)
[![License](https://img.shields.io/badge/license-Apache--2.0-green.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-alpha-yellow.svg)](https://github.com/softmata/horus/releases)
[![Discord](https://img.shields.io/badge/Discord-Join%20Us-7289da?logo=discord&logoColor=white)](https://discord.gg/hEZC3ev2Nf)

[![IPC Latency](https://img.shields.io/badge/IPC%20latency-248ns-brightgreen.svg)](#performance)
[![Throughput](https://img.shields.io/badge/throughput-6M%2B%20msg%2Fs-green.svg)](#performance)

**Hybrid Optimized Robotics Unified System**

A production-grade robotics framework built in Rust for **real-time performance** and **memory safety**. HORUS delivers sub-microsecond IPC latency (50-500x faster than ROS2) while maintaining a simpler developer experience.

[![Documentation](https://img.shields.io/badge/Documentation-Read%20the%20Docs-blue?style=for-the-badge)](https://docs.horus-registry.dev)
[![Installation](https://img.shields.io/badge/Installation-Get%20Started-green?style=for-the-badge)](https://docs.horus-registry.dev/installation)
[![Benchmarks](https://img.shields.io/badge/Benchmarks-Performance-red?style=for-the-badge)](https://docs.horus-registry.dev/benchmarks)

</div>

---

> [!IMPORTANT]
> **Having issues?** If something breaks while using HORUS, please [create an issue](https://github.com/softmata/horus/issues) on GitHub. We appreciate your feedback and will help resolve problems quickly.

---

## Why HORUS?

**Coming from ROS/ROS2?** HORUS provides the same multi-process architecture with **significantly better performance** and **memory safety guarantees**. Perfect for hard real-time control, safety-critical systems, and edge robotics.

**New to robotics?** HORUS's simple CLI and single-file project support gets you from zero to working robot code in minutes, not hours.

### vs ROS2

| Feature | HORUS | ROS2 |
|---------|-------|------|
| **Message Latency** | Sub-microsecond (248ns - 437ns) | 50μs - 500μs |
| **Memory Safety** | Rust (zero-cost) | C++ (manual) |
| **Getting Started** | 1 command | 10+ commands + config files |
| **IPC Mechanism** | Lock-free shared memory | DDS middleware |
| **Real-time Control** | Native priority scheduling | Executor model |
| **Learning Curve** | Gentle | Steep |

**When to use HORUS:**
- Hard real-time control (<1ms loops)
- Safety-critical systems
- Edge/embedded robotics
- New projects wanting modern tooling
- Teams valuing simplicity and performance

## Key Features

### Real-Time Performance
- **Sub-Microsecond Messaging**: Median 248ns (Link/SPSC), 437ns (Hub/MPMC)
- **High Throughput**: 6+ million messages per second sustained
- **Enhanced Smart Scheduler**: Automatic optimization with JIT compilation, async I/O, and fault tolerance
- **Priority-Based Scheduling**: Deterministic execution order with intelligent tier classification
- **Lock-Free Communication**: Atomic operations with cache-line alignment
- **Zero-Copy IPC**: Direct shared memory access

### Developer Experience
- **Simple Node API**: Clean `tick()` method with lifecycle hooks
- **Macro-Based Development**: `node!` macro eliminates boilerplate
- **Multi-Language Support**: Rust, Python, C with unified workflow
- **Built-in Logging**: Automatic pub/sub tracking with IPC timing
- **Unified CLI**: `horus` command for all operations

### Production Ready
- **Memory-Safe Messaging**: Fixed-size structures prevent corruption
- **Cross-Process Communication**: POSIX shared memory
- **Performance Benchmarks**: Comprehensive latency testing
- **Dashboard Monitoring**: Web UI for real-time system monitoring

## Installation

### Prerequisites

**Required:**
- **Rust 1.70+** (install from [rustup.rs](https://rustup.rs))
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  ```

- **Build tools**
  ```bash
  # Ubuntu/Debian
  sudo apt update && sudo apt install \
    build-essential \
    pkg-config \
    libudev-dev \
    libssl-dev \
    libasound2-dev

  # Fedora/RHEL
  sudo dnf groupinstall "Development Tools"
  sudo dnf install pkg-config systemd-devel openssl-devel alsa-lib-devel

  # Arch Linux
  sudo pacman -S base-devel pkg-config systemd openssl alsa-lib

  # macOS
  xcode-select --install
  brew install pkg-config openssl
  ```

**Optional:**
- Python 3.9+ for Python bindings: `sudo apt install python3 python3-pip`

### Quick Install

```bash
git clone https://github.com/softmata/horus.git
cd horus
./install.sh
```

The installer will:
- Build all packages in release mode
- Install `horus` CLI to `~/.cargo/bin/`
- Install runtime libraries to `~/.horus/cache/`
- Install Python bindings (if Python 3.9+ detected)

### Verify Installation

```bash
horus --help
ls ~/.horus/cache/
```

## Quick Start

### 1. Create a Project
```bash
horus new my_robot
cd my_robot
```

### 2. Simple Node Example
```rust
use horus::prelude::*;  // Imports Result<T> as alias for HorusResult<T>

// Define a custom message type with one line!
// The message! macro automatically implements required traits
message!(SensorReading = (f64, u32));  // (value, counter)

pub struct SensorNode {
    publisher: Hub<SensorReading>,
    counter: u32,
}

impl Node for SensorNode {
    fn name(&self) -> &'static str { "sensor_node" }

    // init() is optional - only override if you need setup logic
    fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        ctx.log_info("SensorNode initialized");
        Ok(())
    }

    // tick() is required - this is your main logic that runs every cycle
    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Simple sensor reading
        let reading = SensorReading(self.counter as f64 * 0.1, self.counter);

        // Send message (use None for ctx to disable logging overhead)
        let _ = self.publisher.send(reading, ctx);
        self.counter += 1;
    }

    // shutdown() is optional - only override if you need cleanup logic
    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        ctx.log_info(&format!("SensorNode sent {} readings", self.counter));
        Ok(())
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new().name("sensor_app");

    scheduler.add(
        Box::new(SensorNode {
            publisher: Hub::new("sensor_data")?,
            counter: 0,
        }),
        0,           // Priority (0 = highest)
        Some(true)   // Enable logging
    );

    scheduler.run()
}
```

### 3. Run the Project
```bash
horus run --release
```

## Concurrent Multi-Process Execution

HORUS supports running multiple nodes concurrently as separate processes using glob patterns:

```bash
horus run "nodes/*.py"          # Run all Python nodes concurrently
horus run "src/*.rs"            # Run all Rust nodes concurrently
horus run "nodes/*.c"           # Run all C nodes concurrently
```

**Features:**
- **Two-Phase Execution**: Builds all files sequentially (respects Cargo lock), then executes concurrently
- **Color-Coded Output**: Each node's output is prefixed with `[node_name]` in a unique color
- **Graceful Shutdown**: Ctrl+C cleanly terminates all running processes
- **Multi-Language**: Works with Rust, Python, and C files

**Example:**
```bash
$ horus run "nodes/*.py"
 Executing 3 files concurrently:
  1. nodes/sensor.py (python)
  2. nodes/controller.py (python)
  3. nodes/logger.py (python)

 Phase 1: Building all files...
 Phase 2: Starting all processes...

[sensor] Sensor reading: 25.3°C
[controller] Motor speed: 45%
[logger] System operational
```

Each file runs in a separate process with its own HORUS scheduler, communicating via shared memory IPC.

**Running nodes separately** (e.g., in different terminals):

Projects created with `horus new` include a `session_id` in `horus.yaml`:

```yaml
session_id: "my-project-session"  # Auto-generated from project name
```

This allows running nodes in separate terminals that can communicate:

```bash
# Terminal 1
horus run sensor.rs

# Terminal 2
horus run controller.rs
```

Both share the same session → communication works!

**Alternative**: Use `Hub::new_global()` for cross-session communication (no session_id needed).

**Note:** `horus run` works for single-file projects and projects with `horus.yaml` or a single `Cargo.toml`. It automatically handles dependencies and builds in a managed workspace.

## Core API

### Message Types

All message types sent through Hub/Link must implement the `LogSummary` trait:

```rust
use horus::prelude::*;

// Option 1: Use the message! macro (automatic trait implementation)
message!(Position = (f32, f32));
message!(Velocity = (f32, f32, f32));

// Option 2: Manual struct with LogSummary implementation
#[derive(Clone, Debug)]
pub struct CustomMessage {
    pub value: f64,
}

impl LogSummary for CustomMessage {
    fn log_summary(&self) -> String {
        format!("CustomMessage({})", self.value)
    }
}
```

### Scheduler

```rust
use horus::prelude::*;

let mut scheduler = Scheduler::new().name("my_app");

// Add nodes with priority (0 = highest) and logging
scheduler.add(Box::new(my_node), 0, Some(true));

// Run options:
scheduler.run()?;                                  // Run continuously until Ctrl+C
scheduler.run_for(Duration::from_secs(10))?;       // Run for duration
scheduler.tick(&["node1", "node2"])?;              // Run specific nodes
scheduler.tick_for(&["node1"], Duration::from_secs(5))?;  // Run specific nodes for duration
```

### Hub (Pub/Sub)

```rust
use horus::prelude::*;

// Create Hub for any type implementing LogSummary
let hub: Hub<f64> = Hub::new("topic_name")?;

// Send returns Result<(), T> - returns message back on failure
hub.send(42.0, None)?;  // None = no logging (best performance)
hub.send(42.0, Some(&mut ctx))?;  // Some(ctx) = enable logging

// Receive returns Option<T>
if let Some(msg) = hub.recv(None) {
    eprintln!("Received: {}", msg);
}
```

**Performance (on modern x86_64 systems):**
- **Link (SPSC)**: Median 248ns, 6M+ msg/s throughput
- **Hub (MPMC)**: Median 481ns, flexible pub/sub
- Production-validated with 6.2M+ test messages
- Up to 369 MB/s bandwidth for burst messages

### Node Trait

The core trait that all nodes must implement:

```rust
use horus::prelude::*;

pub trait Node: Send {
    fn name(&self) -> &'static str;                          // Required: Node identifier
    fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {   // Optional: Setup logic
        Ok(())
    }
    fn tick(&mut self, ctx: Option<&mut NodeInfo>);          // Required: Main loop logic
    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<()> { // Optional: Cleanup logic
        Ok(())
    }
}
```

**Method requirements:**
- **`name()`** - Required: Returns unique node identifier
- **`tick()`** - Required: Your main logic that runs every cycle
- **`init()`** - Optional: Override only if you need setup logic (default: empty)
- **`shutdown()`** - Optional: Override only if you need cleanup logic (default: empty)

**About the `ctx` parameter:**
- `ctx: Option<&mut NodeInfo>` - Use `None` for maximum performance (no logging)
- When passing ctx to multiple calls, declare as `mut ctx` and pass `ctx` directly
- The scheduler provides ctx based on logging settings in `scheduler.add()`

### node! Macro

The `node!` macro eliminates boilerplate by auto-generating the Node implementation:

```rust
use horus::prelude::*;

// Define message type first
message!(SensorData = (f64, u32));

node! {
    MyNode {
        // Publishers (use 'pub' section)
        pub {
            output: SensorData -> "sensor/output",
        }

        // Subscribers (use 'sub' section)
        sub {
            input: SensorData -> "sensor/input",
        }

        // Node state (use 'data' section)
        data {
            counter: u32 = 0,
        }

        // Lifecycle hooks
        init(ctx) {
            ctx.log_info("MyNode initialized");
            Ok(())
        }

        tick(ctx) {
            // ctx is Option<&mut NodeInfo> here
            if let Some(value) = self.input.recv(None) {
                self.counter += 1;
                let processed = SensorData(value.0 * 2.0, value.1);
                self.output.send(processed, ctx).ok();
            }
        }

        shutdown(ctx) {
            ctx.log_info("MyNode shutdown");
            Ok(())
        }
    }
}
```

**Note:** The macro uses `->` for both publishers and subscribers. The section name (`pub` vs `sub`) determines the behavior.

## Example Applications

### SnakeSim
```bash
# From HORUS root directory
cd horus_library/apps/snakesim

# Terminal 1: Run backend (keyboard input + game logic)
horus run

# Terminal 2: Run GUI (visual display)
cd snakesim_gui && horus run
```

Multi-node game demonstrating:
- KeyboardInputNode (priority 0): Arrow key/WASD input
- JoystickInputNode (priority 1): Joystick input
- SnakeControlNode (priority 2): Game logic
- GUI: Graphical display with animated snake (separate window)

## Multi-Language Support

### Python

Python bindings are automatically installed with `./install.sh` (requires Python 3.9+).

```python
import horus

def process(node):
    node.send("output", 42.0)

node = horus.Node(pubs="output", tick=process, rate=30)
horus.run(node, duration=5)
```

See [horus_py/README.md](horus_py/README.md) for complete documentation.

## Performance

### Enhanced Scheduler Features

The HORUS scheduler now includes intelligent runtime optimization that automatically adapts to your workload:

- **JIT Compilation**: Hot paths compiled to native code for ultra-fast execution
- **Async I/O Tier**: Non-blocking execution for I/O-heavy operations prevents system stalls
- **Fault Tolerance**: Circuit breaker pattern with automatic recovery for resilient operations
- **Smart Classification**: Automatic node categorization into 5 execution tiers based on runtime profiling
- **Zero Configuration**: All optimizations happen automatically while maintaining the simple API

### Benchmark Results

Latest comprehensive benchmarks show excellent performance across different workload types:

| Workload Type | Performance | Description |
|--------------|-------------|-------------|
| **UltraFastControl** | 2.387s | High-frequency control loops with JIT optimization |
| **FastSensor** | 2.382s | Rapid sensor processing and fusion |
| **HeavyIO** | 3.988s | I/O-intensive operations with async handling |
| **MixedRealistic** | 4.064s | Real-world mixed workload simulation |
| **Scalability (10-200 nodes)** | 106-120ms | Near-linear scaling across system sizes |

## Contributing

We welcome contributions to HORUS! Whether you're fixing bugs, adding features, or improving documentation, your help is appreciated.

**Before contributing:**
- Read [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines
- Check existing issues and pull requests to avoid duplicates
- For major features, open an issue first to discuss the approach

**Development workflow:**
1. Fork the repository and create a feature branch
2. Make your changes with appropriate tests
3. Submit a pull request with a clear description

See [CONTRIBUTING.md](CONTRIBUTING.md) for complete guidelines including code style, testing requirements, and PR process.

## License

Apache License 2.0 - see [LICENSE](LICENSE) for details.

By contributing, you agree to the [Contributor License Agreement](.github/CLA.md).
