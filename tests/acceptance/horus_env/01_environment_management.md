# User Acceptance Test: Environment Management (`horus env`)

## Feature
Freeze and restore project environments for reproducible builds and shareable development setups.

## User Story
As a robotics developer, I want to freeze my project's dependency environment and restore it on other machines so that my team can work with identical package versions and ensure reproducible builds.

## Test Scenarios

### Scenario 1: Freeze Current Environment
**Given:** User has a project with installed packages
**When:** User runs `horus env freeze`
**Then:**
- [ ] All installed packages are recorded with exact versions
- [ ] File `horus-freeze.yaml` is created in current directory
- [ ] File includes system information (OS, architecture)
- [ ] File includes HORUS version used
- [ ] Success message lists all frozen packages
- [ ] File is human-readable YAML format

**Acceptance Criteria:**
```bash
$ horus env freeze
 Freezing environment...
 Recorded 5 packages:
  • horus-core v0.1.0
  • lidar-driver v1.2.0
  • sensor-common v1.0.2
  • slam-toolkit v0.8.1
  • nav-stack v2.1.0
 Environment frozen to horus-freeze.yaml
```

**Example `horus-freeze.yaml`:**
```yaml
# HORUS Environment Freeze File
# Generated: 2024-10-24 12:00:00 UTC
# HORUS Version: 0.1.0

system:
  os: linux
  arch: x86_64
  horus_version: 0.1.0

packages:
  - name: horus-core
    version: 0.1.0
    source: registry
  - name: lidar-driver
    version: 1.2.0
    source: registry
  - name: sensor-common
    version: 1.0.2
    source: registry
  - name: slam-toolkit
    version: 0.8.1
    source: registry
  - name: nav-stack
    version: 2.1.0
    source: registry
```

### Scenario 2: Freeze with Custom Output Path
**Given:** User wants to save freeze file to specific location
**When:** User runs `horus env freeze --output ./config/production.yaml`
**Then:**
- [ ] Freeze file created at specified path
- [ ] Directories are created if they don't exist
- [ ] Success message confirms custom path
- [ ] File format same as default

**Acceptance Criteria:**
```bash
$ horus env freeze --output ./config/production.yaml
 Freezing environment...
 Recorded 5 packages
 Environment frozen to ./config/production.yaml
```

### Scenario 3: Freeze and Publish to Registry
**Given:** User wants to share environment via registry
**When:** User runs `horus env freeze --publish`
**Then:**
- [ ] Environment is frozen to local file
- [ ] Environment is uploaded to HORUS registry
- [ ] Unique environment ID is generated
- [ ] Environment ID is displayed
- [ ] Environment can be restored by ID on other machines
- [ ] User must be authenticated to publish

**Acceptance Criteria:**
```bash
$ horus env freeze --publish
 Freezing environment...
 Recorded 5 packages
 Environment frozen to horus-freeze.yaml
📤 Publishing to registry...
 Published environment

Environment ID: env_1a2b3c4d5e6f
Anyone can restore this environment with:
  horus env restore env_1a2b3c4d5e6f
```

### Scenario 4: Freeze with No Packages
**Given:** Project has no installed packages
**When:** User runs `horus env freeze`
**Then:**
- [ ] Warning: "No packages found to freeze"
- [ ] Empty freeze file is created (optional behavior)
- [ ] OR informative message without creating file
- [ ] Exit code 0 (not an error)

**Acceptance Criteria:**
```bash
$ horus env freeze
  No packages installed
Creating empty freeze file for future use...
 Environment frozen to horus-freeze.yaml
```

### Scenario 5: Freeze in Non-HORUS Directory
**Given:** User is not in a HORUS project directory
**When:** User runs `horus env freeze`
**Then:**
- [ ] Command still works (freezes global packages)
- [ ] OR error message: "No HORUS project detected"
- [ ] Behavior should be documented

### Scenario 6: Restore Environment from File
**Given:** `horus-freeze.yaml` exists in current directory
**When:** User runs `horus env restore horus-freeze.yaml`
**Then:**
- [ ] Freeze file is read and validated
- [ ] All packages with exact versions are installed
- [ ] Dependencies are resolved
- [ ] Progress shown for each package
- [ ] Success message confirms restoration

**Acceptance Criteria:**
```bash
$ horus env restore horus-freeze.yaml
 Restoring environment from horus-freeze.yaml...
   horus-core v0.1.0
   lidar-driver v1.2.0
   sensor-common v1.0.2
   slam-toolkit v0.8.1
   nav-stack v2.1.0
 Environment restored successfully (5 packages)
```

### Scenario 7: Restore from Registry by ID
**Given:** Environment was published to registry
**When:** User runs `horus env restore env_1a2b3c4d5e6f`
**Then:**
- [ ] Environment ID is recognized (starts with `env_`)
- [ ] Freeze data is downloaded from registry
- [ ] Packages are installed with exact versions
- [ ] Success message shows environment ID

