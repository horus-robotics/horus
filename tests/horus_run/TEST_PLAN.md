# HORUS Run Command Test Plan

Comprehensive test suite for the `horus run` command, validating production-ready robotics application execution.

## Test Overview

The `horus run` command is the core execution engine for HORUS robotics applications. It must handle:
- Multi-language support (Rust, Python, C)
- Dependency management and auto-installation
- Build systems (cargo, gcc, make, cmake)
- IPC and shared memory communication
- Real-time scheduling
- Environment setup and library paths

### Command Signature
```bash
horus run [FILE] [OPTIONS] [-- ARGS...]
```

### Options
- `FILE` - Optional file/directory/pattern to run (auto-detects if omitted)
- `--release` - Build in release mode with optimizations
- `--clean` - Clean build cache before running
- `--build-only` - Compile but don't execute
- `--remote <ADDR>` - Execute on remote robot (tested separately)
- `-- ARGS...` - Arguments to pass to the program

## Test Categories

### 1. Language Execution Tests
Basic execution for each supported language.

| Test | Command | Validates | Test File |
|------|---------|-----------|-----------|
| Python basic | `horus run test.py` | Python execution works | `test_python_exec.sh` |
| Rust basic | `horus run test.rs` | Rust compilation and execution | `test_rust_exec.sh` |
| C basic | `horus run test.c` | C compilation and execution | `test_c_exec.sh` |
| Rust with macros | `horus run macro_node.rs` | Macro-based code works | `test_rust_macros.sh` |

### 2. Auto-Detection Tests
Tests automatic main file detection.

| Test | Scenario | Expected Result | Test File |
|------|----------|----------------|-----------|
| main.rs auto-detect | `horus run` in dir with main.rs | Finds and runs main.rs | `test_autodetect.sh` |
| main.py auto-detect | `horus run` in dir with main.py | Finds and runs main.py | `test_autodetect.sh` |
| main.c auto-detect | `horus run` in dir with main.c | Finds and runs main.c | `test_autodetect.sh` |
| src/main.rs | `horus run` with src/main.rs | Finds nested main | `test_autodetect.sh` |
| Single file fallback | Only one .py file exists | Runs that file | `test_autodetect.sh` |
| No file error | No suitable file found | ERROR with suggestions | `test_autodetect.sh` |

### 3. Directory and Pattern Tests
Running from directories and glob patterns.

| Test | Command | Expected | Test File |
|------|---------|----------|-----------|
| Run directory | `horus run src/` | Finds main in src/ | `test_directory.sh` |
| Glob pattern | `horus run "nodes/*.py"` | Runs multiple Python files | `test_patterns.sh` |
| Multiple Rust | `horus run "src/*.rs"` | Compiles and runs multiple | `test_patterns.sh` |

### 4. Build Mode Tests
Testing debug vs release builds.

| Test | Command | Validates | Test File |
|------|---------|-----------|-----------|
| Debug mode (default) | `horus run test.rs` | Compiles without -O | `test_build_modes.sh` |
| Release mode | `horus run --release test.rs` | Compiles with -O | `test_build_modes.sh` |
| Build only | `horus run --build-only test.rs` | Compiles but doesn't run | `test_build_modes.sh` |
| Clean build | `horus run --clean test.rs` | Clears cache before build | `test_build_modes.sh` |

### 5. Caching Tests
Validates build cache functionality.

| Test | Scenario | Expected | Test File |
|------|----------|----------|-----------|
| First build | Run file first time | Compiles from scratch | `test_caching.sh` |
| Cached build | Run same file again | Uses cached binary | `test_caching.sh` |
| Source modified | Modify source and run | Recompiles automatically | `test_caching.sh` |
| Cache location | Check .horus/cache/ | Binaries stored correctly | `test_caching.sh` |

### 6. Dependency Tests
Tests import scanning and dependency resolution.

