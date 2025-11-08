# HORUS Production Qualification Tests

Comprehensive test suite to verify that structural changes to `horus_core` don't break production functionality.

## Purpose

Run these tests after making structural changes to core components (memory, IPC, scheduling, etc.) to ensure:
-  API compatibility maintained
-  IPC functionality works correctly
-  Memory safety (no segfaults, proper unsafe usage)
-  Robotics usage patterns work
-  Logging and diagnostics functional
-  Multi-process communication stable
-  Performance within acceptable bounds

## Usage

```bash
# Run all tests
./run_tests.sh

# Run specific test category
./run_tests.sh api
./run_tests.sh ipc
./run_tests.sh safety
./run_tests.sh robotics
./run_tests.sh stress
```

## Test Categories

### 1. API Compatibility Tests (`test_api_compatibility`)
- Node creation and lifecycle
- Hub publish/subscribe
- Link send/receive
- Scheduler execution
- Message type compatibility

### 2. IPC Functionality Tests (`test_ipc_functionality`)
- Hub multi-process MPMC
- Link single-process SPSC
- Cross-process message passing
- Large message handling
- High-frequency communication

### 3. Memory Safety Tests (`test_memory_safety`)
- Shared memory allocation/deallocation
- Bounds checking
- Concurrent access patterns
- Memory leak detection
- Valgrind/MIRI compatibility

### 4. Robotics Usage Tests (`test_robotics_usage`)
- Sensor data flow (IMU, Camera, LiDAR)
- Actuator command patterns
- Control loop timing (1kHz)
- Transform broadcasting
- State machine execution

### 5. Logging Tests (`test_logging`)
- Log level filtering
- Message tracing
- Performance with logging enabled/disabled
- Context propagation

### 6. Stress Tests (`test_stress`)
- 1000+ topics
- 100+ concurrent nodes
- Sustained high-frequency messaging
- Memory pressure scenarios
- Long-running stability (30min+)

## Success Criteria

All tests must pass with:
-  Zero segmentation faults
-  Zero data corruption
-  Zero message loss in normal operation
-  Performance within 20% of baseline
-  Clean shutdown without leaks

## Adding New Tests

1. Create test binary in `benchmarks/tests/src/bin/`
2. Add test to appropriate category in `run_tests.sh`
3. Document expected behavior and success criteria
4. Update this README

## Baseline Performance

Tests will compare against these baselines (established with 64-byte cache alignment):
- Hub latency: ~400-600ns median
- Link latency: ~300-450ns median
- Control loop jitter: <1ms
- Memory usage: <10MB per node
