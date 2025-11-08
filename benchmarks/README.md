# HORUS Benchmark Suite

Professional-grade performance benchmarks for the HORUS robotics framework.

## Overview

This benchmark suite provides rigorous, statistically sound performance measurements of HORUS's inter-process communication (IPC) systems:

- **HORUS Link**: Single-producer, single-consumer (SPSC) optimized channel
- **HORUS Hub**: Multi-producer, multi-consumer (MPMC) publish-subscribe system

## Production-Validated Performance

**Link (SPSC) - Cross-Core Latency:**
- Median latency: 312ns (624 cycles @ 2GHz)
- P95 latency: 444ns
- P99 latency: 578ns
- Burst throughput: 6.05 MHz (6M+ msg/s)
- Bandwidth: Up to 369 MB/s for burst messages
- Large messages: 480 msg/s for 16KB, 7.5 MB/s bandwidth

**Hub (MPMC) - Cross-Core Latency:**
- Median latency: 481ns (962 cycles @ 2GHz)
- P95 latency: 653ns
- Flexible pub/sub architecture

**Comparison:**
- Link is 29% faster than Hub in 1P1C scenarios
- Production-validated with 6.2M+ test messages
- Zero corruptions detected

*Performance measured on modern x86_64 systems. Results vary by hardware.*

## Benchmarks

### `ipc_benchmark` - Multi-Process IPC Latency

The primary benchmark suite for HORUS IPC performance with:

-  **True Multi-Process Testing**: Separate OS processes for accurate IPC measurement
-  **Statistical Rigor**: 5 runs per test, reporting median, P95, P99, stddev
-  **Per-Message Latency Tracking**: High-precision timestamp-based measurement
-  **CPU Affinity**: Pinned cores to eliminate context switching
-  **Barrier Synchronization**: File-based barriers (not sleep-based)
-  **Real Robotics Messages**: Actual HORUS message types used in production

**Message Types Tested:**
- CmdVel (16 bytes) - Velocity commands
- IMU (304 bytes) - Inertial measurement unit data
- Odometry (736 bytes) - Position and velocity estimates
- LaserScan (1480 bytes) - LIDAR sensor data

**Run:**
```bash
cargo build --release --bin ipc_benchmark
./target/release/ipc_benchmark
```

### Criterion Benchmarks

Standard Rust microbenchmarks using the Criterion framework:

- `production_messages` - Message serialization/deserialization performance
- `link_performance` - HORUS Link (SPSC channel) throughput

**Run:**
```bash
cargo bench
```

## Methodology

### Benchmark Design Principles

1. **Eliminate Measurement Overhead**
   - Pre-allocate all test messages before timing
   - Per-message timing with high-precision timestamps
   - Fixed-size message buffers

2. **Proper Warm-up**
   - 1,000 warmup iterations per run
   - Stabilizes JIT compilation, CPU caches, page faults

3. **Statistical Validity**
   - 10,000 measured iterations per run
   - 5 runs per test for distribution analysis
   - Report median (not mean) to reduce outlier impact
   - P95/P99 for tail latency analysis

4. **Process Isolation**
   - Producer and consumer in separate OS processes
   - CPU affinity pinning (core 0 and core 1)
   - Barrier-based synchronization

## Performance Characteristics

HORUS IPC systems demonstrate excellent performance characteristics:

1. **Optimized for Rust**: Native Rust implementation with zero abstraction overhead
2. **Lock-Free Design**: Efficient SPSC (Link) and MPMC (Hub) implementations
3. **Per-Message Timing**: High-precision latency measurement
4. **Cache-Optimized**: Memory layout designed to minimize cache misses

**Link vs Hub Trade-offs:**
- **Link (SPSC)**: Fastest option for point-to-point communication (312ns median)
- **Hub (MPMC)**: Flexible pub/sub with slightly higher latency (481ns median)

Results vary by message size, CPU architecture, and system load. Run benchmarks on your target hardware for accurate measurements.

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
