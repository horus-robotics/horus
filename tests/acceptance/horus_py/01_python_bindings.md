# User Acceptance Test: Python Bindings (horus_py)

## Feature
Python bindings via PyO3 FFI for Hub communication and Node implementation.

## User Story
As a Python developer, I want to use HORUS from Python with the same performance and features as Rust, so I can build robot applications in my preferred language.

## Installation Tests

### Scenario 1: Install Python Bindings
**Given:** User has Python 3.9+ and pip
**When:** User runs `pip install horus-py` or builds from source
**Then:**
- [ ] Installation completes successfully
- [ ] Package is importable: `import horus`
- [ ] No compilation errors
- [ ] Works on Linux and macOS

**Acceptance Criteria:**
```bash
$ pip install horus-py
Collecting horus-py...
Installing...
Successfully installed horus-py-0.1.0

$ python3 -c "import horus; print(horus.__version__)"
0.1.0
```

### Scenario 2: Build from Source
**Given:** User clones repository
**When:** User runs the installation script or builds manually
**Then:**
- [ ] Rust code compiles
- [ ] Python bindings are built
- [ ] Package is available in local environment

**Acceptance Criteria (Automatic via install.sh):**
```bash
$ cd HORUS
$ ./install.sh
   ...
   Installing horus_py...
   Built and installed horus_py Python package
 horus_py is importable in Python
```

**Acceptance Criteria (Manual build):**
```bash
$ cd horus_py
$ maturin develop --release
   Compiling horus_py...
   Finished release [optimized]
 Built wheel for CPython 3.11
🛠 Installed horus-py-0.1.0
```

## Hub Communication Tests

### Scenario 3: Create Hub in Python
**Given:** Python script imports horus
**When:** User creates Hub
**Then:**
- [ ] Hub is created successfully
- [ ] Topic name is registered
- [ ] Type hints work correctly (if available)

**Acceptance Criteria:**
```python
from horus import Hub

hub = Hub(int, "test_topic")
assert hub.topic_name == "test_topic"
```

### Scenario 4: Publish from Python
**Given:** Python Hub is created
**When:** User sends message
**Then:**
- [ ] Message is sent successfully
- [ ] Returns None or success indicator
- [ ] No errors

**Acceptance Criteria:**
```python
hub = Hub(int, "numbers")
hub.send(42)  # Should succeed
```

### Scenario 5: Subscribe from Python
**Given:** Python Hub subscribed to topic
**When:** Message is available
**Then:**
- [ ] recv() returns the message
- [ ] Data type is correct
- [ ] recv() returns None when empty

**Acceptance Criteria:**
```python
pub_hub = Hub(int, "numbers")
sub_hub = Hub(int, "numbers")

pub_hub.send(100)
msg = sub_hub.recv()
assert msg == 100

# Empty case
msg = sub_hub.recv()
assert msg is None
```

### Scenario 6: Python to Rust Communication
**Given:** Python publisher, Rust subscriber
**When:** Python sends message
**Then:**
- [ ] Rust receives message
- [ ] Data integrity maintained
- [ ] Types match across languages

**Acceptance Criteria:**
```python
# Python:
hub = Hub(float, "cross_lang")
hub.send(3.14159)
```
```rust
// Rust:
let hub = Hub::<f64>::new("cross_lang")?;
let msg = hub.recv(None);
assert_eq!(msg, Some(3.14159));
```

### Scenario 7: Rust to Python Communication
**Given:** Rust publisher, Python subscriber
**When:** Rust sends message
**Then:**
- [ ] Python receives message
- [ ] Data integrity maintained
- [ ] Types match across languages

**Acceptance Criteria:**
```rust
// Rust:
let hub = Hub::<i32>::new("from_rust")?;
hub.send(999, None)?;
```
```python
# Python:
hub = Hub(int, "from_rust")
msg = hub.recv()
assert msg == 999
```

### Scenario 8: Complex Data Types
**Given:** Custom struct/dataclass shared between Rust and Python
**When:** Sending complex messages
**Then:**
- [ ] Serialization works
- [ ] Deserialization works
- [ ] All fields preserved

**Acceptance Criteria:**
```python
from dataclasses import dataclass

@dataclass
class CmdVel:
    linear: float
    angular: float

hub = Hub(CmdVel, "cmd_vel")
hub.send(CmdVel(linear=1.0, angular=0.5))

received = hub.recv()
assert received.linear == 1.0
assert received.angular == 0.5
```

## Python Node Implementation

### Scenario 9: Implement Node in Python
**Given:** User wants to create custom node
**When:** Inheriting from Node base class
**Then:**
- [ ] init() method works
- [ ] tick() method is called repeatedly
- [ ] shutdown() method works

**Acceptance Criteria:**
```python
from horus import Node

class SensorNode(Node):
    def __init__(self):
        self.hub = Hub(float, "sensor_data")
        self.counter = 0

    def init(self, ctx):
        print("SensorNode initialized")

    def tick(self, ctx):
        self.hub.send(self.counter * 0.1)
        self.counter += 1

    def shutdown(self, ctx):
        print("SensorNode shutdown")
```

### Scenario 10: Python Node in Scheduler
**Given:** Python Node implementation
**When:** Registered with Scheduler
**Then:**
- [ ] Node executes in scheduler loop
- [ ] Lifecycle methods are called
- [ ] Can coexist with Rust nodes

**Acceptance Criteria:**
```python
from horus import Scheduler, Node

class MyNode(Node):
    def tick(self, ctx):
        print("Ticking from Python!")

scheduler = Scheduler()
scheduler.register(MyNode(), priority=0)
scheduler.tick_all()  # Runs until Ctrl+C
```

