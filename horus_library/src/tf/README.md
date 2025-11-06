# HORUS Transform System (TF)

**Status:** 🚧 **Design Phase** - Implementation not started yet

---

## Quick Links

- **📘 Technical Blueprint:** [BLUEPRINT.md](./BLUEPRINT.md) - Complete technical specification
- **📖 HORUS Docs:** `/docs-site/content/docs/`
- **💬 Discussion:** Open an issue in the HORUS repository

---

## What is TF?

The **Transform (TF) system** tracks coordinate frames and the relationships between them in a robotics system. It's fundamental infrastructure that every robotics framework needs.

### Why It Matters

Without TF, you cannot:
- Relate sensor data from different locations (camera vs LIDAR)
- Transform navigation goals from map to robot coordinates
- Determine if detected objects are within reach
- Fuse data from multiple sensors properly

**TF is to robotics what coordinate systems are to geometry** - absolutely essential.

---

## Example Use Case

```
Robot with camera mounted on top:

    world_frame
        ↓
    map_frame
        ↓
    odom_frame
        ↓
    base_link (robot center)
        ↓
    camera_frame (0.5m forward, 0.2m up)
```

**Questions TF Answers:**
- "Where is the camera relative to the robot base?" → `Transform from base_link to camera_frame`
- "Is detected object reachable?" → `Transform point from camera_frame to base_link, check distance`
- "Where is the robot in the world?" → `Transform chain from world_frame to base_link`

---

## Current Status

### ✅ Completed
- [x] Technical blueprint and specification
- [x] API design (Rust, Python, C++)
- [x] Architecture decisions

### 🚧 In Progress
- [ ] Implementation (not started)

### 📋 Roadmap

**Phase 1 (Week 1-2):** Core Rust implementation
- Transform math (quaternions, composition, inverse)
- TF messages (TransformStamped, StaticTransformStamped)
- TF tree structure

**Phase 2 (Week 3-4):** Node implementations
- TFBroadcaster node
- TFListener node
- High-level API (TFBuffer)

**Phase 3 (Week 5-6):** Multi-language support
- Python bindings (PyO3)
- C++ bindings (FFI)
- Examples and tests

---

## Quick API Preview

### Rust
```rust
use horus::prelude::*;
use horus::library::tf::*;

// Define robot structure (static transforms)
let mut broadcaster = StaticTFBroadcaster::new()?;
broadcaster.send_transform(
    "base_link",
    "camera_frame",
    Transform::from_euler([0.5, 0.0, 0.2], [0.0, 0.0, 0.0]),
    None
)?;

// Use transforms
let tree = TFTree::new();
let transform = tree.lookup_transform("camera_frame", "base_link", now())?;
let point_in_base = transform.transform_point(point_in_camera);
```

### Python
```python
from horus.library.tf import TFBroadcaster, TFTree, Transform

# Define transforms
broadcaster = TFBroadcaster()
broadcaster.send_static_transform(
    parent="base_link",
    child="camera_frame",
    translation=[0.5, 0.0, 0.2],
    rotation=[0.0, 0.0, 0.0, 1.0]
)

# Use transforms
tree = TFTree()
tf = tree.lookup_transform("camera_frame", "base_link")
point_base = tf.transform_point(point_camera)
```

### C++
```cpp
#include <horus/tf.hpp>

using namespace horus::tf;

// Define transforms
TFBroadcaster broadcaster;
Transform camera_tf({0.5, 0.0, 0.2}, {0.0, 0.0, 0.0, 1.0});
broadcaster.send_static_transform("base_link", "camera_frame", camera_tf);

// Use transforms
TFTree tree;
auto tf = tree.lookup_transform("camera_frame", "base_link", now());
auto point_base = tf.transform_point(point_camera);
```

---

## File Structure (Planned)

```
horus_library/src/tf/
├── README.md              # This file
├── BLUEPRINT.md           # Technical specification (READ THIS FIRST!)
├── mod.rs                 # Module exports (TODO)
├── transform.rs           # Transform struct & math (TODO)
├── messages.rs            # TF messages (TODO)
├── tree.rs                # TF tree structure (TODO)
├── broadcaster.rs         # TFBroadcaster node (TODO)
├── listener.rs            # TFListener node (TODO)
├── buffer.rs              # High-level API (TODO)
└── tests/                 # Unit tests (TODO)
```

---

## Contributing

**Want to implement TF?** This is a high-priority, high-impact feature!

1. **Read the blueprint:** [BLUEPRINT.md](./BLUEPRINT.md)
2. **Start with Phase 1:** Transform math and messages
3. **Follow the API design:** Consistency with existing HORUS patterns
4. **Write tests:** >90% coverage target
5. **Open a PR:** Early feedback welcome!

---

## Why TF is Critical

**Without TF, HORUS cannot compete with ROS2 for general robotics applications.**

Every serious robotics framework has a transform system:
- **ROS1:** `tf` package
- **ROS2:** `tf2` package
- **Isaac ROS:** Transform API
- **Drake:** `RigidTransform` system

HORUS needs TF to be taken seriously as a robotics framework.

---

## Questions?

- **Technical questions:** See [BLUEPRINT.md](./BLUEPRINT.md) Section 15 "Open Questions"
- **General discussion:** Open an issue in the HORUS repository
- **Implementation help:** Contact the HORUS maintainers

---

**Last Updated:** 2025-01-05
**Blueprint Version:** 1.0
**Implementation Status:** Not started (design complete)
