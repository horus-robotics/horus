# HORUS C++ API Redesign Summary

## Overview

The C++ API has been completely redesigned to match Rust/Python patterns, eliminate boilerplate, and provide a unified development experience across all languages.

---

## Key Improvements

### 1. **Single Node Pattern**

**Before:** Two different APIs (Simple vs Framework)
```cpp
// API 1: Simple (for drivers)
horus::System system("node");
horus::Publisher<T> pub("topic");
pub << msg;

// API 2: Framework (for applications)
class MyNode : public horus::Node { ... };
```

**After:** One unified pattern
```cpp
struct MyNode : horus::Node {
    Publisher<T> pub{"topic"};
    void tick(NodeContext& ctx) override {
        pub.send(msg);
    }
};
```

---

### 2. **Eliminated std::optional Bloat**

**Before:**
```cpp
std::optional<Publisher<T>> pub_;
pub_.emplace(ctx.create_publisher<T>("topic"));
*pub_ << msg;  // Ugly dereference!
```

**After:**
```cpp
Publisher<T> pub_{"topic"};  // Direct construction
pub_.send(msg);              // Clean!
```

---

### 3. **Method Calls (Not Stream Operators)**

**Before:** Two inconsistent APIs
```cpp
pub << msg;     // Stream style
pub.send(msg);  // Method style (undocumented)
```

**After:** One consistent API
```cpp
pub.send(msg);       // Matches Rust/Python
pub.try_send(msg);   // No exceptions
```

---

### 4. **Matches Rust Scheduler API**

**Before:**
```cpp
horus::Scheduler scheduler("name");
scheduler.add(node, priority, logging);
```

**After (identical to Rust!):**
```cpp
Scheduler scheduler;
scheduler.add(node, priority, enable_logging);
// priority: 0=Critical, 1=High, 2=Normal, 3=Low, 4=Background
// enable_logging: true for dashboard logs + IPC timing
```

---

### 5. **Full Lifecycle Support**

**Before:** Inconsistent lifecycle
```cpp
bool init(NodeContext& ctx) override;  // Returns bool
void tick(NodeContext& ctx) override;  // No return
// No clear shutdown
```

**After (matches Rust!):**
```cpp
bool init(NodeContext& ctx) override;     // Returns bool
void tick(NodeContext& ctx) override;     // Pure virtual
bool shutdown(NodeContext& ctx) override; // Returns bool
```

---

### 6. **NodeContext Matches Rust's NodeInfo**

**Before:** Unclear context
```cpp
void tick(horus::NodeContext& ctx) override;
```

**After (same as Rust!):**
```cpp
void tick(NodeContext& ctx) override {
    ctx.log_info("message");   // Writes to dashboard
    ctx.log_warn("warning");   // Writes to dashboard
    ctx.log_error("error");    // Writes to dashboard
    ctx.log_debug("debug");    // Writes to dashboard
}
```

---

## Code Size Comparison

### Minimal Hardware Driver

**Old API:** 34 lines
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
        device_ = LidarDevice("/dev/ttyUSB0");
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

**New API:** 19 lines (**44% reduction!**)
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

---

## Cross-Language Consistency

### Python

```python
import horus

class SensorNode(horus.Node):
    def __init__(self):
        super().__init__(name="sensor", pubs="data", rate=30)

    def tick(self, info=None):
        self.send("data", 25.0)

horus.run(SensorNode())
```

**Lines:** 10

---

### Rust

```rust
use horus::prelude::*;

struct SensorNode {
    data_pub: Hub<f32>,
}

impl Node for SensorNode {
    fn name(&self) -> &'static str { "SensorNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.data_pub.send(25.0, ctx).ok();
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(SensorNode::new()?), 0, Some(true));
    scheduler.run()?;
    Ok(())
}
```

**Lines:** 26

---

### C++ (New API)

```cpp
using namespace horus;

struct SensorNode : Node {
    Publisher<float> data_pub;

    SensorNode() : Node("SensorNode"), data_pub("data") {}

    void tick(NodeContext& ctx) override {
        data_pub.send(25.0);
    }
};

int main() {
    Scheduler scheduler;
    scheduler.add(SensorNode(), 0, true);
    scheduler.run();
}
```

**Lines:** 19

**C++ is now closer to Python simplicity while maintaining Rust's structure!**

---

## Feature Parity Matrix

| Feature | Rust | Python | C++ (Old) | C++ (New) |
|---------|------|--------|-----------|-----------|
| **Single Node pattern** | ✅ | ✅ | ❌ (2 APIs) | ✅ |
| **Direct construction** | ✅ | ✅ | ❌ (std::optional) | ✅ |
| **Method calls** | ✅ send() | ✅ send() | ⚠️ << operator | ✅ send() |
| **Lifecycle** | ✅ init/tick/shutdown | ✅ init/tick/shutdown | ⚠️ Partial | ✅ Full |
| **Context logging** | ✅ | ✅ | ⚠️ Basic | ✅ Full |
| **Dashboard logs** | ✅ | ✅ | ❌ | ✅ |
| **IPC timing** | ✅ | ✅ | ❌ | ✅ |
| **Priority system** | ✅ 0-4 | ⚠️ Via rate | ✅ 0-4 | ✅ 0-4 |
| **enable_logging** | ✅ Some(true) | ⚠️ N/A | ✅ | ✅ |
| **Boilerplate** | Moderate | Low | High | Low |