**Acceptance Criteria:**
```bash
$ horus env restore env_1a2b3c4d5e6f
📥 Fetching environment from registry...
 Downloaded environment env_1a2b3c4d5e6f
 Restoring 5 packages...
   horus-core v0.1.0
   lidar-driver v1.2.0
   sensor-common v1.0.2
   slam-toolkit v0.8.1
   nav-stack v2.1.0
 Environment restored successfully
```

### Scenario 8: Restore with Automatic Detection
**Given:** `horus-freeze.yaml` exists in current directory
**When:** User runs `horus env restore` (no source specified)
**Then:**
- [ ] Default freeze file is automatically used
- [ ] Same behavior as `horus env restore horus-freeze.yaml`
- [ ] Message indicates which file is being used

**Acceptance Criteria:**
```bash
$ horus env restore
Using horus-freeze.yaml...
 Restoring environment...
   horus-core v0.1.0
  ...
 Environment restored successfully
```

### Scenario 9: Restore Without Freeze File
**Given:** No freeze file exists in current directory
**And:** User doesn't specify a source
**When:** User runs `horus env restore`
**Then:**
- [ ] Error: "No freeze file found"
- [ ] Suggestion: "Run 'horus env freeze' first or specify a file/ID"
- [ ] Exit code is non-zero

**Acceptance Criteria:**
```bash
$ horus env restore
Error: No freeze file found in current directory

To create a freeze file:
  horus env freeze

To restore from a file:
  horus env restore path/to/freeze.yaml

To restore from registry:
  horus env restore env_<ID>
```

### Scenario 10: Restore Non-Existent File
**Given:** User specifies file that doesn't exist
**When:** User runs `horus env restore missing.yaml`
**Then:**
- [ ] Error: "File 'missing.yaml' not found"
- [ ] Exit code is non-zero

**Acceptance Criteria:**
```bash
$ horus env restore missing.yaml
Error: File 'missing.yaml' not found
```

### Scenario 11: Restore Invalid Environment ID
**Given:** User provides invalid or non-existent environment ID
**When:** User runs `horus env restore env_invalid123`
**Then:**
- [ ] Error: "Environment 'env_invalid123' not found in registry"
- [ ] Suggestion to check ID or use file path
- [ ] Exit code is non-zero

**Acceptance Criteria:**
```bash
$ horus env restore env_invalid123
Error: Environment 'env_invalid123' not found in registry

Check the environment ID and try again, or restore from a local file:
  horus env restore path/to/freeze.yaml
```

### Scenario 12: Restore with Version Conflicts
**Given:** Freeze file requests package version not available
**When:** User runs `horus env restore old-freeze.yaml`
**Then:**
- [ ] Conflict is detected during restoration
- [ ] Error message explains which package version is unavailable
- [ ] No packages are installed (atomic operation)
- [ ] Suggestions for resolution

**Acceptance Criteria:**
```bash
$ horus env restore old-freeze.yaml
Error: Cannot restore environment

Package 'lidar-driver v1.0.5' not found in registry
  Latest available version: v1.2.0

Update your freeze file or install available versions manually
```

### Scenario 13: Restore Overwrites Existing Packages
**Given:** User has different versions of packages installed
**When:** User runs `horus env restore production.yaml`
**Then:**
- [ ] Warning shown about overwriting existing packages
- [ ] Confirmation prompt (optional, or use --force)
- [ ] Existing packages are replaced with freeze file versions
- [ ] Restoration completes successfully

**Acceptance Criteria:**
```bash
$ horus env restore production.yaml
  Warning: This will replace existing package versions

Continue? [y/N] y
 Restoring environment...
   Replaced lidar-driver v1.3.0  v1.2.0
   Replaced sensor-common v1.1.0  v1.0.2
   horus-core v0.1.0 (unchanged)
 Environment restored successfully
```

### Scenario 14: Freeze Includes Local/Git Packages (FUTURE)
**Given:** Project uses packages from local paths or Git repos
**When:** User runs `horus env freeze`
**Then:**
- [ ] Local packages recorded with relative paths
- [ ] Git packages recorded with commit hashes
- [ ] Restore reproduces exact state including local deps

**Note:** Mark as FUTURE FEATURE if not yet implemented

### Scenario 15: Environment Diff (FUTURE)
**Given:** User has made changes to environment
**When:** User runs `horus env diff`
**Then:**
- [ ] Shows differences between current and frozen environment
- [ ] Additions shown (packages installed after freeze)
- [ ] Removals shown (packages removed after freeze)
- [ ] Version changes highlighted
- [ ] Suggestion to refreeze if desired

**Acceptance Criteria (FUTURE):**
```bash
$ horus env diff
Environment differences from horus-freeze.yaml:

  + new-package v1.0.0 (added)
  - old-package v0.5.0 (removed)
  ~ lidar-driver v1.2.0  v1.3.0 (upgraded)

Run 'horus env freeze' to update the freeze file
```

**Note:** Mark as FUTURE FEATURE for v0.2.0+

## Edge Cases

### Edge Case 1: Freeze File Corruption
**Given:** `horus-freeze.yaml` is corrupted or invalid
**When:** User runs `horus env restore horus-freeze.yaml`
**Then:**
- [ ] YAML parsing error detected
- [ ] Error: "Freeze file is corrupted or invalid"
- [ ] Specific parse error shown (line number, if possible)
- [ ] Suggestion to regenerate with `freeze`

