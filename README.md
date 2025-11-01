# HORUS Framework

**Hybrid Optimized Robotics Unified System**

A production-grade robotics framework built in Rust for **real-time performance** and **memory safety**. HORUS delivers sub-microsecond IPC latency (50-500x faster than ROS2) while maintaining a simpler developer experience.

## Why HORUS?

**Coming from ROS/ROS2?** HORUS provides the same multi-process architecture with **significantly better performance** and **memory safety guarantees**. Perfect for hard real-time control, safety-critical systems, and edge robotics.

**New to robotics?** HORUS's simple CLI and single-file project support gets you from zero to working robot code in minutes, not hours.

### vs ROS2

| Feature | HORUS | ROS2 |
|---------|-------|------|
| **Message Latency** | Sub-microsecond (312ns - 481ns) | 50μs - 500μs |
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

**When to use ROS2:**
- Legacy ROS1 migration
- Need existing ROS2 packages
- Large teams already trained in ROS
- Projects requiring RViz/Gazebo integration

## Key Features

### Real-Time Performance
- **Sub-Microsecond Messaging**: Median 312ns (Link/SPSC), 481ns (Hub/MPMC)
- **High Throughput**: 6+ million messages per second sustained
- **Priority-Based Scheduling**: Deterministic execution order
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
git clone https://github.com/horus-robotics/horus.git
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
horus --version
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
use horus::prelude::*;
use horus::core::LogSummary;

// Define a custom message type with one line!
message!(SensorReading = (f64, u64));  // (value, timestamp)

pub struct SensorNode {
    publisher: Hub<SensorReading>,
    counter: u32,
}

impl Node for SensorNode {
    fn name(&self) -> &'static str { "sensor_node" }

    fn init(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
        ctx.log_info("SensorNode initialized");
        Ok(())
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        let reading = SensorReading(
            self.counter as f64 * 0.1,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        );
        let _ = self.publisher.send(reading, ctx);
        self.counter += 1;
    }

    fn shutdown(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
        ctx.log_info(&format!("SensorNode sent {} readings", self.counter));
        Ok(())
    }
}

fn main() -> HorusResult<()> {
    let mut scheduler = Scheduler::new().name("sensor_app");

    scheduler.add(
        Box::new(SensorNode {
            publisher: Hub::new("sensor_data")?,
            counter: 0,
        }),
        0,
        Some(true)
    );

    scheduler.run()
}
```

### 3. Run the Project
```bash
horus run --release
```

## Project Structure

```
HORUS/
horus/                      # Main unified crate
horus_core/                 # Core framework
  communication/            # Hub, shared memory
  scheduling/               # Scheduler
  core/                     # Node trait, NodeInfo
  memory/                   # Shared memory management
horus_manager/              # CLI tool
horus_daemon/               # Remote deployment daemon
horus_macros/               # node! procedural macro
horus_py/                   # Python bindings
horus_c/                    # C bindings
horus_library/              # Standard library
  messages/                 # Standard message types
  apps/                     # Example applications
  tools/                    # Development tools
benchmarks/                 # Performance testing
docs-site/                  # Documentation website
```

## CLI Commands

### Project Management
```bash
horus new <name>                # Create new project
horus new my_robot -r           # Rust project
horus new my_robot -p           # Python project
horus new my_robot -c           # C project
horus new my_robot -m           # Rust with macros
```

### Build and Run
```bash
horus run                       # Auto-detect and run
horus run main.rs               # Run specific file
horus run --release             # Optimized build
horus run --build-only          # Build without running
horus run --clean               # Clean build cache
horus run --remote robot:8080   # Deploy to remote robot
```

#### Concurrent Multi-Process Execution

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

**Important:** `horus run` is designed for **single-file HORUS projects** only (main.rs, main.py, main.c). It automatically generates a temporary workspace in `.horus/` and handles all dependencies.

For **multi-crate workspaces** (projects with multiple `Cargo.toml` files or complex module structures), use `cargo` directly:
```bash
cargo build --release
cargo run --release
```

### Package Management
```bash
horus pkg install <package>     # Install package
horus pkg install <pkg> -v 1.0  # Specific version
horus pkg install <pkg> -g      # Install globally
horus pkg remove <package>      # Remove package
horus pkg list                  # List packages
```

### Environment Management
```bash
horus env freeze                # Freeze environment
horus env freeze -o custom.yaml # Custom file
horus env freeze --publish      # Publish to registry
horus env restore <file>        # Restore from file
```

### Publishing
```bash
horus pkg publish                # Publish current package to registry
horus pkg unpublish <name> <ver> # Remove package from registry
```

**Requirements:** GitHub authentication (run `horus auth login --github` first)

You can also publish via web interface at [marketplace.horus-registry.dev/publish](https://marketplace.horus-registry.dev/publish)

### Authentication
```bash
horus auth login --github       # GitHub OAuth login
horus auth generate-key         # Generate API key
horus auth whoami               # Show current user
horus auth logout               # Logout
```

### Dashboard
```bash
horus dashboard                 # Web dashboard
horus dashboard 3001            # Custom port
horus dashboard -t              # Terminal UI
```

### Version Information
```bash
horus version                   # Show version
horus --version
horus -V
```

## Core API

### Scheduler
```rust
use horus_core::scheduling::Scheduler;

