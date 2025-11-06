# HORUS C++ API (Redesigned v0.4.0)

**Modern C++17 API for hardware driver integration - now with unified design matching Rust/Python!**

## What's New in v0.4.0

✅ **Single Node pattern** - no more Simple vs Complex confusion
✅ **Matches Rust API** - same scheduler, priorities, context
✅ **Matches Python API** - similar structure and simplicity
✅ **No std::optional bloat** - clean, direct construction
✅ **Unified logging** - works with dashboard like Rust/Python
✅ **Full lifecycle** - init/tick/shutdown with context
✅ **44% less boilerplate** - from 34 to 19 lines for simple drivers

---

## Quick Start

### Minimal Example (19 lines!)

```cpp
#include <horus.hpp>
using namespace horus;

struct LidarDriver : Node {
    Publisher<LaserScan> scan_pub;
    LidarDevice device;

    LidarDriver() : Node("lidar_driver"), scan_pub("scan"), device("/dev/ttyUSB0") {}

    void tick(NodeContext& ctx) override {
        scan_pub.send(device.read());
    }
};

int main() {
    Scheduler scheduler;
    scheduler.add(LidarDriver(), 2, true);  // priority=2, logging=true
    scheduler.run();
}
```

**Compile and run:**
```bash
horus run lidar_driver.cpp
```

---

## API Overview

### Node - Base class for all nodes

```cpp
struct MyNode : horus::Node {
    MyNode() : Node("my_node") {}

    // Called once at startup (returns true on success)
    bool init(NodeContext& ctx) override {
        ctx.log_info("Initializing...");
        return true;
    }

    // Called at 60 FPS by scheduler
    void tick(NodeContext& ctx) override {
        // Main logic here
    }

    // Called once at shutdown (returns true on success)
    bool shutdown(NodeContext& ctx) override {
        ctx.log_info("Shutting down...");
        return true;
    }
};
```

**Lifecycle:** `init()` → `tick()` loop @ 60 FPS → `shutdown()`

---

### Publisher<T> - Type-safe message publisher

```cpp
Publisher<Twist> cmd_pub;          // Default (invalid)
Publisher<Twist> cmd_pub("topic"); // Create for topic

// Send message (throws on error)
cmd_pub.send(msg);

// Try send (returns false on error, no exceptions)
bool success = cmd_pub.try_send(msg);
```

**Supported types:** All 40+ built-in message types (Twist, LaserScan, Imu, Image, etc.)

---

### Subscriber<T> - Type-safe message subscriber

```cpp
Subscriber<LaserScan> scan_sub("scan");

LaserScan scan;
if (scan_sub.recv(scan)) {
    // Message received
}
```

**Non-blocking:** Returns `false` if no messages available

---

### NodeContext - Runtime context (matches Rust's NodeInfo)

```cpp
void tick(NodeContext& ctx) override {
    // Logging (writes to global log buffer for dashboard)
    ctx.log_info("Information message");
    ctx.log_warn("Warning message");
    ctx.log_error("Error message");
    ctx.log_debug("Debug message");

    // Node information
    const char* name = ctx.node_name();
    uint64_t ticks = ctx.tick_count();
}
```

**All logs appear in dashboard!**

---

### Scheduler - Manages node execution @ 60 FPS

```cpp
Scheduler scheduler;

// Add nodes with priorities and logging
// priority: 0=Critical, 1=High, 2=Normal, 3=Low, 4=Background
// enable_logging: true for dashboard logs + IPC timing
scheduler.add(SafetyNode(),  0, true);  // Critical - runs first
scheduler.add(ControlNode(), 1, true);  // High
scheduler.add(SensorNode(),  2, true);  // Normal

// Run at 60 FPS (blocks until Ctrl+C)
scheduler.run();
```

**Execution order:** Nodes run by priority (0 first, 4 last)

---

## Complete Examples

### 1. Hardware Driver (LiDAR)

