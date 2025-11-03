# HORUS Topic Inspector

A comprehensive feature for inspecting, monitoring, and visualizing HORUS topics within IDE environments.

## Overview

The Topic Inspector provides real-time visibility into HORUS's publish-subscribe communication system:

- **Topic Discovery**: Automatically detect all topics in a project
- **Live Monitoring**: View topic values in real-time during execution
- **Publisher/Subscriber Tracking**: See which nodes publish/subscribe to topics
- **Message Rate Analysis**: Monitor message frequency and bandwidth
- **Type Information**: Display message type schemas and documentation

## User Interface Patterns

Different IDEs implement the Topic Inspector with varying UI approaches:

### VSCode Implementation

**Sidebar Panel**:
```
HORUS TOPICS
├─ cmd_vel (geometry_msgs::Twist)
│   ├─ Publishers: teleop_node (10 Hz)
│   ├─ Subscribers: robot_controller
│   └─ Rate: 10.2 Hz
├─ sensor_data (sensor_msgs::LaserScan)
│   ├─ Publishers: lidar_driver (20 Hz)
│   ├─ Subscribers: slam_node, obstacle_detector
│   └─ Rate: 20.1 Hz
└─ camera/image_raw (sensor_msgs::Image)
    ├─ Publishers: camera_driver (30 Hz)
    ├─ Subscribers: image_processor
    └─ Rate: 30.0 Hz, 9.2 MB/s
```

**Hover Tooltip** (on topic string in code):
```markdown
**Topic**: cmd_vel
**Type**: geometry_msgs::Twist

**Publishers**:
- teleop_node (10 Hz)

**Subscribers**:
- robot_controller

**Current Value**:
```rust
Twist {
    linear: Vec3 { x: 0.5, y: 0.0, z: 0.0 },
    angular: Vec3 { x: 0.0, y: 0.0, z: 0.2 }
}
```

**Message Rate**: 10.2 Hz
**Queue Size**: 3/10
```

**WebView Panel** (detailed inspector):
```
┌─────────────────────────────────────────────────┐
│ Topic Inspector: cmd_vel                        │
├─────────────────────────────────────────────────┤
│ Type: geometry_msgs::Twist                      │
│                                                 │
│ Current Value:                                  │
│ ┌─────────────────────────────────────────────┐ │
│ │ linear:                                     │ │
│ │   x: 0.5                                    │ │
│ │   y: 0.0                                    │ │
│ │   z: 0.0                                    │ │
│ │ angular:                                    │ │
│ │   x: 0.0                                    │ │
│ │   y: 0.0                                    │ │
│ │   z: 0.2                                    │ │
│ └─────────────────────────────────────────────┘ │
│                                                 │
│ Publishers:                                     │
│   • teleop_node (10.0 Hz)                      │
│                                                 │
│ Subscribers:                                    │
│   • robot_controller                           │
│                                                 │
│ Statistics:                                     │
│   Message Rate: 10.2 Hz                        │
│   Total Messages: 1024                         │
│   Bandwidth: 2.4 KB/s                          │
│   Queue: 3/10 messages                         │
│                                                 │
│ [History] [Plot] [Record]                      │
└─────────────────────────────────────────────────┘
```

### IntelliJ Implementation

**Tool Window** (similar to Services or Database tool window):

```
Topics
├─ Active Topics (3)
│   ├─ cmd_vel
│   ├─ sensor_data
│   └─ camera/image_raw
├─ Inactive Topics (2)
│   ├─ debug/log
│   └─ diagnostics
└─ Message Types
    ├─ geometry_msgs::Twist
    ├─ sensor_msgs::LaserScan
    └─ sensor_msgs::Image
```

**Editor Gutter** (inline annotations):

```rust
// When hovering over topic string in code:
node.subscribe("cmd_vel", |msg: Twist| {  // 10.2 Hz - teleop_node -> robot_controller
    // ...
});
```

### Vim/Neovim Implementation

**Floating Window**:

```
╔══════════════════════════════════════════════╗
║ Topic: cmd_vel                               ║
║ Type: geometry_msgs::Twist                   ║
╟──────────────────────────────────────────────╢
║ Publishers: teleop_node (10 Hz)              ║
║ Subscribers: robot_controller                ║
║ Rate: 10.2 Hz                                ║
╟──────────────────────────────────────────────╢
║ Value:                                       ║
║   linear: { x: 0.5, y: 0.0, z: 0.0 }        ║
║   angular: { x: 0.0, y: 0.0, z: 0.2 }       ║
╚══════════════════════════════════════════════╝
```

**Buffer** (topic list):

```
HORUS Topics                                    [Press ? for help]

cmd_vel                  geometry_msgs::Twist   10.2 Hz   teleop_node → robot_controller
sensor_data              sensor_msgs::LaserScan 20.1 Hz   lidar_driver → slam_node, obstacle_detector
camera/image_raw         sensor_msgs::Image     30.0 Hz   camera_driver → image_processor
```

