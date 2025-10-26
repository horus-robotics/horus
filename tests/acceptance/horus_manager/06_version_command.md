# User Acceptance Test: `horus version` Command

## Feature
Display version information for the HORUS CLI and framework.

## User Story
As a developer, I want to check the version of HORUS I'm using so that I can verify I have the correct version, report bugs accurately, and ensure compatibility with packages and documentation.

## Test Scenarios

### Scenario 1: Show Version Information
**Given:** User has HORUS CLI installed
**When:** User runs `horus version`
**Then:**
- [ ] HORUS version number is displayed (semantic versioning format)
- [ ] Clean, readable output
- [ ] Exit code is 0

**Acceptance Criteria:**
```bash
$ horus version
horus 0.1.0
```

### Scenario 2: Version via --version Flag
**Given:** User wants version using standard flag
**When:** User runs `horus --version`
**Then:**
- [ ] Version information is displayed
- [ ] Same format as `horus version` command
- [ ] Exit code is 0

**Acceptance Criteria:**
```bash
$ horus --version
horus 0.1.0
```

### Scenario 3: Version via -V Flag
**Given:** User uses short flag
**When:** User runs `horus -V`
**Then:**
- [ ] Version information is displayed
- [ ] Same format as `horus version` command
- [ ] Exit code is 0

**Acceptance Criteria:**
```bash
$ horus -V
horus 0.1.0
```

### Scenario 4: Version with Git Commit Info (Development Builds)
**Given:** User has development/unreleased build
**When:** User runs `horus version`
**Then:**
- [ ] Version includes git commit hash
- [ ] Optional: build date
- [ ] Indicates development/pre-release status

**Acceptance Criteria:**
```bash
$ horus version
horus 0.1.0-dev+abc1234
```
*Note: This may only apply to development builds*

### Scenario 5: Version in Help Output
**Given:** User views general help
**When:** User runs `horus --help`
**Then:**
- [ ] Version is shown in help header
- [ ] Consistent with `horus version` output
- [ ] Clearly displayed near top of help

**Acceptance Criteria:**
```bash
$ horus --help
HORUS - Hybrid Optimized Robotics Unified System 0.1.0

Usage: horus <COMMAND>

Commands:
  new         Create a new HORUS project
  run         Run a HORUS project or file
  ...
```

### Scenario 6: Subcommand Version Help
**Given:** User wants version info for subcommands
**When:** User runs `horus pkg --version`
**Then:**
- [ ] Shows overall HORUS version (not subcommand-specific)
- [ ] Same output as `horus --version`
- [ ] Exit code is 0

### Scenario 7: Version for Bug Reporting
**Given:** User needs to report a bug with version info
**When:** User runs `horus version --verbose` (if implemented)
**Then:**
- [ ] Detailed version information shown
- [ ] May include: commit hash, build date, rustc version, target triple
- [ ] Useful for debugging and bug reports

**Acceptance Criteria (if verbose flag exists):**
```bash
$ horus version --verbose
horus 0.1.0
commit: abc1234567890
built: 2024-10-24 12:00:00 UTC
rustc: 1.75.0
target: x86_64-unknown-linux-gnu
```

**Note:** If --verbose is not implemented, this scenario can be marked as FUTURE FEATURE

### Scenario 8: Version Check in Scripts
**Given:** User has automation script
**When:** Script runs `horus version` and parses output
**Then:**
- [ ] Output is stable and machine-parseable
- [ ] Version follows semver format (X.Y.Z)
- [ ] No extra decoration in basic version output

**Example Script:**
```bash
#!/bin/bash
VERSION=$(horus version | grep -oP '\d+\.\d+\.\d+')
echo "Detected HORUS version: $VERSION"

# Check minimum version
REQUIRED="0.1.0"
if [ "$VERSION" != "$REQUIRED" ]; then
  echo "Warning: Expected version $REQUIRED, got $VERSION"
fi
```

**Then:**
- [ ] Version can be extracted reliably
- [ ] Comparisons work correctly
- [ ] No parsing errors

### Scenario 9: Version in Package Registry
**Given:** User publishes package with `horus publish`
**When:** Package metadata is generated
**Then:**
- [ ] HORUS version used for building is recorded
- [ ] Package shows minimum required HORUS version
- [ ] Version compatibility is enforced

