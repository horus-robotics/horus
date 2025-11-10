# HORUS Language Comparison: C++ vs Rust vs Python

Complete side-by-side comparison of the HORUS API across all three supported languages.

---

## Simple Temperature Sensor

### Python (10 lines)

```python
import horus

class TempSensor(horus.Node):
    def __init__(self):
        super().__init__(name="sensor", pubs="temperature", rate=10)

    def tick(self, info=None):
        self.send("temperature", 25.0)

horus.run(TempSensor())
```

### Rust (26 lines)

```rust
use horus::prelude::*;

struct TempSensor {
    temp_pub: Hub<f32>,
}

impl TempSensor {
    fn new() -> Result<Self> {
        Ok(Self {
            temp_pub: Hub::new("temperature")?,
        })
    }
}

impl Node for TempSensor {
    fn name(&self) -> &'static str { "TempSensor" }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        self.temp_pub.send(25.0, ctx).ok();
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(TempSensor::new()?), 0, Some(true));
    scheduler.run()?;
    Ok(())
}
```

### C++ (19 lines)

```cpp
#include <horus.hpp>
using namespace horus;

struct TempSensor : Node {
    Publisher<float> temp_pub;

    TempSensor() : Node("TempSensor"), temp_pub("temperature") {}

    void tick(NodeContext& ctx) override {
        temp_pub.send(25.0);
    }
};

int main() {
    Scheduler scheduler;
    scheduler.add(TempSensor(), 0, true);
    scheduler.run();
}
```

---

## LiDAR Driver with Full Lifecycle

### Python (30 lines)

```python
import horus

class LidarDriver(horus.Node):
    def __init__(self):
        super().__init__(name="lidar", pubs="scan", rate=10)
        self.device = None
        self.scan_count = 0

    def init(self, info=None):
        print("Initializing LiDAR...")
        self.device = LidarDevice("/dev/ttyUSB0")
        self.device.open()
        print("LiDAR ready @ 10Hz")

    def tick(self, info=None):
        scan = self.device.read()
        self.send("scan", scan)
        self.scan_count += 1

        if self.scan_count % 60 == 0:
            print(f"Published {self.scan_count} scans")

    def shutdown(self, info=None):
        print(f"Total scans: {self.scan_count}")
        self.device.close()

driver = LidarDriver()
horus.run(driver)
```

### Rust (60 lines)

```rust
use horus::prelude::*;

struct LidarDriver {
    scan_pub: Hub<LaserScan>,
    device: LidarDevice,
    scan_count: u32,
}

impl LidarDriver {
    fn new() -> Result<Self> {
        Ok(Self {
            scan_pub: Hub::new("scan")?,
            device: LidarDevice::new("/dev/ttyUSB0"),
            scan_count: 0,
        })
    }
}

impl Node for LidarDriver {
    fn name(&self) -> &'static str { "LidarDriver" }

    fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        ctx.log_info("Initializing LiDAR...");
        self.device.open()?;
        ctx.log_info("LiDAR ready @ 10Hz");
        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        let scan = self.device.read();
        self.scan_pub.send(scan, ctx).ok();
        self.scan_count += 1;

        if self.scan_count % 60 == 0 {
            ctx.log_info(&format!("Published {} scans", self.scan_count));
        }
    }

    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        ctx.log_info(&format!("Total scans: {}", self.scan_count));
        self.device.close()?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(LidarDriver::new()?), 2, Some(true));
    scheduler.run()?;
    Ok(())
}
```

### C++ (55 lines)

```cpp
#include <horus.hpp>
using namespace horus;

struct LidarDriver : Node {
    Publisher<LaserScan> scan_pub;
    LidarDevice device;
    uint32_t scan_count;

    LidarDriver()
        : Node("LidarDriver"),
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
        scan_pub.send(scan);
        scan_count++;

        if (scan_count % 60 == 0) {
            ctx.log_info("Published " + std::to_string(scan_count) + " scans");
        }
    }

    bool shutdown(NodeContext& ctx) override {
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

## Multi-Node Robot System

### Python (50 lines)

```python
import horus

class ImuDriver(horus.Node):
    def __init__(self):
        super().__init__(name="imu", pubs="imu", rate=60)

    def tick(self, info=None):
        imu_data = read_imu()
        self.send("imu", imu_data)

class Controller(horus.Node):
    def __init__(self):
        super().__init__(name="controller", pubs="cmd_vel", subs=["imu", "scan"], rate=30)

    def tick(self, info=None):
        imu = self.get("imu")
        scan = self.get("scan")

        if imu and scan:
            cmd = compute_control(imu, scan)
            self.send("cmd_vel", cmd)

