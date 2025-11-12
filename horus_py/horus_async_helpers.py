"""
HORUS Async I/O Helpers - Phase 2

Provides reusable async utilities and patterns for HORUS nodes.
"""

import time
import queue
import threading
from concurrent.futures import ThreadPoolExecutor, Future
from typing import Optional, Callable, Any, Dict, List, Tuple
from dataclasses import dataclass


@dataclass
class AsyncResult:
    """Result from an async operation"""
    operation_id: str
    success: bool
    result: Any = None
    error: Exception = None
    duration: float = 0.0


class AsyncHelper:
    """
    Helper class for async I/O operations in HORUS nodes.

    Provides common patterns and utilities for non-blocking I/O with tracking and error handling.

    Example:
        class SensorNode(horus.Node):
            def __init__(self):
                super().__init__()
                self.async_helper = AsyncHelper(max_workers=4)

            def tick(self):
                # Submit async operation
                self.async_helper.submit("read_sensor", self._read_sensor)

                # Check completed operations
                for op_id, result, error in self.async_helper.check_completed():
                    if error:
                        print(f"Operation {op_id} failed: {error}")
                    else:
                        self.send("sensor_out", result)
    """

    def __init__(self, max_workers: int = 4):
        """
        Initialize async helper.

        Args:
            max_workers: Maximum number of worker threads
        """
        self.executor = ThreadPoolExecutor(max_workers=max_workers)
        self.futures: Dict[str, Tuple[Future, float]] = {}  # operation_id -> (future, start_time)
        self.completed_count = 0
        self.failed_count = 0

    def submit(self, operation_id: str, func: Callable, *args, **kwargs) -> Future:
        """
        Submit async operation with tracking.

        Args:
            operation_id: Unique identifier for this operation
            func: Function to execute asynchronously
            *args: Positional arguments for func
            **kwargs: Keyword arguments for func

        Returns:
            Future object for this operation
        """
        future = self.executor.submit(func, *args, **kwargs)
        self.futures[operation_id] = (future, time.time())
        return future

    def check_completed(self) -> List[Tuple[str, Any, Optional[Exception]]]:
        """
        Check all completed operations and return results.

        Returns:
            List of tuples: (operation_id, result, error)
        """
        completed = []
        for op_id, (future, start_time) in list(self.futures.items()):
            if future.done():
                duration = time.time() - start_time
                try:
                    result = future.result(timeout=0)
                    completed.append((op_id, result, None))
                    self.completed_count += 1
                except Exception as e:
                    completed.append((op_id, None, e))
                    self.failed_count += 1
                del self.futures[op_id]
        return completed

    def is_pending(self, operation_id: str) -> bool:
        """Check if an operation is still pending"""
        return operation_id in self.futures

    def pending_count(self) -> int:
        """Get count of pending operations"""
        return len(self.futures)

    def get_stats(self) -> Dict[str, int]:
        """Get statistics"""
        return {
            'pending': len(self.futures),
            'completed': self.completed_count,
            'failed': self.failed_count,
        }

    def shutdown(self, wait: bool = True):
        """
        Gracefully shutdown the helper.

        Args:
            wait: Whether to wait for pending operations
        """
        self.executor.shutdown(wait=wait)


