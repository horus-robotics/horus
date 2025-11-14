# Async I/O Guide for HORUS Python

## Overview

This guide explains how to handle I/O operations (network requests, file access, camera reads, etc.) in HORUS nodes without blocking the scheduler. Blocking I/O operations can severely degrade system performance by preventing other nodes from executing.

## The Problem: Blocking I/O

When a node performs blocking I/O directly in its `tick()` method, it blocks the entire scheduler:

```python
class BlockingCameraNode(horus.Node):
    def tick(self):
        # BAD: This blocks the scheduler for 100ms!
        frame = self.camera.read()  # Takes 100ms
        self.send("camera_out", frame)
```

**Impact:**
- Scheduler cannot execute other nodes during I/O
- Tick rate drops dramatically (10 Hz instead of 100 Hz)
- Real-time performance degraded
- System responsiveness suffers

## The Solution: Threaded I/O (Phase 1)

Use Python's `ThreadPoolExecutor` to run I/O operations in background threads while the scheduler continues executing.

### Pattern 1: Basic Non-Blocking I/O

```python
import horus
from concurrent.futures import ThreadPoolExecutor, Future
from typing import Optional

class AsyncCameraNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=2)
        self.pending_future: Optional[Future] = None

    def _read_camera_blocking(self):
        """Runs in background thread"""
        return self.camera.read()  # Blocking call

    def tick(self):
        # Check if previous I/O completed
        if self.pending_future and self.pending_future.done():
            try:
                frame = self.pending_future.result(timeout=0)
                self.send("camera_out", frame)
            except Exception as e:
                print(f"Camera error: {e}")
            finally:
                self.pending_future = None

        # Submit new I/O if idle
        if not self.pending_future:
            self.pending_future = self.executor.submit(self._read_camera_blocking)
```

**Key Points:**
- I/O runs in background threads via `executor.submit()`
- `tick()` returns immediately (non-blocking)
- Results retrieved asynchronously with `future.result()`
- Scheduler continues executing other nodes during I/O

### Pattern 2: I/O with Timeout Handling

```python
class NetworkSensorNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=1)
        self.pending_future: Optional[Future] = None
        self.last_result = None  # Cache last good result

    def _fetch_data(self):
        """May timeout or fail"""
        response = requests.get("http://sensor.local/data", timeout=5.0)
        return response.json()

    def tick(self):
        # Check for completed request
        if self.pending_future and self.pending_future.done():
            try:
                result = self.pending_future.result(timeout=0)
                self.last_result = result
                self.send("sensor_out", result)
            except requests.Timeout:
                print("Network timeout, using cached data")
                if self.last_result:
                    self.send("sensor_out", self.last_result)
            except Exception as e:
                print(f"Request failed: {e}")
            finally:
                self.pending_future = None

        # Submit new request if idle
        if not self.pending_future:
            self.pending_future = self.executor.submit(self._fetch_data)
```

**Features:**
- Graceful timeout handling
- Fallback to cached data on failures
- Error logging without crashing

### Pattern 3: Batched File I/O

For high-frequency logging, batch writes to reduce I/O overhead:

```python
class LogWriterNode(horus.Node):
    def __init__(self, filename: str, batch_size: int = 10):
        super().__init__()
        self.filename = filename
        self.batch_size = batch_size
        self.executor = ThreadPoolExecutor(max_workers=1)
        self.buffer = []
        self.pending_future: Optional[Future] = None

    def _write_batch(self, batch):
        """Write entire batch in one I/O operation"""
        with open(self.filename, 'a') as f:
            for item in batch:
                f.write(f"{item}\n")

    def tick(self):
        # Receive incoming data (using Hub/Link)
        # ... receive logic ...

        # Write batch when buffer is full
        if len(self.buffer) >= self.batch_size and not self.pending_future:
            batch = self.buffer[:]
            self.buffer.clear()
            self.pending_future = self.executor.submit(self._write_batch, batch)

        # Check if write completed
        if self.pending_future and self.pending_future.done():
            self.pending_future = None
```

**Benefits:**
- Reduces I/O overhead by batching
- Improves throughput for high-frequency operations
- Non-blocking execution

### Pattern 4: Polling I/O at Lower Rate

For sensors that don't need high-frequency updates:

