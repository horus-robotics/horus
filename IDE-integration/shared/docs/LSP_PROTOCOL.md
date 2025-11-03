# HORUS Language Server Protocol Extensions

This document describes custom LSP methods and extensions specific to HORUS that extend the standard Language Server Protocol 3.17.

## Overview

The HORUS Language Server implements all standard LSP features plus custom methods for HORUS-specific functionality:

- **Topic Inspection**: Get real-time information about HORUS topics
- **Node Graph Data**: Retrieve computational graph structure
- **Live Node Listing**: List active nodes in running systems
- **Dependency Resolution**: Analyze HORUS project dependencies

## Custom LSP Methods

### `horus/topicInfo`

Get detailed information about a specific HORUS topic.

**Request**:
```typescript
interface TopicInfoParams {
    topic: string;           // Topic name (e.g., "cmd_vel")
    includeValue?: boolean;  // Include current value (default: true)
    includeRate?: boolean;   // Include message rate (default: true)
}
```

**Response**:
```typescript
interface TopicInfo {
    name: string;                // Topic name
    messageType: string;         // Rust type (e.g., "geometry_msgs::Twist")
    publishers: PublisherInfo[]; // Publishing nodes
    subscribers: SubscriberInfo[];// Subscribing nodes
    currentValue?: any;          // Latest message (if available)
    messageRate?: number;        // Messages per second
    queueSize?: number;          // Queue size configuration
}

interface PublisherInfo {
    nodeName: string;   // Publishing node name
    rate?: number;      // Publishing rate (Hz)
}

interface SubscriberInfo {
    nodeName: string;   // Subscribing node name
    callback: string;   // Callback function name
}
```

**Example**:

Request:
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "horus/topicInfo",
    "params": {
        "topic": "cmd_vel",
        "includeValue": true,
        "includeRate": true
    }
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "name": "cmd_vel",
        "messageType": "geometry_msgs::Twist",
        "publishers": [
            {
                "nodeName": "teleop_node",
                "rate": 10.0
            }
        ],
        "subscribers": [
            {
                "nodeName": "robot_controller",
                "callback": "on_cmd_vel"
            }
        ],
        "currentValue": {
            "linear": { "x": 0.5, "y": 0.0, "z": 0.0 },
            "angular": { "x": 0.0, "y": 0.0, "z": 0.2 }
        },
        "messageRate": 10.2,
        "queueSize": 10
    }
}
```

**Use Cases**:
- Hover tooltip over topic string literals
- Autocomplete for topic names
- Dashboard integration
- Topic inspector panel

---

### `horus/graphData`

Retrieve the complete node graph for visualization.

**Request**:
```typescript
interface GraphDataParams {
    includeInactive?: boolean;  // Include inactive nodes (default: false)
    includeTypes?: boolean;     // Include message types (default: true)
}
```

**Response**:
```typescript
interface GraphData {
    nodes: NodeInfo[];
    topics: TopicGraphInfo[];
    connections: Connection[];
}

interface NodeInfo {
    name: string;           // Node name
    type: string;           // Node type/category
    status: NodeStatus;     // Running, stopped, error
    tickRate: number;       // Execution rate (Hz)
    cpuUsage?: number;      // CPU usage percentage
    memoryUsage?: number;   // Memory usage (bytes)
}

enum NodeStatus {
    Running = "running",
    Stopped = "stopped",
    Error = "error",
    Unknown = "unknown"
}

interface TopicGraphInfo {
    name: string;           // Topic name
    messageType: string;    // Message type
    messageCount: number;   // Total messages
    bandwidth?: number;     // Bytes per second
}

interface Connection {
    from: string;           // Publisher node name
    to: string;             // Subscriber node name
    topic: string;          // Topic name
    messageRate?: number;   // Messages per second
}
```

**Example**:

Request:
```json
{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "horus/graphData",
    "params": {
        "includeInactive": false,
        "includeTypes": true
    }
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 2,
    "result": {
        "nodes": [
            {
                "name": "camera_driver",
                "type": "sensor",
                "status": "running",
                "tickRate": 30.0,
                "cpuUsage": 5.2,
                "memoryUsage": 102400
            },
            {
                "name": "image_processor",
                "type": "processing",
                "status": "running",
                "tickRate": 30.0,
                "cpuUsage": 12.8,
                "memoryUsage": 524288
            }
        ],
        "topics": [
            {
                "name": "camera/image_raw",
                "messageType": "sensor_msgs::Image",
                "messageCount": 1024,
                "bandwidth": 9216000
            }
        ],
        "connections": [
            {
                "from": "camera_driver",
                "to": "image_processor",
                "topic": "camera/image_raw",
                "messageRate": 30.0
            }
        ]
    }
}
```

**Use Cases**:
- Node graph visualization
- System architecture inspection
- Performance monitoring
- Debugging communication issues

---

### `horus/listNodes`

List all active nodes in the running HORUS system.

**Request**:
```typescript
interface ListNodesParams {
    includeMetrics?: boolean;  // Include performance metrics (default: true)
}
```

**Response**:
```typescript
interface NodeInfo {
    name: string;               // Node name
    publishers: TopicEndpoint[];// Published topics
    subscribers: TopicEndpoint[];// Subscribed topics
    tickRate: number;           // Execution rate (Hz)
    cpuUsage?: number;          // CPU usage percentage
    memoryUsage?: number;       // Memory usage (bytes)
    uptime?: number;            // Seconds since start
}

