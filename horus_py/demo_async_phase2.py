#!/usr/bin/env python3
"""
Demo: Async I/O Phase 2 - Async Helpers

Shows how to use the async helper utilities for advanced I/O patterns.
"""

import horus
import time
import random
from horus_async_helpers import (
    AsyncHelper,
    ConnectionPool,
    BatchProcessor,
    RateLimiter,
    AsyncAggregator
)

print("=" * 70)
print("HORUS Async I/O Phase 2 Demo - Async Helpers")
print("=" * 70)

# ============================================================================
# Example 1: AsyncHelper for General Async Operations
# ============================================================================

class SensorNode(horus.Node):
    """Node using AsyncHelper for async sensor reads"""

    def __init__(self):
        super().__init__()
        self.async_helper = AsyncHelper(max_workers=4)
        self.operation_counter = 0

    def _read_sensor(self, sensor_id):
        """Simulated sensor read (50ms)"""
        time.sleep(0.05)
        return {
            'sensor_id': sensor_id,
            'value': 20.0 + random.uniform(-2, 2),
            'timestamp': time.time()
        }

    def tick(self):
        # Submit new operation if no pending operations
        if self.async_helper.pending_count() < 2:
            op_id = f"sensor_read_{self.operation_counter}"
            self.async_helper.submit(op_id, self._read_sensor, self.operation_counter % 3)
            self.operation_counter += 1

        # Check completed operations
        for op_id, result, error in self.async_helper.check_completed():
            if error:
                print(f"  Sensor error: {error}")
            else:
                self.send("sensor_out", result)

# ============================================================================
# Example 2: ConnectionPool for Database/API Connections
# ============================================================================

class MockConnection:
    """Mock database connection"""
    def __init__(self):
        self.queries = 0

    def execute(self, query):
        """Simulate query execution"""
        time.sleep(0.02)
        self.queries += 1
        return f"Result for: {query}"

def create_connection():
    return MockConnection()

class DatabaseNode(horus.Node):
    """Node using ConnectionPool for database queries"""

    def __init__(self, pool):
        super().__init__()
        self.pool = pool
        self.pending_query = None
        self.query_count = 0

    def tick(self):
        # Submit new query if none pending
        if not self.pending_query:
            query = f"SELECT * FROM sensors WHERE id = {self.query_count % 10}"
            self.pending_query = self.pool.execute_async(
                lambda conn, q: conn.execute(q),
                query
            )
            self.query_count += 1

        # Check if query completed
        if self.pending_query and self.pending_query.done():
            try:
                result = self.pending_query.result(timeout=0)
                self.send("db_out", result)
            except Exception as e:
                print(f"  Query error: {e}")
            finally:
                self.pending_query = None

# ============================================================================
# Example 3: BatchProcessor for Log Writing
# ============================================================================

def write_logs(batch):
    """Process a batch of log entries"""
    time.sleep(0.01)  # Simulate I/O
    return len(batch)

class LoggerNode(horus.Node):
    """Node using BatchProcessor for efficient logging"""

    def __init__(self, processor):
        super().__init__()
        self.processor = processor
        self.log_count = 0

    def tick(self):
        # Add log entry
        self.processor.add(f"Log entry {self.log_count}: {time.time()}")
        self.log_count += 1

        # Auto-flush if needed
        self.processor.tick()

        # Check if flush completed
        result = self.processor.check_completed()
        if result:
            print(f"  Flushed batch of {result} log entries")

# ============================================================================
# Example 4: RateLimiter for API Throttling
# ============================================================================

class APINode(horus.Node):
    """Node using RateLimiter for API rate limiting"""

    def __init__(self, limiter):
        super().__init__()
        self.limiter = limiter
        self.api_calls = 0
        self.skipped_calls = 0

    def tick(self):
        if self.limiter.allow():
            # Make API call
            self.api_calls += 1
            self.send("api_out", {'call': self.api_calls})
        else:
            # Rate limit exceeded
            self.skipped_calls += 1

# ============================================================================
# Example 5: AsyncAggregator for Multi-Sensor Fusion
# ============================================================================

class MultiSensorNode(horus.Node):
    """Node using AsyncAggregator to wait for multiple sensors"""

    def __init__(self, aggregator, async_helper):
        super().__init__()
        self.aggregator = aggregator
        self.async_helper = async_helper
        self.fusion_count = 0

    def _read_temp(self):
        time.sleep(0.03)
        return 25.0 + random.uniform(-1, 1)

    def _read_pressure(self):
        time.sleep(0.04)
        return 1013.0 + random.uniform(-5, 5)

    def _read_humidity(self):
        time.sleep(0.02)
        return 50.0 + random.uniform(-10, 10)

    def tick(self):
        # Submit all sensors if no pending operations
        if not self.aggregator.has_pending():
            self.aggregator.add("temp", self.async_helper.submit("temp", self._read_temp))
            self.aggregator.add("pressure", self.async_helper.submit("pressure", self._read_pressure))
            self.aggregator.add("humidity", self.async_helper.submit("humidity", self._read_humidity))

        # Check if all completed
        if self.aggregator.all_completed():
            results = self.aggregator.get_completed()
            self.send("fusion_out", results)
            self.fusion_count += 1
            self.aggregator.clear()