```python
class PeriodicSensorNode(horus.Node):
    def __init__(self, poll_rate_hz: float = 10.0):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=1)
        self.pending_future: Optional[Future] = None
        self.poll_interval = 1.0 / poll_rate_hz
        self.last_poll_time = 0

    def _read_sensor(self):
        """Blocking sensor read"""
        return self.sensor.read()

    def tick(self):
        current_time = time.time()

        # Check if previous read completed
        if self.pending_future and self.pending_future.done():
            try:
                data = self.pending_future.result(timeout=0)
                self.send("sensor_out", data)
            except Exception as e:
                print(f"Sensor error: {e}")
            finally:
                self.pending_future = None
                self.last_poll_time = current_time

        # Submit new read if enough time has passed
        if not self.pending_future:
            if (current_time - self.last_poll_time) >= self.poll_interval:
                self.pending_future = self.executor.submit(self._read_sensor)
```

**Use Cases:**
- Temperature sensors (poll every second)
- GPS updates (poll every 100ms)
- Any sensor with rate limits

## Best Practices

### 1. Choose Appropriate Worker Count

```python
# Light I/O (fast operations):
executor = ThreadPoolExecutor(max_workers=1)

# Medium I/O (multiple concurrent operations):
executor = ThreadPoolExecutor(max_workers=2-4)

# Heavy I/O (many concurrent operations):
executor = ThreadPoolExecutor(max_workers=8-16)
```

**Guidelines:**
- Start with `max_workers=2` for most use cases
- Increase if you need more concurrency
- Too many workers wastes memory and CPU
- Monitor thread count with `threading.active_count()`

### 2. Always Handle Exceptions

```python
def tick(self):
    if self.pending_future and self.pending_future.done():
        try:
            result = self.pending_future.result(timeout=0)
            # Process result
        except Exception as e:
            # ALWAYS catch and log exceptions
            print(f"I/O error: {e}")
            # Optionally: use cached data, retry, etc.
        finally:
            # ALWAYS clear the future
            self.pending_future = None
```

**Why:**
- Exceptions in background threads can be silent
- Unhandled exceptions leave futures pending
- Circuit breaker will trip on repeated failures

### 3. Track Pending Operations

```python
def tick(self):
    # BAD: Submits new operation even if one is pending
    self.executor.submit(self._blocking_io)

    # GOOD: Only submit if no pending operation
    if not self.pending_future:
        self.pending_future = self.executor.submit(self._blocking_io)
```

**Why:**
- Prevents queue buildup
- Avoids memory exhaustion
- Maintains predictable behavior

### 4. Use Appropriate Timeouts

```python
# For network requests:
requests.get(url, timeout=5.0)  # 5 second timeout

# For database queries:
cursor.execute(query, timeout=10.0)

# For file I/O:
with open(file, 'r') as f:
    signal.alarm(5)  # OS-level timeout
    data = f.read()
    signal.alarm(0)
```

**Why:**
- Prevents indefinite blocking
- Detects network/hardware failures
- Enables graceful degradation

### 5. Combine with Deadlines and Watchdogs

```python
# Set deadline for sensor node
scheduler.set_node_deadline("sensor_node", 50.0)  # 50ms deadline

# Enable watchdog for critical node
scheduler.set_node_watchdog("camera_node", True, 1000)  # 1s timeout

# Deadline monitoring + threaded I/O = robust system
```

**Benefits:**
- Deadline monitoring catches slow ticks
- Watchdog detects complete failures
- Threaded I/O prevents blocking
- Triple layer of protection

## Common Pitfalls

### ❌ Pitfall 1: Blocking in Constructor

```python
class BadNode(horus.Node):
    def __init__(self):
        super().__init__()
        # BAD: Blocks during node creation!
        self.data = requests.get("http://api.local/init").json()
```

**Fix:**
```python
class GoodNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=1)
        self.data = None
        self.init_future = self.executor.submit(self._init_data)

    def _init_data(self):
        return requests.get("http://api.local/init").json()

    def tick(self):
        # Wait for initialization
        if self.init_future and self.init_future.done():
            self.data = self.init_future.result()
            self.init_future = None
```

### ❌ Pitfall 2: Forgetting to Clear Futures

```python
def tick(self):
    if self.pending_future and self.pending_future.done():
        result = self.pending_future.result()
        # BAD: Future still set, won't submit new operations!
        # self.pending_future = None  # FORGOT THIS!
```