let mut scheduler = Scheduler::new();
scheduler.register(Box::new(my_node), priority, Some(logging));
scheduler.tick_all()?;
```

### Hub (Pub/Sub)
```rust
use horus_core::communication::Hub;

let hub: Hub<f64> = Hub::new("topic_name")?;
hub.send(42.0, ctx)?;

if let Some(msg) = hub.recv(ctx) {
    // Process message
}
```

**Performance (on modern x86_64 systems):**
- **Link (SPSC)**: Median 312ns, 6M+ msg/s throughput
- **Hub (MPMC)**: Median 481ns, flexible pub/sub
- Production-validated with 6.2M+ test messages
- Up to 369 MB/s bandwidth for burst messages

*Performance varies by hardware. Run `cargo test --release` to benchmark on your system.*

### Node Trait
```rust
use horus_core::core::{Node, NodeInfo};

pub trait Node: Send {
    fn name(&self) -> &'static str;
    fn init(&mut self, ctx: &mut NodeInfo) -> Result<(), String> { Ok(()) }
    fn tick(&mut self, ctx: Option<&mut NodeInfo>);
    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<(), String> { Ok(()) }
}
```

### node! Macro
```rust
use horus_macros::node;

node! {
    MyNode {
        pub { output: f64 -> "output_topic" }
        sub { input: f64 <- "input_topic" }

        data {
            counter: u32 = 0,
        }

        init(ctx) {
            ctx.log_info("MyNode initialized");
            Ok(())
        }

        tick(ctx) {
            if let Some(value) = self.input.recv(ctx) {
                self.counter += 1;
                self.output.send(value * 2.0, ctx).ok();
            }
        }

        shutdown(ctx) {
            ctx.log_info("MyNode shutdown");
            Ok(())
        }
    }
}
```

## Example Applications

### SnakeSim
```bash
cd horus_library/apps/snakesim/snake_scheduler
cargo run --release
```

Multi-node game demonstrating:
- KeyboardInputNode (priority 0): Arrow key input
- SnakeControlNode (priority 2): Game logic
- GUINode (priority 3): Terminal rendering

### Sim2D Physics Simulator
```bash
cd horus_library/tools/sim2d
cargo run --release
```

Complete 2D robotics simulator with Bevy visualization and Rapier2D physics.

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

### C

See [horus_c/README.md](horus_c/README.md) for C bindings documentation.

## Testing

### Unit Tests
```bash
cargo test                  # All tests
cargo test -p horus_core    # Specific component
```

### Acceptance Tests

User acceptance tests are in `tests/acceptance/` documenting expected behavior.

```bash
cat tests/acceptance/README.md
cat tests/acceptance/horus_manager/01_new_command.md
```

### Benchmarks
```bash
cd benchmarks
cargo bench
cargo run --release --bin production_bench
```

## Community

Join our Discord community:

**[Join HORUS Discord](https://discord.gg/hEZC3ev2Nf)**

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Quick start:
1. Fork the repository
2. Create feature branch: `git checkout -b feature/amazing-feature`
3. Make changes and write tests
4. Review acceptance tests in `tests/acceptance/`
5. Run: `cargo test && cargo clippy`
6. Commit: `git commit -m 'Add amazing feature'`
7. Push and open Pull Request

## License

Apache License 2.0 - see [LICENSE](LICENSE) for details.

By contributing, you agree to the [Contributor License Agreement](.github/CLA.md).

## Why HORUS?

- **Ultra-Low Latency**: Sub-microsecond IPC (typically 300-500ns)
- **High Throughput**: 6+ million messages per second
- **Simple Setup**: No complex configuration files
- **Memory Safe**: Rust + fixed-size messages
- **Built-in Debugging**: Integrated dashboard
- **Easy to Learn**: Simple `tick()` pattern
- **Zero-Copy IPC**: Maximum performance
- **Production-Validated**: 6.2M+ test messages, zero corruptions

**HORUS: Real-time robotics made simple**
