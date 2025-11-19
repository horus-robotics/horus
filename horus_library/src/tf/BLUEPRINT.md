# HORUS Transform System (TF) - Technical Blueprint

**Version:** 1.0
**Status:** Design Phase
**Priority:** CRITICAL - Fundamental robotics infrastructure
**Target:** horus_library v0.2.0

---

## Executive Summary

The Transform (TF) system is a **coordinate frame management system** that tracks the relationships between different coordinate frames in a robot over time. This is fundamental robotics infrastructure - without it, robots cannot properly relate sensor data from different locations to each other or to the world.

**Critical Need:** Every robotics framework (ROS1, ROS2, Isaac, Drake) has a TF system. HORUS must have one to be viable for general robotics applications.

---

## Table of Contents

1. [Problem Statement](#1-problem-statement)
2. [Architecture Overview](#2-architecture-overview)
3. [Core Components](#3-core-components)
4. [Implementation Plan](#4-implementation-plan)
5. [API Design](#5-api-design)
6. [Memory & Performance](#6-memory--performance)
7. [Multi-Language Support](#7-multi-language-support)
8. [Integration with HORUS](#8-integration-with-horus)
9. [Testing Strategy](#9-testing-strategy)
10. [Migration from ROS](#10-migration-from-ros)

---

## 1. Problem Statement

### 1.1 What is TF?

Transform (TF) is a system for tracking coordinate frames and the transforms (translations + rotations) between them.

**Example:** A mobile robot with camera

```
         world
           |
        map_frame
           |
        odom_frame
           |
       base_link (robot center)
           |
      camera_frame (mounted on top)
```

**Questions TF Answers:**
- "Where is the camera relative to the robot base?"
- "What's the transform from world to camera at time T?"
- "Is detected object in camera view reachable by the robot arm?"

### 1.2 Why It's Critical

**Without TF:**
```rust
//  Cannot relate sensor data from different frames
let obstacle_in_camera = camera.detect_obstacle()?;
// How do I know if this is in front of the robot or behind?
// Camera might be mounted backwards!
```

**With TF:**
```rust
//  Can transform between any frames
let obstacle_in_camera = camera.detect_obstacle()?;
let obstacle_in_base = tf.transform_point(
    "camera_frame",
    "base_link",
    obstacle_in_camera
)?;

// Now we know: obstacle is 1.5m forward, 0.3m left of robot center
if obstacle_in_base.x < 2.0 {
    send_stop_command();
}
```

### 1.3 Use Cases in HORUS

1. **Sensor Fusion:** Combine LIDAR + camera + GPS data in common frame
2. **Navigation:** Transform path from map frame to robot frame
3. **Manipulation:** Transform gripper target from object frame to arm frame
4. **Multi-robot:** Transform poses between different robots' frames
5. **Simulation (sim3d):** Place objects in world frame, render in camera frame

---

## 2. Architecture Overview

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│           HORUS TF System (horus_library/tf)            │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │  TF Tree (In-Memory)                             │  │
│  │  - Frame hierarchy (tree structure)              │  │
│  │  - Static transforms (robot structure)           │  │
│  │  - Dynamic transforms (moving parts)             │  │
│  │  - Time-based buffer (last 10s)                  │  │
│  └──────────────────────────────────────────────────┘  │
│                        ▲                                 │
│                        │                                 │
│  ┌─────────────────────┴─────────────────────────────┐ │
│  │  TF Messages (via Hub)                            │ │
│  │  - TransformStamped (single transform)            │ │
│  │  - TFMessage (multiple transforms)                │ │
│  │  - StaticTransformStamped (unchanging)            │ │
│  └────────────────────────────────────────────────────┘ │
│                        ▲                                 │
│                        │                                 │
│  ┌─────────────────────┴─────────────────────────────┐ │
│  │  TF Nodes                                         │ │
│  │  - TFBroadcasterNode (publishes transforms)       │ │
│  │  - StaticTFBroadcasterNode (static transforms)    │ │
│  │  - TFListenerNode (subscribes & builds tree)      │ │
│  └────────────────────────────────────────────────────┘ │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  TF API (Rust/Python)                             │ │
│  │  - transform_point()                              │ │
│  │  - transform_pose()                               │ │
│  │  - lookup_transform()                             │ │
│  │  - wait_for_transform()                           │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│           HORUS Core (IPC, Hub, Scheduler)              │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Design Principles

1. **Zero-copy IPC:** Transforms sent via HORUS Hub (shared memory)
2. **Time-based:** All transforms have timestamps, can query historical data
3. **Lock-free reads:** Read-optimized data structure for hot path
4. **Lazy propagation:** Only compute transforms when requested
5. **Thread-safe:** Multiple nodes can publish/query simultaneously
6. **Frame validation:** Detect cycles, orphaned frames, invalid hierarchies

### 2.3 Data Flow

```
┌──────────────┐
│ Robot URDF   │ (Robot structure definition)
│ or code      │
└──────┬───────┘
       │
       ▼
┌──────────────────────┐
│ StaticTFBroadcaster  │ Publishes fixed transforms
│ - base_link  camera │ (e.g., camera 0.5m above base)
│ - base_link  lidar  │
└──────┬───────────────┘
       │
       ▼ Hub<StaticTransformStamped>
┌──────────────────────┐
│   TF Tree (Shared)   │ Receives static transforms
│                      │ Builds frame hierarchy
└──────────────────────┘
       ▲
       │ Hub<TransformStamped>
┌──────┴───────────────┐
│  TFBroadcaster       │ Publishes dynamic transforms
│  - odom  base_link  │ (e.g., robot moving in odom)
│  - map  odom        │ (e.g., localization update)
└──────────────────────┘
       │
       ▼
┌──────────────────────┐
│   Application Node   │ Queries transforms
│   - transform_point  │ "Where is this in base frame?"
│   - lookup_transform │
└──────────────────────┘
```

---

## 3. Core Components

### 3.1 Transform Representation

```rust
// horus_library/src/tf/transform.rs

/// 3D Transform (translation + rotation)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct Transform {
    /// Translation [x, y, z] in meters
    pub translation: [f64; 3],

    /// Rotation as quaternion [x, y, z, w]
    pub rotation: [f64; 4],
}

impl Transform {
    /// Create identity transform (no translation or rotation)
    pub fn identity() -> Self { /* ... */ }

    /// Create from translation and quaternion
    pub fn new(translation: [f64; 3], rotation: [f64; 4]) -> Self { /* ... */ }

    /// Create from translation and Euler angles (roll, pitch, yaw)
    pub fn from_euler(translation: [f64; 3], rpy: [f64; 3]) -> Self { /* ... */ }

    /// Compose two transforms (chain)
    pub fn compose(&self, other: &Transform) -> Transform { /* ... */ }

    /// Invert transform (reverse direction)
    pub fn inverse(&self) -> Transform { /* ... */ }

    /// Apply transform to a 3D point
    pub fn transform_point(&self, point: [f64; 3]) -> [f64; 3] { /* ... */ }

    /// Apply transform to a vector (rotation only, no translation)
    pub fn transform_vector(&self, vector: [f64; 3]) -> [f64; 3] { /* ... */ }

    /// Convert to 4x4 homogeneous matrix
    pub fn to_matrix(&self) -> [[f64; 4]; 4] { /* ... */ }

    /// Interpolate between two transforms (SLERP for rotation)
    pub fn interpolate(&self, other: &Transform, t: f64) -> Transform { /* ... */ }
}
```

### 3.2 TF Messages

```rust
// horus_library/src/tf/messages.rs

use horus::prelude::*;

/// Stamped transform (with timestamp)
message! {
    TransformStamped {
        /// Parent frame ID (e.g., "base_link")
        parent_frame: [u8; 64],

        /// Child frame ID (e.g., "camera_frame")
        child_frame: [u8; 64],

        /// Timestamp (nanoseconds since UNIX epoch)
        timestamp: u64,

        /// Transform (translation + rotation)
        transform: Transform,
    }
}

/// Static transform (never changes)
message! {
    StaticTransformStamped {
        parent_frame: [u8; 64],
        child_frame: [u8; 64],
        transform: Transform,
    }
}

/// Batch of transforms (for efficiency)
message! {
    TFMessage {
        transforms: [TransformStamped; 32],  // Max 32 transforms per message
        count: usize,
    }
}
```

### 3.3 TF Tree Structure

```rust
// horus_library/src/tf/tree.rs

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Frame node in TF tree
#[derive(Debug, Clone)]
struct FrameNode {
    /// Frame ID
    id: String,

    /// Parent frame (None for root)
    parent: Option<String>,

    /// Children frames
    children: Vec<String>,

    /// Transform from parent to this frame
    /// Buffer stores last N seconds of transforms (for moving frames)
    transform_buffer: CircularBuffer<(u64, Transform)>,  // (timestamp, transform)

    /// Is this a static frame?
    is_static: bool,
}

/// Circular buffer for time-based transform storage
#[derive(Debug, Clone)]
struct CircularBuffer<T> {
    buffer: Vec<T>,
    capacity: usize,
    head: usize,
}

impl<T: Clone> CircularBuffer<T> {
    fn new(capacity: usize) -> Self { /* ... */ }
    fn push(&mut self, item: T) { /* ... */ }
    fn get_at_time(&self, timestamp: u64) -> Option<&T> { /* ... */ }
    fn interpolate_at_time(&self, timestamp: u64) -> Option<T> { /* ... */ }
}

/// Main TF Tree (shared across nodes)
pub struct TFTree {
    /// All frames (frame_id -> FrameNode)
    frames: Arc<RwLock<HashMap<String, FrameNode>>>,

    /// Buffer duration (how long to keep historical transforms)
    buffer_duration: Duration,

    /// Cache for frequently-requested transform chains
    transform_cache: Arc<RwLock<HashMap<(String, String), Vec<String>>>>,
}

impl TFTree {
    pub fn new() -> Self { /* ... */ }

    /// Add a static transform (never changes)
    pub fn add_static_transform(
        &mut self,
        parent: &str,
        child: &str,
        transform: Transform,
    ) -> HorusResult<()> { /* ... */ }

    /// Add a dynamic transform (changes over time)
    pub fn add_transform(
        &mut self,
        parent: &str,
        child: &str,
        transform: Transform,
        timestamp: u64,
    ) -> HorusResult<()> { /* ... */ }

    /// Lookup transform from source to target frame at specific time
    pub fn lookup_transform(
        &self,
        source: &str,
        target: &str,
        time: u64,
    ) -> HorusResult<Transform> { /* ... */ }

    /// Get transform chain (path through tree)
    fn find_transform_chain(
        &self,
        source: &str,
        target: &str,
    ) -> Option<Vec<String>> { /* ... */ }

    /// Check if frame exists
    pub fn has_frame(&self, frame_id: &str) -> bool { /* ... */ }

    /// Get all frame IDs
    pub fn get_all_frames(&self) -> Vec<String> { /* ... */ }

    /// Validate tree (check for cycles, orphans)
    pub fn validate(&self) -> HorusResult<()> { /* ... */ }
}
```

### 3.4 TF Broadcaster Node

```rust
// horus_library/src/tf/broadcaster.rs

use horus::prelude::*;
use super::messages::TransformStamped;

/// Publishes transforms to TF tree
pub struct TFBroadcasterNode {
    publisher: Hub<TransformStamped>,
}

impl TFBroadcasterNode {
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new("/tf")?,
        })
    }

    /// Send a single transform
    pub fn send_transform(
        &mut self,
        parent: &str,
        child: &str,
        transform: Transform,
        timestamp: u64,
        ctx: Option<&mut NodeInfo>,
    ) -> HorusResult<()> {
        let mut msg = TransformStamped::default();

        // Copy frame IDs (truncate if too long)
        let parent_bytes = parent.as_bytes();
        let len = parent_bytes.len().min(63);
        msg.parent_frame[..len].copy_from_slice(&parent_bytes[..len]);

        let child_bytes = child.as_bytes();
        let len = child_bytes.len().min(63);
        msg.child_frame[..len].copy_from_slice(&child_bytes[..len]);

        msg.timestamp = timestamp;
        msg.transform = transform;

        self.publisher.send(msg, ctx)
            .map_err(|_| HorusError::PublishFailed)?;

        Ok(())
    }
}

impl Node for TFBroadcasterNode {
    fn name(&self) -> &'static str { "TFBroadcaster" }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Broadcaster doesn't need to tick
        // Transforms are sent via send_transform() when needed
    }
}
```

### 3.5 TF Listener Node

```rust
// horus_library/src/tf/listener.rs

use horus::prelude::*;
use super::{TFTree, messages::TransformStamped};

/// Listens to transforms and builds TF tree
pub struct TFListenerNode {
    subscriber: Hub<TransformStamped>,
    static_sub: Hub<StaticTransformStamped>,
    tree: Arc<RwLock<TFTree>>,
}

impl TFListenerNode {
    pub fn new(tree: Arc<RwLock<TFTree>>) -> HorusResult<Self> {
        Ok(Self {
            subscriber: Hub::new("/tf")?,
            static_sub: Hub::new("/tf_static")?,
            tree,
        })
    }
}

impl Node for TFListenerNode {
    fn name(&self) -> &'static str { "TFListener" }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Process one transform per tick (bounded execution)
        if let Some(msg) = self.subscriber.recv(ctx) {
            let parent = String::from_utf8_lossy(&msg.parent_frame)
                .trim_end_matches('\0')
                .to_string();
            let child = String::from_utf8_lossy(&msg.child_frame)
                .trim_end_matches('\0')
                .to_string();

            if let Ok(mut tree) = self.tree.write() {
                tree.add_transform(
                    &parent,
                    &child,
                    msg.transform,
                    msg.timestamp
                ).ok();
            }
        }

        // Process one static transform per tick (bounded execution)
        if let Some(msg) = self.static_sub.recv(ctx) {
            let parent = String::from_utf8_lossy(&msg.parent_frame)
                .trim_end_matches('\0')
                .to_string();
            let child = String::from_utf8_lossy(&msg.child_frame)
                .trim_end_matches('\0')
                .to_string();

            if let Ok(mut tree) = self.tree.write() {
                tree.add_static_transform(
                    &parent,
                    &child,
                    msg.transform
                ).ok();
            }
        }
    }
}
```

---

## 4. Implementation Plan

### Phase 1: Core Transform Math (Week 1)
- [ ] `Transform` struct
- [ ] Quaternion math (compose, inverse, SLERP)
- [ ] Point/vector transformation
- [ ] Euler angle conversions
- [ ] Unit tests for transform operations

### Phase 2: TF Messages (Week 1)
- [ ] `TransformStamped` message
- [ ] `StaticTransformStamped` message
- [ ] `TFMessage` batch message
- [ ] Implement `LogSummary` for TF messages

### Phase 3: TF Tree Structure (Week 2)
- [ ] `FrameNode` with circular buffer
- [ ] `TFTree` with frame hierarchy
- [ ] Transform chain finding (BFS)
- [ ] Time-based interpolation
- [ ] Cycle detection & validation
- [ ] Unit tests for tree operations

### Phase 4: TF Broadcaster & Listener (Week 2)
- [ ] `TFBroadcasterNode` implementation
- [ ] `StaticTFBroadcasterNode` implementation
- [ ] `TFListenerNode` implementation
- [ ] Integration tests with Hub

### Phase 5: High-Level API (Week 3)
- [ ] `TFBuffer` wrapper (ROS2-like API)
- [ ] Helper functions: `transform_point()`, `transform_pose()`
- [ ] `wait_for_transform()` with timeout
- [ ] Cache optimization for frequent queries

### Phase 6: Python Bindings (Week 3)
- [ ] PyO3 bindings for Transform
- [ ] PyO3 bindings for TFTree
- [ ] Python TFBroadcaster wrapper
- [ ] Python example scripts

### Phase 7: Testing & Documentation (Week 4)
- [ ] End-to-end integration tests
- [ ] Performance benchmarks (< 1μs for cached lookups)
- [ ] API documentation
- [ ] Tutorial: "TF System for Beginners"
- [ ] Example: Mobile robot with camera

---

## 5. API Design

### 5.1 Rust API

```rust
use horus::prelude::*;
use horus::library::tf::*;

fn main() -> HorusResult<()> {
    // Create TF tree (shared)
    let tree = Arc::new(RwLock::new(TFTree::new()));

    // Create scheduler
    let mut scheduler = Scheduler::new();

    // Add TF listener (builds tree from messages)
    scheduler.add(
        Box::new(TFListenerNode::new(tree.clone())?),
        0, // Highest priority
        Some(true)
    );

    // Add broadcaster for static transforms
    let mut static_broadcaster = StaticTFBroadcasterNode::new()?;

    // Define robot structure (static transforms)
    // Camera is 0.5m forward, 0.2m up from base
    static_broadcaster.send_transform(
        "base_link",
        "camera_frame",
        Transform::from_euler(
            [0.5, 0.0, 0.2],  // x, y, z
            [0.0, 0.0, 0.0]   // roll, pitch, yaw
        ),
        None
    )?;

    // LIDAR is at the front
    static_broadcaster.send_transform(
        "base_link",
        "lidar_frame",
        Transform::from_euler(
            [0.3, 0.0, 0.1],
            [0.0, 0.0, 0.0]
        ),
        None
    )?;

    // Add broadcaster to scheduler
    scheduler.add(Box::new(static_broadcaster), 1, Some(true));

    // Add your application nodes
    struct MyNode {
        tree: Arc<RwLock<TFTree>>,
        camera_sub: Hub<Detection>,
    }

    impl Node for MyNode {
        fn name(&self) -> &'static str { "MyNode" }

        fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
            // Get detection from camera
            if let Some(detection) = self.camera_sub.recv(ctx) {
                // Detection is in camera_frame coordinates
                let point_in_camera = [
                    detection.bbox.center_x as f64,
                    detection.bbox.center_y as f64,
                    detection.distance as f64
                ];

                // Transform to base_link
                if let Ok(tree) = self.tree.read() {
                    let now = timestamp_now();
                    if let Ok(transform) = tree.lookup_transform(
                        "camera_frame",
                        "base_link",
                        now
                    ) {
                        let point_in_base = transform.transform_point(point_in_camera);

                        if let Some(ctx) = ctx {
                            ctx.log_info(&format!(
                                "Object at camera ({:.2}, {:.2}, {:.2})  base ({:.2}, {:.2}, {:.2})",
                                point_in_camera[0], point_in_camera[1], point_in_camera[2],
                                point_in_base[0], point_in_base[1], point_in_base[2]
                            ));
                        }
                    }
                }
            }
        }
    }

    scheduler.run()
}
```

### 5.2 Python API

```python
import horus
from horus.library.tf import TFTree, TFBroadcaster, Transform
import numpy as np

# Create TF tree
tree = TFTree()

# Create broadcaster
broadcaster = TFBroadcaster()

# Define static transforms
broadcaster.send_static_transform(
    parent="base_link",
    child="camera_frame",
    translation=[0.5, 0.0, 0.2],  # x, y, z in meters
    rotation=[0.0, 0.0, 0.0, 1.0]  # quaternion [x, y, z, w]
)

# Application node
def process_camera(node: horus.Node):
    if node.has_msg("camera/detections"):
        detection = node.get("camera/detections")

        # Point in camera frame
        point_camera = np.array([
            detection['x'],
            detection['y'],
            detection['distance']
        ])

        # Transform to base frame
        transform = tree.lookup_transform("camera_frame", "base_link")
        point_base = transform.transform_point(point_camera)

        print(f"Object at {point_base} in base frame")

        # Send to planner
        node.send("planner/targets", {
            "position": point_base.tolist(),
            "timestamp": horus.time.now()
        })

# Create node
node = horus.Node(
    name="camera_processor",
    subs="camera/detections",
    pubs="planner/targets",
    tick=process_camera
)

horus.run(node)
```

---

## 6. Memory & Performance

### 6.1 Memory Layout

**Transform:** 56 bytes (fixed size)
```
translation: [f64; 3] = 24 bytes
rotation:    [f64; 4] = 32 bytes
Total: 56 bytes
```

**TransformStamped:** 192 bytes
```
parent_frame: [u8; 64]  = 64 bytes
child_frame:  [u8; 64]  = 64 bytes
timestamp:    u64       = 8 bytes
transform:    Transform = 56 bytes
Total: 192 bytes
```

**TF Tree (50 frames, 10s buffer @ 100Hz):**
```
Frames: 50 * sizeof(FrameNode) ≈ 50 * 1KB = 50 KB
Transform buffer: 50 frames * 1000 samples * 56 bytes = 2.8 MB
Cache: ~100 KB
Total: ~3 MB (acceptable)
```

### 6.2 Performance Targets

| Operation | Target | Strategy |
|-----------|--------|----------|
| **Cached transform lookup** | < 100ns | RwLock read + HashMap lookup |
| **Uncached transform lookup** | < 1μs | BFS + compose chain |
| **Transform composition** | < 50ns | SIMD quaternion math |
| **Publish transform** | < 500ns | Hub send (zero-copy) |
| **Tree update** | < 2μs | RwLock write + insert |

### 6.3 Optimization Strategies

1. **Cache frequently-used chains:** `base_link  camera_frame` computed once, reused
2. **Lock-free reads:** Use `Arc<RwLock<>>` for concurrent reads
3. **SIMD quaternion math:** Use `nalgebra` or `glam` for fast transforms
4. **Lazy evaluation:** Only compute transforms when requested
5. **Circular buffer:** O(1) insert, O(log n) time-based lookup

---

## 7. Multi-Language Support

### 7.1 Rust (Native)

**Location:** `horus_library/src/tf/`
```rust
pub mod tf {
    pub mod transform;
    pub mod tree;
    pub mod broadcaster;
    pub mod listener;
    pub mod messages;
    pub mod buffer;
}
```

### 7.2 Python (PyO3)

**Location:** `horus_py/src/tf.rs`
```rust
use pyo3::prelude::*;

#[pyclass]
struct PyTransform {
    inner: Transform,
}

#[pymethods]
impl PyTransform {
    #[new]
    fn new(translation: [f64; 3], rotation: [f64; 4]) -> Self { /* ... */ }

    fn transform_point(&self, point: [f64; 3]) -> [f64; 3] { /* ... */ }

    fn compose(&self, other: &PyTransform) -> PyTransform { /* ... */ }
}

#[pymodule]
fn tf(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyTransform>()?;
    m.add_class::<PyTFTree>()?;
    m.add_class::<PyTFBroadcaster>()?;
    Ok(())
}
```

---

## 8. Integration with HORUS

### 8.1 File Structure

```
horus_library/
├── src/
│   ├── messages/        # Existing messages
│   ├── nodes/           # Existing nodes
│   └── tf/              # NEW: TF system
│       ├── mod.rs           # Module exports
│       ├── transform.rs     # Transform struct & math
│       ├── messages.rs      # TF messages
│       ├── tree.rs          # TF tree structure
│       ├── broadcaster.rs   # TFBroadcaster node
│       ├── listener.rs      # TFListener node
│       ├── buffer.rs        # TFBuffer (high-level API)
│       └── README.md        # TF documentation
└── lib.rs               # Add: pub mod tf;
```

### 8.2 Prelude Integration

```rust
// horus/src/lib.rs

pub mod prelude {
    // ... existing imports

    // TF system
    pub use horus_library::tf::{
        Transform,
        TransformStamped,
        StaticTransformStamped,
        TFTree,
        TFBroadcaster,
        TFListener,
        TFBuffer,
    };
}
```

### 8.3 Example Projects

Create example projects in `examples/tf/`:
- `examples/tf/static_transforms.rs` - Static robot structure
- `examples/tf/dynamic_transforms.rs` - Moving robot in odom
- `examples/tf/sensor_fusion.rs` - Combine camera + LIDAR
- `examples/tf/multi_robot.rs` - Two robots, transform between them

---

## 9. Testing Strategy

### 9.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_identity() {
        let tf = Transform::identity();
        let point = [1.0, 2.0, 3.0];
        let result = tf.transform_point(point);
        assert_eq!(result, point);
    }

    #[test]
    fn test_transform_composition() {
        // Transform A: translate by (1, 0, 0)
        let tf_a = Transform::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]);

        // Transform B: translate by (0, 2, 0)
        let tf_b = Transform::new([0.0, 2.0, 0.0], [0.0, 0.0, 0.0, 1.0]);

        // Compose: should be (1, 2, 0)
        let tf_ab = tf_a.compose(&tf_b);
        assert_eq!(tf_ab.translation, [1.0, 2.0, 0.0]);
    }

    #[test]
    fn test_tree_lookup() {
        let mut tree = TFTree::new();

        // Build tree: world  base  camera
        tree.add_static_transform(
            "world",
            "base_link",
            Transform::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0])
        ).unwrap();

        tree.add_static_transform(
            "base_link",
            "camera",
            Transform::new([0.5, 0.0, 0.2], [0.0, 0.0, 0.0, 1.0])
        ).unwrap();

        // Lookup world  camera (should compose)
        let tf = tree.lookup_transform("world", "camera", 0).unwrap();
        assert_eq!(tf.translation, [1.5, 0.0, 0.2]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut tree = TFTree::new();

        tree.add_static_transform("A", "B", Transform::identity()).unwrap();
        tree.add_static_transform("B", "C", Transform::identity()).unwrap();

        // This would create a cycle: A  B  C  A
        let result = tree.add_static_transform("C", "A", Transform::identity());
        assert!(result.is_err());
    }
}
```

### 9.2 Integration Tests

```rust
#[test]
fn test_tf_broadcaster_listener() {
    let tree = Arc::new(RwLock::new(TFTree::new()));

    let mut scheduler = Scheduler::new();

    // Add listener
    scheduler.add(
        Box::new(TFListenerNode::new(tree.clone()).unwrap()),
        0,
        Some(false)
    );

    // Add broadcaster
    let mut broadcaster = TFBroadcasterNode::new().unwrap();

    // Send transform
    broadcaster.send_transform(
        "base_link",
        "camera",
        Transform::new([0.5, 0.0, 0.2], [0.0, 0.0, 0.0, 1.0]),
        timestamp_now(),
        None
    ).unwrap();

    scheduler.add(Box::new(broadcaster), 1, Some(false));

    // Run for 1 second
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(1));
        // TODO: Stop scheduler
    });

    scheduler.run().unwrap();

    // Verify tree received transform
    let tree = tree.read().unwrap();
    assert!(tree.has_frame("base_link"));
    assert!(tree.has_frame("camera"));
}
```

### 9.3 Benchmark Tests

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_transform_point(c: &mut Criterion) {
    let tf = Transform::new([1.0, 2.0, 3.0], [0.0, 0.0, 0.0, 1.0]);
    let point = [1.0, 2.0, 3.0];

    c.bench_function("transform_point", |b| {
        b.iter(|| tf.transform_point(black_box(point)))
    });
}

fn bench_lookup_cached(c: &mut Criterion) {
    let tree = setup_tree_with_cache();

    c.bench_function("lookup_cached", |b| {
        b.iter(|| {
            tree.lookup_transform(
                black_box("base_link"),
                black_box("camera"),
                black_box(0)
            ).unwrap()
        })
    });
}

criterion_group!(benches, bench_transform_point, bench_lookup_cached);
criterion_main!(benches);
```

---

## 10. Migration from ROS

### 10.1 ROS1 vs HORUS TF API

| ROS1 | HORUS |
|------|-------|
| `tf::TransformListener` | `TFListenerNode` |
| `tf::TransformBroadcaster` | `TFBroadcaster` |
| `tf::StampedTransform` | `TransformStamped` |
| `listener.lookupTransform()` | `tree.lookup_transform()` |
| `broadcaster.sendTransform()` | `broadcaster.send_transform()` |

### 10.2 Example Migration

**ROS1 (C++):**
```cpp
#include <tf/transform_broadcaster.h>

int main() {
    tf::TransformBroadcaster br;
    tf::Transform transform;
    transform.setOrigin(tf::Vector3(0.5, 0.0, 0.2));
    transform.setRotation(tf::Quaternion(0, 0, 0, 1));
    br.sendTransform(
        tf::StampedTransform(transform, ros::Time::now(), "base_link", "camera")
    );
}
```

**HORUS (Rust):**
```rust
use horus::library::tf::*;

fn main() -> HorusResult<()> {
    let mut broadcaster = TFBroadcaster::new()?;
    let transform = Transform::new([0.5, 0.0, 0.2], [0.0, 0.0, 0.0, 1.0]);
    broadcaster.send_transform(
        "base_link",
        "camera",
        transform,
        timestamp_now(),
        None
    )?;
    Ok(())
}
```

---

## 11. Dependencies

### 11.1 Required Crates

```toml
# horus_library/Cargo.toml

[dependencies]
# Existing
horus_core = { path = "../horus_core" }
serde = { version = "1.0", features = ["derive"] }

# NEW for TF
nalgebra = "0.32"      # Linear algebra, quaternions (OR glam)
# glam = "0.24"        # Alternative: lighter weight, SIMD
chrono = "0.4"         # Timestamps
```

**Decision:** Use `nalgebra` for comprehensive linear algebra support, or `glam` for performance-critical applications. Recommend `nalgebra` for now (more features, good docs).

### 11.2 Optional Features

```toml
[features]
default = ["tf"]
tf = []  # TF system enabled by default
tf-visualizer = ["egui", "plotters"]  # Optional TF tree visualization
```

---

## 12. Documentation Requirements

### 12.1 API Documentation

- Rustdoc for all public types
- Python docstrings for PyO3 bindings

### 12.2 Tutorials

1. **TF Basics** - What is TF, why it matters
2. **Static Transforms** - Define robot structure
3. **Dynamic Transforms** - Moving robots
4. **Sensor Fusion** - Combine multiple sensors
5. **Debugging TF** - Visualize tree, detect problems

### 12.3 Examples

- Minimal example (< 50 lines)
- Mobile robot with camera
- Robot arm with gripper
- Multi-robot coordination

---

## 13. Success Criteria

### 13.1 Must Have (MVP)

-  Transform math (compose, inverse, SLERP)
-  TF messages (TransformStamped, StaticTransformStamped)
-  TF tree structure (add, lookup, validate)
-  TFBroadcaster node
-  TFListener node
-  Rust API
-  Unit tests (>90% coverage)
-  Documentation

### 13.2 Should Have (v1.0)

-  Python bindings
-  Time-based interpolation
-  Transform caching
-  Performance benchmarks
-  Integration tests
-  Tutorial + examples

### 13.3 Nice to Have (Future)

-  TF tree visualization tool
-  URDF parser (import robot structure from URDF)
-  TF debugging CLI (`horus tf view`, `horus tf check`)
-  Real-time TF monitor in dashboard
-  Transform prediction (extrapolate into future)

---

## 14. Timeline

| Week | Milestone |
|------|-----------|
| **Week 1** | Core transform math, TF messages |
| **Week 2** | TF tree structure, broadcaster/listener nodes |
| **Week 3** | High-level API, Python bindings |
| **Week 4** | Testing, documentation |
| **Week 5** | Integration with dashboard, examples |
| **Week 6** | Buffer optimization, polish, release |

**Total: 6 weeks to production-ready TF system**

---

## 15. Open Questions

1. **Quaternion library:** nalgebra vs glam vs custom?
   - **Recommendation:** nalgebra (comprehensive, well-tested)

2. **Transform interpolation:** SLERP vs NLERP?
   - **Recommendation:** SLERP (accurate, industry standard)

3. **Frame ID format:** String vs fixed-size array?
   - **Recommendation:** Fixed [u8; 64] for shared memory compatibility

4. **Buffer size:** 10 seconds default? Configurable?
   - **Recommendation:** 10s default, configurable via API

5. **Cache invalidation:** When to clear transform cache?
   - **Recommendation:** LRU cache, max 1000 entries, clear on tree update

6. **Multi-session support:** One TF tree per session or global?
   - **Recommendation:** Per-session for isolation (use HORUS_SESSION_ID)

---

## 16. References

- **ROS1 TF:** http://wiki.ros.org/tf
- **ROS2 TF2:** https://docs.ros.org/en/rolling/Tutorials/Intermediate/Tf2/Tf2-Main.html
- **Quaternion math:** https://en.wikipedia.org/wiki/Quaternions_and_spatial_rotation
- **SLERP:** https://en.wikipedia.org/wiki/Slerp
- **HORUS Architecture:** `/docs-site/content/docs/`

---

**This blueprint is a living document. Update as implementation progresses.**
