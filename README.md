# HORUS Framework

**Hybrid Optimized Robotics Unified System**

HORUS is a modern Rust-based robotics framework designed for real-time performance, shared memory communication, and comprehensive system monitoring. Built for both research and production robotics applications.

## Key Features

### **Real-Time Performance**
- **Production Latency**: 296ns-176μs (16B-120KB messages)
- **Sub-Microsecond Messaging**: 296-718ns for control messages, ultra-low latency for time-critical robotics
- **Priority-Based Scheduling**: Deterministic execution order (0 = highest priority)
- **Lock-Free Communication**: Atomic operations with cache-line alignment
- **Built-in Ctrl+C Handling**: Reliable termination with proper cleanup

### **Developer Experience**
- **Simple Node API**: Clean `tick()` method with `init()` and `shutdown()` lifecycle
- **Macro-Based Development**: `node!` macro eliminates boilerplate code
- **Multi-Language Support**: Rust, Python, C with unified workflow
- **Built-in Logging**: Automatic pub/sub tracking with IPC timing
- **Unified CLI**: `horus` command for project management

### **Production Ready**
- **Memory-Safe Messaging**: Fixed-size structures prevent corruption
- **Cross-Process Communication**: POSIX shared memory with proper alignment
- **Performance Benchmarks**: Comprehensive latency testing and optimization
- **Dashboard Monitoring**: Web and terminal UI for real-time system monitoring

## Alpha Release Status

HORUS is currently in **alpha development** (v0.1.0-alpha). The core framework is production-grade with proven sub-microsecond latency, but some ecosystem tools are still incomplete.

### What Works

**Core Framework (Production-Ready)**
- Hub pub/sub messaging with 296ns-1.31μs latency (control/sensor messages)
- Priority-based scheduler with deterministic execution
- Multi-language support (Rust, Python, C)
- Shared memory IPC with zero-copy performance
- Built-in logging and metrics

**CLI and Tools (Functional)**
- Project creation and management
- Build and execution workflows
- Package installation and search
- Basic monitoring dashboard
- GitHub OAuth authentication

**Package Registry and Marketplace (Production-Ready)**
- Full package registry backend with SQLite
- Package upload, download, and versioning
- GitHub OAuth authentication for publishing
- Package search and metadata management
- Documentation serving (local and external)
- Web-based marketplace at marketplace.horus-registry.dev

### What's Incomplete

**Dashboard**
- Web dashboard works
- TUI mode is incomplete (in development)

**Remote Deployment**
- Basic HTTP deployment works
- Versioning and rollback features not yet implemented

**Language Bindings**
- Python bindings functional but missing type hints
- C bindings are in alpha (minimal operations only, under active development)

### Recommendations

**Safe to Use For:**
- Research and education projects
- Prototyping robotics applications
- Learning real-time systems
- Performance-critical local IPC
- Publishing and sharing packages via the registry

**Wait for v0.1.5+ For:**
- Advanced remote deployment features (versioning, rollback)
- Complete dashboard TUI experience

### Roadmap

See [ROADMAP.md](ROADMAP.md) for detailed development plans and timelines.

## Installation

### Platform Support

HORUS works on systems with POSIX shared memory support:

| Platform | Status | Notes |
|----------|--------|-------|
| **Ubuntu 20.04+** | ✅ Fully Supported | Recommended for production |
| **Ubuntu 22.04+** | ✅ Fully Supported | Best performance |
| **Debian 11+** | ✅ Supported | Tested and working |
| **Fedora 36+** | ✅ Supported | Use dnf for packages |
| **Arch Linux** | ✅ Supported | Community maintained |
| **macOS 11+** | ✅ Supported | Limited shared memory size |
| **Windows** | ⚠️ Via WSL Only | Use WSL 2 for best results |
| **Raspberry Pi** | ✅ Supported | ARM64 tested on Ubuntu |

### Prerequisites