### Emacs Implementation

**Buffer** (topic inspector):

```
HORUS Topics Inspector

Topic: cmd_vel
Type: geometry_msgs::Twist
Rate: 10.2 Hz

Publishers:
  - teleop_node (10.0 Hz)

Subscribers:
  - robot_controller

Current Value:
┌────────────────────────────────────────┐
│ linear:                                │
│   x: 0.5                               │
│   y: 0.0                               │
│   z: 0.0                               │
│ angular:                               │
│   x: 0.0                               │
│   y: 0.0                               │
│   z: 0.2                               │
└────────────────────────────────────────┘

[Refresh] [Pin] [History] [Record]
```

---

## Implementation Architecture

### Data Flow

```
HORUS Runtime
     │
     ├─ Topic Registry
     │   └─ Active Topics
     │
     ▼
Language Server
     │
     ├─ Cache Topic Metadata
     ├─ Monitor Topic Values
     │
     ▼
IDE Extension
     │
     ├─ UI Rendering
     ├─ User Interaction
     │
     ▼
Developer
```

### Language Server Integration

The Topic Inspector relies on custom LSP methods:

**`horus/topicInfo`** - Get topic metadata:
```typescript
const topicInfo = await client.sendRequest('horus/topicInfo', {
    topic: 'cmd_vel',
    includeValue: true,
    includeRate: true
});
```

**`horus/listTopics`** - Get all topics:
```typescript
const topics = await client.sendRequest('horus/listTopics', {
    filter: 'active'
});
```

**`horus/subscribeTopicUpdates`** - Real-time updates:
```typescript
// Subscribe to updates
await client.sendRequest('horus/subscribeTopicUpdates', {
    topics: ['cmd_vel', 'sensor_data']
});

// Receive notifications
client.onNotification('horus/topicUpdated', (params) => {
    console.log(`${params.topic}: ${params.value}`);
});
```

### Runtime Connection

Topic values require connection to running HORUS system:

**Static Analysis** (no runtime):
- Topic discovery from code
- Publisher/subscriber detection via static analysis
- Message type inference

**Runtime Integration** (with running system):
- Current topic values
- Message rates
- Bandwidth statistics
- Queue sizes

---

## Features in Detail

### 1. Topic Discovery

**Static Discovery**:

Parse source code to find topics:

```rust
// Detect from subscribe calls
node.subscribe("cmd_vel", |msg: Twist| { ... });
//             ^^^^^^^^^ topic discovered

// Detect from publish calls
let pub = node.advertise::<Twist>("cmd_vel");
//                                 ^^^^^^^^^ topic discovered
```

**Runtime Discovery**:

Query running HORUS system for active topics.

### 2. Hover Information

When hovering over topic strings in code:

```rust
node.subscribe("cmd_vel", |msg: Twist| {
    // Hovering over "cmd_vel" shows:
    // - Message type
    // - Publishers
    // - Subscribers
    // - Current value (if running)
    // - Message rate (if running)
});
```

### 3. Code Completion

When typing topic names:

```rust
node.subscribe("|", |msg| {
//             ^ trigger completion
});

// Completions:
// - cmd_vel (geometry_msgs::Twist) - 10 Hz
// - sensor_data (sensor_msgs::LaserScan) - 20 Hz
// - camera/image_raw (sensor_msgs::Image) - 30 Hz
```

### 4. Topic Navigation

**Go to Definition**:

```rust
node.subscribe("cmd_vel", |msg: Twist| {
    // Ctrl+Click on "cmd_vel" jumps to:
    // 1. First advertise() call for this topic, OR
    // 2. List of all publishers/subscribers
});
```

**Find References**:

```rust
// Find all uses of a topic across the project
// Shows all subscribe() and advertise() calls
```

### 5. Live Value Monitoring

**Static View** (code):
```rust
node.subscribe("cmd_vel", |msg: Twist| {
    println!("Received: {:?}", msg);
});
```

**Runtime Annotation** (inline in editor):
```rust
node.subscribe("cmd_vel", |msg: Twist| {  // ← Latest: Twist { linear: Vec3 { x: 0.5, ... }, ... }
    println!("Received: {:?}", msg);
});
```

### 6. Message Rate Visualization

**Text Display**:
```
cmd_vel: 10.2 Hz
sensor_data: 20.1 Hz
```

**Sparkline Graph** (VSCode/IntelliJ):
```
cmd_vel       ▁▂▃▅▇█▇▅▃▂▁  10.2 Hz
sensor_data   ████████████  20.1 Hz
camera/raw    ▁▁▁▁▁▁▁▁▁▁▁▁   0.0 Hz (stalled!)
```

### 7. Topic History