class ConnectionPool:
    """
    Async-friendly connection pool for databases, APIs, sensors, etc.

    Manages a pool of reusable connections and executes operations without blocking.

    Example:
        def create_db_connection():
            import psycopg2
            return psycopg2.connect(...)

        pool = ConnectionPool(create_db_connection, max_connections=10)

        class DatabaseNode(horus.Node):
            def __init__(self, pool):
                super().__init__()
                self.pool = pool
                self.pending_query = None

            def tick(self):
                if not self.pending_query:
                    self.pending_query = self.pool.execute_async(
                        lambda conn: conn.cursor().execute("SELECT * FROM sensors")
                    )

                if self.pending_query and self.pending_query.done():
                    result = self.pending_query.result()
                    self.send("db_out", result)
                    self.pending_query = None
    """

    def __init__(self, create_connection: Callable, max_connections: int = 10):
        """
        Initialize connection pool.

        Args:
            create_connection: Function that creates a new connection
            max_connections: Maximum number of connections in the pool
        """
        self.create_connection = create_connection
        self.max_connections = max_connections
        self.pool = queue.Queue(maxsize=max_connections)
        self.executor = ThreadPoolExecutor(max_workers=max_connections)
        self.total_operations = 0
        self.failed_operations = 0
        self._initialize_pool()

    def _initialize_pool(self):
        """Pre-create connections"""
        for _ in range(self.max_connections):
            try:
                conn = self.create_connection()
                self.pool.put(conn)
            except Exception as e:
                print(f"Failed to create connection: {e}")

    def execute_async(self, operation: Callable, *args, **kwargs) -> Future:
        """
        Execute operation with pooled connection.

        Args:
            operation: Function that takes connection as first argument
            *args: Additional arguments for operation
            **kwargs: Keyword arguments for operation

        Returns:
            Future for the operation result
        """
        def _execute():
            conn = None
            try:
                conn = self.pool.get(timeout=5.0)
                result = operation(conn, *args, **kwargs)
                self.total_operations += 1
                return result
            except Exception as e:
                self.failed_operations += 1
                raise
            finally:
                if conn is not None:
                    self.pool.put(conn)

        return self.executor.submit(_execute)

    def get_stats(self) -> Dict[str, int]:
        """Get pool statistics"""
        return {
            'available_connections': self.pool.qsize(),
            'max_connections': self.max_connections,
            'total_operations': self.total_operations,
            'failed_operations': self.failed_operations,
        }

    def shutdown(self):
        """Shutdown pool and close all connections"""
        self.executor.shutdown(wait=True)
        while not self.pool.empty():
            try:
                conn = self.pool.get_nowait()
                if hasattr(conn, 'close'):
                    conn.close()
            except:
                pass


class BatchProcessor:
    """
    Batches operations for efficient I/O.

    Collects items and processes them in batches either when the batch is full
    or after a timeout, whichever comes first.

    Example:
        def write_to_file(batch):
            with open('log.txt', 'a') as f:
                for item in batch:
                    f.write(f"{item}\n")

        processor = BatchProcessor(
            batch_size=10,
            flush_interval=1.0,
            process_batch=write_to_file
        )

        class LoggerNode(horus.Node):
            def __init__(self, processor):
                super().__init__()
                self.processor = processor

            def tick(self):
                # Add items to batch
                self.processor.add(f"Log entry: {time.time()}")

                # Auto-flush if needed
                self.processor.tick()

                # Check if flush completed
                result = self.processor.check_completed()
                if result:
                    print(f"Flushed {result} items")
    """

    def __init__(self, batch_size: int, flush_interval: float, process_batch: Callable):
        """
        Initialize batch processor.

        Args:
            batch_size: Maximum batch size before auto-flush
            flush_interval: Maximum time between flushes (seconds)
            process_batch: Function that processes a batch
        """
        self.batch_size = batch_size
        self.flush_interval = flush_interval
        self.process_batch = process_batch
        self.buffer = []
        self.executor = ThreadPoolExecutor(max_workers=1)
        self.last_flush = time.time()
        self.pending_future: Optional[Future] = None
        self.total_batches = 0
        self.total_items = 0

    def add(self, item):
        """Add item to batch"""
        self.buffer.append(item)

    def should_flush(self) -> bool:
        """Check if batch should be flushed"""
        return (len(self.buffer) >= self.batch_size or
                (len(self.buffer) > 0 and time.time() - self.last_flush >= self.flush_interval))

    def flush_async(self):
        """Flush batch asynchronously"""
        if self.buffer and not self.pending_future:
            batch = self.buffer[:]
            self.buffer.clear()
            self.pending_future = self.executor.submit(self.process_batch, batch)
            self.last_flush = time.time()
            self.total_batches += 1
            self.total_items += len(batch)

    def tick(self):
        """Call this in your node's tick() to auto-flush"""
        if self.should_flush():
            self.flush_async()

    def check_completed(self) -> Optional[int]:
        """
        Check if flush completed.

        Returns:
            Number of items in completed batch, or None if no completion
        """
        if self.pending_future and self.pending_future.done():
            try:
                result = self.pending_future.result(timeout=0)
                self.pending_future = None
                return result
            except Exception as e:
                print(f"Batch processing failed: {e}")
                self.pending_future = None
        return None

    def get_stats(self) -> Dict[str, Any]:
        """Get processor statistics"""
        return {
            'buffer_size': len(self.buffer),
            'batch_size': self.batch_size,
            'total_batches': self.total_batches,
            'total_items': self.total_items,
            'pending': self.pending_future is not None,
        }

    def force_flush(self):
        """Force immediate flush"""
        if self.buffer:
            self.flush_async()

    def shutdown(self, wait: bool = True):
        """Shutdown and optionally flush remaining items"""
        if wait and self.buffer:
            self.force_flush()
        self.executor.shutdown(wait=wait)


