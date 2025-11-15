# HORUS IPC Benchmark - Quick Start Guide

## Overview

The HORUS IPC Benchmark is a comprehensive benchmark for measuring inter-process communication latency with:
- Bootstrap confidence intervals (distribution-free)
- 41 comprehensive unit tests
- Formal methodology documentation
- Rigorous statistical methods

---

## Key Files

### Main Benchmark Files

| File | Purpose |
|------|---------|
| **`src/bin/ipc_benchmark.rs`** | Main benchmark (2,016 lines) |
| **`METHODOLOGY.md`** | Formal statistical methodology (463 lines) |
| **`BENCHMARK_README.md`** | Detailed benchmark documentation |
| **`Cargo.toml`** | Dependencies and build configuration |

### Results & Documentation

| File | Purpose |
|------|---------|
| **`benchmarks/benchmark_results.json`** | Results database (created after run) |
| **`results/RESULTS.md`** | Historical results and analysis |
| **`QUICK_START.md`** | This guide |

### Setup Scripts

| Script | Purpose |
|--------|---------|
| **`benchmark_setup.sh`** | Optimize system for benchmarking |
| **`benchmark_restore.sh`** | Restore normal system settings |

---

## Quick Start

### 1. Build the Benchmark

```bash
cd /home/lord-patpak/horus/HORUS
cargo build --release --bin ipc_benchmark
```

**Build time**: ~2 minutes
**Binary location**: `target/release/ipc_benchmark`

---

### 2. Run the Benchmark (Simple)

```bash
./target/release/ipc_benchmark
```

**Duration**: ~5-8 minutes (with bootstrap CI)
**Results saved to**: `benchmarks/benchmark_results.json`

---

### 3. Run with Optimal Settings (Recommended)

For **best results**:

```bash
# Step 1: Optimize system
cd benchmarks
sudo ./benchmark_setup.sh

# Step 2: Run benchmark
cd ..
./target/release/ipc_benchmark

# Step 3: Restore system (optional)
cd benchmarks
sudo ./benchmark_restore.sh
```

---

## Where Are Results Saved?

### Primary Results File

**Location**: `benchmarks/benchmark_results.json` (in benchmarks directory)

**Format**: JSON array with complete metadata

**Example**:
```json
[
  {
    "platform": {
      "cpu_vendor": "Intel",
      "cpu_model": "Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz",
      "base_frequency_ghz": 2.593,
      "num_physical_cores": 6,
      "num_logical_cores": 12
    },
    "timestamp": "1736612345",
    "link_stats": {
      "median_cycles": 1075,
      "median_ns": 415,
      "mean_cycles": 1089.3,
      "p95_cycles": 1156,
      "p99_cycles": 1234,
      "ci_lower_cycles": 1050,
      "ci_upper_cycles": 1100,
      "sample_count": 485420,
      "outliers_removed": 14580
    },
    "hub_stats": { ... },
    "tsc_verification_passed": true,
    "cpu_frequency_source": "rdtsc_measured",
    "measurement_quality": "high"
  }
]
```

### Viewing Results

```bash
# Pretty-print latest result
jq '.[-1]' benchmarks/benchmark_results.json

# View only high-quality results
jq '.[] | select(.measurement_quality == "high")' benchmarks/benchmark_results.json

# Compare Link vs Hub latencies
jq '.[] | {link: .link_stats.median_ns, hub: .hub_stats.median_ns}' benchmarks/benchmark_results.json

# Count results by quality
jq 'group_by(.measurement_quality) | map({quality:.[0].measurement_quality, count:length})' benchmarks/benchmark_results.json
```

---

## Understanding the Output

### 1. Platform Information
```
Platform Information:
  • CPU: Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz
  • Cores: 6 physical, 12 logical
```
Shows hardware configuration.

### 2. TSC Verification
```
TSC Verification:
  • Invariant TSC: [OK] YES
  • Cross-core TSC drift: 245 cycles ([OK] excellent)
```
- **< 1,000 cycles**: Excellent
- **1,000-10,000 cycles**: Acceptable
- **> 10,000 cycles**: Poor (results marked LOW QUALITY)

