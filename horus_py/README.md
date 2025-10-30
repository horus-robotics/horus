# HORUS Python - Simple & Intuitive Robotics Framework

A user-friendly Python API for the HORUS robotics framework that makes creating distributed robotic systems as easy as writing a simple function.

## Quick Start

### Minimal Example (10 lines!)

```python
import horus

def process(node):
    node.send("output", "Hello HORUS!")

node = horus.Node(pubs="output", tick=process, rate=1)
horus.run(node, duration=3)
```

That's it! No classes to inherit, no boilerplate, just pure logic.

## Installation

### Automatic Installation (Recommended)

The easiest way to install HORUS Python bindings is using the main installation script:

```bash
# From the HORUS root directory
./install.sh
```

The script will:
- Check for Python 3.9+ and pip
- Automatically install maturin if needed
- Build and install the horus_py package
- Verify the installation

### Manual Installation

If you prefer to install manually or the automatic installation fails:

```bash
# Install maturin (Python/Rust build tool)
pip install maturin

# Build and install
cd horus_py
maturin develop --release
```

### Requirements

- Python 3.9+
- Rust 1.70+
- Linux (for shared memory support)
- pip (Python package manager)

## Core Concepts

HORUS Python uses just 3 simple concepts:

1. **Node** - A processing unit with inputs/outputs
2. **Scheduler** - Manages and runs nodes
3. **Topics** - Named channels for communication

## The Simple API

### Creating a Node

```python
node = horus.Node(
    name="my_node",           # Optional: auto-generated if not provided
    pubs=["topic1", "topic2"], # Topics to publish to
    subs=["input1", "input2"],  # Topics to subscribe to
    tick=my_function,          # Function to call repeatedly
    rate=30,                   # Hz (default: 30)
    init=setup_fn,             # Optional: called once at start
    shutdown=cleanup_fn        # Optional: called once at end
)
```

### Node Functions

Your tick function receives the node as a parameter:

```python
def my_function(node):
    # Check for messages
    if node.has_msg("input1"):
        data = node.get("input1")  # Get one message

    # Or get all messages
    all_msgs = node.get_all("input1")

    # Send messages
    node.send("topic1", {"sensor": 42})
```

### Running Nodes

```python
# Single node
horus.run(node)

# Multiple nodes
horus.run(node1, node2, node3, duration=10)

# Using scheduler with default priority (insertion order)
scheduler = horus.Scheduler()
scheduler.add(node1, node2)
scheduler.run(duration=5)

# Using scheduler with explicit priorities (deterministic execution)
scheduler = horus.Scheduler()
scheduler.register(sensor_node, priority=0, logging=True)   # Runs first
scheduler.register(control_node, priority=1, logging=False) # Runs second
scheduler.register(motor_node, priority=2, logging=True)    # Runs third
scheduler.run(duration=5)
```

## Examples

### 1. Simple Transform

```python
import horus

# Producer
producer = horus.Node(
    pubs="numbers",
    tick=lambda n: n.send("numbers", 42),
    rate=1
)

# Transformer
doubler = horus.Node(
    subs="numbers",
    pubs="doubled",
    tick=lambda n: n.send("doubled", n.get("numbers") * 2) if n.has_msg("numbers") else None
)

# Run both
horus.run(producer, doubler, duration=5)
```

### 2. Multi-Topic Robot

```python
def robot_controller(node):
    # Read from multiple sensors
    if node.has_msg("lidar"):
        lidar = node.get("lidar")

    if node.has_msg("camera"):
        camera = node.get("camera")

    # Compute and send commands
    cmd = compute_command(lidar, camera)
    node.send("motors", cmd)
    node.send("status", "active")

robot = horus.Node(
    name="robot",
    subs=["lidar", "camera"],
    pubs=["motors", "status"],
    tick=robot_controller,
    rate=50  # 50Hz control loop
)
```

### 3. Using Quick Helpers

```python
# One-line transform node
node = horus.quick(
    sub="celsius",
    pub="fahrenheit",
    fn=lambda c: c * 9/5 + 32
)

# Run a simple pipe
horus.pipe("input", "output", lambda x: x ** 2)

# Echo data between topics
horus.echo("sensor_raw", "sensor_backup")

# Filter messages
horus.filter_node("all_data", "positive_only", lambda x: x > 0)
```