| Test | Code | Expected Behavior | Test File |
|------|------|-------------------|-----------|
| Rust: use horus::* | `use horus::prelude::*;` | Auto-resolves horus | `test_dependencies.sh` |
| Python: import horus | `import horus` | Auto-resolves horus_py | `test_dependencies.sh` |
| Cargo.toml deps | Dependencies in Cargo.toml | Scans and resolves | `test_dependencies.sh` |
| Missing dependency | Import unknown package | Prompts to install | `test_dependencies.sh` |
| Auto-install yes | User accepts install | Installs from registry | `test_dependencies.sh` |
| Auto-install no | User declines install | ERROR: missing deps | `test_dependencies.sh` |

### 7. IPC and Scheduler Tests
Tests HORUS runtime integration.

| Test | Code Pattern | Validates | Test File |
|------|--------------|-----------|-----------|
| Hub creation | `Hub::new("topic")` | Shared memory IPC works | `test_ipc.sh` |
| Pub/Sub | Publisher + Subscriber | Message passing works | `test_ipc.sh` |
| Scheduler | Multiple nodes | Priority execution | `test_ipc.sh` |
| Ctrl+C handling | Send SIGINT | Graceful shutdown | `test_ipc.sh` |

### 8. Build System Tests
Tests integration with various build systems.

| Test | Build System | Command | Test File |
|------|--------------|---------|-----------|
| Cargo project | Cargo.toml | `horus run` | `test_build_systems.sh` |
| Makefile project | Makefile | `horus run` | `test_build_systems.sh` |
| CMake project | CMakeLists.txt | `horus run` | `test_build_systems.sh` |
| Single Rust file | Single .rs file | Uses cargo via generated Cargo.toml | `test_build_systems.sh` |

### 9. Environment Setup Tests
Validates library paths and environment variables.

| Test | Variable | Expected | Test File |
|------|----------|----------|-----------|
| PATH | Check PATH | .horus/bin prepended | `test_environment.sh` |
| LD_LIBRARY_PATH | Check LD_LIBRARY_PATH | .horus/lib included | `test_environment.sh` |
| PYTHONPATH | Check PYTHONPATH | .horus/packages included | `test_environment.sh` |
| Global cache | Check global libs | ~/.horus/cache libs found | `test_environment.sh` |

### 10. Python-Specific Tests
Python environment and execution.

| Test | Scenario | Expected | Test File |
|------|----------|----------|-----------|
| Python interpreter | Auto-detect python3 | Uses correct interpreter | `test_python_advanced.sh` |
| Virtual env | .horus/venv exists | Uses venv Python | `test_python_advanced.sh` |
| PYTHONPATH | Package imports | Finds HORUS packages | `test_python_advanced.sh` |
| Python wrapper | Scheduler integration | Wrapper script created | `test_python_advanced.sh` |

### 11. C-Specific Tests
C compilation and HORUS C API.

| Test | Code | Expected | Test File |
|------|------|----------|-----------|
| Basic C | Simple hello world | Compiles and runs | `test_c_advanced.sh` |
| HORUS C API | `#include <horus/node.h>` | horus.h found | `test_c_advanced.sh` |
| Compiler detection | Auto-detect gcc/clang | Uses available compiler | `test_c_advanced.sh` |
| Linking | Links with -lhorus_c | Shared library linked | `test_c_advanced.sh` |

### 12. Error Handling Tests
Tests error conditions and messages.

| Test | Error Condition | Expected Message | Test File |
|------|-----------------|------------------|-----------|
| No compiler | Rust without cargo | "No Rust compiler found" | `test_errors.sh` |
| Compilation failure | Syntax error in code | Shows compiler errors | `test_errors.sh` |
| Runtime failure | Program crashes | Exit code != 0 | `test_errors.sh` |
| Missing file | File doesn't exist | "File not found" | `test_errors.sh` |
| Unsupported extension | .txt file | "Unsupported file type" | `test_errors.sh` |

