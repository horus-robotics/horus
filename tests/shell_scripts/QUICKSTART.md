# Shell Script Testing - Quick Start

Test your installation scripts on clean systems before pushing to production.

## Prerequisites

```bash
# Install Docker (if not already installed)
# Ubuntu/Debian:
sudo apt-get update
sudo apt-get install docker.io
sudo usermod -aG docker $USER
newgrp docker

# Verify Docker works
docker run hello-world
```

## Run Tests

### 1. Test Single Script (Fastest)

```bash
cd tests/shell_scripts

# Test install.sh on clean Ubuntu 22.04
./run_tests.sh install

# Test verify.sh
./run_tests.sh verify
```

### 2. Test Full Installation Flow

```bash
# Tests install → verify → update → uninstall in sequence
./run_tests.sh full-flow
```

### 3. Test on Different Distribution

```bash
# Test on Ubuntu 24.04
./run_tests.sh install ubuntu-24.04

# Test on Debian 12
./run_tests.sh install debian-12

# Test on Fedora 39
./run_tests.sh install fedora-39
```

### 4. Test All Scripts on All Distros

```bash
# This takes ~30-40 minutes
./run_tests.sh all all-distros
```

## Debugging Failed Tests

### Keep Container for Inspection

```bash
# Container stays alive after test
./run_tests.sh install --keep

# Find container name
docker ps -a | grep horus-test

# Inspect the container
docker exec -it <container-name> bash

# Inside container, check installation
ls -la ~/.cargo/bin/
ls -la ~/.horus/
cat ~/.horus/installed_version
```

### Rebuild Docker Images

```bash
# Force rebuild (no cache)
./run_tests.sh install --no-cache
```

### Verbose Output

```bash
# See all commands executed
./run_tests.sh install --verbose
```

## Common Test Scenarios

### Before Pushing Changes

```bash
# Quick smoke test (2-3 minutes)
./run_tests.sh install ubuntu-22.04
./run_tests.sh verify ubuntu-22.04
```

### Before Release

```bash
# Full test on primary platforms (10-15 minutes)
./run_tests.sh full-flow ubuntu-22.04
./run_tests.sh full-flow debian-12
```

### Testing Specific Bug Fix

```bash
# If you fixed install.sh, test just that
./run_tests.sh install ubuntu-22.04

# Keep container to verify the fix
./run_tests.sh install ubuntu-22.04 --keep
```

## Understanding Test Results

### Success Output
```
[INFO] Building Docker image for ubuntu-22.04...
[PASS] Image built: horus-shell-test:ubuntu-22.04
[INFO] Starting container: horus-test-install-ubuntu-22.04-12345
[PASS] Binary is executable
[PASS] Library installed: horus
[PASS] Test passed: install on ubuntu-22.04
```

### Failure Output
```
[FAIL] Binary not found: horus
[FAIL] Test failed: install on ubuntu-22.04 (exit code: 1)
```

## Integration with CI/CD

### Automatic Testing

Tests run automatically on:
- Every push to `main`, `dev`, `develop` (smoke test only)
- Every pull request (smoke test only)
- Weekly schedule (full matrix)
- Manual trigger via GitHub Actions

### Manual Trigger from GitHub

1. Go to Actions tab
2. Select "Shell Scripts Tests"
3. Click "Run workflow"
4. Choose test type and distro
5. Click "Run workflow"

### Trigger Full Test from Commit

```bash
git commit -m "fix: install.sh improvements [test-all-distros]"
git push
```

## Cleanup

### Remove All Test Containers
```bash
docker ps -a | grep horus-test | awk '{print $1}' | xargs docker rm -f
```

### Remove Test Images
```bash
docker images | grep horus-shell-test | awk '{print $3}' | xargs docker rmi -f
```

### Full Docker Cleanup
```bash
# WARNING: Removes ALL unused Docker data
docker system prune -af
```

## Performance Tips

### Use Docker Layer Caching

Docker caches layers, so subsequent runs are faster:
- First run: ~5 minutes (builds image)
- Subsequent runs: ~2 minutes (uses cached image)

### Parallel Testing

Test multiple distros simultaneously:
```bash
# Terminal 1
./run_tests.sh install ubuntu-22.04 &

# Terminal 2
./run_tests.sh install debian-12 &

# Wait for both
wait
```

## What Each Test Validates

### test_install.sh
- Detects missing dependencies (Rust, system libs)
- Builds all packages from source
- Installs binaries to correct location
- Creates cache structure
- Verifies `horus --version` works
- Checks all libraries installed

### test_verify.sh
- Runs system requirement checks
- Validates installation integrity
- Tests basic functionality
- Reports accurate status codes

### test_uninstall.sh
- Removes all binaries
- Cleans library cache
- Removes shared memory files
- Leaves no artifacts

## Troubleshooting

### "Permission denied" on Docker
```bash
sudo usermod -aG docker $USER
newgrp docker
```

### "Cannot connect to Docker daemon"
```bash
sudo systemctl start docker
sudo systemctl enable docker
```

### Tests pass locally but fail in CI
- Check Docker version: `docker --version`
- Clean state: `docker system prune -af`
- Rebuild images: `./run_tests.sh install --no-cache`

### Out of disk space
```bash
# Check Docker disk usage
docker system df

# Clean up
docker system prune -af --volumes
```

## Next Steps

1. Run smoke test before every push
2. Run full flow test before releases
3. Check GitHub Actions results after push
4. Add new test scenarios as needed

For more details, see [README.md](README.md)
