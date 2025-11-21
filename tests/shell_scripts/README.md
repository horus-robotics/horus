# Shell Script Testing Framework

Automated testing for HORUS installation and maintenance scripts on clean systems.

## What This Tests

This framework tests all HORUS shell scripts on **completely fresh** systems with no dependencies:

- `install.sh` - Fresh installation
- `update.sh` - Update from previous version
- `verify.sh` - Installation verification
- `recovery_install.sh` - Recovery from broken state
- `uninstall.sh` - Clean removal

## Supported Platforms

Tests run on Docker containers simulating:
- **Ubuntu 22.04** (primary target)
- **Ubuntu 24.04** (latest LTS)
- **Debian 12** (Bookworm)
- **Fedora 39** (RPM-based)

## Quick Start

### Run All Tests Locally

```bash
cd tests/shell_scripts
./run_tests.sh
```

### Run Specific Script Test

```bash
./run_tests.sh install      # Test install.sh only
./run_tests.sh verify       # Test verify.sh only
./run_tests.sh all-distros  # Test on all distros
```

### Run on Specific Distro

```bash
./run_tests.sh install ubuntu-22.04
./run_tests.sh install fedora-39
```

## How It Works

1. **Clean Container**: Spins up fresh Docker container with **zero dependencies**
2. **Install System Deps**: Installs only what the script requires
3. **Run Script**: Executes the shell script
4. **Verify Success**: Checks exit codes and validates installation
5. **Cleanup**: Removes container

## Test Structure

```
tests/shell_scripts/
├── README.md                  # This file
├── run_tests.sh              # Main test runner
├── dockerfiles/              # Clean system images
│   ├── ubuntu-22.04.Dockerfile
│   ├── ubuntu-24.04.Dockerfile
│   ├── debian-12.Dockerfile
│   └── fedora-39.Dockerfile
├── test_scenarios/           # Test cases
│   ├── test_install.sh
│   ├── test_update.sh
│   ├── test_verify.sh
│   ├── test_recovery.sh
│   └── test_uninstall.sh
└── helpers/                  # Shared utilities
    └── common.sh
```

## CI/CD Integration

GitHub Actions automatically runs these tests on:
- Every push to `main` or `dev`
- Every pull request
- Weekly scheduled runs

See `.github/workflows/test-shell-scripts.yml`

## Adding New Tests

1. Create test scenario in `test_scenarios/test_<name>.sh`
2. Follow the template:

```bash
#!/bin/bash
# Test: <description>

source "$(dirname "$0")/../helpers/common.sh"

test_<name>() {
    log_info "Testing <script name>"

    # Your test logic here
    run_script "<script>.sh"

    assert_exit_code 0
    assert_command_exists "horus"
}

run_test test_<name>
```

3. Add to `run_tests.sh` test list

## Requirements

- **Docker** (for containerized testing)
- **Bash 4.0+**
- ~2GB disk space (for Docker images)

## Performance

- Single script test: ~2-5 minutes
- All scripts on one distro: ~10-15 minutes
- All scripts on all distros: ~30-40 minutes

Much faster than full CI/CD matrix builds!

## Troubleshooting

### Docker Permission Denied
```bash
sudo usermod -aG docker $USER
newgrp docker
```

### Tests Failing Locally But Pass in CI
- Check Docker version: `docker --version`
- Ensure clean state: `docker system prune -af`

### Container Cleanup
```bash
# Remove all test containers
docker ps -a | grep horus-shell-test | awk '{print $1}' | xargs docker rm -f

# Remove test images
docker images | grep horus-shell-test | awk '{print $3}' | xargs docker rmi -f
```

## Examples

### Test install.sh on Fresh Ubuntu
```bash
./run_tests.sh install ubuntu-22.04
```

### Test Full Installation Flow
```bash
# Install → Verify → Update → Uninstall
./run_tests.sh full-flow ubuntu-22.04
```

### Parallel Testing (Fast)
```bash
# Run all distros in parallel
./run_tests.sh install --parallel
```

## What Gets Validated

### install.sh
- Detects missing dependencies
- Installs Rust if needed
- Builds from source
- Creates cache structure
- Binary works (`horus --version`)
- Libraries installed correctly

### verify.sh
- Checks system requirements
- Validates installation
- Tests functionality
- Reports accurate status

### update.sh
- Updates from git
- Rebuilds if needed
- Preserves user config
- Migrates versions correctly

### recovery_install.sh
- Diagnoses issues
- Cleans corrupted state
- Fresh reinstall works
- Verification passes

### uninstall.sh
- Removes all binaries
- Cleans cache
- Prompts for config removal
- No leftover files

## Why This Approach?

**Problem**: CI/CD runners have pre-installed dependencies, don't simulate real user machines

**Solution**: Docker containers with **completely clean** base images

**Benefits**:
- Tests real user experience
- Faster than full CI matrix
- Run locally before pushing
- Catch dependency issues early
- Multiple distros covered
