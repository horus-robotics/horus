# User Acceptance Test: `horus auth` and `horus publish` Commands

## Feature
GitHub OAuth authentication and package publishing to the HORUS registry.

## User Story
As a package developer, I want to authenticate with GitHub and publish my packages to the registry so that other developers can discover and use my work.

## Authentication Tests

### Scenario 1: Login Flow
**Given:** User is not authenticated
**When:** User runs `horus auth login`
**Then:**
- [ ] Browser opens to GitHub OAuth page
- [ ] User authorizes HORUS application
- [ ] Token is received and stored securely
- [ ] Success message: "Logged in as @username"
- [ ] Token stored in `~/.horus/auth_token` or similar

**Acceptance Criteria:**
```bash
$ horus auth login
Opening browser for GitHub authentication...
Waiting for authorization...
✓ Logged in as @robotics-dev
```

### Scenario 2: Check Authentication Status
**Given:** User is logged in
**When:** User runs `horus auth status`
**Then:**
- [ ] Shows: "Logged in as @username"
- [ ] Shows token expiration date (if applicable)
- [ ] Shows associated email

**Acceptance Criteria:**
```bash
$ horus auth status
✓ Logged in as @robotics-dev
Email: dev@robotics.com
Token expires: Never (GitHub PAT)
```

### Scenario 3: Status When Not Logged In
**Given:** User is not authenticated
**When:** User runs `horus auth status`
**Then:**
- [ ] Shows: "Not logged in"
- [ ] Suggestion: "Run 'horus auth login' to authenticate"
- [ ] Exit code 0 (not an error)

**Acceptance Criteria:**
```bash
$ horus auth status
Not logged in
Run 'horus auth login' to authenticate
```

### Scenario 4: Logout
**Given:** User is logged in
**When:** User runs `horus auth logout`
**Then:**
- [ ] Confirmation prompt: "Logout from HORUS? [y/N]"
- [ ] Token is removed from storage
- [ ] Success message: "Logged out successfully"

**Acceptance Criteria:**
```bash
$ horus auth logout
Logout from HORUS? [y/N] y
✓ Logged out successfully
```

### Scenario 5: Force Logout (No Prompt)
**Given:** User wants non-interactive logout
**When:** User runs `horus auth logout --force`
**Then:**
- [ ] No confirmation prompt
- [ ] Token is removed immediately
- [ ] Success message shown

## Publishing Tests

### Scenario 6: Publish Package (First Time)
**Given:** User has a valid HORUS package
**And:** User is authenticated
**When:** User runs `horus publish`
**Then:**
- [ ] Package metadata is validated
- [ ] Version is checked (semantic versioning)
- [ ] Package is packaged (tar.gz or similar)
- [ ] Upload progress is shown
- [ ] Success message with registry URL
- [ ] Package is immediately searchable

**Acceptance Criteria:**
```bash
$ cd my-lidar-driver
$ horus publish
Validating package...
  ✓ Package name: lidar-driver
  ✓ Version: 1.0.0
  ✓ Author: @robotics-dev
  ✓ License: MIT
Packaging...
Uploading to registry...
[████████████████████] 100% (2.3 MB)
✓ Published lidar-driver v1.0.0
View at: https://horus-registry.dev/packages/lidar-driver
```

### Scenario 7: Publish Update
**Given:** Package already exists in registry
**When:** User runs `horus publish` with new version
**Then:**
- [ ] Version is compared with latest
- [ ] New version must be higher (semver)
- [ ] Package is uploaded
- [ ] Both versions exist in registry

**Acceptance Criteria:**
```bash
$ horus publish
Current version in registry: v1.0.0
New version: v1.1.0
✓ Version is valid
Publishing update...
✓ Published lidar-driver v1.1.0
```

### Scenario 8: Publish Duplicate Version
**Given:** Version 1.0.0 already exists
**When:** User tries to publish v1.0.0 again
**Then:**
- [ ] Error: "Version 1.0.0 already exists"
- [ ] Suggestion to increment version
- [ ] No upload occurs
- [ ] Exit code is non-zero

**Acceptance Criteria:**
```bash
$ horus publish
Error: Version 1.0.0 is already published
Bump the version in horus.yaml and try again
```

### Scenario 9: Publish Without Authentication
**Given:** User is not logged in
**When:** User runs `horus publish`
**Then:**
- [ ] Error: "Not authenticated"
- [ ] Instruction: "Run 'horus auth login' first"
- [ ] No package validation occurs
- [ ] Exit code is non-zero

