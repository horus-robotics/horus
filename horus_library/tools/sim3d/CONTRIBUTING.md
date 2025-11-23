# Contributing to sim3d

Thank you for your interest in contributing to sim3d! This document provides guidelines and instructions for contributing to the 3D robotics simulator.

## Table of Contents

- [Development Environment Setup](#development-environment-setup)
- [Code Style Guidelines](#code-style-guidelines)
- [Pull Request Process](#pull-request-process)
- [Testing Requirements](#testing-requirements)
- [Benchmarking](#benchmarking)
- [Documentation](#documentation)
- [Issue Guidelines](#issue-guidelines)

## Development Environment Setup

### Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (stable, 1.75.0 or later): Install via [rustup](https://rustup.rs/)
- **Git**: For version control
- **System Dependencies**: Platform-specific requirements listed below

### System Dependencies

#### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y \
    libopencv-dev \
    libclang-dev \
    clang \
    llvm \
    pkg-config \
    libssl-dev \
    libudev-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libgtk-3-dev \
    libasound2-dev \
    libwayland-dev \
    libxrandr-dev
```

#### macOS

```bash
brew install opencv llvm pkg-config

# Add to your shell profile (.zshrc or .bashrc):
export LIBCLANG_PATH="$(brew --prefix llvm)/lib"
export OPENCV_INCLUDE_PATHS="$(brew --prefix opencv)/include/opencv4"
export OPENCV_LINK_PATHS="$(brew --prefix opencv)/lib"
export OPENCV_LINK_LIBS="opencv_core,opencv_imgproc,opencv_imgcodecs,opencv_highgui"
```

#### Windows

```powershell
choco install llvm opencv -y

# Set environment variables:
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
$env:OPENCV_INCLUDE_PATHS = "C:\tools\opencv\build\include"
$env:OPENCV_LINK_PATHS = "C:\tools\opencv\build\x64\vc16\lib"
```

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/HORUS/HORUS.git
cd HORUS/horus_library/tools/sim3d

# Build in debug mode
cargo build

# Build in release mode (recommended for testing performance)
cargo build --release

# Build headless mode (no GUI, for CI/servers)
cargo build --no-default-features --features headless
```

### IDE Setup

#### VS Code (Recommended)

Install the following extensions:
- rust-analyzer
- CodeLLDB (for debugging)
- Even Better TOML

Recommended settings (`.vscode/settings.json`):
```json
{
    "rust-analyzer.cargo.features": ["headless"],
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.checkOnSave.extraArgs": ["--", "-W", "clippy::all"]
}
```

#### CLion/IntelliJ

Install the Rust plugin and configure the toolchain to use your system Rust installation.

### Running the Simulator

```bash
# Run with visual mode (default)
cargo run

# Run headless (for RL training)
cargo run --no-default-features --features headless

# Run with editor tools
cargo run --features editor
```

## Code Style Guidelines

### Rust Style

We follow the official Rust style guide with some additional conventions:

1. **Formatting**: All code must pass `cargo fmt`. Run before committing:
   ```bash
   cargo fmt --all
   ```

2. **Linting**: All code must pass Clippy without warnings:
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```

3. **Naming Conventions**:
   - Use `snake_case` for functions, methods, variables, and modules
   - Use `CamelCase` for types, traits, and enum variants
   - Use `SCREAMING_SNAKE_CASE` for constants and statics
   - Prefix private helper functions with `_` if they're only used within a module

4. **Documentation**:
   - All public items must have documentation comments (`///`)
   - Use `//!` for module-level documentation
   - Include examples in doc comments for complex APIs
   ```rust
   /// Computes the inverse kinematics solution for a robot arm.
   ///
   /// # Arguments
   ///
   /// * `target` - The target end-effector position in world coordinates
   /// * `config` - IK solver configuration
   ///
   /// # Returns
   ///
   /// Returns `Ok(JointPositions)` if a solution is found, or an error otherwise.
   ///
   /// # Example
   ///
   /// ```
   /// let target = Vec3::new(0.5, 0.0, 0.3);
   /// let solution = solve_ik(&robot, target, &config)?;
   /// ```
   pub fn solve_ik(robot: &Robot, target: Vec3, config: &IKConfig) -> Result<JointPositions> {
       // implementation
   }
   ```

5. **Error Handling**:
   - Use `thiserror` for defining error types
   - Use `anyhow` for application-level error handling
   - Avoid `unwrap()` and `expect()` except in tests or when the invariant is guaranteed
   - Provide meaningful error messages

6. **Module Organization**:
   - Keep modules focused and cohesive
   - Use `mod.rs` files sparingly; prefer named files
   - Re-export commonly used items at the crate root

### Code Organization

```
src/
├── lib.rs           # Library root, public API
├── main.rs          # CLI entry point
├── error.rs         # Error types
├── config/          # Configuration parsing
├── physics/         # Physics simulation
├── robot/           # Robot models and control
├── sensors/         # Sensor simulation
├── rl/              # Reinforcement learning interface
├── gpu/             # GPU acceleration
└── utils/           # Utility functions
```

### Performance Guidelines

1. **Avoid allocations in hot paths**: Use pre-allocated buffers for physics and sensor updates
2. **Use SIMD where appropriate**: The `nalgebra` crate provides SIMD support
3. **Profile before optimizing**: Use `cargo flamegraph` or `perf` to identify bottlenecks
4. **Benchmark changes**: Run benchmarks before and after performance-related changes

## Pull Request Process

### Before Submitting

1. **Create an issue first**: For significant changes, create an issue to discuss the approach
2. **Branch from `dev`**: Create a feature branch from the `dev` branch
   ```bash
   git checkout dev
   git pull origin dev
   git checkout -b feature/your-feature-name
   ```

3. **Make atomic commits**: Each commit should represent a logical unit of change
4. **Write meaningful commit messages**: Follow the conventional commits format
   ```
   feat(physics): add support for soft body simulation

   - Implement PBD-based soft body solver
   - Add collision handling for soft-rigid interactions
   - Include unit tests for basic soft body scenarios
   ```

5. **Run all checks locally**:
   ```bash
   # Format code
   cargo fmt --all

   # Run linter
   cargo clippy --all-targets -- -D warnings

   # Run tests
   cargo test --all-targets

   # Run doc tests
   cargo test --doc

   # Build documentation
   cargo doc --no-deps
   ```

### Submitting the PR

1. **Push your branch**:
   ```bash
   git push -u origin feature/your-feature-name
   ```

2. **Create the Pull Request**: Use the GitHub UI to create a PR to the `dev` branch

3. **Fill out the PR template**:
   - Describe what the PR does
   - Reference related issues
   - List breaking changes if any
   - Include test plan

4. **Request review**: Tag relevant maintainers for review

### PR Review Process

- All PRs require at least one approving review
- CI must pass (formatting, linting, tests, benchmarks)
- Performance regressions will block the PR unless justified
- Address all review comments before merging
- Squash commits when merging if the commit history is messy

### After Merging

- Delete your feature branch
- Update related issues
- If applicable, update documentation

## Testing Requirements

### Unit Tests

- All new functionality must include unit tests
- Aim for at least 80% code coverage for new code
- Place unit tests in the same file as the code being tested:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_physics_step() {
          // test implementation
      }
  }
  ```

### Integration Tests

- Place integration tests in the `tests/` directory
- Test interactions between modules
- Include tests for common use cases

```rust
// tests/integration/physics_test.rs
use sim3d::physics::PhysicsWorld;

#[test]
fn test_rigid_body_simulation() {
    let mut world = PhysicsWorld::new();
    // test implementation
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_physics_step

# Run tests with output
cargo test -- --nocapture

# Run tests in release mode (faster, but harder to debug)
cargo test --release

# Run tests for headless mode
cargo test --no-default-features --features headless
```

### Test Guidelines

1. **Test behavior, not implementation**: Focus on what the code does, not how
2. **Use descriptive test names**: `test_physics_step_applies_gravity_correctly`
3. **One assertion per test when possible**: Makes failures easier to diagnose
4. **Use test fixtures**: Create helper functions for common test setup
5. **Test edge cases**: Empty inputs, boundary conditions, error cases

## Benchmarking

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench physics

# Run benchmarks in headless mode
cargo bench --no-default-features --features headless

# Save baseline for comparison
cargo bench -- --save-baseline main
```

### Adding Benchmarks

Add benchmarks to the `benches/` directory using Criterion:

```rust
// benches/my_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_my_function(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        b.iter(|| {
            my_function(black_box(input))
        });
    });
}

criterion_group!(benches, benchmark_my_function);
criterion_main!(benches);
```

### Performance Regression Policy

- PRs that cause >10% performance regression in benchmarks will be flagged
- Regressions must be justified with a clear explanation
- If a regression is acceptable, update the baseline with the PR

## Documentation

### Building Documentation

```bash
# Build documentation
cargo doc --no-deps --open

# Build with private items
cargo doc --no-deps --document-private-items
```

### Documentation Requirements

1. **API Documentation**: All public items must be documented
2. **Examples**: Include code examples for complex APIs
3. **Architecture Docs**: Update `docs/` for architectural changes
4. **README Updates**: Update README.md for user-facing changes

## Issue Guidelines

### Bug Reports

Include:
- sim3d version
- Rust version (`rustc --version`)
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs or screenshots

### Feature Requests

Include:
- Use case description
- Proposed solution
- Alternatives considered
- Impact on existing functionality

### Labels

- `bug`: Something isn't working
- `enhancement`: New feature or improvement
- `documentation`: Documentation improvements
- `performance`: Performance-related changes
- `breaking-change`: Changes that break backward compatibility
- `good-first-issue`: Good for newcomers

## Questions?

- Check the [documentation](docs/)
- Search existing [issues](https://github.com/HORUS/HORUS/issues)
- Ask in discussions or create a new issue

Thank you for contributing to sim3d!
