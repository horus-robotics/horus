# Async I/O Tier Design

## Overview

The Async I/O tier enables non-blocking execution of nodes that perform I/O operations (network requests, file I/O, camera reads, etc.). This prevents slow I/O from blocking the main scheduler loop.

## Architecture

### Rust Core (AsyncIOExecutor)
- **Location**: `horus_core/src/scheduling/executors/async_io.rs`
- **Runtime**: Tokio multi-threaded runtime with 4 worker threads
- **Mechanism**: Nodes run in separate async tasks via `spawn_blocking`
- **Communication**: Message passing (Tick/Shutdown) via mpsc channels
- **Results**: AsyncResult sent back via unbounded channel

### Python Integration Challenges

1. **Event Loop Mismatch**
   - Python: asyncio event loop
   - Rust: Tokio runtime
   - These are separate and don't integrate directly

2. **PyO3 Async Support**
   - PyO3 has limited async support (experimental)
   - Can't directly await Python coroutines from Rust
   - Can't directly call Rust async from Python async

3. **GIL Constraints**
   - Python's Global Interpreter Lock
   - Async doesn't release GIL automatically
   - Need spawn_blocking or similar for true parallelism

## Proposed Approach

### Phase 1: Threaded Async Nodes (Simple)
Use Python threading with the existing synchronous API but run I/O in background threads.

**Pros:**
- Works with current architecture
- No complex async bridging
- Easy to implement and test

**Cons:**
- Not true async/await
- Thread overhead
- Limited to thread-based concurrency

**Implementation:**
```python
class AsyncNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.executor = ThreadPoolExecutor(max_workers=4)

    def tick(self):
        # Submit I/O work to thread pool
        future = self.executor.submit(self._async_work)
        result = future.result(timeout=1.0)
        return result
```

### Phase 2: Rust-Managed Async Nodes (Complex)
Create a separate AsyncScheduler that manages nodes in Tokio runtime.

**Pros:**
- True async execution
- Better performance
- Leverages Rust async ecosystem

**Cons:**
- Complex PyO3 integration
- Requires separate scheduler
- More difficult to debug

**Implementation:**
- Create `PyAsyncScheduler` wrapping `AsyncIOExecutor`
- Python nodes register with async scheduler
- Scheduler runs in background Tokio runtime
- Results collected via callbacks

### Phase 3: Python Asyncio Bridge (Most Complex)
Bridge Python asyncio directly with Tokio runtime.

**Pros:**
- Native Python async/await syntax
- Best developer experience
- Idiomatic Python

**Cons:**
- Extremely complex
- Requires pyo3-asyncio (experimental)
- Hard to maintain
- May have performance issues

## Recommended Implementation: Phase 1

Start with threaded async nodes for initial release:

1. **Simple to implement**: Uses existing threading
2. **Good enough**: Handles most I/O use cases
3. **Stable**: No experimental dependencies
4. **Easy to migrate**: Can upgrade to Phase 2 later

## API Design

### Python API (Phase 1)

```python
import horus
import time

class CameraNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.camera = Camera()

    def tick(self):
        # This will block but that's OK for now
        # In future, could use threading internally
        frame = self.camera.read()  # Blocking I/O
        self.send("camera_out", frame)

# Scheduler handles this normally
scheduler = horus.Scheduler()
scheduler.add(CameraNode())
scheduler.run()
```

### Future API (Phase 2)

```python
import horus

class CameraNode(horus.AsyncNode):  # Different base class
    async def tick_async(self):
        # True async/await
        frame = await self.camera.read_async()
        await self.send_async("camera_out", frame)

# Separate async scheduler
async_scheduler = horus.AsyncScheduler()
async_scheduler.add_async(CameraNode())
await async_scheduler.run_async()
```

## Implementation Status

### Completed
- ✅ Rust AsyncIOExecutor exists in core
- ✅ Design documentation (ASYNC_IO_DESIGN.md)
- ✅ Phase 1: Threaded approach (recommended)
  - ✅ Pattern documentation (ASYNC_IO_GUIDE.md)
  - ✅ Comprehensive demo (demo_async_io_threaded.py)
  - ✅ Multiple examples showing different I/O patterns
  - ✅ Best practices and common pitfalls documented
- ✅ Phase 2: Async Utilities and Helpers (revised scope)
  - ✅ AsyncHelper - General async operation tracking
  - ✅ ConnectionPool - Connection pooling for databases/APIs
  - ✅ BatchProcessor - Batching for efficient I/O
  - ✅ RateLimiter - Rate throttling
  - ✅ AsyncAggregator - Multi-operation aggregation
  - ✅ Comprehensive demo (demo_async_phase2.py)
  - ✅ Production-ready utilities (horus_async_helpers.py)

### Future Work
- ⏳ Phase 3: Full Tokio/asyncio integration (when PyO3-asyncio is stable)

## Notes for Implementation

1. **Start Simple**: Phase 1 (threading) is sufficient for most use cases
2. **Measure First**: Profile before implementing complex async
3. **Consider pyo3-asyncio**: Monitor for stability before using
4. **Thread Safety**: Ensure Hub/Link are thread-safe for concurrent access
5. **Testing**: Need tests for concurrent I/O operations

## References

- Tokio Documentation: https://tokio.rs
- PyO3 Async: https://pyo3.rs (experimental)
- pyo3-asyncio: https://github.com/awestlake87/pyo3-asyncio
- Python threading: https://docs.python.org/3/library/threading.html
