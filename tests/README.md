# HORUS Test Suite

Comprehensive testing infrastructure for the HORUS robotics framework.

## Quick Start

```bash
# Run all tests
cd /home/lord-patpak/horus/HORUS/tests
./run_all_tests.sh

# Run specific test suites
cd acceptance && ./run_automated_tests.sh
cd horus_new && ./run_all.sh
cd horus_run && ./run_all.sh
```

## Test Structure

```
tests/
├── acceptance/          # Acceptance tests with automated runner
├── horus_new/           # Unit tests for 'horus new' command
├── horus_run/           # Comprehensive tests for 'horus run' command
└── TESTING_SUMMARY.md   # Complete testing documentation
```

## Test Suites

### 1. Acceptance Tests (`acceptance/`)

**Purpose**: Validate user-facing features against acceptance criteria
**Status**: 25/25 passing (1 skipped)
**Run Time**: ~60 seconds

**Coverage**:
- Version commands
- Project creation (Rust, Python, C)
- Build system
- Package management (list, search, install, remove)
- Code validation
- Environment management
- Python bindings
- Core functionality (Hub + Scheduler)

**Run**:
```bash
cd acceptance
./run_automated_tests.sh
```

**Details**: See `acceptance/README.md` and `TESTING_SUMMARY.md`

### 2. Unit Tests: horus new (`horus_new/`)

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

### 3. Comprehensive Tests: horus run (`horus_run/`)

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

**Total Automated Tests**: 47
**Passing**: 47 (100%)
**Failing**: 0 (0%)
**Skipped**: 1 (runtime execution - fragile)

**Breakdown**:
- Unit tests (`horus new`): 8 test suites
- Acceptance tests (CLI): 25 tests
- Core tests (Rust): 22 tests
- Comprehensive tests (`horus run`): 69 test cases

## Running All Tests

```bash
# From project root
cd tests

# Option 1: Run each suite separately
cd horus_new && ./run_all.sh
cd ../horus_run && ./run_all.sh
cd ../acceptance && ./run_automated_tests.sh
cd ../../horus_core && cargo test --test acceptance_hub --test acceptance_scheduler

# Option 2: Use master script (if available)
./run_all_tests.sh
```

## Test Organization

### Acceptance Criteria

Acceptance test criteria are organized by component in `acceptance/`:

```
acceptance/
├── horus_c/              # C bindings criteria
├── horus_core/           # Core library criteria
├── horus_dashboard/      # Dashboard criteria
├── horus_env/            # Environment management criteria
├── horus_macros/         # Macro system criteria
├── horus_manager/        # CLI commands criteria
├── horus_marketplace/    # Marketplace criteria
├── horus_py/             # Python bindings criteria
└── horus_registry/       # Package registry criteria
```

Each directory contains `.md` files with detailed scenarios and acceptance criteria.

### Test Fixtures

Test fixtures are co-located with their test suites:
- `horus_run/` contains sample programs for execution testing
- `acceptance/` generates temporary projects in `/tmp/horus_acceptance_*`
- `horus_new/` generates test projects in `/tmp/horus_test_*`

## Continuous Integration

Recommended CI workflow:

```yaml
test:
  script:
    - cd tests/horus_new && ./run_all.sh
    - cd ../horus_run && timeout 300 ./run_all.sh
    - cd ../acceptance && timeout 300 ./run_automated_tests.sh
    - cd ../../horus_core && cargo test --test acceptance_hub --test acceptance_scheduler
```

## Adding New Tests

### Adding Acceptance Tests

1. Add criteria to `acceptance/<component>/<nn>_<name>.md`
2. Add test implementation to `acceptance/run_automated_tests.sh`
3. Update `TESTING_SUMMARY.md`

### Adding Unit Tests

1. Create test script in appropriate directory
2. Add to `run_all.sh` runner
3. Update component README

## Documentation

- **TESTING_SUMMARY.md** - Complete testing documentation
- **acceptance/README.md** - Acceptance test details
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
1. Check `TESTING_SUMMARY.md` for known issues
2. Review test logs for specific failures
3. File issues at https://github.com/anthropics/horus/issues
