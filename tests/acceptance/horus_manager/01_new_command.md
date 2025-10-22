# User Acceptance Test: `horus new` Command

## Feature
Project scaffolding and template generation for Rust, Python, and C projects.

## User Story
As a robotics developer, I want to quickly create a new HORUS project with proper structure and boilerplate code so that I can start building my robot application immediately.

## Test Scenarios

### Scenario 1: Create Basic Rust Project
**Given:** User has HORUS CLI installed
**When:** User runs `horus new my_robot`
**Then:**
- [ ] Directory `my_robot/` is created
- [ ] `Cargo.toml` exists with correct dependencies
- [ ] `src/main.rs` exists with compilable Node implementation
- [ ] `.gitignore` is created
- [ ] Success message is displayed: "Created Rust project: my_robot"
- [ ] Project compiles: `cd my_robot && cargo check` succeeds
- [ ] Generated code uses `HorusResult<T>` (not `Result<T>`)
- [ ] No `std::thread::sleep()` in generated code

**Acceptance Criteria:**
```bash
$ horus new my_robot
Created Rust project: my_robot
$ cd my_robot
$ cargo check
   Compiling my_robot v0.1.0
    Finished dev [unoptimized + debuginfo] target(s)
```

### Scenario 2: Create Rust Project with Macro
**Given:** User wants to use the `node!` macro
**When:** User runs `horus new my_robot --macro`
**Then:**
- [ ] Project is created with `node!` macro usage
- [ ] `Cargo.toml` includes `horus_macros` dependency
- [ ] Generated code compiles without errors
- [ ] Macro expansion produces correct Node trait implementation

**Acceptance Criteria:**
```bash
$ horus new my_robot --macro
Created Rust project with macros: my_robot
$ cd my_robot && cargo check
    Finished dev [unoptimized + debuginfo] target(s)
```

### Scenario 3: Create Python Project
**Given:** User wants to use Python bindings
**When:** User runs `horus new my_robot -p` or `horus new my_robot --python`
**Then:**
- [ ] Directory `my_robot/` is created
- [ ] `main.py` exists with Python Hub/Node example
- [ ] `requirements.txt` or `pyproject.toml` is created
- [ ] `.gitignore` is created for Python
- [ ] Success message: "Created Python project: my_robot"
- [ ] Python script is syntactically correct
- [ ] Imports use correct Python bindings API

**Acceptance Criteria:**
```bash
$ horus new my_robot -p
Created Python project: my_robot
$ cd my_robot
$ python -m py_compile main.py  # No syntax errors
```

### Scenario 4: Create C Project (Under Development)
**Given:** User wants to use C bindings
**When:** User runs `horus new my_robot -c` or `horus new my_robot --c`
**Then:**
- [ ] Directory `my_robot/` is created
- [ ] `main.c` exists with clear "under development" message
- [ ] Message explains C bindings are alpha status
- [ ] Directs user to use Rust or Python instead
- [ ] Success message indicates C project was created

**Acceptance Criteria:**
```bash
$ horus new my_robot -c
Created C project: my_robot (⚠️  C bindings are in alpha)
$ cat my_robot/main.c
// C bindings for HORUS are under development
// Please use Rust or Python for now
// See: https://docs.horus.dev/c-bindings for updates
```

### Scenario 5: Error - Project Already Exists
**Given:** Directory `my_robot/` already exists
**When:** User runs `horus new my_robot`
**Then:**
- [ ] Command fails with clear error message
- [ ] Existing directory is not modified
- [ ] Exit code is non-zero

**Acceptance Criteria:**
```bash
$ horus new my_robot
Created Rust project: my_robot
$ horus new my_robot
Error: Directory 'my_robot' already exists
```

### Scenario 6: Help Documentation
**Given:** User needs help with the command
**When:** User runs `horus new --help`
**Then:**
- [ ] Usage information is displayed
- [ ] All flags are documented (-p, -c, --macro)
- [ ] Examples are shown
- [ ] Output is clear and readable

**Acceptance Criteria:**
```bash
$ horus new --help
Create a new HORUS project

Usage: horus new [OPTIONS] <PROJECT_NAME>

Arguments:
  <PROJECT_NAME>  Name of the project to create

Options:
  -p, --python     Create a Python project
  -c, --c          Create a C project (⚠️ alpha)
      --macro      Use horus_macros for node definition
  -h, --help       Print help
```

## Edge Cases

### Edge Case 1: Special Characters in Project Name
**When:** User runs `horus new my-robot-123`
**Then:**
- [ ] Project is created with valid Rust package name
- [ ] Hyphens are preserved in directory name
- [ ] Cargo.toml uses valid package name (hyphens converted to underscores if needed)

### Edge Case 2: Long Project Name
**When:** User runs `horus new very_long_project_name_for_testing`
**Then:**
- [ ] Project is created successfully
- [ ] All paths and imports work correctly

### Edge Case 3: No Arguments
**When:** User runs `horus new` without project name
**Then:**
- [ ] Clear error message: "Missing required argument: PROJECT_NAME"
- [ ] Help text is shown or referenced
- [ ] Exit code is non-zero

## Non-Functional Requirements

- [ ] Command completes in < 2 seconds for Rust projects
- [ ] Command completes in < 1 second for Python/C projects
- [ ] Generated code follows HORUS best practices
- [ ] Template code is properly formatted (rustfmt, black)
- [ ] No hardcoded paths in generated code
- [ ] All examples use correct API (HorusResult, no thread::sleep)

## Regression Tests

- [ ] Templates match current API documentation
- [ ] Generated projects work with latest horus version
- [ ] No deprecated APIs in generated code
- [ ] Cross-platform compatibility (Linux, macOS, Windows)
