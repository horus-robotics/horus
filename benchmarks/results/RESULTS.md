# HORUS Production Benchmark Results

**Generated**: October 3, 2025

## Quick Results

### Production Performance Summary

| Message Type | Size | Avg Latency | Throughput | Use Case |
|--------------|------|-------------|------------|----------|
| **CmdVel** | 16 B | **~500 ns** | 2.7M msg/s | Motor control @ 1000Hz |
| **IMU** | 304 B | **~940 ns** | 1.8M msg/s | Sensor fusion @ 100Hz |
| **Odometry** | 736 B | **~1.1 μs** | 1.3M msg/s | Localization @ 50Hz |
| **LaserScan** | 1.5 KB | **~2.2 μs** | 633K msg/s | 2D Lidar @ 10Hz |
| **PointCloud (10K)** | 120 KB | **~360 μs** | 4.7K msg/s | 3D Perception @ 30Hz |

### Performance Highlights

 **Sub-microsecond latency** for control messages (CmdVel: 296-643ns)
 **Low-microsecond latency** for sensor data (LaserScan: 1.31-2.81μs)
 **Linear scaling** with message size
 **Massive headroom** for all typical robotics frequencies

## Latest Run

See [`latest_run.txt`](latest_run.txt) for most recent benchmark output.

**Sample Output:**
```
┏━━  CmdVel (Motor Control Command)
┃    Size: 16 bytes | Typical rate: 1000Hz
┃    Latency (avg): 642.97 ns
┃    Throughput: 1555280.58 msg/s
┗━━

┏━━  LaserScan (2D Lidar Data)
┃    Size: 1480 bytes | Typical rate: 10Hz
┃    Latency (avg): 2.81 μs
┃    Throughput: 356478.18 msg/s
┗━━
```

## Comparison with traditional frameworks

| Framework | Small Msg | Medium Msg | Large Msg | Speedup |
|-----------|-----------|------------|-----------|---------|
| **HORUS** | **500 ns** | **940 ns** | **2.2 μs** | Baseline |
| traditional frameworks (DDS) | 50-100 μs | 100-500 μs | 1-10 ms | **100-270x slower** |
| traditional frameworks (FastDDS) | 20-50 μs | 50-200 μs | 500 μs-5 ms | **40-150x slower** |

## Methodology

- **Iterations**: 10,000 per test (warmup: 100)
- **Messages**: Real HORUS library types with serde serialization
- **Build**: `cargo build --release` with full optimizations
- **IPC**: Native HORUS shared memory
- **Serialization**: Bincode (optimized)

## Running Benchmarks

```bash
# Standalone benchmark
./target/release/production_bench

# Criterion benchmarks
cargo bench --bench production_messages

# Build first
cargo build --release --bin production_bench
```

## Message Types Tested

### Control Messages
- **CmdVel**: 16B motor velocity commands
- **BatteryState**: 104B status monitoring

### Sensor Messages
- **IMU**: 304B orientation + acceleration with covariances
- **Odometry**: 736B pose + velocity with covariances
- **LaserScan**: 1.5KB 360-degree lidar scan

### Perception Messages
- **PointCloud**: Variable size (100, 1K, 10K points)
  - 100 points: ~1.2KB
  - 1,000 points: ~12KB
  - 10,000 points: ~120KB

### Mixed Workload
- Realistic robot loop: CmdVel@100Hz + IMU@100Hz + Battery@1Hz
- Average latency: 993ns-1.77μs

## Detailed Analysis

See [`../README.md`](../README.md) and [`../SUMMARY.md`](../SUMMARY.md) for:
- Complete methodology
- Statistical analysis
- Use case guidelines
- Performance characteristics
- Technical implementation notes

## Conclusion

**HORUS delivers production-grade performance:**

 **296ns-643ns** - CmdVel (motor control)
 **718ns-1.37μs** - IMU (sensor fusion)
 **1.31-2.81μs** - LaserScan (2D lidar)
 **650ns-1.43μs** - Odometry (localization)
 **215-507μs** - PointCloud (10K points)

