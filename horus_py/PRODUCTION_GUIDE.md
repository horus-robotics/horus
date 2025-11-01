# HORUS Python - Production Guide

This guide covers production-grade usage of the HORUS Python API for real-world robotics applications.

## Table of Contents
1. [Production API Overview](#production-api-overview)
2. [Type Safety](#type-safety)
3. [Error Handling](#error-handling)
4. [Performance Monitoring](#performance-monitoring)
5. [Multi-Process Architecture](#multi-process-architecture)
6. [Best Practices](#best-practices)

---

## Production API Overview

### Exports

```python
import horus

# Core classes
horus.Node          # Node creation
horus.Scheduler     # Node orchestration
horus.NodeState     # State enum
horus.run()         # Quick run helper
```

### API Coverage

| Feature | Rust | Python | Status |
|---------|------|--------|--------|
| Node trait/class | ✅ | ✅ | Full |
| Type hints | ✅ | ✅ | Full |
| State management | ✅ | ✅ | Full |
| Error handling | ✅ | ✅ | Full |
| Performance metrics | ✅ | ✅ | Full |
| Topic introspection | ✅ | ✅ | Full |
| Multi-process | ✅ | ✅ | Full |

**Coverage: ~75% of Rust API** (missing: resource monitoring, advanced heartbeats)

---

## Type Safety

### Full Type Annotations

All API methods now have complete type hints for IDE autocomplete and type checking:

```python
from typing import Optional
import horus

def my_tick(node: horus.Node) -> None:
    """Type-safe tick function"""
    data: Optional[dict] = node.get("sensor")
    if data:
        node.send("output", data)

def my_error_handler(node: horus.Node, exception: Exception) -> None:
    """Type-safe error handler"""
    node.log_error(f"Error occurred: {exception}")

# Create node with full type checking
node = horus.Node(
    name="typed_node",
    pubs=["output"],
    subs=["sensor"],
    tick=my_tick,
    rate=100,
    on_error=my_error_handler
)
```

### Benefits

- **IDE Autocomplete**: Full method suggestions
- **Type Checking**: Use `mypy` or `pyright` for static analysis
- **Documentation**: Better inline documentation
- **Refactoring**: Safer code changes

---

## Error Handling

### Production Error Handling Pattern

```python
import horus

class RobustController:
    def __init__(self):
        self.consecutive_errors = 0
        self.max_errors = 10
        self.safe_mode = False

    def tick(self, node: horus.Node) -> None:
        """Production tick with robust error handling"""
        try:
            # Check node state before processing
            if node.info and node.info.state() == horus.NodeState.ERROR:
                self.enter_safe_mode(node)
                return

            # Normal operation
            if node.has_msg("sensor"):
                data = node.get("sensor")
                self.process_data(node, data)
                self.consecutive_errors = 0  # Reset on success

        except ValueError as e:
            node.log_warning(f"Recoverable error: {e}")
            # Continue execution
        except Exception as e:
            # Let error handler deal with it
            raise

    def on_error(self, node: horus.Node, exception: Exception) -> None:
        """Handle errors gracefully"""
        self.consecutive_errors += 1
        node.log_error(f"Error #{self.consecutive_errors}: {exception}")

        # Check if we should transition to safe mode
        if self.consecutive_errors >= self.max_errors:
            if node.info:
                node.info.transition_to_error("Too many consecutive errors")
            self.enter_safe_mode(node)

    def enter_safe_mode(self, node: horus.Node) -> None:
        """Enter degraded safe mode"""
        if not self.safe_mode:
            self.safe_mode = True
            node.log_error("Entering SAFE MODE - degraded operation")
            # Publish safe commands (e.g., stop robot)
            node.send("cmd_vel", {"linear": 0.0, "angular": 0.0})

    def process_data(self, node: horus.Node, data: dict) -> None:
        """Your actual processing logic"""
        result = {"processed": True, "value": data}
        node.send("output", result)

# Create production node
controller = RobustController()
node = horus.Node(
    name="robust_controller",
    pubs=["cmd_vel", "output"],
    subs=["sensor"],
    tick=controller.tick,
    on_error=controller.on_error,
    rate=100
)
```

### Error Handling Behavior

| Scenario | on_error provided | Behavior |
|----------|-------------------|----------|
| Exception in tick() | ❌ | Exception propagates, node crashes |
| Exception in tick() | ✅ | on_error() called, execution continues |
| Exception > 10 times | ✅ | Auto-transition to ERROR state |
| Exception in on_error() | ✅ | Logged, original exception suppressed |

---

## Performance Monitoring

### Accessing Metrics During Execution

```python
import horus

def performance_aware_tick(node: horus.Node) -> None:
    """Tick function with performance monitoring"""
    # Access node info (only available during tick)
    if node.info:
        # Get current state
        state = node.info.state()

        # Get comprehensive metrics
        metrics = node.info.get_metrics()
        avg_duration = metrics['avg_tick_duration_ms']
        error_count = metrics['errors_count']
        total_ticks = metrics['total_ticks']

        # Check for performance degradation
        if avg_duration > 100:
            node.log_warning(f"Slow tick detected: {avg_duration:.1f}ms")

        # Monitor error rate
        error_rate = error_count / max(total_ticks, 1)
        if error_rate > 0.01:  # More than 1% error rate
            node.log_error(f"High error rate: {error_rate:.2%}")

        # Get uptime
        uptime = node.info.get_uptime()
        node.log_info(f"Running for {uptime:.1f}s")

    # Your normal tick logic
    if node.has_msg("input"):
        data = node.get("input")
        node.send("output", data)
```

### Available Metrics

```python
metrics = node.info.get_metrics()
# Returns:
{
    'total_ticks': 1000,          # Total ticks executed
    'successful_ticks': 995,       # Ticks without errors
    'failed_ticks': 5,             # Ticks with errors
    'errors_count': 5,             # Total errors
    'avg_tick_duration_ms': 2.5,   # Average duration
    'min_tick_duration_ms': 1.0,   # Fastest tick
    'max_tick_duration_ms': 15.0,  # Slowest tick
    'last_tick_duration_ms': 2.3   # Most recent tick
}
```

### Individual Metric Access

```python
# Quick access methods
total = node.info.tick_count()          # Total ticks
errors = node.info.error_count()        # Error count
avg_ms = node.info.avg_tick_duration_ms()  # Average duration
uptime = node.info.get_uptime()        # Uptime in seconds
failed = node.info.failed_ticks()       # Failed tick count
success = node.info.successful_ticks()  # Successful ticks
```

---

## Multi-Process Architecture

### Running Multiple Nodes Concurrently

HORUS automatically runs multiple Python files as separate processes, bypassing the GIL:

```bash
# Run all nodes in parallel (each in separate process)
horus run "nodes/*.py"

# Explicit file list
horus run sensor.py controller.py motor.py logger.py
```

### Project Structure

```
my_robot/
├── horus.yaml              # Optional project config
├── nodes/
│   ├── sensor_node.py      # IMU sensor (100Hz)
│   ├── controller_node.py  # Control loop (50Hz)
│   ├── motor_node.py       # Motor commands (100Hz)
│   └── logger_node.py      # Data logging (10Hz)
└── README.md
```

### Example: Multi-Node System

**nodes/sensor_node.py**:
```python
import horus

def sensor_tick(node: horus.Node) -> None:
    # Read IMU sensor
    imu_data = {
        "accel_x": 1.0,
        "accel_y": 0.0,
        "accel_z": 9.8,
        "gyro_x": 0.0,
        "gyro_y": 0.0,
        "gyro_z": 0.1
    }
    node.send("imu_data", imu_data)

sensor = horus.Node(
    name="imu_sensor",
    pubs=["imu_data"],
    tick=sensor_tick,
    rate=100  # 100Hz sensor reading
)

if __name__ == "__main__":
    horus.run(sensor)
```

**nodes/controller_node.py**:
```python
import horus

def control_tick(node: horus.Node) -> None:
    # Process IMU data
    if node.has_msg("imu_data"):
        imu = node.get("imu_data")

        # Simple control logic
        cmd = {
            "linear": 1.0,
            "angular": imu["gyro_z"] * -2.0  # Stabilize rotation
        }
        node.send("cmd_vel", cmd)

controller = horus.Node(
    name="controller",
    subs=["imu_data"],
    pubs=["cmd_vel"],
    tick=control_tick,
    rate=50  # 50Hz control loop
)

if __name__ == "__main__":
    horus.run(controller)
```

**nodes/motor_node.py**:
```python
import horus

def motor_tick(node: horus.Node) -> None:
    # Execute motor commands
    if node.has_msg("cmd_vel"):
        cmd = node.get("cmd_vel")
        # Apply to motors
        node.log_info(f"Motors: linear={cmd['linear']}, angular={cmd['angular']}")

motor = horus.Node(
    name="motor_controller",
    subs=["cmd_vel"],
    tick=motor_tick,
    rate=100  # 100Hz motor update
)

if __name__ == "__main__":
    horus.run(motor)
```

### Running the System

```bash
cd my_robot
horus run "nodes/*.py"
```

Output:
```
 Executing 3 files concurrently:
  🔒 Session: abc-123-def
  1. controller_node.py (python)
  2. motor_node.py (python)
  3. sensor_node.py (python)

 Phase 1: Building all files...
 All files built successfully!

 Phase 2: Starting all processes...
   Started [controller_node]
   Started [motor_node]
   Started [sensor_node]

 All processes running. Press Ctrl+C to stop.
```

### Communication Between Processes

- **Shared Memory IPC**: Zero-copy communication via `/dev/shm/horus/topics/`
- **Topic-based**: Pub/sub architecture, any node can publish/subscribe
- **Session Isolation**: Each run has unique session ID, topics don't conflict
- **Performance**: ~1-5μs message passing latency

---

## Best Practices

### 1. Use Class-Based Nodes for Complex Logic

```python
class MyNode:
    def __init__(self):
        # Initialize state
        self.calibration = 0.0
        self.mode = "idle"

    def tick(self, node: horus.Node) -> None:
        # Access self.* for state
        pass

    def on_error(self, node: horus.Node, error: Exception) -> None:
        # Handle errors
        pass

logic = MyNode()
node = horus.Node(
    name="my_node",
    tick=logic.tick,
    on_error=logic.on_error,
    rate=100
)
```

### 2. Set Appropriate Rates

- **Sensors**: 100-1000Hz (high frequency)
- **Control**: 50-200Hz (medium frequency)
- **Logging**: 1-10Hz (low frequency)
- **Networking**: 10-30Hz (low frequency)

### 3. Monitor Performance

```python
def tick(node: horus.Node) -> None:
    if node.info:
        metrics = node.info.get_metrics()
        # Log slow ticks
        if metrics['last_tick_duration_ms'] > 10:
            node.log_warning(f"Slow tick: {metrics['last_tick_duration_ms']:.1f}ms")
```

### 4. Handle Stale Data

```python
def tick(node: horus.Node) -> None:
    # Check for stale messages (older than 50ms)
    if node.is_stale("sensor", max_age=0.05):
        node.log_warning("Sensor data is stale!")
        return  # Skip this tick

    data = node.get("sensor")
    # Process fresh data...
```

### 5. Use Type Hints

```python
from typing import Dict, Optional
import horus

def typed_tick(node: horus.Node) -> None:
    data: Optional[Dict[str, float]] = node.get("sensor")
    if data:
        result: Dict[str, bool] = {"success": True}
        node.send("output", result)
```

### 6. Implement Error Recovery

```python
def on_error(node: horus.Node, error: Exception) -> None:
    # Log and continue
    node.log_error(f"Recoverable error: {error}")

    # Send safe command
    node.send("cmd_vel", {"linear": 0.0, "angular": 0.0})
```

### 7. Topic Introspection

```python
def init(node: horus.Node) -> None:
    # Log node configuration
    node.log_info(f"Publishers: {node.get_publishers()}")
    node.log_info(f"Subscribers: {node.get_subscribers()}")
```

---

## Performance Characteristics

### Python vs Rust Performance

| Aspect | Python | Rust | Notes |
|--------|--------|------|-------|
| Tick overhead | ~10-50μs | ~1-5μs | Python GIL + function call |
| IPC latency | ~1-5μs | ~1-5μs | Same (shared memory) |
| Max tick rate | ~10kHz | ~1MHz | Practical limits |
| Memory overhead | ~50MB/node | ~5MB/node | Python interpreter |

### When to Use Python vs Rust

**Use Python for:**
- Prototyping and development
- Integration with Python libraries (OpenCV, NumPy, etc.)
- Control loops up to 500Hz
- Non-critical paths
- Rapid iteration

**Use Rust for:**
- High-frequency control (>500Hz)
- Hard real-time requirements
- Performance-critical paths
- Low-latency requirements (<1ms)
- Embedded systems

**Mixed Approach** (Recommended for production):
```bash
# Run Python and Rust nodes together
horus run "nodes/*.py" "nodes/*.rs"
```

---

## Migration from Development to Production

### Development Code

```python
# Simple development node
def tick(node):
    data = node.get("sensor")
    node.send("output", data * 2)

node = horus.Node(name="dev", subs="sensor", pubs="output", tick=tick)
horus.run(node)
```

### Production Code

```python
from typing import Optional
import horus

class ProductionNode:
    def __init__(self):
        self.error_count = 0
        self.safe_mode = False

    def tick(self, node: horus.Node) -> None:
        """Type-safe, monitored, error-handled"""
        # Check state
        if node.info and node.info.state() == horus.NodeState.ERROR:
            return

        # Check performance
        if node.info:
            metrics = node.info.get_metrics()
            if metrics['avg_tick_duration_ms'] > 100:
                node.log_warning(f"Performance degraded: {metrics['avg_tick_duration_ms']:.1f}ms")

        # Check for stale data
        if node.is_stale("sensor", max_age=0.1):
            node.log_warning("Stale sensor data")
            return

        # Process
        data: Optional[dict] = node.get("sensor")
        if data:
            result = {"value": data.get("value", 0) * 2}
            node.send("output", result)

    def on_error(self, node: horus.Node, error: Exception) -> None:
        """Graceful error handling"""
        self.error_count += 1
        node.log_error(f"Error #{self.error_count}: {error}")

        if self.error_count > 10:
            self.safe_mode = True
            node.send("output", {"safe_mode": True})

logic = ProductionNode()
node = horus.Node(
    name="prod",
    subs=["sensor"],
    pubs=["output"],
    tick=logic.tick,
    on_error=logic.on_error,
    rate=100
)

if __name__ == "__main__":
    horus.run(node)
```

---

## Troubleshooting

### High Tick Duration

```python
def tick(node: horus.Node) -> None:
    if node.info:
        duration = node.info.avg_tick_duration_ms()
        if duration > 10:
            # Investigate slow operations
            node.log_warning(f"Slow tick: {duration:.1f}ms")
```

**Solutions:**
- Move heavy computation to separate thread
- Reduce tick rate
- Optimize algorithms
- Consider Rust for this node

### High Error Rate

```python
def tick(node: horus.Node) -> None:
    if node.info:
        metrics = node.info.get_metrics()
        error_rate = metrics['errors_count'] / max(metrics['total_ticks'], 1)
        if error_rate > 0.01:
            node.log_error(f"Error rate: {error_rate:.2%}")
```

**Solutions:**
- Check error handler implementation
- Add try/except blocks
- Validate input data
- Add timeout mechanisms

### Process Not Starting

Check logs:
```bash
horus run "nodes/*.py" 2>&1 | tee run.log
```

Common issues:
- Import errors (missing dependencies)
- Syntax errors in node files
- Port/resource conflicts

---

## Summary

The HORUS Python API now provides **~75% coverage of Rust features** for production use:

✅ Type hints and IDE support
✅ State management (NodeState enum)
✅ Error handling (on_error callback)
✅ Performance monitoring (metrics, uptime)
✅ Topic introspection (get_publishers/subscribers)
✅ Multi-process support (GIL bypass)

**Production-ready for:**
- Mobile robots
- Manipulation arms (non-critical)
- Sensor fusion systems
- Computer vision pipelines
- Control loops up to 500Hz

**Consider Rust for:**
- Hard real-time control (>500Hz)
- Safety-critical systems
- Ultra-low latency (<1ms)
- Flight controllers
