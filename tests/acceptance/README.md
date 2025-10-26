# HORUS User Story Acceptance Tests

## Overview

This directory contains comprehensive user acceptance tests (UAT) for the HORUS robotics framework. These tests are written as user stories with detailed acceptance criteria to ensure HORUS is ready for open source launch and provides a confidence-inspiring experience for new users.

> **📋 Recent Updates (2024-10-26):**
> - Added `horus_manager/07_version_command.md` - Version information display and verification
> - Updated `horus_manager/02_run_command.md` - Added --build-only flag scenarios (Scenario 8 & 8a)
> - Added `horus_env/01_environment_management.md` - Comprehensive freeze/restore environment testing
> - Updated test priorities and coverage matrix to reflect current implementation state

## Purpose

These acceptance tests serve multiple purposes:

1. **Pre-Launch Validation**: Verify all features work as documented before open source release
2. **User Confidence**: Ensure users can successfully complete common workflows
3. **Documentation Reference**: Detailed examples of expected behavior
4. **Regression Prevention**: Checklist for testing after changes
5. **Feature Completeness**: Identify gaps in functionality

## Structure

```
user_story_acceptance_test/
├── README.md (this file)
├── horus_manager/          # CLI commands (new, run, pkg, auth, publish, env, dashboard)
├── horus_core/             # Core framework (Hub, Node, Scheduler)
├── horus_py/               # Python bindings
├── horus_macros/           # node! procedural macro
├── horus_dashboard/        # Web and TUI monitoring
├── horus_registry/         # Package registry backend
├── horus_marketplace/      # Web marketplace frontend
├── horus_c/                # C bindings (alpha)
└── horus_env/              # Environment management

```

## Test Categories

### 1. horus_manager (CLI)
**Files:**
- `01_new_command.md` - Project creation (Rust, Python, C)
- `02_run_command.md` - Build and execution (including --build-only, --clean, --remote deployment)
- `03_pkg_command.md` - Package management (install, remove, list, publish, unpublish)
- `04_auth_publish.md` - Authentication (login, logout, generate-key, whoami) and publishing
- `05_env_dashboard.md` - Dashboard monitoring (web and TUI modes)
- `06_version_command.md` - Version information display

**Coverage:**
- All CLI subcommands including remote deployment
- Build-only mode for compilation without execution
- API key generation and management
- Package unpublishing with version control
- Version information for bug reporting
- Error handling and edge cases
- Cross-platform compatibility
- Help documentation
- Performance requirements

### 2. horus_core (Framework)
**Files:**
- `01_hub_communication.md` - Pub/sub messaging, types, performance
- `02_node_lifecycle_scheduler.md` - Node lifecycle, priorities, scheduling

**Coverage:**
- Lock-free shared memory communication
- Sub-microsecond latency
- Node lifecycle (init/tick/shutdown)
- Priority-based scheduling
- Context and logging
- Error handling

### 3. horus_py (Python Bindings)
**Files:**
- `01_python_bindings.md` - PyO3 FFI, Hub/Node APIs, cross-language communication

**Coverage:**
- Python package installation
- Hub operations from Python
- Python Node implementations
- Rust ↔ Python communication
- Type hints and IDE support
- Performance benchmarks

### 4. horus_macros (Procedural Macros)
**Files:**
- `01_node_macro.md` - node! macro functionality and code generation

**Coverage:**
- Section-based syntax (pub, sub, data, tick, init, shutdown, impl)
- Code generation correctness
- Type safety
- Error messages
- Integration with manual implementations

### 5. horus_dashboard (Monitoring)
**Files:**
- `01_monitoring_dashboard.md` - Web and TUI dashboards

**Coverage:**
- Real-time node monitoring
- Topic activity visualization
- Performance metrics
- WebSocket/polling updates
- TUI keyboard navigation
- Responsive web design

### 6. horus_registry (Backend)
**Files:**
- `01_package_registry.md` - Package upload, download, search, versioning

**Coverage:**
- RESTful API endpoints
- Package validation
- Version management
- Search and discovery
- Authentication
- Database operations

### 7. horus_marketplace (Frontend)
**Files:**
- `01_web_marketplace.md` - Web UI for browsing packages

**Coverage:**
- Package browsing and search
- Package detail pages
- User authentication
- Publishing workflow
- Responsive design
- SEO and accessibility

### 8. horus_env (Environment Management)
**Files:**
- `01_environment_management.md` - Freeze and restore environments for reproducibility

