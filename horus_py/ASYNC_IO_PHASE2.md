# Async I/O Phase 2: Re-scoped Implementation

## Challenge Identified

The original Phase 2 design attempted to run Python nodes directly in a Tokio runtime. However, this presents fundamental challenges:

1. **PyNode uses Python callbacks**: Python code execution requires holding the GIL
2. **Tokio runtime is Rust-native**: Can't directly execute Python code
3. **PyO3 async support is experimental**: Limited and unstable

## Revised Phase 2 Scope

Instead of forcing Python nodes into Tokio, Phase 2 should focus on **better async abstractions and utilities** that work with the existing Python-based scheduler.

### Phase 2 Goals (Revised):

1. **Async Helper Utilities**
   - Provide reusable async I/O helpers
   - Connection pools
   - Async file I/O helpers
   - Batch processing utilities

2. **Advanced Patterns**
   - Producer/Consumer patterns
   - Pipeline processing
   - Async aggregators
   - Rate limiting

3. **Integration Examples**
   - Integration with `asyncio` event loops
   - Integration with `aiohttp`, `asyncpg`, etc.
   - Real-world sensor/actuator examples

4. **Performance Tools**
   - Async profiling utilities
   - I/O bottleneck detection
   - Queue monitoring

## Implementation Strategy

### 1. AsyncHelper Class (Python)

```python
import horus
from concurrent.futures import ThreadPoolExecutor
from typing import Optional, Callable, Any
import queue
import threading

class AsyncHelper:
    """
    Helper class for async I/O operations in HORUS nodes.

    Provides common patterns and utilities for non-blocking I/O.
    """

    def __init__(self, max_workers: int = 4):
        self.executor = ThreadPoolExecutor(max_workers=max_workers)
        self.futures = {}
        self.results_queue = queue.Queue()

    def submit(self, operation_id: str, func: Callable, *args, **kwargs):
        """Submit async operation with tracking"""
        future = self.executor.submit(func, *args, **kwargs)
        self.futures[operation_id] = future
        return future

    def check_completed(self) -> list:
        """Check all completed operations"""
        completed = []
        for op_id, future in list(self.futures.items()):
            if future.done():
                try:
                    result = future.result(timeout=0)
                    completed.append((op_id, result, None))
                except Exception as e:
                    completed.append((op_id, None, e))
                del self.futures[op_id]
        return completed

    def shutdown(self):
        """Graceful shutdown"""
        self.executor.shutdown(wait=True)
```

### 2. ConnectionPool (Python)

```python
class ConnectionPool:
    """
    Async-friendly connection pool for databases, APIs, etc.
    """

    def __init__(self, create_connection: Callable, max_connections: int = 10):
        self.create_connection = create_connection
        self.max_connections = max_connections
        self.pool = queue.Queue(maxsize=max_connections)
        self.executor = ThreadPoolExecutor(max_workers=max_connections)
        self._initialize_pool()

    def _initialize_pool(self):
        """Pre-create connections"""
        for _ in range(self.max_connections):
            conn = self.create_connection()
            self.pool.put(conn)

    def execute_async(self, operation: Callable, *args, **kwargs):
        """Execute operation with pooled connection"""
        def _execute():
            conn = self.pool.get()
            try:
                return operation(conn, *args, **kwargs)
            finally:
                self.pool.put(conn)

        return self.executor.submit(_execute)
```

### 3. BatchProcessor (Python)

```python
class BatchProcessor:
    """
    Batches operations for efficient I/O
    """

    def __init__(self, batch_size: int, flush_interval: float, process_batch: Callable):
        self.batch_size = batch_size
        self.flush_interval = flush_interval
        self.process_batch = process_batch
        self.buffer = []
        self.executor = ThreadPoolExecutor(max_workers=1)
        self.last_flush = time.time()
        self.pending_future = None

    def add(self, item):
        """Add item to batch"""
        self.buffer.append(item)

    def should_flush(self) -> bool:
        """Check if batch should be flushed"""
        return (len(self.buffer) >= self.batch_size or
                time.time() - self.last_flush >= self.flush_interval)

    def flush_async(self):
        """Flush batch asynchronously"""
        if self.buffer and not self.pending_future:
            batch = self.buffer[:]
            self.buffer.clear()
            self.pending_future = self.executor.submit(self.process_batch, batch)
            self.last_flush = time.time()

    def check_completed(self):
        """Check if flush completed"""
        if self.pending_future and self.pending_future.done():
            result = self.pending_future.result(timeout=0)
            self.pending_future = None
            return result
        return None
```

## Phase 2 vs Phase 3

**Phase 2 (Current - Practical)**:
- Python-based utilities and helpers
- Works with existing PyNode/PyScheduler
- No experimental dependencies
- Immediate value for users

**Phase 3 (Future - Advanced)**:
- True Rust/Tokio integration
- Requires stable PyO3-asyncio
- Python `async`/`await` syntax
- Best performance

## Status

- ✅ Phase 1: Threading patterns (Complete)
- 🔄 Phase 2: Async utilities and helpers (Revised scope - In Progress)
- ⏳ Phase 3: Full Tokio/asyncio integration (Future)

## Implementation Plan

1. Create `async_helpers.py` module with utility classes
2. Create comprehensive examples using helpers
3. Document integration with popular async libraries
4. Provide migration guide from Phase 1 to Phase 2 patterns

## Benefits of Revised Phase 2

1. **Practical**: Solves real user problems today
2. **Stable**: No experimental dependencies
3. **Compatible**: Works with existing HORUS infrastructure
4. **Extensible**: Easy to add more helpers as needed
5. **Educational**: Teaches async patterns effectively

## Next Steps

Rather than pursuing the original Tokio-based Phase 2 (which has fundamental GIL/PyO3 limitations), implement the revised Phase 2 focusing on Python-based async utilities that provide immediate practical value.