```cpp
#include <horus.hpp>
using namespace horus;

struct LidarDriver : Node {
    Publisher<LaserScan> scan_pub;
    LidarDevice device;
    uint32_t scan_count;

    LidarDriver()
        : Node("lidar_driver"),
          scan_pub("scan"),
          scan_count(0) {}

    bool init(NodeContext& ctx) override {
        ctx.log_info("Initializing LiDAR...");

        if (!device.open("/dev/ttyUSB0")) {
            ctx.log_error("Failed to open LiDAR");
            return false;
        }

        ctx.log_info("LiDAR ready @ 10Hz");
        return true;
    }

    void tick(NodeContext& ctx) override {
        LaserScan scan = device.read();
        scan_pub.send(scan);  // Logged to dashboard with IPC timing!
        scan_count++;

        if (scan_count % 60 == 0) {
            ctx.log_info("Published " + std::to_string(scan_count) + " scans");
        }
    }

    bool shutdown(NodeContext& ctx) override {
        ctx.log_info("Shutting down LiDAR");
        ctx.log_info("Total scans: " + std::to_string(scan_count));
        device.close();
        return true;
    }
};

int main() {
    Scheduler scheduler;
    scheduler.add(LidarDriver(), 2, true);
    scheduler.run();
}
```

---

### 2. Multi-Node Application

```cpp
#include <horus.hpp>
using namespace horus;

// Sensor node
struct ImuDriver : Node {
    Publisher<Imu> imu_pub;

    ImuDriver() : Node("imu_driver"), imu_pub("imu") {}

    void tick(NodeContext& ctx) override {
        Imu data = read_imu_hardware();
        imu_pub.send(data);
    }
};

// Control node
struct Controller : Node {
    Subscriber<Imu> imu_sub;
    Publisher<Twist> cmd_pub;

    Controller() : Node("controller"), imu_sub("imu"), cmd_pub("cmd_vel") {}

    void tick(NodeContext& ctx) override {
        Imu imu;
        if (imu_sub.recv(imu)) {
            Twist cmd = compute_control(imu);
            cmd_pub.send(cmd);
        }
    }
};

// Safety node
struct SafetyMonitor : Node {
    Subscriber<Twist> cmd_sub;
    Publisher<EmergencyStop> estop_pub;

    SafetyMonitor() : Node("safety"), cmd_sub("cmd_vel"), estop_pub("estop") {}

    void tick(NodeContext& ctx) override {
        Twist cmd;
        if (cmd_sub.recv(cmd) && is_unsafe(cmd)) {
            ctx.log_warn("UNSAFE COMMAND!");
            estop_pub.send(EmergencyStop::engage("Velocity limit"));
        }
    }
};

int main() {
    Scheduler scheduler;

    // Add by priority (Critical first, Background last)
    scheduler.add(SafetyMonitor(), 0, true);  // Critical
    scheduler.add(Controller(),    1, true);  // High
    scheduler.add(ImuDriver(),     2, true);  // Normal

    scheduler.run();  // 60 FPS
}
```

---

## Comparison: Old vs New API

### Old API (34 lines)

```cpp
#include <horus.hpp>

class LidarDriver : public horus::Node {
private:
    std::optional<horus::Publisher<LaserScan>> scan_pub_;
    LidarDevice device_;

public:
    LidarDriver() : Node("lidar_driver") {}

    bool init(horus::NodeContext& ctx) override {
        scan_pub_.emplace(ctx.create_publisher<LaserScan>("scan"));
        device_.open("/dev/ttyUSB0");
        return true;
    }

    void tick(horus::NodeContext& ctx) override {
        LaserScan scan = device_.read();
        *scan_pub_ << scan;
    }

    void shutdown(horus::NodeContext& ctx) override {}
};

int main() {
    try {
        horus::Scheduler scheduler("scheduler");
        LidarDriver driver;
        scheduler.add(driver, 2, true);
        scheduler.run();
    } catch (const horus::HorusException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    return 0;
}
```

### New API (19 lines - 44% less!)

```cpp
#include <horus.hpp>
using namespace horus;

struct LidarDriver : Node {
    Publisher<LaserScan> scan_pub;
    LidarDevice device;

    LidarDriver() : Node("lidar_driver"), scan_pub("scan"), device("/dev/ttyUSB0") {}

    void tick(NodeContext& ctx) override {
        scan_pub.send(device.read());
    }
};

int main() {
    Scheduler scheduler;
    scheduler.add(LidarDriver(), 2, true);
    scheduler.run();
}
```

**Improvements:**
- ❌ No `std::optional` wrapper
- ❌ No `emplace()` calls
- ❌ No stream operators (`<<`)
- ❌ No try-catch boilerplate
- ✅ Direct member initialization
- ✅ Clean method calls
- ✅ Matches Rust/Python patterns

---

## Dashboard Integration

**All C++ nodes are automatically monitored!**

```bash
# Terminal 1: Run your robot
horus run robot.cpp

# Terminal 2: Start dashboard
horus dashboard
```

