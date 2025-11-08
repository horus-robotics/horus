# HORUS Debug Adapter Protocol Extensions

This document describes custom DAP features and extensions specific to HORUS that extend the standard Debug Adapter Protocol 1.51.

## Overview

The HORUS Debug Adapter implements all standard DAP features plus HORUS-specific debugging capabilities:

- **Topic Value Watches**: Monitor HORUS topic values in real-time
- **Node State Inspection**: View internal node state during debugging
- **Publisher/Subscriber Tracking**: See communication between nodes
- **Breakpoint Enhancements**: Topic-based and node-lifecycle breakpoints

## Standard DAP Capabilities

The HORUS Debug Adapter supports all standard DAP features:

```json
{
    "supportsConfigurationDoneRequest": true,
    "supportsFunctionBreakpoints": true,
    "supportsConditionalBreakpoints": true,
    "supportsHitConditionalBreakpoints": true,
    "supportsEvaluateForHovers": true,
    "supportsStepBack": false,
    "supportsSetVariable": true,
    "supportsRestartFrame": true,
    "supportsGotoTargetsRequest": true,
    "supportsStepInTargetsRequest": true,
    "supportsCompletionsRequest": true,
    "supportsModulesRequest": true,
    "supportsExceptionOptions": true,
    "supportsValueFormattingOptions": true,
    "supportsExceptionInfoRequest": true,
    "supportTerminateDebuggee": true,
    "supportSuspendDebuggee": true,
    "supportsDelayedStackTraceLoading": true,
    "supportsLoadedSourcesRequest": true,
    "supportsLogPoints": true,
    "supportsTerminateThreadsRequest": true,
    "supportsSetExpression": true,
    "supportsReadMemoryRequest": true,
    "supportsWriteMemoryRequest": true,
    "supportsDisassembleRequest": true
}
```

## Custom DAP Requests

### `horus/setTopicWatch`

Set a watch on a HORUS topic to monitor its values during debugging.

**Request**:
```typescript
interface SetTopicWatchRequest {
    topic: string;              // Topic name to watch
    format?: 'json' | 'pretty'; // Display format (default: 'pretty')
    condition?: string;         // Optional filter condition
}
```

**Response**:
```typescript
interface SetTopicWatchResponse {
    watchId: number;    // Unique watch ID
    topic: string;      // Topic name
    messageType: string;// Message type
}
```

**Example**:

Request:
```json
{
    "seq": 1,
    "type": "request",
    "command": "horus/setTopicWatch",
    "arguments": {
        "topic": "cmd_vel",
        "format": "pretty"
    }
}
```

Response:
```json
{
    "seq": 2,
    "type": "response",
    "request_seq": 1,
    "success": true,
    "command": "horus/setTopicWatch",
    "body": {
        "watchId": 1,
        "topic": "cmd_vel",
        "messageType": "geometry_msgs::Twist"
    }
}
```

**Events**:

When topic value changes, the debug adapter sends a `horus/topicValue` event:

```json
{
    "seq": 3,
    "type": "event",
    "event": "horus/topicValue",
    "body": {
        "watchId": 1,
        "topic": "cmd_vel",
        "value": {
            "linear": { "x": 0.5, "y": 0.0, "z": 0.0 },
            "angular": { "x": 0.0, "y": 0.0, "z": 0.2 }
        },
        "timestamp": 1609459200.123
    }
}
```

---

### `horus/removeTopicWatch`

Remove a topic watch.

**Request**:
```typescript
interface RemoveTopicWatchRequest {
    watchId: number;    // Watch ID from setTopicWatch
}
```

**Response**:
```typescript
interface RemoveTopicWatchResponse {
    success: boolean;
}
```

---

### `horus/getNodeState`

Get the current internal state of a HORUS node.

**Request**:
```typescript
interface GetNodeStateRequest {
    nodeName: string;   // Node name
    includeTopics?: boolean;  // Include topic info (default: true)
}
```