interface TopicEndpoint {
    topic: string;      // Topic name
    type: string;       // Message type
    rate?: number;      // Message rate (Hz)
}
```

**Example**:

Request:
```json
{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "horus/listNodes",
    "params": {
        "includeMetrics": true
    }
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 3,
    "result": [
        {
            "name": "robot_controller",
            "publishers": [
                {
                    "topic": "motor_commands",
                    "type": "std_msgs::Float64MultiArray",
                    "rate": 50.0
                }
            ],
            "subscribers": [
                {
                    "topic": "cmd_vel",
                    "type": "geometry_msgs::Twist",
                    "rate": 10.0
                },
                {
                    "topic": "sensor_data",
                    "type": "sensor_msgs::LaserScan",
                    "rate": 20.0
                }
            ],
            "tickRate": 50.0,
            "cpuUsage": 8.5,
            "memoryUsage": 204800,
            "uptime": 3600.5
        }
    ]
}
```

**Use Cases**:
- Runtime monitoring
- Node selection in UI
- Performance profiling
- System health checks

---

### `horus/projectInfo`

Get information about the HORUS project configuration.

**Request**:
```typescript
interface ProjectInfoParams {}
```

**Response**:
```typescript
interface ProjectInfo {
    name: string;               // Project name
    version: string;            // Project version
    horusSource: string;        // HORUS framework path
    dependencies: Dependency[]; // Project dependencies
    nodes: string[];            // Node file paths
    rootPath: string;           // Project root directory
}

interface Dependency {
    name: string;       // Dependency name
    version?: string;   // Version constraint
    path?: string;      // Local path dependency
    git?: string;       // Git repository
}
```

**Example**:

Request:
```json
{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "horus/projectInfo",
    "params": {}
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 4,
    "result": {
        "name": "my_robot",
        "version": "0.1.0",
        "horusSource": "/home/user/horus/HORUS",
        "dependencies": [
            {
                "name": "common_msgs",
                "path": "../common_msgs"
            }
        ],
        "nodes": [
            "/home/user/my_robot/controller.rs",
            "/home/user/my_robot/sensors/camera.rs"
        ],
        "rootPath": "/home/user/my_robot"
    }
}
```

**Use Cases**:
- Project configuration display
- Dependency management UI
- Project structure navigation

---

### `horus/resolveSymbol`

Resolve HORUS framework symbols (topics, nodes, message types).

**Request**:
```typescript
interface ResolveSymbolParams {
    symbol: string;      // Symbol to resolve
    type: SymbolType;    // Type of symbol
}

enum SymbolType {
    Topic = "topic",
    Node = "node",
    MessageType = "message_type"
}
```

**Response**:
```typescript
interface SymbolResolution {
    symbol: string;         // Resolved symbol
    location?: Location;    // Definition location
    documentation?: string; // Documentation
    type?: string;          // Type information
}