**Coverage:**
- Freeze current environment with exact versions
- Restore from local freeze files
- Publish environments to registry
- Restore from registry by environment ID
- Cross-platform environment handling
- Version conflict detection
- Team collaboration workflows
- CI/CD integration

### 9. horus_c (C Bindings - Alpha)
**Files:**
- `01_c_bindings.md` - C API for hardware integration

**Coverage:**
- Basic Hub operations
- Memory management
- Cross-language communication
- Thread safety
- Error handling
- Current limitations

## How to Use These Tests

### For Developers (Pre-Release)

**Before Launch:**
1. Review each test file in relevant directories
2. Check off scenarios as you test them
3. Fix any failing tests before release
4. Add new tests for new features

**Manual Testing:**
```bash
# Example: Testing CLI commands
cd /path/to/horus/HORUS
# Follow scenarios in horus_manager/01_new_command.md
horus new test_project
cd test_project
horus run
# ✓ Scenario 1: Create Basic Rust Project
```

**Automated Testing:**
While these are written as user stories, many can be automated:
```bash
# Integration tests (for HORUS core framework)
cd /path/to/horus/HORUS
cargo test --test acceptance_tests  # Core framework tests only

# Python bindings tests
cd horus_py
pytest tests/acceptance/

# CLI tests
./scripts/run_cli_acceptance_tests.sh
```

### For QA/Testing Teams

**Test Execution:**
1. Start with critical path: `horus_manager/01_new_command.md` → `02_run_command.md`
2. Test each scenario sequentially
3. Mark checkboxes as you complete tests
4. Note any failures or deviations
5. Report bugs with reference to test scenario number

**Example Test Report:**
```
Component: horus_manager
Test File: 01_new_command.md
Scenario: 4 (Create C Project)
Status: ✅ Pass
Notes: Warning message appears as expected

Scenario: 5 (Error - Project Already Exists)
Status: ❌ Fail
Issue: Error message not displayed, silent failure
Bug #: 123
```

### For New Contributors

**Understanding HORUS:**
1. Read test files to understand expected behavior
2. Tests serve as detailed usage examples
3. Acceptance criteria show exact API usage
4. Edge cases document important constraints

**Before Submitting PR:**
1. Review relevant test files
2. Ensure your changes don't break existing tests
3. Add new test scenarios for new features
4. Update acceptance criteria if APIs change

## Test Priorities

### Critical (Must Pass for Launch)
- [ ] `horus_manager/01_new_command.md` - Basic project creation
- [ ] `horus_manager/02_run_command.md` - Build and run (including --build-only)
- [ ] `horus_manager/07_version_command.md` - Version information
- [ ] `horus_core/01_hub_communication.md` - Pub/sub works
- [ ] `horus_core/02_node_lifecycle_scheduler.md` - Scheduler works
- [ ] All templates generate correct, compilable code

### High Priority (Should Pass for Launch)
- [ ] `horus_manager/03_pkg_command.md` - Package management (including unpublish)
- [ ] `horus_manager/04_auth_publish.md` - Authentication and publishing
- [ ] `horus_env/01_environment_management.md` - Freeze/restore environments
- [ ] `horus_py/01_python_bindings.md` - Python bindings functional
- [ ] `horus_macros/01_node_macro.md` - Macros work correctly

### Medium Priority (Nice to Have)
- [ ] `horus_manager/05_env_dashboard.md` - Dashboard command
- [ ] `horus_dashboard/01_monitoring_dashboard.md` - Dashboard functionality
- [ ] `horus_registry/01_package_registry.md` - Registry backend stable
- [ ] `horus_marketplace/01_web_marketplace.md` - Marketplace UI polished

### Low Priority (Alpha/Beta Features)
- [ ] `horus_c/01_c_bindings.md` - C bindings (marked as alpha)
- [ ] Advanced dashboard features (TUI mode)
- [ ] Analytics and statistics
- [ ] Environment diff command (future feature)

## Test Coverage Matrix