### 4. Lifecycle Management

```python
class Context:
    def __init__(self):
        self.count = 0
        self.file = None

ctx = Context()

def init(node):
    print("Starting up!")
    ctx.file = open("data.txt", "w")

def tick(node):
    ctx.count += 1
    data = f"Tick {ctx.count}"
    node.send("data", data)
    ctx.file.write(data + "\n")

def shutdown(node):
    print(f"Processed {ctx.count} messages")
    ctx.file.close()

node = horus.Node(
    pubs="data",
    init=init,
    tick=tick,
    shutdown=shutdown,
    rate=10
)
```

## Advanced Features

### Priority-based Execution (Deterministic)

For robotics applications where execution order matters, use explicit priorities:

```python
import horus

# Create nodes
sensor = horus.Node(
    name="sensor",
    pubs="sensor_data",
    tick=lambda n: n.send("sensor_data", read_sensor())
)

controller = horus.Node(
    name="controller",
    subs="sensor_data",
    pubs="motor_cmd",
    tick=lambda n: n.send("motor_cmd", compute_control(n.get("sensor_data"))) if n.has_msg("sensor_data") else None
)

actuator = horus.Node(
    name="actuator",
    subs="motor_cmd",
    tick=lambda n: send_to_motor(n.get("motor_cmd")) if n.has_msg("motor_cmd") else None
)

# Register with priorities (lower = higher priority)
scheduler = horus.Scheduler()
scheduler.register(sensor, priority=0, logging=True)      # Runs FIRST
scheduler.register(controller, priority=1, logging=False) # Runs SECOND
scheduler.register(actuator, priority=2, logging=True)    # Runs THIRD
scheduler.run()
```

**Why priorities matter:**
- **Deterministic execution**: Nodes always execute in the same order
- **Correct data flow**: Sensors read before controllers compute
- **Reproducible behavior**: Same input produces same output every time
- **Debugging**: Easier to reason about system behavior

### Broadcast (Fanout)
```python
horus.fanout("sensor", ["log", "display", "storage"])
```

### Merge Multiple Inputs
```python
horus.merge(["sensor1", "sensor2", "sensor3"], "all_sensors")
```

### Chainable Registration
```python
scheduler = horus.Scheduler()
scheduler.register(node1, 0, True) \
         .register(node2, 1, False) \
         .register(node3, 2, True) \
         .run()
```

## Design Philosophy

The HORUS Python API follows these principles:

1. **Simple things should be simple** - A working node in 5 lines
2. **No mandatory inheritance** - Use functions, not classes
3. **Explicit is better** - Everything visible in the constructor
4. **Progressive complexity** - Start simple, add features as needed
5. **Pythonic** - Feels like native Python, not wrapped C++

## Running Examples

Check out the examples directory:

```bash
# Minimal example
python examples/minimal_example.py

# Complete demo
python examples/simple_api_demo.py

# Robot simulation
python examples/robot_example.py
```

##  Why This Design?

The HORUS Python API is designed for simplicity and productivity:

- **Function-based** - No class inheritance required
- **Minimal boilerplate** - Node creation in one line
- **Clear data flow** - Explicit `get`/`send` operations
- **Testable** - Functions can be tested independently
- **Gradual complexity** - Start with 5 lines, scale as needed

## Performance Tips

1. **Use appropriate tick rates**: Higher rates increase CPU usage
2. **Batch operations**: Process multiple messages per tick when possible
3. **Keep tick() fast**: Avoid blocking operations
4. **Use mock mode**: Test without Rust bindings using the built-in mock

## Development

### Building from Source

```bash
# Debug build
maturin develop

# Release build (optimized)
maturin develop --release

# Build wheel
maturin build --release
```

### Running Tests

```bash
# Install test dependencies
pip install pytest

# Run tests
pytest tests/
```

### Mock Mode

The API includes a mock mode for testing without Rust bindings:

```python
# If Rust bindings aren't available, mock mode activates automatically
# You'll see: "Warning: Rust bindings not available. Running in mock mode."
```

## License

Same as HORUS core - see main project LICENSE file.

---

**Remember**: With HORUS Python, you focus on *what* your robot does, not *how* the framework works!