**Note:** This is an integration test with package system

## Edge Cases

### Edge Case 1: Version from Any Directory
**Given:** User is in any directory (not a HORUS project)
**When:** User runs `horus version`
**Then:**
- [ ] Version is displayed correctly
- [ ] No error about missing project
- [ ] Works the same as in a HORUS project

### Edge Case 2: Multiple HORUS Installations
**Given:** User has multiple HORUS versions installed (e.g., system and user)
**When:** User runs `horus version`
**Then:**
- [ ] Shows version of the `horus` binary in PATH
- [ ] User can distinguish which installation is active
- [ ] Consistent with `which horus` output

### Edge Case 3: Version After Upgrade
**Given:** User upgrades HORUS to new version
**When:** User runs `horus version`
**Then:**
- [ ] New version is displayed immediately
- [ ] No caching of old version
- [ ] Reflects actual binary version

### Edge Case 4: Version with Damaged Installation
**Given:** HORUS installation is corrupted or incomplete
**When:** User runs `horus version`
**Then:**
- [ ] Version command still works (minimal dependencies)
- [ ] Displays version correctly
- [ ] Does not crash

## Integration Tests

### Integration 1: Version Compatibility Check
**Scenario:**
1. Create project with HORUS v0.1.0
2. Document version in horus.yaml
3. Attempt to build with future version v0.2.0
4. System warns about potential incompatibility

**Then:**
- [ ] Version mismatch is detected
- [ ] Warning or error message shown
- [ ] User can override if desired

### Integration 2: Version in Error Reports
**Scenario:**
1. Trigger an error in HORUS
2. Error message includes version info for debugging

**Example:**
```bash
$ horus run
Error: Failed to compile project

HORUS v0.1.0
Please report this issue at: https://github.com/neos-builder/horus/issues
```

**Then:**
- [ ] Version is included in error context
- [ ] Helps with bug triaging and support

### Integration 3: Version in Dashboard
**Scenario:**
1. Launch HORUS dashboard
2. Check dashboard header or about section
3. HORUS version is displayed

**Then:**
- [ ] Dashboard shows HORUS version
- [ ] Version matches CLI output
- [ ] Easily accessible to users

## Non-Functional Requirements

- [ ] Version command executes in < 50ms
- [ ] Output is concise (single line for basic version)
- [ ] Version follows semantic versioning (semver) strictly
- [ ] No network requests required to display version
- [ ] Version format is consistent across all platforms
- [ ] Version command never fails (even if other parts of HORUS are broken)

## Documentation Requirements

- [ ] README shows current version prominently
- [ ] Changelog maps versions to release notes
- [ ] Installation docs explain how to verify version
- [ ] Troubleshooting guide uses version checks
- [ ] Package metadata includes minimum HORUS version

## Comparison with Other Tools

**Cargo:**
```bash
$ cargo --version
cargo 1.75.0 (1d8b05cdd 2023-11-20)
```

**Rustc:**
```bash
$ rustc --version
rustc 1.75.0 (82e1608df 2023-12-21)
```

**Python:**
```bash
$ python --version
Python 3.11.5
```

**HORUS should follow similar conventions:**
- Tool name + version number
- Optional build information in parentheses
- Clean, single-line output

## Future Enhancements

### Planned for Future Releases:
- [ ] `horus version --check-update` - Check if newer version available
- [ ] `horus version --changelog` - Show what's new in this version
- [ ] `horus version --deps` - Show version of core dependencies
- [ ] JSON output format for scripting: `horus version --json`

**Example JSON output:**
```json
{
  "version": "0.1.0",
  "commit": "abc1234",
  "build_date": "2024-10-24",
  "rust_version": "1.75.0",
  "target": "x86_64-unknown-linux-gnu"
}
```

## Success Criteria

The version command is successful when:
- [ ] Users can easily determine their HORUS version
- [ ] Version follows semantic versioning standards
- [ ] Version info is useful for bug reports and support
- [ ] Command is fast, reliable, and always available
- [ ] Output is machine-parseable for automation
- [ ] Documentation clearly explains versioning scheme
