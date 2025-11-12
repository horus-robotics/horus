#!/usr/bin/env python3
"""
Demo: Async I/O with Threading (Phase 1)
Shows how to handle I/O operations in nodes using threading
"""
import horus
import time
import threading
from concurrent.futures import ThreadPoolExecutor, Future
from typing import Optional
import random

print("=" * 70)
print("HORUS Async I/O Demo - Threaded Approach (Phase 1)")
print("=" * 70)

# ============================================================================
# Pattern 1: Blocking I/O Node (Simple but blocks scheduler)
# ============================================================================
class BlockingCameraNode(horus.Node):
    """
    Simple camera node that blocks during read.

    PROBLEM: The tick() method blocks the entire scheduler while waiting
    for the camera. This prevents other nodes from executing.
    """
    def __init__(self):
        super().__init__()
        self.frame_count = 0

    def tick(self):
        # Simulate blocking camera read (100ms)
        time.sleep(0.1)

        self.frame_count += 1
        frame = {"id": self.frame_count, "timestamp": time.time()}
        self.send("camera_out", frame)


# ============================================================================
# Pattern 2: Non-Blocking I/O Node with ThreadPoolExecutor (RECOMMENDED)
# ============================================================================
class AsyncCameraNode(horus.Node):
    """
    Non-blocking camera node using ThreadPoolExecutor.

    SOLUTION: Submits I/O work to a thread pool and returns immediately.
    The scheduler can continue executing other nodes while I/O happens
    in background threads.
    """
    def __init__(self):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=2)
        self.frame_count = 0
        self.pending_future: Optional[Future] = None

    def _read_camera_blocking(self):
        """This runs in a background thread"""
        # Simulate blocking camera read
        time.sleep(0.1)
        self.frame_count += 1
        return {"id": self.frame_count, "timestamp": time.time()}

    def tick(self):
        # Check if previous read completed
        if self.pending_future and self.pending_future.done():
            try:
                frame = self.pending_future.result(timeout=0)
                self.send("camera_out", frame)
                self.pending_future = None
            except Exception as e:
                print(f"Camera read failed: {e}")
                self.pending_future = None

        # Submit new read if no pending operation
        if not self.pending_future:
            self.pending_future = self.executor.submit(self._read_camera_blocking)


# ============================================================================
# Pattern 3: Network I/O Node with Timeout
# ============================================================================
class NetworkSensorNode(horus.Node):
    """
    Network sensor that fetches data with timeout handling.

    Shows how to handle timeouts and failures in I/O operations.
    """
    def __init__(self):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=1)
        self.request_count = 0
        self.pending_future: Optional[Future] = None
        self.last_result = None

    def _fetch_data(self):
        """Simulates network request"""
        # Simulate variable network delay
        delay = random.uniform(0.05, 0.15)
        time.sleep(delay)

        # Simulate occasional failures
        if random.random() < 0.1:
            raise ConnectionError("Network timeout")

        self.request_count += 1
        return {
            "sensor_id": "temp_01",
            "value": 20.0 + random.uniform(-2, 2),
            "request_count": self.request_count
        }

    def tick(self):
        # Check for completed request
        if self.pending_future:
            if self.pending_future.done():
                try:
                    result = self.pending_future.result(timeout=0)
                    self.last_result = result
                    self.send("sensor_out", result)
                except Exception as e:
                    print(f"Network error: {e}")
                    # Send last known good result
                    if self.last_result:
                        self.send("sensor_out", self.last_result)
                finally:
                    self.pending_future = None

        # Submit new request if idle
        if not self.pending_future:
            self.pending_future = self.executor.submit(self._fetch_data)


# ============================================================================
# Pattern 4: File I/O Node with Batching
# ============================================================================
class LogWriterNode(horus.Node):
    """
    Logs data to file using background thread with batching.

    Shows how to batch I/O operations for better performance.
    """
    def __init__(self, filename: str, batch_size: int = 10):
        super().__init__()
        self.filename = filename
        self.batch_size = batch_size
        self.executor = ThreadPoolExecutor(max_workers=1)
        self.buffer = []
        self.pending_future: Optional[Future] = None

        self.link = self.create_link("log_in")

    def _write_batch(self, batch):
        """Write batch to file in background thread"""
        with open(self.filename, 'a') as f:
            for item in batch:
                f.write(f"{item}\n")

    def tick(self):
        # Check if previous write completed
        if self.pending_future and self.pending_future.done():
            self.pending_future = None

        # Receive incoming data
        if self.link.has_message():
            data = self.link.receive()
            self.buffer.append(f"{time.time()}: {data}")

        # Write batch if buffer is full and no pending write
        if len(self.buffer) >= self.batch_size and not self.pending_future:
            batch_to_write = self.buffer[:]
            self.buffer.clear()
            self.pending_future = self.executor.submit(self._write_batch, batch_to_write)


