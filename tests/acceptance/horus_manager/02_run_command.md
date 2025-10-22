# User Acceptance Test: `horus run` Command

## Feature
Smart build and execution of HORUS projects with automatic dependency management.

## User Story
As a developer, I want to run my HORUS project with a single command that handles building, dependency resolution, and execution so that I can test my robot application quickly.

## Test Scenarios

### Scenario 1: Run Rust Project in Debug Mode
**Given:** User has a valid HORUS Rust project
**When:** User runs `horus run`
**Then:**
- [ ] Project is built with `cargo build`
- [ ] Build output is displayed
- [ ] Executable runs after successful build
- [ ] Scheduler starts and nodes execute
- [ ] Ctrl+C gracefully stops the application
- [ ] Exit code 0 on normal shutdown

**Acceptance Criteria:**
```bash
$ cd my_robot
$ horus run
   Compiling my_robot v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 2.5s
     Running target/debug/my_robot
Registered node 'Controller' with priority 0
[12:34:56] Controller initialized
^C
Shutting down gracefully...
```

### Scenario 2: Run Rust Project in Release Mode
**Given:** User wants optimized performance
**When:** User runs `horus run --release`
**Then:**
- [ ] Project is built with `cargo build --release`
- [ ] Build shows optimization in progress
- [ ] Release binary executes from `target/release/`
- [ ] Performance is noticeably faster (for benchmarks)

**Acceptance Criteria:**
```bash
$ horus run --release
   Compiling my_robot v0.1.0
    Finished release [optimized] target(s) in 15.2s
     Running target/release/my_robot
Registered node 'Controller' with priority 0
...
```

### Scenario 3: Run Python Project
**Given:** User has a HORUS Python project
**When:** User runs `horus run` in Python project directory
**Then:**
- [ ] Python dependencies are checked
- [ ] main.py is executed
- [ ] Python bindings work correctly
- [ ] Nodes can publish/subscribe
- [ ] Ctrl+C stops execution gracefully

**Acceptance Criteria:**
```bash
$ cd my_python_robot
$ horus run
Running Python project: my_python_robot
Scheduler started
Node 'PySensor' registered
^C
Shutdown complete
```

### Scenario 4: Run with Specific Timeout
**Given:** User wants to run for limited time (testing)
**When:** User runs `timeout 5 horus run`
**Then:**
- [ ] Project runs for 5 seconds
- [ ] Timeout kills the process
- [ ] Cleanup happens properly
- [ ] Shared memory is cleared

### Scenario 5: Compilation Error
**Given:** User has syntax error in code
**When:** User runs `horus run`
**Then:**
- [ ] Build fails with compiler error
- [ ] Error message is displayed clearly
- [ ] Execution does not start
- [ ] Exit code is non-zero
- [ ] Error points to problematic code location

**Acceptance Criteria:**
```bash
$ horus run
   Compiling my_robot v0.1.0
error[E0425]: cannot find value `Hub` in this scope
 --> src/main.rs:10:5
  |
10|     Hub::new("test")
  |     ^^^ not found in this scope

error: could not compile `my_robot`
```

### Scenario 6: Runtime Error
**Given:** Project compiles but fails at runtime
**When:** User runs `horus run`
**Then:**
- [ ] Build succeeds
- [ ] Execution starts
- [ ] Runtime error is caught and displayed
- [ ] Meaningful error message (not just panic)
- [ ] Exit code is non-zero

**Acceptance Criteria:**
```bash
$ horus run
    Finished dev target(s) in 0.5s
     Running target/debug/my_robot
Error: Failed to create Hub 'cmd_vel'
Caused by: Permission denied: /dev/shm/horus/cmd_vel
```

### Scenario 7: No Project Detected
**Given:** User is not in a HORUS project directory
**When:** User runs `horus run`
**Then:**
- [ ] Error: "No HORUS project found in current directory"
- [ ] Helpful message suggests `horus new` to create project
- [ ] Exit code is non-zero

### Scenario 8: Run Specific Binary
**Given:** Workspace has multiple binaries
**When:** User runs `horus run --bin my_node`
**Then:**
- [ ] Only specified binary is built
- [ ] Only specified binary runs
- [ ] Other binaries are not affected

## Edge Cases

### Edge Case 1: Incremental Build
**Given:** Project was already built
**When:** User runs `horus run` again without changes
**Then:**
- [ ] Cargo detects no changes
- [ ] Build completes in < 1 second
- [ ] "Finished" message appears quickly
- [ ] Binary executes immediately

### Edge Case 2: Build Cache Corruption
**Given:** `target/` directory is corrupted
**When:** User runs `horus run`
**Then:**
- [ ] Build system detects corruption
- [ ] Clean rebuild is triggered
- [ ] Project builds successfully

### Edge Case 3: Shared Memory Cleanup
**Given:** Previous run crashed and left shared memory
**When:** User runs `horus run`
**Then:**
- [ ] Old shared memory is detected
- [ ] Warning or cleanup message shown
- [ ] New run starts successfully
- [ ] No conflicts with old memory

## Performance Requirements

- [ ] Debug build completes in < 5 seconds for simple project
- [ ] Release build completes in < 30 seconds for simple project
- [ ] Incremental builds (no changes) complete in < 1 second
- [ ] Execution starts within 1 second after build

## Integration Tests

### Integration 1: Run After Modifying Code
**Given:** User modified source code
**When:** User runs `horus run`
**Then:**
- [ ] Changes are detected
- [ ] Only modified crates rebuild
- [ ] New code executes correctly

### Integration 2: Run with Dependencies
**Given:** Project has external dependencies
**When:** User runs `horus run` (first time)
**Then:**
- [ ] Dependencies are downloaded
- [ ] Dependencies are compiled
- [ ] Project compiles with dependencies
- [ ] All features work correctly

### Integration 3: Cross-Language Projects
**Given:** Project has both Rust and Python nodes
**When:** User runs `horus run`
**Then:**
- [ ] Rust components build
- [ ] Python scripts are valid
- [ ] Both languages can communicate via Hub
- [ ] Scheduler runs both node types

## Help and Documentation

**When:** User runs `horus run --help`
**Then:**
- [ ] Usage information is displayed
- [ ] --release flag is documented
- [ ] --bin flag is documented (if applicable)
- [ ] Examples are shown

**Acceptance Criteria:**
```bash
$ horus run --help
Build and run a HORUS project

Usage: horus run [OPTIONS]

Options:
      --release    Build in release mode (optimized)
      --bin <BIN>  Run specific binary (workspace only)
  -h, --help       Print help
```

## Non-Functional Requirements

- [ ] Output is colored and readable
- [ ] Progress indicators for long builds
- [ ] Graceful shutdown on Ctrl+C
- [ ] Clean error messages (no stack traces for user errors)
- [ ] Cross-platform compatibility