**Impact:**
- No new I/O operations submitted
- Node appears to "hang"
- Data stops flowing

### ❌ Pitfall 3: Using blocking calls in tick()

```python
def tick(self):
    # BAD: Still blocking!
    future = self.executor.submit(self._io)
    result = future.result()  # This blocks!
    self.send("out", result)
```

**Fix:**
```python
def tick(self):
    # GOOD: Non-blocking
    if not self.pending_future:
        self.pending_future = self.executor.submit(self._io)

    if self.pending_future and self.pending_future.done():
        result = self.pending_future.result(timeout=0)
        self.send("out", result)
        self.pending_future = None
```

## Performance Comparison

### Scenario: 100 Hz scheduler with 50ms camera read

**Blocking I/O:**
- Camera read: 50ms
- Actual tick rate: 20 Hz (1000ms / 50ms)
- Other nodes blocked: 50ms per tick
- System throughput: **Poor**

**Threaded I/O:**
- Camera read: 50ms (in background)
- Actual tick rate: 100 Hz (unaffected)
- Other nodes blocked: 0ms
- System throughput: **Excellent**

### Benchmark Results

```
Blocking I/O Node:
  Expected ticks (1s @ 100 Hz): 100
  Actual ticks: 20
  Efficiency: 20%

Threaded I/O Node:
  Expected ticks (1s @ 100 Hz): 100
  Actual ticks: 98
  Efficiency: 98%

Speedup: 4.9x
```

## Thread Safety Considerations

### Hub/Link Communication

HORUS Hub and Link are designed to be thread-safe:

```python
class ThreadedNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=2)

    def _background_work(self):
        # Safe: send() is thread-safe
        result = do_io_work()
        self.send("output", result)  # [OK] OK

    def tick(self):
        self.executor.submit(self._background_work)
```

**Note:** While `send()` is thread-safe, it's recommended to send from the main tick() method for consistency:

```python
def _background_work(self):
    return do_io_work()  # Just return data

def tick(self):
    if self.future and self.future.done():
        result = self.future.result()
        self.send("output", result)  # Send from main thread
```

### Node State

Be careful with shared state between threads:

```python
class SharedStateNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=2)
        self.lock = threading.Lock()
        self.counter = 0  # Shared state

    def _increment_counter(self):
        with self.lock:  # Protect shared state
            self.counter += 1

    def tick(self):
        with self.lock:
            current = self.counter
        # Use current safely
```

## Future: Phase 2 and 3

This guide covers **Phase 1: Threaded I/O**, which is the recommended approach for most use cases.

### Phase 2: Rust-Managed Async Nodes (Future)

- Separate AsyncScheduler running in Tokio runtime
- True async/await in Rust
- Better performance for I/O-heavy workloads
- More complex implementation

### Phase 3: Python Asyncio Bridge (Future)

- Native Python `async`/`await` syntax
- Direct integration with asyncio ecosystem
- Most developer-friendly
- Requires experimental pyo3-asyncio

See `ASYNC_IO_DESIGN.md` for detailed design documentation.

## Examples

See the following files for complete examples:

- `demo_async_io_threaded.py` - Comprehensive demo with 5 patterns
- `test_async_io_simple.py` - Simple test showing basic pattern
- `test_async_io_threaded.py` - Multi-node test

## Summary

**Use threaded I/O when:**
- [OK] Node performs blocking I/O (network, file, camera, database)
- [OK] I/O operation takes >10ms
- [OK] System requires high tick rates (>50 Hz)
- [OK] You want simple, stable solution

**Pattern:**
1. Create `ThreadPoolExecutor` in `__init__()`
2. Submit I/O work to executor (non-blocking)
3. Check if future is done in `tick()`
4. Retrieve result with `future.result(timeout=0)`
5. Always handle exceptions
6. Clear future after processing

**Benefits:**
- Non-blocking I/O
- High scheduler throughput
- Simple to implement and test
- Works with existing Node API
- No complex async/await integration

**Combine with:**
- Deadline monitoring (detect slow ticks)
- Watchdog timers (detect failures)
- Circuit breaker (handle repeated failures)
- Node introspection (monitor health)

This creates a robust, high-performance system that handles I/O gracefully while maintaining real-time responsiveness.