**Required:**
- **Rust 1.70+** (install from [rustup.rs](https://rustup.rs))
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  ```

- **Build tools** (C compiler, pkg-config, system libraries)
  ```bash
  # Ubuntu/Debian - COMPLETE dependencies
  sudo apt update && sudo apt install \
    build-essential \
    pkg-config \
    libudev-dev \
    libssl-dev \
    libasound2-dev

  # Fedora/RHEL/CentOS
  sudo dnf groupinstall "Development Tools"
  sudo dnf install pkg-config systemd-devel openssl-devel alsa-lib-devel

  # Arch Linux
  sudo pacman -S base-devel pkg-config systemd openssl alsa-lib

  # macOS
  xcode-select --install
  brew install pkg-config openssl
  ```

- **Linux** (Ubuntu 20.04+, other distros supported) or **macOS**

**Optional:**
- Python 3.8+ for Python bindings: `sudo apt install python3 python3-pip`

### Quick Install (Recommended)

**One command to install everything:**

```bash
git clone https://github.com/neos-builder/horus.git
cd horus
./install.sh
```

**What happens during installation:**
- Builds all packages in release mode (`cargo build --release`)
- Installs `horus` CLI to `~/.cargo/bin/`
- Installs runtime libraries to `~/.horus/cache/`
- Verifies installation and tests `horus` command

**After installation, you can create projects anywhere:**
```bash
cd ~/my_projects
horus new my_robot
cd my_robot
horus run  # Just works - no registry downloads needed!
```

### Verify Installation

```bash
# Check CLI is available
horus --version

# Check libraries are installed
ls ~/.horus/cache/
# Should show: horus@0.1.0, horus_core@0.1.0, horus_macros@0.1.0, horus_library@0.1.0
```

### Troubleshooting

**`horus: command not found`**
```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.cargo/bin:$PATH"
source ~/.bashrc
```

**Build fails with linker errors**
```bash
# Ubuntu/Debian
sudo apt install build-essential pkg-config libssl-dev

# Fedora/RHEL
sudo dnf install gcc gcc-c++ openssl-devel
```

**Libraries not found during `horus run`**
```bash
# Re-run installation
cd path/to/HORUS
./install.sh
```

### Uninstallation

To completely remove HORUS:

```bash
cd HORUS
./uninstall.sh
```

The script will:
- Remove CLI binary from `~/.cargo/bin/`
- Remove global library cache from `~/.horus/cache/`
- Ask before removing entire `~/.horus/` directory
- Preserve project-local `.horus/` directories

## Quick Start

### 1. Create a New Project
```bash
horus new my_robot
cd my_robot
```

### 2. Simple Node Example
```rust
use horus::prelude::*;

pub struct SensorNode {
    publisher: Hub<f64>,
    counter: u32,
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

fn main() -> HorusResult<()> {
    let mut scheduler = Scheduler::new();

    scheduler.register(
        Box::new(SensorNode {
            publisher: Hub::new("sensor_data")?,
            counter: 0,
        }),
        0,           // Priority (0 = highest)
        Some(true)   // Enable logging
    );

    scheduler.tick_all()
}
```

### 3. Run the Project
```bash
horus run --release
```

**Expected Output:**
```
Registered node 'SensorNode' with priority 0 (logging: true)
SensorNode initialized
[12:34:56.123] [IPC: 296ns | Tick: 12µs] SensorNode --PUB--> 'sensor_data' = 0.0
[12:34:56.223] [IPC: 370ns | Tick: 11µs] SensorNode --PUB--> 'sensor_data' = 0.1
```

## Project Structure

```
HORUS/
├── horus/                      # Main unified crate
├── horus_core/                 # Core framework implementation
│   ├── communication/          # Hub, shared memory (ShmTopic)
│   ├── scheduling/             # Scheduler
│   ├── core/                   # Node trait, NodeInfo
│   └── memory/                 # Shared memory management
├── horus_manager/              # CLI tool (horus command)
├── horus_daemon/               # Remote deployment daemon
├── horus_macros/               # node! procedural macro
├── horus_py/                   # Python bindings
├── horus_c/                    # C bindings
├── horus_library/              # Standard library and examples
│   ├── messages/               # Standard message types
│   ├── apps/snakesim/          # Example app: Snake game demo
│   ├── apps/tanksim/           # Example app: Tank simulation
│   └── tools/sim2d/            # 2D physics simulator
├── benchmarks/                 # Performance testing
└── docs-site/                  # Documentation website
```

## CLI Commands

### Project Management
```bash
horus new <name>                # Create new project (interactive)
horus new my_robot -r           # Create Rust project
horus new my_robot -p           # Create Python project
horus new my_robot -c           # Create C project (⚠️ alpha - under development)
horus new my_robot -m           # Create Rust project with macros
```

### Build and Run
```bash
horus run                       # Auto-detect and run
horus run main.rs               # Run specific file
horus run --release             # Build in release mode
horus run --build-only          # Build without running
horus run --clean               # Clean build cache
horus run --remote robot:8080   # Deploy to remote robot
```

### Package Management
```bash
horus pkg install <package>              # Install package
horus pkg install <package> -v 1.0.0     # Install specific version
horus pkg install <package> -g           # Install to global cache
horus pkg remove <package>               # Remove package
horus pkg list                           # List local packages
horus pkg list -g                        # List global cache
horus pkg list <query>                   # Search registry
```

### Environment Management
```bash
horus env freeze                         # Freeze to horus-freeze.yaml
horus env freeze -o custom.yaml          # Freeze to custom file
horus env freeze --publish               # Publish to registry
horus env restore horus-freeze.yaml      # Restore from file
horus env restore env_abc123             # Restore from registry ID
```

### Publishing
```bash
horus publish                   # Publish package to registry
horus publish --freeze          # Publish and generate freeze file
```

### Authentication
```bash
horus auth login --github       # Login with GitHub OAuth
horus auth generate-key         # Generate API key
horus auth whoami               # Show current user
horus auth logout               # Logout
```

### Dashboard
```bash
horus dashboard                 # Web dashboard (port 3000, auto-opens browser)
horus dashboard 3001            # Custom port
horus dashboard -t              # Terminal UI mode
```

## Core API

### Scheduler
```rust
use horus_core::scheduling::Scheduler;

let mut scheduler = Scheduler::new();

// Register nodes with priority and logging
scheduler.register(Box::new(my_node), priority, Some(logging));

// Run scheduler (blocks until Ctrl+C)
scheduler.tick_all()?;
```

**Methods:**
- `new()` - Create scheduler
- `register(node, priority, logging)` - Add node (priority: 0=highest)
- `tick_all()` - Run main loop with Ctrl+C handling
- `tick_node(&[names])` - Run specific nodes only
- `stop()` - Stop scheduler
- `is_running()` - Check if running

### Hub (Pub/Sub)
```rust
use horus_core::communication::horus::Hub;

// Create publisher/subscriber
let hub: Hub<f64> = Hub::new("topic_name")?;
let hub_custom: Hub<MyMsg> = Hub::new_with_capacity("topic", 2048)?;

// Publish
hub.send(42.0, ctx)?;  // With logging context
hub.send(42.0, None)?; // Without logging

// Subscribe
if let Some(msg) = hub.recv(ctx) {
    // Process message
}
```

**Performance:**
- 296ns for small messages (16B)
- 718ns for IMU data (304B)
- 1.31μs for LaserScan (1.5KB)
- 2.8μs for PointCloud (120KB)

### Node Trait
```rust
use horus_core::core::{Node, NodeInfo};

pub trait Node: Send {
    fn name(&self) -> &'static str;
    fn init(&mut self, ctx: &mut NodeInfo) -> Result<(), String> { Ok(()) }
    fn tick(&mut self, ctx: Option<&mut NodeInfo>);
    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<(), String> { Ok(()) }
    fn get_publishers(&self) -> Vec<TopicMetadata> { Vec::new() }
    fn get_subscribers(&self) -> Vec<TopicMetadata> { Vec::new() }
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

### SnakeSim (Example App - Multi-Node Game)
```bash
cd horus_library/apps/snakesim/snake_scheduler
cargo run --release
```

**Architecture:**
- **KeyboardInputNode** (priority 0): Arrow key input
- **SnakeControlNode** (priority 2): Game logic
- **GUINode** (priority 3): Terminal rendering

### Sim2D Physics Simulator
```bash
cd horus_library/tools/sim2d
cargo run --release
```

Complete 2D robotics simulator with Bevy visualization and Rapier2D physics.

## Monitoring

HORUS provides automatic monitoring through NodeInfo:

```rust
fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
    if let Some(ctx) = ctx {
        // Automatic metrics tracking
        println!("Avg tick: {}μs", ctx.metrics().avg_tick_duration_ms * 1000.0);
        println!("Total ticks: {}", ctx.metrics().total_ticks);
        println!("Errors: {}", ctx.metrics().errors_count);
    }
}
```

**Metrics Available:**
- `total_ticks` - Total number of ticks
- `avg_tick_duration_ms` - Average tick time
- `max_tick_duration_ms` - Worst-case tick time
- `messages_sent` - Published messages
- `messages_received` - Subscribed messages
- `errors_count` - Error count
- `uptime_seconds` - Node uptime

## Remote Deployment

Deploy to physical robots via HTTP:

```bash
horus run --remote 192.168.1.100:8080
```

**Process:**
1. Packages project as `.tar.gz`
2. POSTs to `http://ROBOT:8080/deploy`
3. `horus_daemon` receives and builds
4. Returns deployment ID + PID

**Start daemon on robot:**
```bash
cargo run --release -p horus_daemon
```

## Shared Memory

HORUS uses `/dev/shm` for zero-copy IPC:

```bash
# View shared memory regions
ls -lh /dev/shm/horus/

# Check space
df -h /dev/shm
```

**Configuration:**
- Default capacity: 1024 slots per topic
- Custom: `Hub::new_with_capacity(topic, 2048)`
- Location: `/dev/shm/horus/<topic_name>`

## Multi-Language Support

HORUS supports Python and C in addition to Rust. Language bindings are currently in alpha and under active development.

### Python

See [horus_py/README.md](horus_py/README.md) for the complete Python API documentation and examples.

**Quick Example:**
```python
import horus

def process(node):
    node.send("output", 42.0)

node = horus.Node(pubs="output", tick=process, rate=30)
horus.run(node, duration=5)
```

### C

See [horus_c/README.md](horus_c/README.md) for C bindings documentation.

**Note:** C bindings currently support minimal operations. Full API coverage is planned for future releases.

## Use Cases

### Research & Education
- Clean API for learning robotics
- Built-in monitoring eliminates debugging complexity
- Complete examples demonstrate best practices

### Real-Time Systems
- Priority-based scheduling for timing guarantees
- Shared memory for microsecond latencies
- Thread-safe communication

### Production Robotics
- Reliable Ctrl+C handling and cleanup
- Comprehensive system monitoring
- Memory-safe message passing for 24/7 operation

## Community

Join our Discord community for support, discussions, and updates:

**[Join HORUS Discord](https://discord.gg/hEZC3ev2Nf)**

Get help, share your projects, and connect with other HORUS developers!

## Contributing

1. Fork the repository
2. Create feature branch: `git checkout -b feature/amazing-feature`
3. Commit changes: `git commit -m 'Add amazing feature'`
4. Push to branch: `git push origin feature/amazing-feature`
5. Open Pull Request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Contributor License Agreement

By contributing to HORUS, you agree to our [Contributor License Agreement](.github/CLA.md). This helps us maintain the project's licensing integrity while protecting both contributors and users.

---

## Why HORUS?

**Key Advantages:**

- **Ultra-Low Latency**: 296ns-2.8μs message passing with shared memory architecture
- **Simple Setup**: `register()` → `tick_all()` - no complex configuration files
- **Memory Safe**: Rust + fixed-size messages prevent corruption and race conditions
- **Built-in Debugging**: Integrated dashboard for real-time system monitoring
- **Easy to Learn**: Simple `tick()` method pattern with intuitive lifecycle
- **Zero-Copy IPC**: Direct shared memory access for maximum performance

**HORUS: Real-time robotics made simple**

*Build faster. Debug easier. Deploy with confidence.*