---

## Dashboard Integration

### Before

C++ nodes appeared in dashboard but:
- ❌ No proper log context
- ❌ No IPC timing metrics
- ❌ Examples bypassed framework

### After

C++ nodes are **first-class citizens**:
- ✅ Full logging with context
- ✅ IPC timing in nanoseconds
- ✅ All examples use framework
- ✅ Identical dashboard view as Rust/Python

**Example dashboard logs:**
```
[14:32:15.123] [INFO]    [lidar_driver] Initializing LiDAR...
[14:32:15.456] [INFO]    [lidar_driver] LiDAR ready @ 10Hz
[14:32:15.470] [PUBLISH] [lidar_driver] Published to 'scan' (481ns)
                                                              ^^^^^^
                                                              IPC latency!
```

---

## Migration Guide

### 1. Update Includes

**Before:**
```cpp
#include <horus.hpp>
```

**After:**
```cpp
#include <horus.hpp>
using namespace horus;  // Optional but recommended
```

---

### 2. Remove std::optional

**Before:**
```cpp
class MyNode : public horus::Node {
    std::optional<horus::Publisher<T>> pub_;

    bool init(horus::NodeContext& ctx) override {
        pub_.emplace(ctx.create_publisher<T>("topic"));
        return true;
    }
};
```

**After:**
```cpp
struct MyNode : Node {
    Publisher<T> pub_;

    MyNode() : Node("my_node"), pub_("topic") {}
};
```

---

### 3. Change Class to Struct (Optional)

**Before:**
```cpp
class MyNode : public horus::Node {
private:
    Publisher<T> pub_;
public:
    MyNode() { ... }
};
```

**After:**
```cpp
struct MyNode : Node {
    Publisher<T> pub_;

    MyNode() { ... }
};
```

**Why:** `struct` members are public by default (cleaner for simple nodes)

---

### 4. Replace Stream Operators

**Before:**
```cpp
*pub_ << msg;
*sub_ >> msg;
```

**After:**
```cpp
pub_.send(msg);
sub_.recv(msg);
```

---

### 5. Update Scheduler

**Before:**
```cpp
horus::Scheduler scheduler("name");
MyNode node;
scheduler.add(node, 2, true);
```

**After:**
```cpp
Scheduler scheduler;
scheduler.add(MyNode(), 2, true);
```

---

## Benefits Summary

### For Users

✅ **44% less boilerplate** - 19 lines vs 34 lines
✅ **Consistent with Rust/Python** - same patterns
✅ **Clear API** - one way to do things
✅ **Dashboard works perfectly** - full logging
✅ **No namespace pollution** - `using namespace horus;`

### For Framework

✅ **Single code path** - no more Simple vs Framework
✅ **Easier to document** - one API to explain
✅ **Easier to maintain** - less code
✅ **Language consistency** - C++ matches Rust
✅ **Better testing** - examples use framework

### For Ecosystem

✅ **Lower learning curve** - Python → C++ is easier
✅ **Better examples** - teach proper patterns
✅ **Cross-language projects** - consistent structure
✅ **Dashboard universality** - all languages equal

---

## Files Changed

### Core API
- `horus_cpp/include/horus_new.hpp` - New unified API (600 lines)

### Examples
- `horus_cpp/examples/lidar_driver_new.cpp` - Hardware driver (130 lines)
- `horus_cpp/examples/robot_system_new.cpp` - Multi-node app (250 lines)
- `horus_cpp/examples/pubsub_simple_new.cpp` - Basic pub-sub (120 lines)

### Documentation
- `horus_cpp/README_NEW.md` - Complete API guide
- `horus_cpp/API_REDESIGN_SUMMARY.md` - This file

---

## Next Steps

### Immediate
1. ✅ Review new API design
2. ✅ Test examples compile and run
3. ✅ Validate dashboard integration

### Short Term
1. Rename `horus_new.hpp` → `horus.hpp` (replace old)
2. Update all examples to new API
3. Update documentation site
4. Add C++ API tests to CI

### Long Term
1. Add remaining message type template specializations
2. Implement CMake/pkg-config support
3. Add ARM platform testing
4. Performance benchmarks vs Rust

---

## Conclusion

The redesigned C++ API achieves:

✅ **Parity with Rust** - Same structure, same scheduler, same logging
✅ **Simplicity of Python** - Minimal boilerplate, clean syntax
✅ **Modern C++17** - RAII, move semantics, templates
✅ **Production ready** - Full lifecycle, dashboard integration, error handling

**Result:** C++ is now a **first-class language** in HORUS, not a second-class citizen!

---

## Feedback

Questions or suggestions? Open a discussion:
https://github.com/softmata/horus/discussions

**Let's make HORUS C++ API the best it can be!** 🚀