### Scenario 11: Mixed Rust and Python Nodes
**Given:** Rust node publishes, Python node subscribes
**When:** Both run in same scheduler
**Then:**
- [ ] Communication works seamlessly
- [ ] Performance is maintained
- [ ] No type errors

## Error Handling

### Scenario 12: Hub Creation Failure
**Given:** Invalid topic name or permissions issue
**When:** Creating Hub in Python
**Then:**
- [ ] Raises Python exception
- [ ] Exception message is clear
- [ ] Includes error details

**Acceptance Criteria:**
```python
from horus import Hub

try:
    hub = Hub(int, "")  # Empty topic name
except ValueError as e:
    print(f"Error: {e}")
```

### Scenario 13: Type Mismatch
**Given:** Python sends wrong type
**When:** Serialization occurs
**Then:**
- [ ] Raises TypeError
- [ ] Error explains type mismatch
- [ ] No crash

**Acceptance Criteria:**
```python
hub = Hub(int, "numbers")
try:
    hub.send("not an int")  # Wrong type
except TypeError as e:
    print(f"Error: {e}")
```

### Scenario 14: Node Method Exceptions
**Given:** Python node raises exception in tick()
**When:** Scheduler executes
**Then:**
- [ ] Exception is caught
- [ ] Error logged
- [ ] Scheduler continues (or crashes, document behavior)

## Performance Tests

### Scenario 15: Python Hub Latency
**Given:** Python pub/sub on same topic
**When:** Measuring round-trip time
**Then:**
- [ ] Latency is < 10μs for small messages
- [ ] Comparable to Rust performance
- [ ] No significant overhead from PyO3

**Acceptance Criteria:**
```python
import time

hub_pub = Hub(int, "perf_test")
hub_sub = Hub(int, "perf_test")

start = time.perf_counter()
hub_pub.send(42)
msg = hub_sub.recv()
elapsed = time.perf_counter() - start

assert elapsed < 0.00001  # < 10μs
```

### Scenario 16: High Frequency Publishing
**Given:** Python node publishes at 1000 Hz
**When:** Running for extended period
**Then:**
- [ ] No memory leaks
- [ ] Stable performance
- [ ] No GIL contention issues

### Scenario 17: Large Message Performance
**Given:** Python sends 120KB message
**When:** Measuring latency
**Then:**
- [ ] Comparable to Rust performance
- [ ] No excessive copying
- [ ] Shared memory utilized

## Type Hints and IDE Support

### Scenario 18: Type Hints Available
**Given:** User has IDE with type checking
**When:** Writing Python code with horus
**Then:**
- [ ] Type hints are provided
- [ ] IDE autocomplete works
- [ ] Type errors caught before runtime

**Acceptance Criteria:**
```python
from horus import Hub

# IDE should autocomplete:
hub: Hub[int] = Hub(int, "topic")
hub.send(42)  # OK
hub.send("str")  # Type error caught by IDE
```

### Scenario 19: Docstrings Available
**Given:** User explores API
**When:** Calling help(Hub) in Python
**Then:**
- [ ] Docstrings are shown
- [ ] Parameters documented
- [ ] Return types documented
- [ ] Examples provided

**Acceptance Criteria:**
```python
help(Hub)
# Shows:
# class Hub:
#     """Lock-free pub/sub communication hub"""
#     def send(self, msg: T) -> None:
#         """Send a message to the topic"""
#     ...
```

## Integration Tests

### Scenario 20: Python Package in HORUS Project
**Given:** User creates project with `horus new -p`
**When:** Running Python project
**Then:**
- [ ] Python bindings work out of the box
- [ ] No additional setup needed
- [ ] Template code runs successfully

### Scenario 21: Debugging Python Nodes
**Given:** User sets breakpoint in Python tick()
**When:** Running under debugger
**Then:**
- [ ] Debugger stops at breakpoint
- [ ] Can inspect variables
- [ ] Scheduler continues after resume

### Scenario 22: Exception Traceback
**Given:** Python node raises exception
**When:** Error occurs
**Then:**
- [ ] Full Python traceback shown
- [ ] Line numbers point to user code
- [ ] Error message is actionable

## Documentation and Examples

### Scenario 23: Python Examples
**Given:** User reads documentation
**When:** Following Python examples
**Then:**
- [ ] All examples work as shown
- [ ] Code is copy-paste ready
- [ ] Covers common use cases

### Scenario 24: API Parity with Rust
**Given:** User knows Rust API
**When:** Using Python API
**Then:**
- [ ] Concepts map 1:1 where possible
- [ ] Naming is consistent
- [ ] Behavior is equivalent

## Cross-Platform Support

### Scenario 25: Linux Support
**Given:** Ubuntu 20.04+
**When:** Installing and using horus-py
**Then:**
- [ ] Installation works
- [ ] All features function
- [ ] No platform-specific issues

### Scenario 26: macOS Support
**Given:** macOS 11+
**When:** Installing and using horus-py
**Then:**
- [ ] Installation works
- [ ] POSIX shared memory available
- [ ] All features function

### Scenario 27: Windows Support (Future)
**Given:** Windows 10+
**When:** Installing horus-py
**Then:**
- [ ] Clear error or warning about Windows support
- [ ] Or it works if Windows support is added

## Non-Functional Requirements

- [ ] Python package size < 10MB
- [ ] Import time < 100ms
- [ ] Memory overhead < 5MB
- [ ] No GIL held during shared memory operations
- [ ] Compatible with Python 3.9, 3.10, 3.11, 3.12
- [ ] Works with pip, poetry, conda
- [ ] Wheels available for common platforms
- [ ] Source distribution available for custom builds