**Acceptance Criteria:**
```bash
$ horus env restore horus-freeze.yaml
Error: Failed to parse freeze file

YAML error at line 12: unexpected character
Consider regenerating the freeze file:
  horus env freeze
```

### Edge Case 2: Network Failure During Publish
**Given:** User runs `horus env freeze --publish`
**And:** Network fails during upload
**When:** Publish attempt is made
**Then:**
- [ ] Local freeze file is still created successfully
- [ ] Error message about network failure
- [ ] User can retry publish separately (if command exists)

### Edge Case 3: Large Number of Packages
**Given:** Project has 50+ packages installed
**When:** User runs `horus env freeze` and `restore`
**Then:**
- [ ] Freeze completes in reasonable time (< 5 seconds)
- [ ] Restore shows progress indicator
- [ ] All packages handled correctly
- [ ] No performance degradation

### Edge Case 4: Cross-Platform Freeze/Restore
**Given:** Freeze created on Linux, restore on macOS
**When:** User restores environment on different OS
**Then:**
- [ ] Warning about OS difference (if packages are OS-specific)
- [ ] Restoration attempts compatible packages
- [ ] OR clear error if packages are platform-incompatible

### Edge Case 5: Freeze with Development Dependencies
**Given:** Project has both normal and dev dependencies
**When:** User runs `horus env freeze`
**Then:**
- [ ] Both types of dependencies are frozen
- [ ] Dependencies are marked in freeze file
- [ ] Restore reinstalls all (or option to skip dev deps)

## Integration Tests

### Integration 1: Team Collaboration Workflow
**Scenario:**
1. Developer A freezes environment: `horus env freeze --publish`
2. Environment ID shared with team
3. Developer B restores: `horus env restore env_<ID>`
4. Both developers have identical package versions

**Then:**
- [ ] Environment is identical across machines
- [ ] Projects build and run consistently
- [ ] No version mismatch errors

### Integration 2: CI/CD Pipeline
**Scenario:**
1. Repository includes `horus-freeze.yaml`
2. CI script runs `horus env restore` before build
3. Build uses exact package versions
4. Deployment is reproducible

**Then:**
- [ ] CI can restore environment reliably
- [ ] Builds are reproducible
- [ ] No unexpected package updates

### Integration 3: Environment Versioning
**Scenario:**
1. Freeze environment for v1.0: `horus env freeze -o v1.0-freeze.yaml`
2. Update packages
3. Freeze environment for v2.0: `horus env freeze -o v2.0-freeze.yaml`
4. Can restore either version as needed

**Then:**
- [ ] Multiple freeze files can coexist
- [ ] Easy to switch between environment versions
- [ ] Project versions are reproducible

## Non-Functional Requirements

- [ ] Freeze completes in < 2 seconds for typical projects
- [ ] Restore shows progress for packages taking > 3 seconds
- [ ] Freeze file is human-readable and editable (YAML)
- [ ] Atomic restore (all packages or none)
- [ ] Freeze file includes metadata (date, HORUS version, OS)
- [ ] Published environments are immutable (cannot be modified)
- [ ] Environment IDs are globally unique

## Documentation Requirements

- [ ] README explains freeze/restore workflow
- [ ] Examples for team collaboration
- [ ] CI/CD integration guide
- [ ] Freeze file format documented
- [ ] Best practices for environment management
- [ ] Troubleshooting common issues

## Security Considerations

- [ ] Freeze files do not contain sensitive data (tokens, passwords)
- [ ] Published environments are public by default (warn user)
- [ ] Restore validates package checksums
- [ ] Freeze includes package hashes for verification

## Help and Usage

**When:** User runs `horus env --help`
**Then:**
```bash
$ horus env --help
Manage project environment

Usage: horus env <COMMAND>

Commands:
  freeze   Freeze current environment to a manifest file
  restore  Restore environment from freeze file or registry ID
  help     Print this message

Options:
  -h, --help  Print help
```

**When:** User runs `horus env freeze --help`
**Then:**
```bash
$ horus env freeze --help
Freeze current environment to a manifest file

Usage: horus env freeze [OPTIONS]

Options:
  -o, --output <FILE>  Output file path [default: horus-freeze.yaml]
      --publish        Publish environment to registry for sharing by ID
  -h, --help           Print help
```

**When:** User runs `horus env restore --help`
**Then:**
```bash
$ horus env restore --help
Restore environment from freeze file or registry ID

Usage: horus env restore [SOURCE]

Arguments:
  [SOURCE]  Path to freeze file or environment ID (defaults to horus-freeze.yaml)

Options:
  -h, --help  Print help
```

## Success Criteria

Environment management is successful when:
- [ ] Users can freeze and restore environments reliably
- [ ] Team members can share identical development environments
- [ ] CI/CD pipelines can reproduce builds exactly
- [ ] Freeze files are portable across machines
- [ ] Registry publishing enables easy environment sharing
- [ ] Process is fast and user-friendly
- [ ] Documentation clearly explains workflow