class SafetyMonitor(horus.Node):
    def __init__(self):
        super().__init__(name="safety", pubs="estop", subs="cmd_vel", rate=60)

    def tick(self, info=None):
        cmd = self.get("cmd_vel")

        if cmd and is_unsafe(cmd):
            print("UNSAFE COMMAND!")
            self.send("estop", True)

# Run all nodes
imu = ImuDriver()
controller = Controller()
safety = SafetyMonitor()

# Python doesn't have explicit priority, but can run sequentially
horus.run([safety, controller, imu])
```

### Rust (120 lines)

```rust
use horus::prelude::*;

// IMU Driver
struct ImuDriver {
    imu_pub: Hub<Imu>,
}

impl ImuDriver {
    fn new() -> Result<Self> {
        Ok(Self { imu_pub: Hub::new("imu")? })
    }
}

impl Node for ImuDriver {
    fn name(&self) -> &'static str { "ImuDriver" }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        let imu = read_imu();
        self.imu_pub.send(imu, ctx).ok();
    }
}

// Controller
struct Controller {
    imu_sub: Hub<Imu>,
    scan_sub: Hub<LaserScan>,
    cmd_pub: Hub<Twist>,
}

impl Controller {
    fn new() -> Result<Self> {
        Ok(Self {
            imu_sub: Hub::new("imu")?,
            scan_sub: Hub::new("scan")?,
            cmd_pub: Hub::new("cmd_vel")?,
        })
    }
}

impl Node for Controller {
    fn name(&self) -> &'static str { "Controller" }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        if let (Some(imu), Some(scan)) = (self.imu_sub.recv(ctx), self.scan_sub.recv(ctx)) {
            let cmd = compute_control(imu, scan);
            self.cmd_pub.send(cmd, ctx).ok();
        }
    }
}

// Safety Monitor
struct SafetyMonitor {
    cmd_sub: Hub<Twist>,
    estop_pub: Hub<EmergencyStop>,
}

impl SafetyMonitor {
    fn new() -> Result<Self> {
        Ok(Self {
            cmd_sub: Hub::new("cmd_vel")?,
            estop_pub: Hub::new("estop")?,
        })
    }
}

impl Node for SafetyMonitor {
    fn name(&self) -> &'static str { "SafetyMonitor" }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        if let Some(cmd) = self.cmd_sub.recv(ctx) {
            if is_unsafe(cmd) {
        ctx.log_warn("UNSAFE COMMAND!");
                self.estop_pub.send(EmergencyStop::engage("Unsafe"), ctx).ok();
            }
        }
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();

    // Add with priorities
    scheduler.add(Box::new(SafetyMonitor::new()?), 0, Some(true));  // Critical
    scheduler.add(Box::new(Controller::new()?), 1, Some(true));     // High
    scheduler.add(Box::new(ImuDriver::new()?), 2, Some(true));      // Normal

    scheduler.run()?;
    Ok(())
}
```

### C++ (100 lines)

```cpp
#include <horus.hpp>
using namespace horus;

// IMU Driver
struct ImuDriver : Node {
    Publisher<Imu> imu_pub;

    ImuDriver() : Node("ImuDriver"), imu_pub("imu") {}

    void tick(NodeContext& ctx) override {
        Imu imu = read_imu();
        imu_pub.send(imu);
    }
};

// Controller
struct Controller : Node {
    Subscriber<Imu> imu_sub;
    Subscriber<LaserScan> scan_sub;
    Publisher<Twist> cmd_pub;

    Controller()
        : Node("Controller"),
          imu_sub("imu"),
          scan_sub("scan"),
          cmd_pub("cmd_vel") {}

    void tick(NodeContext& ctx) override {
        Imu imu;
        LaserScan scan;

        if (imu_sub.recv(imu) && scan_sub.recv(scan)) {
            Twist cmd = compute_control(imu, scan);
            cmd_pub.send(cmd);
        }
    }
};

// Safety Monitor
struct SafetyMonitor : Node {
    Subscriber<Twist> cmd_sub;
    Publisher<EmergencyStop> estop_pub;

    SafetyMonitor()
        : Node("SafetyMonitor"),
          cmd_sub("cmd_vel"),
          estop_pub("estop") {}

