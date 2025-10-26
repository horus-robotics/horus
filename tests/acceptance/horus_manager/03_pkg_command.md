# User Acceptance Test: `horus pkg` Commands

## Feature
Package management for discovering, installing, and managing HORUS packages from the registry.

## User Story
As a robotics developer, I want to easily find and install community packages so that I can reuse nodes, messages, and tools without reinventing the wheel.

## Test Scenarios

### Scenario 1: Search for Packages
**Given:** Registry has published packages
**When:** User runs `horus pkg search lidar`
**Then:**
- [ ] All packages matching "lidar" are displayed
- [ ] Results show package name, version, and description
- [ ] Results are formatted clearly
- [ ] Empty search returns all packages

**Acceptance Criteria:**
```bash
$ horus pkg search lidar
Found 3 packages:

 lidar-driver v1.2.0
   USB Lidar sensor driver with ROS compatibility
   Author: robotics-team

 lidar-slam v0.5.1
   Real-time SLAM using lidar data
   Author: slam-lab

 multi-sensor v2.1.0
   Multi-sensor fusion including lidar, IMU, GPS
   Author: sensors-inc
```

### Scenario 2: Search with No Results
**Given:** No packages match query
**When:** User runs `horus pkg search nonexistent-package`
**Then:**
- [ ] Message: "No packages found matching 'nonexistent-package'"
- [ ] Suggestion to check spelling or browse all packages
- [ ] Exit code 0 (not an error)

### Scenario 3: Install Package
**Given:** User wants to use a package
**When:** User runs `horus pkg install lidar-driver`
**Then:**
- [ ] Package is downloaded from registry
- [ ] Dependencies are resolved and downloaded
- [ ] Package is cached in `~/.horus/cache/`
- [ ] Success message with version installed
- [ ] Package can be imported in projects

**Acceptance Criteria:**
```bash
$ horus pkg install lidar-driver
Fetching lidar-driver from registry...
Resolving dependencies...
  ─ horus-core v0.1.0
  ─ sensor-common v1.0.2
Installing lidar-driver v1.2.0...
 Installed successfully
```

### Scenario 4: Install Specific Version
**Given:** User needs specific version
**When:** User runs `horus pkg install lidar-driver@1.1.0`
**Then:**
- [ ] Specified version is installed
- [ ] Newer versions are ignored
- [ ] Dependencies compatible with 1.1.0 are installed
- [ ] Confirmation shows exact version

**Acceptance Criteria:**
```bash
$ horus pkg install lidar-driver@1.1.0
Installing lidar-driver v1.1.0...
 Installed lidar-driver v1.1.0
```

### Scenario 5: Install Already Installed Package
**Given:** Package is already in cache
**When:** User runs `horus pkg install lidar-driver`
**Then:**
- [ ] Message: "lidar-driver v1.2.0 is already installed"
- [ ] No download occurs
- [ ] Suggestion to use `--force` to reinstall
- [ ] Exit code 0

### Scenario 6: Force Reinstall
**Given:** Package is already installed
**When:** User runs `horus pkg install lidar-driver --force`
**Then:**
- [ ] Existing package is removed
- [ ] Fresh download from registry
- [ ] Package is reinstalled
- [ ] Confirmation of reinstall

### Scenario 7: Install Non-Existent Package
**Given:** Package doesn't exist in registry
**When:** User runs `horus pkg install fake-package`
**Then:**
- [ ] Error: "Package 'fake-package' not found in registry"
- [ ] Suggestion to run `horus pkg search`
- [ ] Exit code is non-zero

### Scenario 8: List Installed Packages
**Given:** User has installed packages
**When:** User runs `horus pkg list`
**Then:**
- [ ] All installed packages are shown
- [ ] Each entry shows name, version, install date
- [ ] If no packages: "No packages installed"

**Acceptance Criteria:**
```bash
$ horus pkg list
Installed packages:

lidar-driver v1.2.0 (installed 2 days ago)
sensor-common v1.0.2 (installed 2 days ago)
slam-toolkit v0.8.1 (installed 1 week ago)

Total: 3 packages
```

### Scenario 9: Remove Package
**Given:** User wants to uninstall a package
**When:** User runs `horus pkg remove lidar-driver`
**Then:**
- [ ] Confirmation prompt: "Remove lidar-driver v1.2.0? [y/N]"
- [ ] User confirms with 'y'
- [ ] Package is removed from cache
- [ ] Success message
- [ ] Dependent packages are NOT removed

**Acceptance Criteria:**
```bash
$ horus pkg remove lidar-driver
Remove lidar-driver v1.2.0? [y/N] y
Removing lidar-driver v1.2.0...
 Removed successfully
```

### Scenario 10: Remove with Force (No Prompt)
**Given:** User wants non-interactive removal
**When:** User runs `horus pkg remove lidar-driver --force`
**Then:**
- [ ] No confirmation prompt
- [ ] Package is removed immediately
- [ ] Success message

### Scenario 11: Remove Non-Existent Package
**Given:** Package is not installed
**When:** User runs `horus pkg remove fake-package`
**Then:**
- [ ] Error: "Package 'fake-package' is not installed"
- [ ] Suggestion to run `horus pkg list`
- [ ] Exit code is non-zero