**Response**:
```typescript
interface GetNodeStateResponse {
    nodeName: string;
    state: NodeState;
    topics?: TopicState[];
}

interface NodeState {
    status: 'running' | 'paused' | 'stopped';
    tickCount: number;          // Total ticks executed
    tickRate: number;           // Current tick rate (Hz)
    cpuUsage: number;           // CPU usage percentage
    memoryUsage: number;        // Memory usage (bytes)
    fields: NamedValue[];       // Node struct fields
}

interface TopicState {
    topic: string;
    direction: 'publish' | 'subscribe';
    messageType: string;
    messageCount: number;       // Total messages sent/received
    lastMessageTime?: number;   // Timestamp of last message
    queueSize?: number;         // Current queue size
}

interface NamedValue {
    name: string;       // Field name
    value: string;      // String representation
    type: string;       // Type name
    variablesReference?: number;  // For complex types
}
```

**Example**:

Request:
```json
{
    "seq": 4,
    "type": "request",
    "command": "horus/getNodeState",
    "arguments": {
        "nodeName": "robot_controller",
        "includeTopics": true
    }
}
```

Response:
```json
{
    "seq": 5,
    "type": "response",
    "request_seq": 4,
    "success": true,
    "command": "horus/getNodeState",
    "body": {
        "nodeName": "robot_controller",
        "state": {
            "status": "running",
            "tickCount": 1024,
            "tickRate": 50.0,
            "cpuUsage": 8.5,
            "memoryUsage": 204800,
            "fields": [
                {
                    "name": "controller_state",
                    "value": "ControllerState::Active",
                    "type": "ControllerState"
                },
                {
                    "name": "last_cmd",
                    "value": "Twist { linear: Vec3 { x: 0.5, ... }, ... }",
                    "type": "geometry_msgs::Twist",
                    "variablesReference": 1001
                }
            ]
        },
        "topics": [
            {
                "topic": "cmd_vel",
                "direction": "subscribe",
                "messageType": "geometry_msgs::Twist",
                "messageCount": 512,
                "lastMessageTime": 1609459200.123,
                "queueSize": 3
            },
            {
                "topic": "motor_commands",
                "direction": "publish",
                "messageType": "std_msgs::Float64MultiArray",
                "messageCount": 1024,
                "lastMessageTime": 1609459200.145
            }
        ]
    }
}
```

---

### `horus/listNodes`

List all nodes in the current debug session.

**Request**:
```typescript
interface ListNodesRequest {
    filter?: 'all' | 'active' | 'paused';  // Filter by status (default: 'all')
}
```

**Response**:
```typescript
interface ListNodesResponse {
    nodes: NodeSummary[];
}

interface NodeSummary {
    name: string;
    status: 'running' | 'paused' | 'stopped';
    threadId?: number;      // DAP thread ID for the node
    tickRate: number;
}
```

---

### `horus/setTopicBreakpoint`

Set a breakpoint that triggers when a message is published/received on a topic.

**Request**:
```typescript
interface SetTopicBreakpointRequest {
    topic: string;              // Topic name
    direction: 'publish' | 'subscribe' | 'both';  // When to break
    condition?: string;         // Optional condition (Rust expression)
}
```

**Response**:
```typescript
interface SetTopicBreakpointResponse {
    breakpointId: number;   // Unique breakpoint ID
    verified: boolean;      // Whether breakpoint is valid
    message?: string;       // Error message if not verified
}
```

**Example**:

Request:
```json
{
    "seq": 6,
    "type": "request",
    "command": "horus/setTopicBreakpoint",
    "arguments": {
        "topic": "cmd_vel",
        "direction": "subscribe",
        "condition": "msg.linear.x > 1.0"
    }
}
```

Response:
```json
{
    "seq": 7,
    "type": "response",
    "request_seq": 6,
    "success": true,
    "command": "horus/setTopicBreakpoint",
    "body": {
        "breakpointId": 2,
        "verified": true
    }
}
```

When breakpoint hits:
```json
{
    "seq": 8,
    "type": "event",
    "event": "stopped",
    "body": {
        "reason": "topic breakpoint",
        "description": "Breakpoint on topic 'cmd_vel' (subscribe)",
        "threadId": 1,
        "allThreadsStopped": false,
        "hitBreakpointIds": [2]
    }
}
```

