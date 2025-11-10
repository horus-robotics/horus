# HORUS Production Benchmark Summary

## Quick Reference

**Last Updated**: October 31, 2025

### Production-Validated IPC Performance

**Link (SPSC) - Cross-Core Latency:**
- **Median latency**: 248ns (496 cycles @ 2GHz)
- **P95 latency**: 444ns
- **P99 latency**: 578ns
- **Burst throughput**: 6.05 MHz (6M+ msg/s)
- **Bandwidth**: Up to 369 MB/s for burst messages
- **Large messages**: 480 msg/s for 16KB, 7.5 MB/s bandwidth

**Hub (MPMC) - Cross-Core Latency:**
- **Median latency**: 481ns (962 cycles @ 2GHz)
- **P95 latency**: 653ns
- **Flexible pub/sub** architecture

**Key Results:**
- Link is **48% faster** than Hub in 1P1C scenarios
- **Production-validated** with 6.2M+ test messages
- **Zero corruptions** detected
- Tested on modern x86_64 systems

### Performance by Message Type (Legacy Hub Measurements)

*Note: These are older Hub measurements. Link (SPSC) shows 48% better performance.*

| Message Type | Size | Latency Range | Avg Latency | Throughput | Target Rate | Headroom |
|--------------|------|---------------|-------------|------------|-------------|----------|
| **CmdVel** | 16 B | 237-355 ns | **296 ns** | **3.4M msg/s** | 1000 Hz | **3,380x** |
| **BatteryState** | 104 B | 284-426 ns | **355 ns** | **2.8M msg/s** | 1 Hz | **2.8M x** |
| **IMU** | 304 B | 574-861 ns | **718 ns** | **1.4M msg/s** | 100 Hz | **13,937x** |
| **Odometry** | 736 B | 520-780 ns | **650 ns** | **1.5M msg/s** | 50 Hz | **30,783x** |
| **LaserScan** | 1.5 KB | 1.05-1.58μs | **1.31 μs** | **762K msg/s** | 10 Hz | **76,203x** |
| **PointCloud (100pt)** | 1.2 KB | 1.48-2.22μs | **1.85 μs** | **540K msg/s** | 30 Hz | **17,984x** |
| **PointCloud (1Kpt)** | 12 KB | 6.04-9.06μs | **7.55 μs** | **132K msg/s** | 30 Hz | **4,414x** |
| **PointCloud (10Kpt)** | 120 KB | 141-211μs | **176 μs** | **5.7K msg/s** | 30 Hz | **189x** |
| **Mixed Loop** | - | 518-778 ns | **648 ns** | **1.5M msg/s** | 100 Hz | **15,431x** |

### vs ROS2 Comparison

| IPC Type | HORUS (Link) | HORUS (Hub) | ROS2 (DDS) | ROS2 (FastDDS) | Speedup vs ROS2 |
|----------|--------------|-------------|------------|----------------|-----------------|
| **Small (16B)** | **248 ns** | **481 ns** | 50-100 μs | 20-50 μs | **80-403x faster** |
| **Medium (304B)** | **~400 ns** | **~620 ns** | 50-100 μs | 20-50 μs | **50-250x faster** |
| **Large (1.5KB)** | **~900 ns** | **~1.4 μs** | 100-500 μs | 50-200 μs | **71-556x faster** |

## Key Findings

**Sub-microsecond latency** on modern x86_64 systems
- Link (SPSC): Typically 300-500ns
- Hub (MPMC): Typically 400-700ns

**High throughput** sustained in production
- 6+ million messages per second
- Up to 369 MB/s bandwidth for burst messages

**Production-validated reliability**
- 6.2M+ test messages with zero corruptions
- Comprehensive test suite validates all claims

**Predictable performance**
- Linear scaling with message size
- Massive headroom for typical robotics frequencies

## Running Benchmarks

```bash
# Quick run
./target/release/production_bench

# Build and run
cargo build --release --bin production_bench
./target/release/production_bench

# Criterion benchmarks
cargo bench --bench production_messages
```

## Files

- **`production_bench.rs`** - Standalone binary for quick testing
- **`production_messages.rs`** - Criterion benchmark suite
- **`results/production_messages_benchmark.md`** - Full detailed report
- **`results/latest_run.txt`** - Most recent benchmark output
- **`README.md`** - Complete benchmark documentation

## Methodology

- **Iterations**: 10,000 per test (warmup: 100)
- **Messages**: Real HORUS library types with serde
- **Build**: `--release` with full optimizations
- **IPC**: Native HORUS shared memory
- **Serialization**: Bincode (optimized)

## Conclusion

**HORUS delivers production-grade performance** optimized for real-time robotics:

**IPC Performance:**
- **Link (SPSC)**: 248ns median, 6M+ msg/s - Best for point-to-point
- **Hub (MPMC)**: 481ns median - Flexible pub/sub architecture
- **48% faster** with Link vs Hub in 1P1C scenarios

**Use Case Guidelines:**
- **Link**: Direct node-to-node communication, control loops
- **Hub**: Multi-subscriber topics, sensor broadcasting, flexible architectures

**Production-Ready:**
- Sub-microsecond latency on modern x86_64 systems
- 6.2M+ test messages with zero corruptions
- Comprehensive validation suite

*Performance varies by hardware. Run `cargo test --release` to benchmark on your system.*

See `results/` directory for detailed analysis and test reports.