class RateLimiter:
    """
    Rate limiter for throttling async operations.

    Ensures operations don't exceed a specified rate.

    Example:
        limiter = RateLimiter(max_rate=10.0)  # 10 operations per second

        class APINode(horus.Node):
            def __init__(self, limiter):
                super().__init__()
                self.limiter = limiter

            def tick(self):
                if self.limiter.allow():
                    # Make API request
                    pass
                else:
                    # Skip this tick, rate limit exceeded
                    pass
    """

    def __init__(self, max_rate: float):
        """
        Initialize rate limiter.

        Args:
            max_rate: Maximum operations per second
        """
        self.max_rate = max_rate
        self.min_interval = 1.0 / max_rate
        self.last_operation = 0.0
        self.total_allowed = 0
        self.total_rejected = 0

    def allow(self) -> bool:
        """
        Check if an operation is allowed under the rate limit.

        Returns:
            True if operation is allowed, False otherwise
        """
        current_time = time.time()
        elapsed = current_time - self.last_operation

        if elapsed >= self.min_interval:
            self.last_operation = current_time
            self.total_allowed += 1
            return True
        else:
            self.total_rejected += 1
            return False

    def wait_time(self) -> float:
        """Get time until next operation is allowed"""
        elapsed = time.time() - self.last_operation
        remaining = self.min_interval - elapsed
        return max(0.0, remaining)

    def get_stats(self) -> Dict[str, Any]:
        """Get rate limiter statistics"""
        return {
            'max_rate': self.max_rate,
            'total_allowed': self.total_allowed,
            'total_rejected': self.total_rejected,
            'current_rate': self.total_allowed / (time.time() - (self.last_operation - self.total_allowed * self.min_interval)) if self.total_allowed > 0 else 0.0,
        }


class AsyncAggregator:
    """
    Aggregates results from multiple async operations.

    Useful for waiting on multiple independent I/O operations.

    Example:
        aggregator = AsyncAggregator()

        class MultiSensorNode(horus.Node):
            def __init__(self, aggregator):
                super().__init__()
                self.aggregator = aggregator
                self.executor = ThreadPoolExecutor(max_workers=4)

            def tick(self):
                # Submit multiple operations
                if not self.aggregator.has_pending():
                    self.aggregator.add("temp", self.executor.submit(read_temperature))
                    self.aggregator.add("pressure", self.executor.submit(read_pressure))
                    self.aggregator.add("humidity", self.executor.submit(read_humidity))

                # Check if all completed
                if self.aggregator.all_completed():
                    results = self.aggregator.get_all_results()
                    self.send("sensors_out", results)
                    self.aggregator.clear()
    """

    def __init__(self):
        """Initialize aggregator"""
        self.operations: Dict[str, Future] = {}

    def add(self, name: str, future: Future):
        """Add a named async operation"""
        self.operations[name] = future

    def has_pending(self) -> bool:
        """Check if any operations are pending"""
        return len(self.operations) > 0

    def all_completed(self) -> bool:
        """Check if all operations completed"""
        return all(f.done() for f in self.operations.values())

    def get_completed(self) -> Dict[str, Any]:
        """Get results from completed operations"""
        results = {}
        for name, future in list(self.operations.items()):
            if future.done():
                try:
                    results[name] = future.result(timeout=0)
                    del self.operations[name]
                except Exception as e:
                    results[name] = {'error': str(e)}
                    del self.operations[name]
        return results

    def get_all_results(self) -> Dict[str, Any]:
        """Get all results (waits for pending)"""
        results = {}
        for name, future in self.operations.items():
            try:
                results[name] = future.result(timeout=5.0)
            except Exception as e:
                results[name] = {'error': str(e)}
        return results

    def clear(self):
        """Clear all operations"""
        self.operations.clear()

    def count_pending(self) -> int:
        """Count pending operations"""
        return sum(1 for f in self.operations.values() if not f.done())