### 13. Robotics Application Tests
Real-world robotics use cases.

| Test | Application Type | Components | Test File |
|------|------------------|------------|-----------|
| Single node | Publisher node | Hub, publish loop | `test_robotics.sh` |
| Multi-node | Pub+Sub nodes | IPC communication | `test_robotics.sh` |
| Sensor node | Read + publish | Simulated sensor data | `test_robotics.sh` |
| Control node | Subscribe + command | Motor control logic | `test_robotics.sh` |
| Full system | 3+ nodes | Complete robot app | `test_robotics.sh` |

### 14. Performance Tests
Validates performance characteristics.

| Test | Metric | Target | Test File |
|------|--------|--------|-----------|
| Compile time (cargo) | Single file | < 5 seconds | `test_performance.sh` |
| IPC latency | Message passing | < 1μs | `test_performance.sh` |
| Startup time | Python node | < 2 seconds | `test_performance.sh` |
| Memory usage | Running node | Reasonable | `test_performance.sh` |

## Test Execution

### Prerequisites
```bash
# Build HORUS
cd /home/lord-patpak/horus/HORUS
cargo build

# Ensure compilers available
rustc --version
python3 --version
gcc --version  # or clang
```

### Run All Tests
```bash
cd /home/lord-patpak/horus/HORUS/tests/horus_run
./run_all.sh
```

### Run Individual Test Suites
```bash
./test_python_exec.sh       # Python execution
./test_rust_exec.sh          # Rust execution
./test_c_exec.sh             # C execution
./test_autodetect.sh         # Auto-detection
./test_dependencies.sh       # Dependency resolution
./test_ipc.sh                # IPC and messaging
./test_build_systems.sh      # Build system integration
./test_robotics.sh           # Robotics applications
```

## Success Criteria

For production readiness:
- All language executions work (Python, Rust, C)
- Auto-detection finds correct main files
- Build caching works correctly
- Dependencies auto-install
- IPC communication verified
- Environment variables set correctly
- All build systems supported
- Error messages are helpful
- Real robotics apps run successfully
- Performance meets targets

## Test File Organization

```
tests/horus_run/
── TEST_PLAN.md              # This file
── README.md                 # Quick start guide
── run_all.sh                # Master test runner

── test_python_exec.sh       # Python execution tests
── test_rust_exec.sh         # Rust execution tests
── test_c_exec.sh            # C execution tests
── test_rust_macros.sh       # Rust macro tests

── test_autodetect.sh        # Auto-detection tests
── test_directory.sh         # Directory execution
── test_patterns.sh          # Glob pattern tests

── test_build_modes.sh       # Debug/release/clean
── test_caching.sh           # Build cache tests

── test_dependencies.sh      # Dependency resolution
── test_ipc.sh               # IPC and messaging
── test_build_systems.sh     # Cargo/Make/CMake
── test_environment.sh       # Environment variables

── test_python_advanced.sh   # Python-specific
── test_c_advanced.sh        # C-specific
── test_errors.sh            # Error handling
── test_robotics.sh          # Robotics applications
── test_performance.sh       # Performance validation

── fixtures/                 # Test code samples
    ── simple_python.py
    ── simple_rust.rs
    ── simple_c.c
    ── pub_node.rs
    ── sub_node.rs
    ── multi_node/
    ── with_deps/
```

## Coverage

This test suite covers:
- **Languages:** 100% (Python, Rust, C)
- **Execution modes:** 100% (single, directory, pattern, multiple)
- **Build modes:** 100% (debug, release, clean, build-only)
- **Build systems:** 100% (cargo, gcc, make, cmake)
- **IPC:** Core messaging, pub/sub, scheduling
- **Dependencies:** Auto-detection, resolution, installation
- **Environment:** All required variables
- **Error cases:** All major failure scenarios
- **Real-world:** Multi-node robotics applications

**Total Estimated Tests: 100+ individual test cases across 15 test suites**