interface Location {
    uri: string;    // File URI
    range: Range;   // Line/column range
}
```

**Example**:

Request:
```json
{
    "jsonrpc": "2.0",
    "id": 5,
    "method": "horus/resolveSymbol",
    "params": {
        "symbol": "cmd_vel",
        "type": "topic"
    }
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 5,
    "result": {
        "symbol": "cmd_vel",
        "location": {
            "uri": "file:///home/user/my_robot/controller.rs",
            "range": {
                "start": { "line": 45, "character": 20 },
                "end": { "line": 45, "character": 27 }
            }
        },
        "documentation": "Velocity command topic for robot base control",
        "type": "geometry_msgs::Twist"
    }
}
```

**Use Cases**:
- Go-to-definition for topics
- Symbol search
- Autocomplete with documentation

---

## Standard LSP Methods (Enhanced)

### `textDocument/completion`

Standard LSP completion enhanced with HORUS-specific completions:

**HORUS-Specific Completions**:
- Topic names when inside topic string literals
- Message type imports from HORUS framework
- Node names for testing/simulation
- HORUS prelude items

**Example**:

When typing inside `subscribe("...")`:
```rust
node.subscribe("cm|")  // cursor at |
```

Completions include:
- `cmd_vel` (geometry_msgs::Twist)
- `camera/image_raw` (sensor_msgs::Image)
- ... (all topics in project)

---

### `textDocument/hover`

Standard LSP hover enhanced with HORUS context:

**Enhanced Hover Information**:
- Topic details when hovering over topic strings
- Node information when hovering over node names
- Message type documentation from HORUS framework

**Example**:

Hovering over `"cmd_vel"`:
```markdown
**Topic**: cmd_vel
**Type**: geometry_msgs::Twist
**Publishers**: teleop_node (10 Hz)
**Subscribers**: robot_controller
**Current Rate**: 10.2 Hz
```

---

## Error Codes

Custom HORUS error codes:

| Code | Message | Description |
|------|---------|-------------|
| -32001 | HORUS project not found | No `horus.yaml` in workspace |
| -32002 | HORUS_SOURCE not set | Framework source not configured |
| -32003 | Invalid topic name | Topic not found in system |
| -32004 | Runtime not available | HORUS system not running |
| -32005 | Parse error | Invalid `horus.yaml` syntax |

**Example Error**:
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "error": {
        "code": -32003,
        "message": "Invalid topic name",
        "data": {
            "topic": "unknown_topic",
            "availableTopics": ["cmd_vel", "sensor_data"]
        }
    }
}
```

---

## Notifications

### `horus/topicUpdated`

Server-to-client notification when topic values change (if enabled).

```typescript
interface TopicUpdatedParams {
    topic: string;      // Topic name
    value: any;         // New value
    timestamp: number;  // Unix timestamp
}
```

**Example**:
```json
{
    "jsonrpc": "2.0",
    "method": "horus/topicUpdated",
    "params": {
        "topic": "cmd_vel",
        "value": {
            "linear": { "x": 1.0, "y": 0.0, "z": 0.0 },
            "angular": { "x": 0.0, "y": 0.0, "z": 0.5 }
        },
        "timestamp": 1609459200
    }
}
```

---

## Implementation Notes

### Performance Considerations

- **Caching**: Topic information and project structure are cached
- **Incremental Updates**: Only changed files trigger re-analysis
- **Lazy Loading**: Runtime data fetched on-demand
- **Debouncing**: Rapid file changes are debounced before analysis

### Security

- Language server runs with same permissions as IDE
- No network access required for basic features
- Runtime integration requires HORUS system access

### Compatibility

All custom methods are optional. Standard LSP clients will work with basic features even if custom methods are not supported.

---

## Client Implementation Example

### TypeScript (VSCode)

```typescript
// Send custom request
const topicInfo = await client.sendRequest('horus/topicInfo', {
    topic: 'cmd_vel',
    includeValue: true
});

console.log(`Topic type: ${topicInfo.messageType}`);
console.log(`Publishers: ${topicInfo.publishers.map(p => p.nodeName).join(', ')}`);
```

### Lua (Neovim)

```lua
-- Send custom request
vim.lsp.buf_request(0, 'horus/topicInfo', {
    topic = 'cmd_vel',
    includeValue = true
}, function(err, result)
    if result then
        print('Topic type: ' .. result.messageType)
    end
end)
```

### Emacs Lisp

```elisp
;; Send custom request
(lsp-request
 "horus/topicInfo"
 `(:topic "cmd_vel" :includeValue t))
```

---

## Testing

Language server custom methods should be tested with:

1. **Unit Tests**: Test request/response parsing
2. **Integration Tests**: Test with mock HORUS projects
3. **Protocol Compliance**: Validate against LSP 3.17 spec

Example test:
```rust
#[tokio::test]
async fn test_topic_info_request() {
    let server = create_test_server();
    let result = server.topic_info(TopicInfoParams {
        topic: "cmd_vel".to_string(),
        include_value: Some(true),
        include_rate: Some(true),
    }).await.unwrap();

    assert_eq!(result.name, "cmd_vel");
    assert_eq!(result.message_type, "geometry_msgs::Twist");
    assert!(result.publishers.len() > 0);
}
```

---

## See Also

- [Language Server README](../language-server/README.md)
- [TOPIC_INSPECTOR.md](./TOPIC_INSPECTOR.md)
- [NODE_GRAPH.md](./NODE_GRAPH.md)
- [LSP 3.17 Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
