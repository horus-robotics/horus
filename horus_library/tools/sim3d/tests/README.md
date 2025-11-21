# Sim3D Testing Framework

Comprehensive testing infrastructure for the sim3d robotics simulator.

## Test Categories

### Integration Tests (`integration_suite/`)

Located in `tests/integration_suite/`, these tests validate multi-system interactions:

#### Benchmarks (`benchmarks.rs`)
- Performance tracking and regression detection
- Automated benchmark runner
- Baseline comparison
- Throughput measurements

```rust
let mut runner = BenchmarkRunner::new();
runner.run("my_operation", 1000, || {
    // Operation to benchmark
});
```

#### Navigation Tests (`navigation.rs`)
- Path planning in cluttered environments
- Obstacle avoidance validation
- Multiple scenario complexities (simple, cluttered, maze)

#### Manipulation Tests (`manipulation.rs`)
- Pick and place operations
- Grasp accuracy validation
- Multi-object sorting
- Heavy object handling

#### Multi-Robot Tests (`multi_robot.rs`)
- Swarm coordination
- Formation control
- Inter-robot communication
- Scalability (5-100 robots)

#### Sensor Throughput (`sensors.rs`)
- Data generation rate benchmarks
- Camera, Lidar, Radar, Thermal sensors
- Resolution and update rate validation

#### Determinism Tests (`determinism.rs`)
- Reproducibility validation
- State hashing
- Divergence detection
- Fixed-seed simulation verification

```rust
let mut test = DeterminismTest::new("physics_sim", seed, 1000);
for step in 0..1000 {
    // Run simulation step
    test.record_state(&robot_transform);
}
assert!(test1.is_deterministic(&test2));
```

#### Stress Tests (`stress.rs`)
- 1000+ objects simulation
- 100+ robots coordination
- Memory usage tracking
- Performance degradation detection

Configurations:
- `many_objects()` - 1000 objects
- `many_robots()` - 100 robots
- `extreme_load()` - 1000 objects + 100 robots
- `physics_only()` - 5000 objects

## Running Tests

### All Tests
```bash
cargo test --all-features
```

### Integration Tests Only
```bash
cargo test --test integration
```

### Specific Test Module
```bash
cargo test --test integration test_stress_configs
```

### With Output
```bash
cargo test -- --nocapture
```

## Benchmarking

### Run Benchmarks
```bash
cargo bench
```

### Benchmark Specific Module
```bash
cargo bench --bench my_benchmark
```

## Performance Regression Detection

The benchmark runner automatically detects regressions:

```rust
runner.load_baselines(previous_results);
let regressions = runner.check_regressions(0.10); // 10% threshold

for (name, regression_pct) in regressions {
    println!("{} regressed by {:.1}%", name, regression_pct);
}
```

## CI/CD Integration

Continuous integration pipeline (`.github/workflows/ci.yml`):

- **Test Suite**: All unit and integration tests
- **Benchmarks**: Performance tracking
- **Coverage**: Code coverage reports (Codecov)
- **Lints**: rustfmt and clippy checks

## Test Coverage

Generate coverage report:

```bash
cargo llvm-cov --all-features --workspace --html
open target/llvm-cov/html/index.html
```

## Memory Leak Detection

Use Valgrind for memory leak detection:

```bash
cargo build --tests
valgrind --leak-check=full target/debug/deps/integration-*
```

## Determinism Validation

Ensure simulations are reproducible:

```bash
# Run same simulation twice with fixed seed
cargo test --test integration test_determinism_detection
```

Key requirements:
- Fixed random seed
- Deterministic physics stepping
- Consistent iteration order
- Platform-independent floating point

## Stress Testing

Validate performance under load:

- **Objects**: Test up to 5000 physics objects
- **Robots**: Test up to 100 simultaneous robots
- **Sensors**: Test sensor throughput limits
- **Memory**: Track peak memory usage

Performance criteria:
- Average step time < 10ms for 1000 objects
- Peak memory < 500MB for extreme load
- Zero failed steps

## Nightly Benchmarks

Automated nightly benchmarks track performance trends:

1. Runs full benchmark suite
2. Compares with previous results
3. Generates performance graphs
4. Alerts on regressions > 15%

## Best Practices

### Writing Tests
- Use descriptive test names
- Test one thing per test
- Use appropriate assertions
- Clean up resources

### Writing Benchmarks
- Include warmup iterations
- Run sufficient iterations for statistical significance
- Minimize external dependencies
- Use `black_box()` to prevent optimization

### Performance Testing
- Test on consistent hardware
- Disable CPU scaling
- Close background applications
- Run multiple times and average

## Test Organization

```
tests/
├── integration.rs              # Main integration test file
└── integration_suite/
    ├── mod.rs                  # Module exports
    ├── benchmarks.rs           # Performance tracking
    ├── navigation.rs           # Path planning tests
    ├── manipulation.rs         # Grasp and manipulation
    ├── multi_robot.rs          # Multi-robot coordination
    ├── sensors.rs              # Sensor throughput
    ├── determinism.rs          # Reproducibility
    └── stress.rs               # Load testing
```

## Metrics

Current test coverage:
- **Unit Tests**: 200+ tests across all modules
- **Integration Tests**: 30+ scenario tests
- **Benchmarks**: 15+ performance benchmarks
- **Stress Tests**: 4+ load configurations

## Troubleshooting

### Tests Fail Intermittently
- Check for race conditions
- Verify deterministic behavior
- Increase timeout values
- Use fixed random seeds

### Poor Performance
- Check debug vs release build
- Verify GPU acceleration is enabled
- Profile with `cargo flamegraph`
- Review memory allocations

### Memory Issues
- Run with `--release` flag
- Check for leaks with Valgrind
- Review object pooling
- Monitor peak allocations
