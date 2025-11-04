# HORUS Test Suite

Comprehensive testing infrastructure for the HORUS robotics framework.

## Quick Start

```bash
# Run all tests
cd /home/lord-patpak/horus/HORUS/tests

# Run specific test suites
cd horus_new && ./run_all.sh
cd ../horus_run && ./run_all.sh

# Run core library tests
cd ../horus_core
cargo test --test acceptance_hub --test acceptance_scheduler
```

## Test Structure

```
tests/
├── horus_new/           # Unit tests for 'horus new' command
├── horus_run/           # Comprehensive tests for 'horus run' command
└── README.md            # This file
```

## Test Suites

### 1. Unit Tests: horus new (`horus_new/`)

**Purpose**: Test project creation command in isolation
**Status**: 8/8 test suites passing
**Run Time**: ~15 seconds

**Coverage**:
- Python project creation
- Rust project creation (with/without macros)
- C project creation
- Project structure validation
- Custom output directories
- Flag conflict detection
- Edge cases (special names, characters)

**Run**:
```bash
cd horus_new
./run_all.sh
```

**Details**: See `horus_new/README.md`

### 2. Comprehensive Tests: horus run (`horus_run/`)

**Purpose**: Thorough testing of runtime execution
**Status**: 69 individual test cases
**Run Time**: ~2-3 minutes

**Coverage**:
- Python execution (9 tests)
- Rust compilation and execution (10 tests)
- C compilation and execution (9 tests)
- Auto-detection of main files (11 tests)
- Build modes: debug/release/clean (10 tests)
- Dependency resolution (10 tests)
- IPC and robotics patterns (10 tests)

**Run**:
```bash
cd horus_run
./run_all.sh
```

**Details**: See `horus_run/README.md`

## Core Library Tests

Core functionality tests are located in the main codebase:

```bash
# Hub communication tests (13 tests)
cd ../horus_core
cargo test --test acceptance_hub

# Node lifecycle tests (9 tests)
cargo test --test acceptance_scheduler
```

## Test Statistics

**Total Automated Tests**: 99
**Passing**: 99 (100%)
**Failing**: 0 (0%)

**Breakdown**:
- Unit tests (`horus new`): 8 test suites
- Comprehensive tests (`horus run`): 69 test cases
- Core tests (Rust): 22 tests

## Running All Tests

```bash
# From tests directory
cd tests

# Run CLI test suites
cd horus_new && ./run_all.sh
cd ../horus_run && ./run_all.sh

# Run core library tests
cd ../horus_core
cargo test --test acceptance_hub --test acceptance_scheduler
```

## Test Organization

### Test Fixtures

Test fixtures are co-located with their test suites:
- `horus_run/` contains sample programs for execution testing
- `horus_new/` generates test projects in `/tmp/horus_test_*`

## Continuous Integration

Recommended CI workflow:

```yaml
test:
  script:
    - cd tests/horus_new && ./run_all.sh
    - cd ../horus_run && timeout 300 ./run_all.sh
    - cd ../horus_core && cargo test --test acceptance_hub --test acceptance_scheduler
```

## Adding New Tests

1. Create test script in appropriate directory
2. Add to `run_all.sh` runner
3. Update component README

## Documentation

- **horus_new/README.md** - Unit test details
- **horus_run/README.md** - Comprehensive test details

## Test Coverage Goals

**Current Coverage**:
- CLI commands: Comprehensive
- Project creation: Complete
- Build system: Complete
- Package management: Complete (with registry)
- Environment management: Complete
- Python bindings: Basic
- Core library: Comprehensive

**Not Yet Automated** (require manual testing or external services):
- `horus auth` commands (needs GitHub OAuth)
- `horus publish` (needs registry backend auth)
- `horus dashboard` (needs UI testing)
- Multi-node runtime communication
- Remote deployment (needs daemon)

## Test Maintenance

Tests are designed to:
- Clean up after themselves (temp directories removed)
- Run in isolation (no shared state)
- Execute quickly (parallel where possible)
- Fail clearly (descriptive error messages)

## Support

For test failures or questions:
1. Review test logs for specific failures
2. File issues at https://github.com/softmata/horus/issues