---

## Enhanced Standard Requests

### `evaluate` (Enhanced)

The standard `evaluate` request is enhanced to support HORUS-specific expressions:

**HORUS-Specific Expressions**:

```rust
// Get topic value
topic("cmd_vel")
// Returns: Twist { linear: Vec3 { x: 0.5, y: 0.0, z: 0.0 }, ... }

// Get node state
node("controller").state
// Returns: NodeState { status: Running, tick_count: 1024, ... }

// Get publishers
topic("cmd_vel").publishers
// Returns: ["teleop_node"]

// Get subscribers
topic("cmd_vel").subscribers
// Returns: ["robot_controller", "logger"]

// Get message rate
topic("cmd_vel").rate
// Returns: 10.2

// Get node tick rate
node("controller").tick_rate
// Returns: 50.0
```

**Example**:

Request:
```json
{
    "seq": 9,
    "type": "request",
    "command": "evaluate",
    "arguments": {
        "expression": "topic(\"cmd_vel\")",
        "frameId": 1,
        "context": "watch"
    }
}
```

Response:
```json
{
    "seq": 10,
    "type": "response",
    "request_seq": 9,
    "success": true,
    "command": "evaluate",
    "body": {
        "result": "Twist { linear: Vec3 { x: 0.5, y: 0.0, z: 0.0 }, angular: Vec3 { x: 0.0, y: 0.0, z: 0.2 } }",
        "type": "geometry_msgs::Twist",
        "variablesReference": 1002,
        "namedVariables": 2
    }
}
```

---

### `variables` (Enhanced)

When expanding HORUS topic values or node state, the `variables` request returns structured data:

**Example** (expanding topic value from above):

Request:
```json
{
    "seq": 11,
    "type": "request",
    "command": "variables",
    "arguments": {
        "variablesReference": 1002
    }
}
```

Response:
```json
{
    "seq": 12,
    "type": "response",
    "request_seq": 11,
    "success": true,
    "command": "variables",
    "body": {
        "variables": [
            {
                "name": "linear",
                "value": "Vec3 { x: 0.5, y: 0.0, z: 0.0 }",
                "type": "Vec3",
                "variablesReference": 1003
            },
            {
                "name": "angular",
                "value": "Vec3 { x: 0.0, y: 0.0, z: 0.2 }",
                "type": "Vec3",
                "variablesReference": 1004
            }
        ]
    }
}
```

---

## Launch Configuration

### HORUS-Specific Launch Options

```json
{
    "type": "horus",
    "request": "launch",
    "name": "Debug HORUS Node",
    "program": "${file}",
    "args": [],
    "cwd": "${workspaceFolder}",

    // HORUS-specific options
    "horusDebug": true,              // Enable HORUS debugging features
    "autoWatchTopics": true,         // Automatically watch all topics
    "pauseOnTopicError": true,       // Pause when topic error occurs
    "showTopicPanel": true,          // Show topic monitor panel
    "topicRefreshRate": 10,          // Topic update rate (Hz)

    // Standard debugging options
    "stopOnEntry": false,
    "MIMode": "lldb",
    "environment": [],
    "externalConsole": false
}
```

### Attach Configuration

```json
{
    "type": "horus",
    "request": "attach",
    "name": "Attach to HORUS Node",
    "processId": "${command:pickProcess}",

    // HORUS-specific options
    "horusDebug": true,
    "autoWatchTopics": true,
    "detectNodeName": true           // Auto-detect node name from process
}
```

---

## Events

### `horus/topicValue`

Emitted when a watched topic receives a new message.

```typescript
interface TopicValueEvent {
    watchId: number;
    topic: string;
    value: any;
    timestamp: number;
}
```

---

### `horus/nodeStatusChanged`

Emitted when a node's status changes.

```typescript
interface NodeStatusChangedEvent {
    nodeName: string;
    oldStatus: 'running' | 'paused' | 'stopped';
    newStatus: 'running' | 'paused' | 'stopped';
    reason?: string;
}
```

