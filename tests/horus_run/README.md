# HORUS Run Command Test Suite

Comprehensive test suite for the `horus run` command, ensuring production-ready execution of robotics applications across multiple languages.

## Quick Start

```bash
# Run all tests
cd /home/lord-patpak/horus/HORUS/tests/horus_run
./run_all.sh
```

## Prerequisites

1. Build the HORUS CLI:
```bash
cd /home/lord-patpak/horus/HORUS
cargo build
```

2. Ensure you have required compilers:
```bash
# For Python execution
python3 --version

# For Rust compilation
cargo --version
rustc --version

# For C compilation
gcc --version  # or clang --version
```

## Test Files

| File | Purpose | Tests |
|------|---------|-------|
| `TEST_PLAN.md` | Full test documentation and plan | N/A |
| `test_python_exec.sh` | Python execution | 9 tests |
| `test_rust_exec.sh` | Rust compilation and execution | 10 tests |
| `test_c_exec.sh` | C compilation and execution | 9 tests |
| `test_autodetect.sh` | Auto-detection of main files | 11 tests |
| `test_build_modes.sh` | Debug/Release/Clean modes | 10 tests |
| `test_dependencies.sh` | Dependency resolution | 10 tests |
| `test_ipc.sh` | IPC and robotics patterns | 10 tests |
| `run_all.sh` | Master test runner | Runs all |
| `fixtures/` | Sample code for testing | 8 files |

**Total: 69 individual test cases across 7 test suites**

## Running Tests

### Run All Tests
```bash
./run_all.sh
```

### Run Individual Test Suites
```bash
./test_python_exec.sh     # Test Python execution
./test_rust_exec.sh        # Test Rust compilation and execution
./test_c_exec.sh           # Test C compilation and execution
./test_autodetect.sh       # Test auto-detection features
./test_build_modes.sh      # Test build modes (debug/release/clean)
./test_dependencies.sh     # Test dependency resolution
./test_ipc.sh              # Test IPC and robotics applications
```

### Debug Individual Tests
```bash
bash -x ./test_python_exec.sh
```

## Test Output

Each test outputs:
- ✅ **PASS** - Test succeeded
- ❌ **FAIL** - Test failed with error message

Example:
```bash
=== Testing Python Execution with horus run ===
Test 1: Run simple Python file... ✅ PASS
Test 2: Python with shebang... ✅ PASS
Test 3: Python file without extension... ✅ PASS
...
All Python execution tests passed!
```

## Test Coverage

### Language Support
- [x] Python execution and imports
- [x] Rust compilation (rustc and cargo)
- [x] C compilation (gcc/clang)
- [x] Multi-language projects

### Execution Features
- [x] Auto-detection of main files
- [x] Debug and release builds
- [x] Build caching
- [x] Clean builds
- [x] Build-only mode
- [x] Passing command-line arguments
- [x] Exit code preservation

### Dependency Management
- [x] Rust `use` statement scanning
- [x] Cargo.toml dependency resolution
- [x] Python import detection
- [x] C include detection
- [x] Missing dependency detection

### Robotics Applications
- [x] Node trait implementations
- [x] Publisher/Subscriber patterns
- [x] Sensor simulation
- [x] Control loops
- [x] Multi-threaded nodes
- [x] State machines
- [x] Data pipelines
- [x] Shared memory (Arc/Mutex)
- [x] Timing and scheduling
- [x] Error handling

### Error Handling
- [x] Compilation errors
- [x] Runtime errors
- [x] Missing files
- [x] Syntax errors
- [x] Missing dependencies

## Fixture Files

The `fixtures/` directory contains sample robotics code:

- `simple_python.py` - Basic Python hello world
- `simple_rust.rs` - Basic Rust hello world
- `simple_c.c` - Basic C hello world
- `pub_node.rs` - Publisher node using HORUS
- `sub_node.rs` - Subscriber node using HORUS
- `sensor_node.py` - Python sensor simulation
- `macro_node.rs` - Rust node using macros
- `with_args.py` - Test command-line arguments

## Test Categories

### 1. Basic Execution (28 tests)
- Python: syntax validation, imports, exit codes
- Rust: compilation, caching, panics, arguments
- C: stdlib, math library, environment variables

### 2. Auto-Detection (11 tests)
- main.py, main.rs, main.c detection
- src/main.* detection
- Single file fallback
- Cargo.toml project detection

### 3. Build Modes (10 tests)
- Debug mode (default)
- Release mode (--release)
- Clean builds (--clean)
- Build-only (--build-only)
- Flag combinations

### 4. Dependencies (10 tests)
- Rust use statements
- Cargo.toml parsing
- Python imports
- C includes
- Missing dependency errors

### 5. Robotics & IPC (10 tests)
- Node implementations
- Publisher/Subscriber
- Sensor simulations
- Control loops
- Threading and synchronization

## Adding New Tests

1. Create test file `test_feature.sh`
2. Add to `run_all.sh`:
   ```bash
   run_test "test_feature.sh" "Feature Description"
   ```
3. Update `TEST_PLAN.md` with new test cases
4. Make executable: `chmod +x test_feature.sh`

## Test Environment

Tests use isolated temporary directories:
- Pattern: `/tmp/horus_test_*_$$`
- Auto-cleanup on exit (trap EXIT)
- No interference between test runs

## Cleanup

Tests automatically clean up their temporary directories. If manual cleanup is needed:

```bash
rm -rf /tmp/horus_test_*
```

## Continuous Integration

To integrate with CI/CD:

```bash
#!/bin/bash
# In your CI script

# Build horus
cd /home/lord-patpak/horus/HORUS
cargo build

# Run tests
cd tests/horus_run
./run_all.sh

# Exit code 0 = all passed, 1 = failures
```

## Troubleshooting

### Tests Fail: "horus binary not found"
```bash
cd /home/lord-patpak/horus/HORUS
cargo build
```

### Tests Fail: Permission denied
```bash
cd tests/horus_run
chmod +x *.sh
```

### Individual Test Fails
Run with bash -x for debugging:
```bash
bash -x ./test_python_exec.sh
```

### Compilation Errors
Ensure compilers are installed:
```bash
rustc --version
python3 --version
gcc --version
```

## Success Criteria

For production readiness:
- ✅ All 69 tests must pass
- ✅ All languages execute correctly
- ✅ Auto-detection works reliably
- ✅ Build caching functions properly
- ✅ Dependencies resolve correctly
- ✅ Robotics patterns work as expected
- ✅ Error messages are helpful

## Production Launch Checklist

Before launching `horus run` to production:

- [ ] All test suites pass (69/69 tests)
- [ ] No compilation warnings in HORUS itself
- [ ] Performance benchmarks meet targets
- [ ] Documentation is complete
- [ ] Examples work for all languages
- [ ] Error messages are user-friendly
- [ ] Remote execution tested (separate suite)

## Maintenance

- Review and update tests when adding new features
- Keep TEST_PLAN.md synchronized
- Add regression tests for bug fixes
- Update README when adding new test suites

## Performance Targets

From `TEST_PLAN.md`:
- Rust compilation (single file): < 5 seconds
- IPC latency: < 1μs
- Python startup: < 2 seconds
- Cached execution: < 3 seconds

## Related Documentation

- `TEST_PLAN.md` - Comprehensive test plan with 100+ planned tests
- `TESTS_CREATED.md` - Summary of created tests
- `/home/lord-patpak/horus/HORUS/README.md` - Main HORUS documentation

## License

Part of the HORUS project. See main LICENSE file.