### 3. CPU Frequency Detection
```
CPU Frequency Detection:
  • Measured frequency: 2.593 GHz
```
If this fails, benchmark **exits with error** (no arbitrary fallbacks).

### 4. Benchmark Results
```
HORUS LINK (SPSC):
  Median Latency:    415 ns  (1075 cycles)
  95% CI:           [410-420 ns]    ← Bootstrap CI
  P95:               450 ns
  P99:               480 ns
  Samples:           485,420 (outliers removed: 14,580)
```

### 5. Quality Assessment
```
MEASUREMENT QUALITY ASSESSMENT
  [OK] HIGH QUALITY - All validation checks passed
  • TSC verification: PASSED
  • CPU frequency: Measured via RDTSC
  • TSC drift: 245 cycles (excellent)

  [OK] These results meet high quality standards.
```

**Quality Levels**:
- **HIGH**: All checks passed, suitable for benchmarking
- **MEDIUM**: Moderate TSC drift, usable for trends
- **LOW**: High TSC drift, not recommended for benchmarking
- **INVALID**: Critical failures (TSC failed, missing data)

---

## Advanced Usage

### Run Tests

Test all statistical functions (41 tests):
```bash
cargo test --bin ipc_benchmark
```

**Expected output**:
```
test result: ok. 41 passed; 0 failed; 0 ignored
```

### Run with Longer Timeout

If benchmark times out:
```bash
timeout 600 ./target/release/ipc_benchmark  # 10 minutes
```

### Save Output to File

```bash
./target/release/ipc_benchmark 2>&1 | tee benchmark_run_$(date +%Y%m%d_%H%M%S).log
```

### Run in Background

```bash
nohup ./target/release/ipc_benchmark > benchmark.log 2>&1 &
```

Monitor progress:
```bash
tail -f benchmark.log
```

---

## System Optimization (For Best Results)

### What `benchmark_setup.sh` Does:

1. **Sets CPU governor to performance** (locks frequency)
2. **Disables frequency scaling** (prevents variance)
3. **Disables CPU idle states** (reduces jitter)
4. **Disables ASLR** (address space layout randomization)
5. **Configures scheduler** (reduces context switches)

### Run Setup Script:

```bash
cd benchmarks
sudo ./benchmark_setup.sh
```

**Warning**: This impacts system performance. Use only during benchmarking.

### Restore Normal Settings:

```bash
cd benchmarks
sudo ./benchmark_restore.sh
```

---

## Interpreting Quality Ratings

### HIGH QUALITY [OK]
```
[OK] HIGH QUALITY - All validation checks passed
[OK] These results meet high quality standards.
```
**Use for**: Performance analysis, hardware comparisons

### MEDIUM QUALITY [WARNING]
```
[WARNING] MEDIUM QUALITY - Moderate TSC drift detected
[WARNING] Usable for performance trends, but note increased variance.
```
**Use for**: Internal monitoring, trend analysis

### LOW QUALITY [WARNING]
```
[WARNING] LOW QUALITY - High TSC drift detected
[WARNING] Not recommended for benchmarking.
```
**Use for**: Development only, re-run with system optimization

### INVALID [FAIL]
```
[FAIL] INVALID - Critical validation failures
[FAIL] These results CANNOT be used.
```
**Action**: Fix system issues (TSC sync, frequency detection) and re-run

---

## Troubleshooting

### Benchmark Times Out

**Cause**: Bootstrap CI takes 2-3 seconds per run (×20 runs = 40-60 seconds)

**Solution**: Use longer timeout:
```bash
timeout 600 ./target/release/ipc_benchmark
```

### TSC Drift Too High

**Cause**: Virtualized environment, frequency scaling, or busy system

**Solutions**:
1. Run `benchmark_setup.sh` to lock CPU frequency
2. Stop background processes
3. Use bare metal instead of VM
4. Use core isolation (add `isolcpus=0,1` to kernel cmdline)

### CPU Frequency Detection Failed

**Cause**: No RDTSC support or invalid TSC

**Solution**: Benchmark requires x86_64 with invariant TSC. Check:
```bash
grep -i tsc /proc/cpuinfo
```
Look for `constant_tsc` and `nonstop_tsc`.