Example:
```json
{
    "seq": 13,
    "type": "event",
    "event": "horus/nodeStatusChanged",
    "body": {
        "nodeName": "robot_controller",
        "oldStatus": "running",
        "newStatus": "paused",
        "reason": "breakpoint hit"
    }
}
```

---

### `horus/topicError`

Emitted when a topic-related error occurs.

```typescript
interface TopicErrorEvent {
    topic: string;
    error: string;
    severity: 'warning' | 'error';
}
```

Example:
```json
{
    "seq": 14,
    "type": "event",
    "event": "horus/topicError",
    "body": {
        "topic": "cmd_vel",
        "error": "Message queue overflow (10 messages dropped)",
        "severity": "warning"
    }
}
```

---

## Error Codes

Custom HORUS debug adapter error codes:

| Code | Message | Description |
|------|---------|-------------|
| 9001 | Topic not found | Specified topic does not exist |
| 9002 | Node not found | Specified node does not exist |
| 9003 | Invalid watch expression | Watch expression syntax error |
| 9004 | HORUS runtime unavailable | Cannot connect to HORUS runtime |
| 9005 | Unsupported message type | Cannot inspect this message type |

---

## Implementation Notes

### Thread Mapping

Each HORUS node is mapped to a DAP thread:

```
Node "robot_controller"  Thread ID 1
Node "sensor_driver"     Thread ID 2
Node "logger"            Thread ID 3
```

This allows standard DAP "pause thread" / "continue thread" to work on individual nodes.

### Variable References

Topic values and node state are assigned variable references for hierarchical inspection:

```
topic("cmd_vel")  variablesReference: 1000
  ├─ linear  variablesReference: 1001
  │   ├─ x  0.5
  │   ├─ y  0.0
  │   └─ z  0.0
  └─ angular  variablesReference: 1002
      ├─ x  0.0
      ├─ y  0.0
      └─ z  0.2
```

### Performance

- Topic watches are rate-limited to avoid flooding the IDE
- Default refresh rate: 10 Hz
- Configurable via `topicRefreshRate` in launch config
- Large messages are truncated with "..." and require manual expansion

### Security

- Debug adapter requires same permissions as HORUS runtime
- No remote debugging support (security consideration)
- Topic watches respect HORUS access controls

---

## Client Implementation Example

### TypeScript (VSCode)

```typescript
// Set topic watch
const response = await debugSession.customRequest('horus/setTopicWatch', {
    topic: 'cmd_vel',
    format: 'pretty'
});

console.log(`Watch ID: ${response.watchId}`);

// Listen for topic value events
debugSession.onDidReceiveDebugSessionCustomEvent(event => {
    if (event.event === 'horus/topicValue') {
        console.log(`Topic ${event.body.topic}: ${JSON.stringify(event.body.value)}`);
    }
});
```

### Kotlin (IntelliJ)

```kotlin
// Set topic watch
val response = debugProcess.sendCommand(
    HorusSetTopicWatchCommand(
        topic = "cmd_vel",
        format = TopicFormat.PRETTY
    )
)

println("Watch ID: ${response.watchId}")
```

---

## Testing

Debug adapter custom features should be tested with:

1. **Unit Tests**: Test request/response handling
2. **Integration Tests**: Test with actual HORUS processes
3. **Protocol Compliance**: Validate against DAP 1.51 spec

Example test:
```rust
#[tokio::test]
async fn test_set_topic_watch() {
    let adapter = create_test_adapter();
    let response = adapter.set_topic_watch(SetTopicWatchRequest {
        topic: "cmd_vel".to_string(),
        format: Some("pretty".to_string()),
        condition: None,
    }).await.unwrap();

    assert_eq!(response.topic, "cmd_vel");
    assert!(response.watch_id > 0);
}
```

---

## See Also

- [Debug Adapter README](../debug-adapter/README.md)
- [LSP_PROTOCOL.md](./LSP_PROTOCOL.md)
- [TOPIC_INSPECTOR.md](./TOPIC_INSPECTOR.md)
- [DAP 1.51 Specification](https://microsoft.github.io/debug-adapter-protocol/)