**Timeline View**:
```
15:23:45.123 | cmd_vel | Twist { linear: Vec3 { x: 0.5, y: 0.0, z: 0.0 }, ... }
15:23:45.223 | cmd_vel | Twist { linear: Vec3 { x: 0.6, y: 0.0, z: 0.0 }, ... }
15:23:45.323 | cmd_vel | Twist { linear: Vec3 { x: 0.7, y: 0.0, z: 0.0 }, ... }
```

**Plot View** (VSCode WebView):

```
cmd_vel.linear.x
 1.0 ┤       ╭─╮
     │      ╱   ╰╮
 0.5 ┤ ╭───╯     ╰─╮
     │╱            ╰
 0.0 ┼──────────────────
     0s   2s   4s   6s
```

### 8. Topic Recording

**Record Session**:
```
[Start Recording] → [Stop Recording]

Saved to: topics_2024-01-15_15-23-45.horus
Size: 2.4 MB
Duration: 30.5 seconds
Topics: cmd_vel, sensor_data, camera/image_raw
```

**Playback**:
```
[Load Recording] → [Play] [Pause] [Step]

Playing: topics_2024-01-15_15-23-45.horus
Position: 15.2 / 30.5 seconds
```

---

## Configuration

### VSCode Settings

```json
{
    "horus.topicInspector.enabled": true,
    "horus.topicInspector.showInHover": true,
    "horus.topicInspector.showInSidebar": true,
    "horus.topicInspector.autoRefresh": true,
    "horus.topicInspector.refreshRate": 10,
    "horus.topicInspector.maxHistorySize": 1000,
    "horus.topicInspector.formatValues": "pretty",
    "horus.topicInspector.showRateSparklines": true
}
```

### IntelliJ Settings

```
Settings > Tools > HORUS > Topic Inspector
  [x] Enable Topic Inspector
  [x] Show hover tooltips
  [x] Auto-refresh during run
      Refresh rate: [10] Hz
  [x] Show message rate graphs
      History size: [1000] messages
```

### Vim Configuration

```lua
require('horus').setup({
    topic_inspector = {
        enabled = true,
        show_hover = true,
        auto_refresh = true,
        refresh_rate = 10,
        format = 'pretty'
    }
})
```

---

## Performance Considerations

### Refresh Rate Throttling

Topic values are rate-limited to avoid overwhelming the IDE:

- Default: 10 Hz (100ms updates)
- High-frequency topics downsampled
- User-configurable via settings

### Large Messages

Large messages (images, point clouds) are handled specially:

```
camera/image_raw: sensor_msgs::Image
  Size: 2.4 MB
  Dimensions: 1920x1080
  Encoding: rgb8
  [Show Thumbnail] [View Full Image]
```

### Memory Management

- History buffer limited (default: 1000 messages)
- Old messages automatically discarded
- Recording to disk for long sessions

---

## Error Handling

### Topic Not Found

```
Topic "unknown_topic" not found

Did you mean:
  - known_topic
  - cmd_vel
  - sensor_data
```

### Runtime Unavailable

```
Topic Inspector (Static Mode)

Live values unavailable. Start HORUS application to see:
  - Current topic values
  - Message rates
  - Runtime statistics

[Start Debug Session] [Run Application]
```

### Type Mismatch

```
Warning: Type mismatch on topic "cmd_vel"

Publisher: geometry_msgs::Twist
Subscriber: geometry_msgs::TwistStamped

This may cause runtime errors.
```

---

## Testing

### Unit Tests

```typescript
test('Topic info hover provider', async () => {
    const doc = await vscode.workspace.openTextDocument(...);
    const position = new vscode.Position(10, 25);  // Position of "cmd_vel"

    const hover = await provider.provideHover(doc, position);

    assert.ok(hover);
    assert.ok(hover.contents[0].includes('geometry_msgs::Twist'));
});
```

### Integration Tests

```rust
#[tokio::test]
async fn test_topic_discovery() {
    let project = TestProject::new("test_topics");
    let inspector = TopicInspector::new(&project);

    let topics = inspector.discover_topics().await.unwrap();

    assert_eq!(topics.len(), 3);
    assert!(topics.iter().any(|t| t.name == "cmd_vel"));
}
```

---

## Future Enhancements

### Planned Features

- **Topic Filtering**: Filter by namespace, type, or rate
- **Topic Comparison**: Compare multiple topics side-by-side
- **Export to CSV**: Export topic history for analysis
- **Custom Visualizations**: Plugin system for custom topic renderers
- **Network Topology**: Show topic flow across network boundaries

### Research Areas

- **3D Visualization**: Integrate with viz tools for 3D data
- **AI-Assisted Analysis**: Detect anomalies in topic patterns
- **Performance Profiling**: Link topic rates to CPU/memory usage

---

## See Also

- [LSP_PROTOCOL.md](./LSP_PROTOCOL.md) - LSP custom methods
- [DAP_PROTOCOL.md](./DAP_PROTOCOL.md) - Debugging integration
- [NODE_GRAPH.md](./NODE_GRAPH.md) - Node graph visualization
- [Language Server README](../language-server/README.md)