### Results Marked as LOW QUALITY

**Cause**: System not optimized for benchmarking

**Solution**: Run `benchmark_setup.sh` before benchmarking:
```bash
cd benchmarks
sudo ./benchmark_setup.sh
cd ..
./target/release/ipc_benchmark
```

---

## Files Created During Benchmark

| File | Location | Purpose |
|------|----------|---------|
| **benchmark_results.json** | benchmarks/ directory | Results database |
| **/tmp/barrier_hub_[PID]** | /tmp | Temporary barrier files (auto-cleaned) |
| **/tmp/barrier_link_[PID]** | /tmp | Temporary barrier files (auto-cleaned) |

**Note**: Barrier files are automatically cleaned up after each run.

---

## Benchmark Configuration

Located in `src/bin/ipc_benchmark.rs` (lines 31-34):

```rust
const ITERATIONS: usize = 50_000;   // Iterations per run
const WARMUP: usize = 5_000;        // Warmup iterations
const NUM_RUNS: usize = 10;         // Number of runs
```

**Total samples**: 50,000 × 10 runs = 500,000 samples per IPC type

---

## Features Implemented

### 1. Bootstrap Confidence Intervals
- Distribution-free (no normality assumption)
- 2,000 bootstrap resamples
- Reference: Efron & Tibshirani (1994)

### 2. Comprehensive Testing
- 41 unit tests (100% pass rate)
- All statistical functions validated
- Edge cases tested

### 3. Formal Methodology
- 463-line technical document
- 9 academic references
- Complete formulas and assumptions

### 4. Quality Gates
- TSC verification
- Frequency validation
- Quality ratings (high/medium/low/invalid)

---

## Technical Documentation

### Key Files:

1. **METHODOLOGY.md** - Statistical methodology details
2. **benchmark_results.json** - Raw benchmark data
3. **BENCHMARK_README.md** - Benchmark specifications

### Citation:

```bibtex
@software{horus_ipc_benchmark,
  title = {HORUS IPC Latency Benchmark},
  author = {HORUS Team},
  year = {2025},
  version = {2.0},
  note = {Comprehensive benchmark with bootstrap confidence intervals},
  url = {https://github.com/softmata/horus}
}
```

### Methodology Details:

Includes:
- Bootstrap CI methodology (cite Efron & Tibshirani, 1994)
- NIST R-7 percentile method
- Tukey's 1.5×IQR outlier filtering
- Sample size (n=500,000)
- Quality assessment criteria

See `METHODOLOGY.md` for complete details.

---

## Quick Reference

### Essential Commands

```bash
# Build
cargo build --release --bin ipc_benchmark

# Run (simple)
./target/release/ipc_benchmark

# Run (optimized for best results)
cd benchmarks && sudo ./benchmark_setup.sh && cd .. && ./target/release/ipc_benchmark

# View results
jq '.[-1]' benchmarks/benchmark_results.json

# Run tests
cargo test --bin ipc_benchmark

# Restore system
cd benchmarks && sudo ./benchmark_restore.sh
```

### File Locations

```
HORUS/
├── target/release/ipc_benchmark    ← Binary here
└── benchmarks/
    ├── benchmark_results.json      ← Results saved here
    ├── src/bin/ipc_benchmark.rs    ← Main benchmark source
    ├── METHODOLOGY.md              ← Formal methodology
    ├── BENCHMARK_README.md         ← Detailed docs
    ├── QUICK_START.md              ← This file
    └── benchmark_setup.sh          ← System optimization
```

---

## Next Steps

1. **Run benchmark**: `./target/release/ipc_benchmark`
2. **Check results**: `jq '.[-1]' benchmarks/benchmark_results.json`
3. **Verify quality**: Look for "HIGH QUALITY" in output
4. **Technical details**: Read `METHODOLOGY.md`

---

## Getting Help

- **Benchmark docs**: `benchmarks/BENCHMARK_README.md`
- **Methodology**: `benchmarks/METHODOLOGY.md`
- **Issues**: https://github.com/softmata/horus/issues

---

**The HORUS IPC Benchmark - Comprehensive Statistical Testing**

Last updated: 2025-11-11
Version: 2.0