| Component | Basic Functionality | Error Handling | Performance | Cross-Platform | Documentation |
|-----------|-------------------|----------------|-------------|----------------|---------------|
| CLI (manager) | ✅ Comprehensive | ✅ All cases | ⚠️ Basic | ✅ Linux/macOS | ✅ Complete |
| Version | ✅ Complete | N/A | ✅ Instant | ✅ All platforms | ✅ Complete |
| Env Mgmt | ✅ Freeze/restore | ✅ Conflicts | ✅ Fast | ✅ Portable | ✅ Complete |
| Hub | ✅ Complete | ✅ All cases | ✅ Benchmarked | ✅ POSIX shm | ✅ Complete |
| Node/Sched | ✅ Complete | ✅ Lifecycle | ✅ Priority | ✅ Cross-platform | ✅ Complete |
| Python | ✅ Core features | ✅ Exceptions | ⚠️ Basic | ⚠️ Linux/macOS | ⚠️ Type hints |
| Macros | ✅ All sections | ✅ Compile errors | N/A | ✅ Rust standard | ✅ Examples |
| Dashboard | ⚠️ Web only | ⚠️ Basic | ⚠️ Needs testing | ✅ Browsers | ⚠️ Partial |
| Registry | ✅ Full CRUD | ✅ Validation | ⚠️ Needs load test | ✅ SQLite | ✅ API docs |
| Marketplace | ⚠️ UI incomplete | ⚠️ Basic | ⚠️ Needs testing | ✅ Responsive | ⚠️ Partial |
| C Bindings | ⚠️ Alpha | ⚠️ Minimal | ❌ Not tested | ⚠️ Experimental | ❌ Incomplete |

**Legend:**
- ✅ Complete and tested
- ⚠️ Partial or needs improvement
- ❌ Not yet implemented
- N/A Not applicable

## Running All Tests

### Quick Validation (30 minutes)
```bash
# Test core functionality only
./scripts/quick_acceptance_test.sh

# Covers:
# - horus new (all languages)
# - horus run (build and execute)
# - Hub communication (basic send/recv)
# - Node lifecycle (init/tick/shutdown)
```

### Full Test Suite (2-3 hours)
```bash
# Comprehensive testing
./scripts/full_acceptance_test.sh

# Covers all scenarios in:
# - horus_manager/
# - horus_core/
# - horus_py/
# - horus_macros/
```

### Manual Testing Checklist
Download printable checklist:
```bash
./scripts/generate_test_checklist.sh > test_checklist.md
# Print and check off as you test
```

## Reporting Issues

When a test fails:

**Issue Template:**
```
Title: [Component] Scenario X fails - Brief description

Component: horus_manager
Test File: 01_new_command.md
Scenario: 5 - Error - Project Already Exists

Expected Behavior:
- Error message displayed
- Non-zero exit code

Actual Behavior:
- Silent failure
- Exit code 0

Steps to Reproduce:
1. horus new my_project
2. horus new my_project (again)

Environment:
- OS: Ubuntu 22.04
- HORUS version: 0.1.0-alpha
- Rust version: 1.75.0
```

## Contributing New Tests

When adding new features:

1. **Create test file** in appropriate directory
2. **Follow format:**
   ```markdown
   # User Acceptance Test: Feature Name

   ## Feature
   Brief description

   ## User Story
   As a [role], I want [feature] so that [benefit]

   ## Test Scenarios

   ### Scenario 1: [Name]
   **Given:** [precondition]
   **When:** [action]
   **Then:** [expected result]

   - [ ] Acceptance criteria 1
   - [ ] Acceptance criteria 2

   **Acceptance Criteria:**
   ```bash
   # Example command and output
   ```
   ```

3. **Include:**
   - Happy path scenarios
   - Error cases
   - Edge cases
   - Performance requirements
   - Non-functional requirements

4. **Review:**
   - Get feedback from user perspective
   - Ensure tests are verifiable
   - Add automation where possible

## Maintenance

### Updating Tests

When APIs change:
1. Update affected test files
2. Mark deprecated scenarios
3. Add migration notes
4. Update version references

### Test Review Schedule

- **Weekly**: Review failed tests during development
- **Before Release**: Full test suite execution
- **After Release**: Validate with real users
- **Monthly**: Update based on user feedback

## Success Criteria

**HORUS is ready for launch when:**
- [ ] 100% of Critical tests pass
- [ ] 95%+ of High Priority tests pass
- [ ] All documented features work as described
- [ ] No known blocking bugs
- [ ] Templates generate correct, compilable code
- [ ] Documentation matches actual behavior
- [ ] New users can complete "Getting Started" without issues

## Resources

- **HORUS Documentation**: https://docs.horus.dev
- **GitHub Repository**: https://github.com/neos-builder/horus
- **Issue Tracker**: https://github.com/neos-builder/horus/issues
- **CI/CD Pipeline**: .github/workflows/

## Questions or Issues?

If you find issues with these tests themselves:
1. Open issue with label `acceptance-tests`
2. Describe the problem with test scenario
3. Suggest improvement

**Remember:** These tests exist to give users confidence in HORUS. If a test doesn't reflect real-world usage, it should be updated or removed.
