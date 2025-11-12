#!/usr/bin/env python3
"""
Simple test for threaded async I/O - demonstrates non-blocking pattern
"""
import horus
import time
from concurrent.futures import ThreadPoolExecutor, Future
from typing import Optional

print("=" * 70)
print("Threaded Async I/O - Simple Test")
print("=" * 70)

# Node with blocking I/O in background thread
class AsyncIONode(horus.Node):
    def __init__(self, scheduler_ref):
        super().__init__()
        self.scheduler = scheduler_ref
        self.executor = ThreadPoolExecutor(max_workers=2)
        self.operation_count = 0
        self.completed_operations = 0
        self.pending_future: Optional[Future] = None
        self.tick_count = 0
        self.max_ticks = 100  # Stop after 100 ticks

    def _blocking_io(self):
        """Simulates blocking I/O (50ms)"""
        time.sleep(0.05)
        self.operation_count += 1
        return {"result": self.operation_count}

    def tick(self):
        self.tick_count += 1

        # Check if we should stop
        if self.tick_count >= self.max_ticks:
            self.scheduler.stop()
            return

        # Check if previous I/O completed
        if self.pending_future and self.pending_future.done():
            try:
                result = self.pending_future.result(timeout=0)
                self.completed_operations += 1
                self.send("io_out", result)
            except Exception as e:
                print(f"I/O failed: {e}")
            finally:
                self.pending_future = None

        # Submit new I/O if no pending operation
        if not self.pending_future:
            self.pending_future = self.executor.submit(self._blocking_io)


# Create scheduler
config = horus.SchedulerConfig.standard()
config.tick_rate = 100.0  # 100 Hz
scheduler = horus.Scheduler.from_config(config)

print(f"\nConfiguration: {config.tick_rate} Hz tick rate")

# Create node (pass scheduler reference for stopping)
io_node = AsyncIONode(scheduler)
scheduler.add(io_node, priority=1)

names = scheduler.get_node_names()
print(f"Node: {names[0]}")

print(f"\nRunning for 100 ticks (should take ~1 second at 100 Hz)...")

start_time = time.time()
scheduler.run()
duration = time.time() - start_time

# Check results
print("\n" + "=" * 70)
print("Results:")
print("=" * 70)

nodes = scheduler.get_all_nodes()
node_info = nodes[0]

ticks = node_info.get('total_ticks', 0)
print(f"\nTotal ticks: {ticks}")
print(f"Duration: {duration:.2f}s")
print(f"Completed I/O operations: {io_node.completed_operations}")
print(f"Pending operations: {io_node.operation_count - io_node.completed_operations}")

# Calculate expected behavior
expected_duration = ticks / config.tick_rate
print(f"\nExpected duration: {expected_duration:.2f}s")
print(f"Actual duration: {duration:.2f}s")

# With 50ms I/O blocking, we should complete ~20 operations per second
# But since it's in a background thread, the scheduler should NOT be blocked
if duration < 1.5:  # Should complete in ~1 second, allow some overhead
    print("\n✓ SUCCESS: Scheduler was NOT blocked by I/O operations")
    print(f"  I/O happens in background threads while scheduler runs at {config.tick_rate} Hz")
else:
    print(f"\n✗ FAILURE: Scheduler appears to have been blocked")

print("\n" + "=" * 70)
print("Key Point:")
print("=" * 70)
print("""
The node uses ThreadPoolExecutor to run blocking I/O in background threads.
This allows the scheduler to continue executing at high rates (100 Hz)
without being blocked by slow I/O operations (50ms each).

Pattern:
1. Submit I/O work to thread pool (non-blocking)
2. Continue ticking while I/O happens in background
3. Check if I/O completed and retrieve results
4. Submit next I/O operation

This is Phase 1 of the async I/O implementation - simple threading
that works with the existing synchronous Node API.
""")

print("=" * 70)
print("Test complete!")
print("=" * 70)