    void tick(NodeContext& ctx) override {
        Twist cmd;

        if (cmd_sub.recv(cmd) && is_unsafe(cmd)) {
            ctx.log_warn("UNSAFE COMMAND!");
            estop_pub.send(EmergencyStop::engage("Unsafe"));
        }
    }
};

int main() {
    Scheduler scheduler;

    // Add with priorities
    scheduler.add(SafetyMonitor(), 0, true);  // Critical
    scheduler.add(Controller(),    1, true);  // High
    scheduler.add(ImuDriver(),     2, true);  // Normal

    scheduler.run();
}
```

---

## Feature Comparison Table

| Feature | Python | Rust | C++ (New) |
|---------|--------|------|-----------|
| **Lines (simple)** | 10 | 26 | 19 |
| **Lines (complex)** | 50 | 120 | 100 |
| **Boilerplate** | Minimal | Moderate | Low |
| **Type safety** | Runtime | Compile-time | Compile-time |
| **Performance** | ~500ns IPC | ~400ns IPC | ~400ns IPC |
| **Learning curve** | Easy | Moderate | Moderate |
| **Node pattern** | Class inheritance | Trait impl | Class inheritance |
| **Lifecycle** | init/tick/shutdown | init/tick/shutdown | init/tick/shutdown |
| **Error handling** | Exceptions | Result<T> | Exceptions + bool |
| **Dashboard logs** |  |  |  |
| **IPC timing** |  |  |  |
| **Priority** | Via rate | 0-4 explicit | 0-4 explicit |
| **Memory safety** | GC | Ownership | Manual |
| **FFI** | Native (PyO3) | Native | Native |

---

## When to Use Each Language

### Python
**Best for:**
- Rapid prototyping
- Data processing (NumPy, OpenCV)
- Machine learning integration
- Scripting and automation
- High-level logic

**Use when:**
- Speed isn't critical
- Simplicity > performance
- Need ML/data science libraries

**Example:** Vision processing, ML inference, high-level planning

---

### Rust
**Best for:**
- Core framework development
- Performance-critical nodes
- Complex state machines
- Custom message types
- Library development

**Use when:**
- Need maximum performance
- Want compile-time guarantees
- Building reusable components

**Example:** Core scheduler, custom IPC, high-frequency control loops

---

### C++
**Best for:**
- Hardware driver integration
- Legacy code integration
- Existing C++ codebases
- Direct hardware access
- Real-time systems

**Use when:**
- Interfacing with hardware SDKs
- Porting existing C++ code
- Need C++ ecosystem libraries

**Example:** LiDAR drivers, camera drivers, motor controllers, IMU integration

---

## Performance Comparison

| Operation | Python | Rust | C++ |
|-----------|--------|------|-----|
| **Hub (MPMC) IPC** | ~500ns | ~481ns | ~481ns |
| **Link (SPSC) IPC** | ~350ns | ~248ns | ~248ns |
| **Node init** | Fast | Fast | Fast |
| **Logging** | Fast | Fast | Fast |
| **Memory** | GC overhead | Zero-cost | Manual mgmt |

**All languages use the same shared memory IPC = same performance!**

---

## Consistency Summary

###  Consistent Across All Languages

- Lifecycle: `init`  `tick` @ 60 FPS  `shutdown`
- Logging: `log_info/warn/error/debug`
- Dashboard integration
- IPC timing metrics
- Message types (40+ built-in)
- Cross-language communication

###  Language-Specific Differences

**Priority:**
- Python: Via `rate` parameter (indirect)
- Rust/C++: Explicit 0-4 priorities

**Error Handling:**
- Python: Exceptions
- Rust: `Result<T>`
- C++: Exceptions + bool returns

**Memory Management:**
- Python: Garbage collected
- Rust: Ownership system
- C++: Manual (RAII)

---

## Recommendation

**Start with Python** for prototyping and high-level logic

**Use C++** for hardware drivers and integrating existing code

**Use Rust** for performance-critical nodes and custom libraries

**All three work together seamlessly!** 

---

## Example Multi-Language System

```
┌────────────────────────────────────────────────────────────┐
│                     Robot System                           │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  [C++] LiDAR Driver  ── [scan] ── [Python] ML Detector │
│                                                           │
│  [C++] IMU Driver    ── [imu]   ── [Rust] Controller   │
│                                                           │
│  [C++] Camera        ── [image] ── [cmd_vel]           │
│                                                           │
│                                      [C++] Motor Driver   │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

**Hardware (C++)  Processing (Python/Rust)  Control (Rust)  Actuation (C++)**

Perfect division of labor! 
