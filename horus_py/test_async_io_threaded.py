#!/usr/bin/env python3
"""
Simple test for threaded async I/O
"""
import horus
import time
import threading
from concurrent.futures import ThreadPoolExecutor, Future
from typing import Optional

print("=" * 70)
print("Threaded Async I/O Test")
print("=" * 70)

# Non-blocking I/O node using ThreadPoolExecutor
class AsyncIONode(horus.Node):
    def __init__(self):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=2)
        self.operation_count = 0
        self.pending_future: Optional[Future] = None

    def _blocking_io(self):
        """Simulates blocking I/O operation (runs in background thread)"""
        time.sleep(0.05)  # 50ms I/O operation
        self.operation_count += 1
        return {"result": self.operation_count, "timestamp": time.time()}

    def tick(self):
        # Check if previous I/O completed
        if self.pending_future and self.pending_future.done():
            try:
                result = self.pending_future.result(timeout=0)
                self.send("io_out", result)
                self.pending_future = None
            except Exception as e:
                print(f"I/O failed: {e}")
                self.pending_future = None

        # Submit new I/O if no pending operation
        if not self.pending_future:
            self.pending_future = self.executor.submit(self._blocking_io)


# Fast processing node (should not be blocked)
class FastNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        # Fast processing - just counting ticks


# Create scheduler
config = horus.SchedulerConfig.standard()
config.tick_rate = 100.0  # 100 Hz - high rate
scheduler = horus.Scheduler.from_config(config)

print(f"\nConfiguration:")
print(f"  Tick Rate: {config.tick_rate} Hz")

# Create nodes
io_node = AsyncIONode()
fast_node = FastNode()

scheduler.add(io_node, priority=1)
scheduler.add(fast_node, priority=2)

names = scheduler.get_node_names()
print(f"\nNodes: {names}")

print("\n" + "=" * 70)
print("Running for 1 second with 100 Hz tick rate...")
print("Fast node should NOT be blocked by I/O node")
print("=" * 70)
print()

# Start scheduler in background
import signal

start_time = time.time()
scheduler.start()
time.sleep(1.0)
duration = time.time() - start_time

print("Stopping...")

# Check results
print()
print("=" * 70)
print("Results:")
print("=" * 70)

nodes = scheduler.get_all_nodes()
for node_info in nodes:
    print(f"\n{node_info['name']}:")
    print(f"  Ticks: {node_info.get('total_ticks', 0)}")

io_ticks = nodes[0].get('total_ticks', 0)
fast_ticks = nodes[1].get('total_ticks', 0)
expected_ticks = duration * config.tick_rate

print(f"\nAnalysis:")
print(f"  Duration: {duration:.2f}s")
print(f"  Expected ticks: {expected_ticks:.0f}")
print(f"  I/O node ticks: {io_ticks}")
print(f"  Fast node ticks: {fast_ticks}")
print(f"  Fast node efficiency: {(fast_ticks / expected_ticks * 100):.1f}%")

# Verify fast node was not blocked
if fast_ticks >= expected_ticks * 0.8:  # Allow 20% overhead
    print("\n✓ SUCCESS: Fast node was NOT blocked by I/O operations")
else:
    print(f"\n✗ FAILURE: Fast node was blocked (only {fast_ticks} ticks)")

print()
print("=" * 70)
print("Test complete!")
print("=" * 70)