### Scenario 12: Show Package Details
**Given:** User wants detailed package info
**When:** User runs `horus pkg info lidar-driver`
**Then:**
- [ ] Full package metadata is displayed
- [ ] Name, version, author, description
- [ ] Dependencies listed
- [ ] Repository URL (if available)
- [ ] Installation status

**Acceptance Criteria:**
```bash
$ horus pkg info lidar-driver

 lidar-driver v1.2.0

Description:
  USB Lidar sensor driver with ROS compatibility

Author: robotics-team
Repository: https://github.com/robotics-team/lidar-driver
License: MIT

Dependencies:
  - horus-core ^0.1.0
  - sensor-common ^1.0.0

Status: Installed
Installed: 2 days ago
Location: ~/.horus/cache/lidar-driver-1.2.0/
```

### Scenario 13: Update Package
**Given:** Newer version is available
**When:** User runs `horus pkg update lidar-driver`
**Then:**
- [ ] Latest version is fetched
- [ ] Old version is replaced
- [ ] Dependencies are updated if needed
- [ ] Confirmation of new version

**Acceptance Criteria:**
```bash
$ horus pkg update lidar-driver
Checking for updates...
Found lidar-driver v1.3.0 (current: v1.2.0)
Updating lidar-driver...
 Updated to v1.3.0
```

### Scenario 14: Update All Packages
**Given:** Multiple packages have updates
**When:** User runs `horus pkg update --all`
**Then:**
- [ ] All packages are checked for updates
- [ ] List of updates is shown
- [ ] User confirms update
- [ ] All packages are updated

### Scenario 15: Unpublish Package from Registry
**Given:** User has previously published a package to the registry
**When:** User runs `horus pkg unpublish my-package --version 1.0.0`
**Then:**
- [ ] Confirmation prompt: "Unpublish my-package v1.0.0? This cannot be undone. [y/N]"
- [ ] User confirms with 'y'
- [ ] Package version is removed from registry
- [ ] Success message confirms removal
- [ ] Package is no longer searchable or installable

**Acceptance Criteria:**
```bash
$ horus pkg unpublish my-package --version 1.0.0
  Warning: Unpublishing a package removes it permanently from the registry.
    Users who depend on this version will not be able to install it.

Unpublish my-package v1.0.0? [y/N] y
Removing my-package v1.0.0 from registry...
 Package unpublished successfully
```

### Scenario 16: Unpublish with --yes Flag (Skip Confirmation)
**Given:** User wants non-interactive unpublish
**When:** User runs `horus pkg unpublish my-package --version 1.0.0 --yes`
**Then:**
- [ ] No confirmation prompt
- [ ] Package is removed immediately
- [ ] Success message shown

### Scenario 17: Unpublish Non-Existent Package
**Given:** Package or version doesn't exist in registry
**When:** User runs `horus pkg unpublish fake-package --version 1.0.0`
**Then:**
- [ ] Error: "Package 'fake-package' version 1.0.0 not found in registry"
- [ ] Exit code is non-zero

### Scenario 18: Unpublish Without Authentication
**Given:** User is not logged in
**When:** User runs `horus pkg unpublish my-package --version 1.0.0`
**Then:**
- [ ] Error: "Authentication required"
- [ ] Suggestion to run `horus auth login`
- [ ] Exit code is non-zero

## Edge Cases

### Edge Case 1: Network Failure During Install
**Given:** Network connection lost during download
**When:** User runs `horus pkg install lidar-driver`
**Then:**
- [ ] Error: "Failed to download package: Network error"
- [ ] Partial download is cleaned up
- [ ] Cache remains in consistent state
- [ ] Retry suggestion is shown

### Edge Case 2: Corrupted Package
**Given:** Downloaded package is corrupted
**When:** Installation proceeds
**Then:**
- [ ] Checksum verification fails
- [ ] Error: "Package verification failed"
- [ ] Corrupted package is not installed
- [ ] Suggestion to retry installation

### Edge Case 3: Dependency Conflict
**Given:** Package A requires horus-core v0.1.x
**And:** Package B requires horus-core v0.2.x
**When:** User tries to install both
**Then:**
- [ ] Conflict is detected
- [ ] Clear error message explaining conflict
- [ ] Suggestion to resolve manually
- [ ] Exit code is non-zero

### Edge Case 4: Insufficient Disk Space
**Given:** Not enough disk space for package
**When:** User runs `horus pkg install large-package`
**Then:**
- [ ] Disk space is checked before download
- [ ] Error: "Insufficient disk space"
- [ ] Shows required vs available space
- [ ] No partial installation

## Help Documentation

**When:** User runs `horus pkg --help`
**Then:**
- [ ] All subcommands are listed
- [ ] Brief description of each command
- [ ] Usage examples shown

**Acceptance Criteria:**
```bash
$ horus pkg --help
Manage HORUS packages

Usage: horus pkg <COMMAND>

Commands:
  search   Search for packages in the registry
  install  Install a package
  remove   Remove an installed package
  list     List installed packages
  info     Show detailed package information
  update   Update a package to latest version
  help     Print this message or help for subcommand

Options:
  -h, --help  Print help
```

## Non-Functional Requirements

- [ ] Search completes in < 2 seconds
- [ ] Install shows progress for large packages
- [ ] Commands work offline with local cache (where applicable)
- [ ] Clear error messages for all failure cases
- [ ] Atomic operations (install/remove complete fully or not at all)
- [ ] Cross-platform path handling
- [ ] Respects $HOME environment variable