**Dashboard shows:**
- ✅ All nodes (C++, Rust, Python)
- ✅ All topics and connections
- ✅ Real-time logs with IPC timing
- ✅ Performance metrics (CPU, tick rate)
- ✅ Error counts

**Example dashboard logs:**
```
[14:32:15.123] [INFO]    [lidar_driver] Initializing LiDAR...
[14:32:15.456] [INFO]    [lidar_driver] LiDAR ready @ 10Hz
[14:32:15.470] [PUBLISH] [lidar_driver] Published to 'scan' (481ns)
[14:32:15.471] [SUBSCRIBE] [controller] Received from 'scan' (312ns)
```

**With `enable_logging=true`, every pub/sub shows IPC latency!**

---

## API Consistency

### C++ vs Rust vs Python

| Feature | Rust | Python | C++ (New) |
|---------|------|--------|-----------|
| **Node creation** | `impl Node` | `class Node(horus.Node)` | `struct Node : horus::Node` |
| **Lifecycle** | `init/tick/shutdown` | `init/tick/shutdown` | `init/tick/shutdown` |
| **Context** | `&mut NodeInfo` | `info` | `NodeContext&` |
| **Publisher** | `Hub<T>` | `pubs="topic"` | `Publisher<T>("topic")` |
| **Subscriber** | `Hub<T>` | `subs="topic"` | `Subscriber<T>("topic")` |
| **Send** | `.send(msg, ctx)` | `.send("topic", msg)` | `.send(msg)` |
| **Receive** | `.recv(ctx)` | `.get("topic")` | `.recv(msg)` |
| **Scheduler** | `.add(node, pri, log)` | `horus.run(node)` | `.add(node, pri, log)` |
| **Run** | `.run()` | `horus.run()` | `.run()` |

**C++ now matches Rust structure exactly!**

---

## Message Types Supported

All 40+ built-in message types work:

**Geometry (6):** Twist, Pose2D, Transform, Vector3, Point3, Quaternion
**Sensors (5):** LaserScan, Imu, Odometry, Range, BatteryState
**Vision (7):** Image, CompressedImage, CameraInfo, Detection, DetectionArray, etc.
**Perception (6):** PointCloud, BoundingBox3D, DepthImage, etc.
**Navigation (9):** Goal, Path, OccupancyGrid, etc.
**Control (6):** MotorCommand, DifferentialDriveCommand, JointCommand, etc.
**Diagnostics (5):** Heartbeat, Status, EmergencyStop, etc.

---

## Building and Running

```bash
# Single file
horus run driver.cpp

# Multiple files
horus run main.cpp sensor.cpp controller.cpp

# With external libraries
horus run realsense_driver.cpp -lrealsense2

# Debug build
horus run --debug robot.cpp

# Release build (optimized)
horus run --release robot.cpp
```

**No CMakeLists.txt needed - HORUS handles everything!**

---

## Migration from Old API

### Before (Old API)

```cpp
std::optional<horus::Publisher<T>> pub_;
pub_.emplace(ctx.create_publisher<T>("topic"));
*pub_ << msg;
```

### After (New API)

```cpp
Publisher<T> pub_{"topic"};
pub_.send(msg);
```

### Before (Old API)

```cpp
class MyNode : public horus::Node {
    bool init(horus::NodeContext& ctx) override;
    void tick(horus::NodeContext& ctx) override;
    void shutdown(horus::NodeContext& ctx) override;
};
```

### After (New API)

```cpp
struct MyNode : horus::Node {
    bool init(NodeContext& ctx) override;
    void tick(NodeContext& ctx) override;
    bool shutdown(NodeContext& ctx) override;
};
```

**Changes:**
1. Remove `std::optional` wrappers
2. Use direct construction in initializer list
3. Use `send()` instead of `<<`
4. Use `using namespace horus;` for cleaner code
5. Prefer `struct` over `class` (public by default)

---

## Examples

All examples are in `horus_cpp/examples/`:

- `pubsub_simple_new.cpp` - Basic pub-sub pattern
- `lidar_driver_new.cpp` - Hardware driver integration
- `robot_system_new.cpp` - Multi-node application

**Run any example:**
```bash
horus run examples/pubsub_simple_new.cpp
```

---

## Next Steps

- See complete API: `include/horus_new.hpp`
- Run examples: `horus run examples/*.cpp`
- Read [Architecture Docs](/docs/architecture)
- Join discussions: https://github.com/softmata/horus/discussions

**Ready to build?** Start with `examples/pubsub_simple_new.cpp`!
