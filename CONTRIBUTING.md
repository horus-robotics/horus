# Contributing to HORUS

Thank you for your interest in contributing to HORUS! This document provides guidelines for contributing to the project.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/your-username/horus.git
   cd horus
   ```
3. **Create a branch** for your feature:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- Rust 1.70+ (`rustup update`)
- Python 3.9+ with `pip` (optional)
- GCC/Clang for C bindings
- Node.js 18+ for documentation site

### Building

```bash
# Build all Rust components
cargo build --release

# Build Python bindings (optional)
cd horus_py
maturin develop --release

# Build C bindings (optional)
cd horus_c
make

# Build documentation site (optional)
cd docs-site
npm install
npm run dev
```

## Testing

### Unit and Integration Tests

```bash
# Rust unit tests
cargo test

# Specific component
cargo test -p horus_core

# Python tests
cd horus_py
pytest tests/

# Benchmarks
cd benchmarks
cargo bench
```

### Acceptance Tests

User acceptance tests are in `tests/acceptance/` and document expected behavior.

**Before submitting a PR:**
1. Review relevant acceptance test files
2. Ensure changes align with documented behavior
3. Update tests if changing functionality
4. Add new scenarios for new features

```bash
# Review test documentation
cat tests/acceptance/README.md

# Check specific tests
cat tests/acceptance/horus_manager/01_new_command.md

# Manually verify scenarios
horus new test_project
cd test_project
horus run
```

## Code Style

### Rust

Follow standard conventions:
- Use `rustfmt`: `cargo fmt`
- Use `clippy`: `cargo clippy -- -D warnings`
- Document public APIs with `///` comments

```rust
/// Creates a new Hub for inter-process communication.
///
/// # Arguments
///
/// * `topic` - The topic name for this hub
///
/// # Examples
///
/// ```
/// let hub = Hub::<f32>::new("temperature")?;
/// ```
pub fn new(topic: &str) -> Result<Self> {
    // implementation
}
```

### Python

Follow PEP 8:
- Use `black` for formatting
- Use `mypy` for type checking
- Use descriptive variable names

### C

Follow standard conventions:
- Use `clang-format`
- Prefix all public APIs with `horus_`
- Document APIs in header files

## What to Contribute

### Good First Issues

Look for issues labeled `good-first-issue`:
- Documentation improvements
- Example programs
- Bug fixes
- Test coverage improvements

### Feature Requests

Before implementing a major feature:
1. Open an issue to discuss the proposal
2. Wait for maintainer feedback
3. Implement with tests and documentation

### Bug Reports

When reporting bugs, include:
- HORUS version (`horus --version`)
- Operating system and version
- Minimal reproducible example
- Expected vs actual behavior
- Relevant logs or error messages

## Pull Request Process

1. **Ensure tests pass**:
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

2. **Check acceptance tests**:
   - Review relevant test files in `tests/acceptance/`
   - Manually verify key scenarios
   - Update test scenarios if behavior changed
   - Add new scenarios for new features

3. **Update documentation**:
   - Update README.md for user-facing changes
   - Update inline code documentation
   - Add examples for new features
   - Update CHANGELOG.md

4. **Write clear commit messages**:
   ```
   Add feature: Brief description

   Detailed explanation of what changed and why.

   - Updated acceptance tests in tests/acceptance/...
   - Added new scenarios for ...

   Fixes #123
   ```

5. **Submit PR** with:
   - Clear title and description
   - Link to related issues
   - Screenshots/examples if UI changes
   - List of verified acceptance test scenarios

6. **Address review feedback** promptly

### PR Description Template

```markdown
## Description
Brief description of changes

## Changes
- Added feature X
- Fixed bug Y
- Updated tests

## Testing
- [ ] Unit tests pass (`cargo test`)
- [ ] Clippy passes (`cargo clippy`)
- [ ] Format check passes (`cargo fmt --check`)
- [ ] Verified acceptance test scenarios
- [ ] Updated/added acceptance tests

## Related Issues
Fixes #123
```

## Architecture Guidelines

### Core Principles

1. **Zero-copy when possible**: Use shared memory
2. **Type safety**: Leverage Rust's type system
3. **Minimal latency**: Profile and optimize hot paths
4. **Multi-language**: Ensure features work across Rust/Python/C

### Code Organization

```
horus/
horus_core/         # Core IPC implementation
horus_macros/       # Procedural macros
horus_py/           # Python bindings
horus_c/            # C bindings
horus_library/      # Standard messages/nodes
horus_daemon/       # Background service
horus_manager/      # CLI tool
docs-site/          # Documentation website
```

## What Not to Do

- Break existing APIs without migration path
- Add dependencies without discussion
- Commit without running tests
- Ignore clippy warnings
- Submit PRs without description

## Code Review

All contributions go through code review:
- Be respectful and constructive
- Respond to feedback promptly
- Ask questions if unclear
- Maintainers have final say

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.

All contributors must agree to the [Contributor License Agreement (CLA)](.github/CLA.md). When submitting your first pull request, add a comment:

```
I have read and agree to the Contributor License Agreement.
```

## Thank You!

Every contribution helps make HORUS better. Thank you for being part of the community!

---

Questions? Open an issue or start a discussion on GitHub!