# ============================================================================
# Pattern 5: Processing Node (Fast, No I/O)
# ============================================================================
class ProcessorNode(horus.Node):
    """
    Fast processing node that doesn't do I/O.

    This node should execute quickly and not be blocked by I/O nodes.
    """
    def __init__(self):
        super().__init__()
        self.camera_link = self.create_link("camera_in")
        self.sensor_link = self.create_link("sensor_in")
        self.processed_frames = 0
        self.processed_sensors = 0

    def tick(self):
        # Process camera frames
        if self.camera_link.has_message():
            frame = self.camera_link.receive()
            # Fast processing
            self.processed_frames += 1

        # Process sensor data
        if self.sensor_link.has_message():
            data = self.sensor_link.receive()
            # Fast processing
            self.processed_sensors += 1


# ============================================================================
# Demo Setup and Execution
# ============================================================================

print("\nCreating scheduler with high tick rate...")
config = horus.SchedulerConfig.standard()
config.tick_rate = 100.0  # 100 Hz - should not be blocked by I/O
scheduler = horus.Scheduler.from_config(config)

print("\nCreating nodes...")

# Create async I/O nodes
camera = AsyncCameraNode()
sensor = NetworkSensorNode()
log_writer = LogWriterNode("/tmp/horus_async_demo.log", batch_size=5)
processor = ProcessorNode()

# Add all nodes
scheduler.add(camera, priority=1)
scheduler.add(sensor, priority=2)
scheduler.add(processor, priority=3)
scheduler.add(log_writer, priority=4)

# Get node names
names = scheduler.get_node_names()
print(f"\nRegistered nodes: {names}")

# Connect nodes
print("\nConnecting nodes...")
scheduler.connect(names[0], "camera_out", names[2], "camera_in")
scheduler.connect(names[1], "sensor_out", names[2], "sensor_in")
scheduler.connect(names[1], "sensor_out", names[3], "log_in")

print("\n" + "=" * 70)
print("Running scheduler for 2 seconds...")
print("Note: I/O operations happen in background threads")
print("=" * 70)
print()

# Run scheduler
def stop_after(seconds):
    time.sleep(seconds)
    scheduler.stop()

stop_thread = threading.Thread(target=lambda: stop_after(2), daemon=True)
stop_thread.start()

start_time = time.time()
try:
    scheduler.run()
except Exception as e:
    print(f"Scheduler error: {e}")

duration = time.time() - start_time

# Analyze results
print()
print("=" * 70)
print("Results Analysis:")
print("=" * 70)

nodes = scheduler.get_all_nodes()
total_ticks = sum(n.get('total_ticks', 0) for n in nodes)
expected_ticks = duration * config.tick_rate

print(f"\nScheduler Performance:")
print(f"  Duration: {duration:.2f}s")
print(f"  Tick rate: {config.tick_rate} Hz")
print(f"  Expected ticks: {expected_ticks:.0f}")
print(f"  Actual ticks: {total_ticks}")
print(f"  Efficiency: {(total_ticks / expected_ticks * 100):.1f}%")

for node_info in nodes:
    print(f"\n{node_info['name']}:")
    print(f"  Total ticks: {node_info.get('total_ticks', 0)}")
    print(f"  Total failures: {node_info.get('total_failures', 0)}")

print()
print("=" * 70)
print("Key Benefits of Threaded I/O:")
print("=" * 70)
print("""
1. Non-Blocking: I/O operations don't block the scheduler
2. High Throughput: Fast nodes can execute at high rates
3. Simple: Uses standard Python threading, no async/await
4. Resilient: Failures in I/O don't crash the scheduler
5. Flexible: Different I/O patterns (polling, batching, timeout)
""")

print("=" * 70)
print("Demo complete!")
print("=" * 70)
print("\nCheck /tmp/horus_async_demo.log for logged data")