# ============================================================================
# Setup and Run Demo
# ============================================================================

print("\nSetting up scheduler and nodes...")

# Create scheduler
config = horus.SchedulerConfig.standard()
config.tick_rate = 50.0  # 50 Hz
scheduler = horus.Scheduler.from_config(config)

# Create shared resources
connection_pool = ConnectionPool(create_connection, max_connections=5)
batch_processor = BatchProcessor(batch_size=10, flush_interval=0.5, process_batch=write_logs)
rate_limiter = RateLimiter(max_rate=20.0)  # 20 ops/second
aggregator = AsyncAggregator()
async_helper_shared = AsyncHelper(max_workers=8)

# Create nodes
sensor_node = SensorNode()
db_node = DatabaseNode(connection_pool)
logger_node = LoggerNode(batch_processor)
api_node = APINode(rate_limiter)
multi_sensor_node = MultiSensorNode(aggregator, async_helper_shared)

# Add nodes
scheduler.add(sensor_node, priority=1)
scheduler.add(db_node, priority=2)
scheduler.add(logger_node, priority=3)
scheduler.add(api_node, priority=4)
scheduler.add(multi_sensor_node, priority=5)

print("\nNodes registered:")
for name in scheduler.get_node_names():
    print(f"  - {name}")

print("\n" + "=" * 70)
print("Running for 2 seconds...")
print("=" * 70)
print()

# Run briefly
import signal
def timeout_handler(signum, frame):
    raise KeyboardInterrupt()

signal.signal(signal.SIGALRM, timeout_handler)
signal.alarm(2)

try:
    scheduler.run()
except KeyboardInterrupt:
    pass

print()
print("=" * 70)
print("Results:")
print("=" * 70)

# Get node stats
nodes = scheduler.get_all_nodes()
for node_info in nodes:
    print(f"\n{node_info['name']}:")
    print(f"  Total ticks: {node_info.get('total_ticks', 0)}")

# Get helper stats
print(f"\nSensor Node (AsyncHelper):")
stats = sensor_node.async_helper.get_stats()
print(f"  Pending: {stats['pending']}")
print(f"  Completed: {stats['completed']}")
print(f"  Failed: {stats['failed']}")

print(f"\nDatabase Node (ConnectionPool):")
stats = connection_pool.get_stats()
print(f"  Total operations: {stats['total_operations']}")
print(f"  Failed operations: {stats['failed_operations']}")
print(f"  Available connections: {stats['available_connections']}/{stats['max_connections']}")

print(f"\nLogger Node (BatchProcessor):")
stats = batch_processor.get_stats()
print(f"  Total batches: {stats['total_batches']}")
print(f"  Total items: {stats['total_items']}")
print(f"  Buffer size: {stats['buffer_size']}")

print(f"\nAPI Node (RateLimiter):")
stats = rate_limiter.get_stats()
print(f"  Allowed: {stats['total_allowed']}")
print(f"  Rejected: {stats['total_rejected']}")
print(f"  Max rate: {stats['max_rate']} ops/sec")

print(f"\nMulti-Sensor Node (AsyncAggregator):")
print(f"  Fusions completed: {multi_sensor_node.fusion_count}")
print(f"  Pending operations: {aggregator.count_pending()}")

# Cleanup
print("\n" + "=" * 70)
print("Cleaning up...")
sensor_node.async_helper.shutdown()
connection_pool.shutdown()
batch_processor.shutdown()
async_helper_shared.shutdown()

print("\n" + "=" * 70)
print("Demo complete!")
print("=" * 70)

print("\nPhase 2 Helpers Demonstrated:")
print("  ✓ AsyncHelper - General async operation tracking")
print("  ✓ ConnectionPool - Connection pooling for databases/APIs")
print("  ✓ BatchProcessor - Batching for efficient I/O")
print("  ✓ RateLimiter - Rate throttling for APIs")
print("  ✓ AsyncAggregator - Multi-operation aggregation")

print("\nKey Benefits:")
print("  • Higher-level abstractions than Phase 1")
print("  • Reusable components across nodes")
print("  • Built-in statistics and monitoring")
print("  • Clean separation of concerns")
print("  • Production-ready patterns")
