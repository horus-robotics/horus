# Contributing to HORUS

Thank you for your interest in contributing to HORUS! This document provides guidelines for contributing to the project.

## 🚀 Getting Started

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

## 🛠️ Development Setup

### Prerequisites

- Rust 1.70+ (`rustup update`)
- Python 3.9+ with `pip`
- GCC/Clang for C bindings
- Node.js 18+ for documentation site

### Building

```bash
# Build all Rust components
cargo build --release

# Build Python bindings
cd horus_py
maturin develop --release

# Build C bindings
cd horus_c
make

# Build documentation site
cd docs-site
npm install
npm run dev
```

## 🧪 Testing

```bash
# Rust tests
cargo test

# Python tests
cd horus_py
pytest tests/

# C tests
cd horus_c
make test

# Integration tests
./run_integration_tests.sh
```

## 📝 Code Style

### Rust

Follow standard Rust conventions:
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
pub fn new(topic: &str) -> Result<Self, HorusError> {
    // implementation
}
```

### Python

Follow PEP 8:
- Use `black` for formatting
- Use `mypy` for type checking
- Use descriptive variable names

### C

Follow standard C conventions:
- Use `clang-format`
- Prefix all public APIs with `horus_`
- Document APIs in header files

## 🎯 What to Contribute

### Good First Issues

Look for issues labeled `good-first-issue`:
- Documentation improvements
- Example programs
- Bug fixes in existing code
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

## 📚 Documentation

- Update documentation when changing APIs
- Add examples for new features
- Keep README.md up to date
- Update CHANGELOG.md

## 🔄 Pull Request Process

1. **Ensure tests pass**: `cargo test && pytest`
2. **Update documentation**: Include docs for new features
3. **Write clear commit messages**:
   ```
   Add feature: Brief description

   Detailed explanation of what changed and why.
   Fixes #123
   ```
4. **Submit PR** with:
   - Clear title and description
   - Link to related issues
   - Screenshots/examples if UI changes

5. **Address review feedback** promptly

## 🏗️ Architecture Guidelines

### Core Principles

1. **Zero-copy when possible**: Use shared memory, avoid serialization
2. **Type safety**: Leverage Rust's type system
3. **Minimal latency**: Profile and optimize hot paths
4. **Multi-language**: Ensure features work across Rust/Python/C

### Code Organization

```
horus/
├── horus_core/         # Core IPC implementation
├── horus_macros/       # Procedural macros
├── horus_py/           # Python bindings
├── horus_c/            # C bindings
├── horus_library/      # Standard messages/nodes
├── horus_daemon/       # Background service
├── horus_manager/      # CLI tool
└── docs-site/          # Documentation website
```

## ⚠️ What Not to Do

- ❌ Break existing APIs without migration path
- ❌ Add dependencies without discussion
- ❌ Commit without running tests
- ❌ Ignore clippy warnings
- ❌ Submit PRs without description

## 🤝 Code Review

All contributions go through code review:
- Be respectful and constructive
- Respond to feedback promptly
- Ask questions if unclear
- Maintainers have final say

## 📜 License

By contributing, you agree that your contributions will be licensed under the MIT License.

## 🙏 Thank You!

Every contribution, no matter how small, helps make HORUS better. Thank you for being part of the community!

---

Questions? Open an issue or start a discussion on GitHub!
