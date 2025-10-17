# HORUS New Command Test Suite

Comprehensive test suite for the `horus new` command, ensuring production readiness.

## Quick Start

```bash
# Run all tests
cd /home/lord-patpak/horus/HORUS/tests/horus_new
./run_all.sh
```

## Prerequisites

1. Build the HORUS CLI:
```bash
cd /home/lord-patpak/horus/HORUS
cargo build
```

2. Ensure you have required tools:
```bash
# For Python syntax validation
python3 --version

# For Rust validation
cargo --version
rustc --version
```

## Test Files

| File | Purpose | Tests |
|------|---------|-------|
| `TEST_PLAN.md` | Full test documentation and plan | N/A |
| `test_python.sh` | Python project creation | 9 tests |
| `test_rust.sh` | Rust project creation (no macros) | 11 tests |
| `test_rust_macro.sh` | Rust project with macros | 8 tests |
| `test_c.sh` | C project creation | 8 tests |
| `test_structure.sh` | Project structure validation | 15 tests |
| `test_output_dir.sh` | Custom output directory | 4 tests |
| `test_conflicts.sh` | Flag conflict handling | 6 tests |
| `test_edge_cases.sh` | Edge cases and boundaries | 7 tests |
| `run_all.sh` | Master test runner | Runs all |

**Total: 68 individual test cases across 8 test suites**

## Running Tests

### Run All Tests
```bash
./run_all.sh
```

### Run Individual Test Suites
```bash
./test_python.sh          # Test Python projects
./test_rust.sh            # Test Rust projects
./test_rust_macro.sh      # Test Rust with macros
./test_c.sh               # Test C projects
./test_structure.sh       # Test project structure
./test_output_dir.sh      # Test output directories
./test_conflicts.sh       # Test flag conflicts
./test_edge_cases.sh      # Test edge cases
```

## Test Output

Each test outputs:
- **PASS** - Test succeeded
- **FAIL** - Test failed with error message
- **WARN** - Test passed with warnings

Example:
```bash
=== Testing Python Project Creation ===
Test 1: Create Python project with -p flag...  PASS
Test 2: Project directory exists...  PASS
Test 3: main.py file exists...  PASS
...
All Python tests passed!
```

## Test Coverage

### Language Support
- [x] Python projects (`-p`)
- [x] Rust projects (`-r`)
- [x] Rust with macros (`-m`)
- [x] C projects (`-c`)

### Flag Combinations
- [x] Single language flags
- [x] Macro + Rust combination
- [x] Output directory option
- [x] All conflict scenarios

### File Validation
- [x] Project structure
- [x] horus.yaml content
- [x] Language-specific files
- [x] Cargo.toml (Rust)
- [x] Python syntax
- [x] Directory hierarchy

### Edge Cases
- [x] Hyphenated names
- [x] Numbers in names
- [x] Single character names
- [x] Very long names
- [x] Existing directories
- [x] Special characters

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
cd tests/horus_new
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
cd tests/horus_new
chmod +x *.sh
```

### Individual Test Fails
Run with bash -x for debugging:
```bash
bash -x ./test_python.sh
```

## Success Criteria

For production readiness:
- All 68 tests must pass
- Zero failures
- No warnings (except documented edge cases)

## Maintenance

- Review and update tests when adding new features
- Keep TEST_PLAN.md synchronized
- Add regression tests for bug fixes
- Update README when adding new test suites

## License

Part of the HORUS project. See main LICENSE file.