**Acceptance Criteria:**
```bash
$ horus publish
Error: Not authenticated
Run 'horus auth login' to authenticate with GitHub
```

### Scenario 10: Invalid Package Structure
**Given:** Project is not a valid HORUS package
**When:** User runs `horus publish`
**Then:**
- [ ] Validation fails with specific errors
- [ ] Lists all validation issues
- [ ] Examples of how to fix
- [ ] No upload occurs

**Acceptance Criteria:**
```bash
$ horus publish
Validation failed:
  ✗ Missing horus.yaml
  ✗ No source files found (main.rs, main.py, or main.c)
  ✗ Missing package description

Fix these issues and try again
```

### Scenario 11: Missing Required Metadata
**Given:** horus.yaml missing description or license
**When:** User runs `horus publish`
**Then:**
- [ ] Specific missing fields are identified
- [ ] Error message shows which fields are required
- [ ] No upload occurs

**Acceptance Criteria:**
```bash
$ horus publish
Validation failed:
  ✗ Missing 'description' in horus.yaml
  ✗ Missing 'license' in horus.yaml

Add these required fields and try again
```

### Scenario 12: Dry Run (Test Publishing)
**Given:** User wants to test without publishing
**When:** User runs `horus publish --dry-run`
**Then:**
- [ ] All validation is performed
- [ ] Package is built and packaged
- [ ] No upload occurs
- [ ] Success message: "Dry run successful"

**Acceptance Criteria:**
```bash
$ horus publish --dry-run
Validating package...
  ✓ All checks passed
Dry run successful - package is ready to publish
Run 'horus publish' to upload to registry
```

### Scenario 13: Network Failure During Upload
**Given:** Network fails during publish
**When:** Upload is interrupted
**Then:**
- [ ] Error: "Upload failed: Network error"
- [ ] Package is NOT partially published
- [ ] Registry remains in consistent state
- [ ] User can retry without issues

### Scenario 14: Publish with Documentation
**Given:** Package has README.md and docs
**When:** User runs `horus publish`
**Then:**
- [ ] README is included in package
- [ ] Documentation is rendered on registry page
- [ ] Examples in README are syntax-highlighted

## Edge Cases

### Edge Case 1: Token Expired
**Given:** GitHub token expired
**When:** User runs `horus publish`
**Then:**
- [ ] Error: "Authentication token expired"
- [ ] Instruction to re-authenticate
- [ ] Logout occurs automatically

### Edge Case 2: Large Package
**Given:** Package is > 100MB
**When:** User runs `horus publish`
**Then:**
- [ ] Upload progress is shown with percentage
- [ ] Estimated time remaining displayed
- [ ] Upload can be interrupted (Ctrl+C) safely

### Edge Case 3: Package Name Conflict
**Given:** Another user has package with same name
**When:** User tries to publish
**Then:**
- [ ] Error: "Package name 'lidar-driver' is taken"
- [ ] Suggestion to choose different name
- [ ] No upload occurs

### Edge Case 4: Invalid Version Format
**Given:** Version in horus.yaml is "1.0" (not semver)
**When:** User runs `horus publish`
**Then:**
- [ ] Validation fails
- [ ] Error: "Invalid version format"
- [ ] Example of valid version shown (e.g., "1.0.0")

## Help Documentation

**When:** User runs `horus auth --help`
**Then:**
```bash
$ horus auth --help
Authenticate with HORUS registry

Usage: horus auth <COMMAND>

Commands:
  login   Login with GitHub OAuth
  logout  Logout and remove credentials
  status  Show authentication status
  help    Print this message

Options:
  -h, --help  Print help
```

**When:** User runs `horus publish --help`
**Then:**
```bash
$ horus publish --help
Publish a package to the HORUS registry

Usage: horus publish [OPTIONS]

Options:
      --dry-run  Validate without publishing
  -h, --help     Print help
```

## Security Requirements

- [ ] Tokens are stored securely (not in plain text if possible)
- [ ] Token file has restrictive permissions (0600)
- [ ] Tokens are never logged or displayed
- [ ] HTTPS is used for all registry communication
- [ ] Package uploads are authenticated and authorized

## Non-Functional Requirements

- [ ] OAuth flow completes in < 30 seconds
- [ ] Upload progress updates every second
- [ ] Clear error messages for all auth failures
- [ ] Publish command validates before uploading
- [ ] Atomic publish (succeeds completely or fails completely)
