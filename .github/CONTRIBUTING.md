# Contributing to HORUS

Thank you for your interest in contributing to HORUS! This guide will help you get started.

## Getting Started

### Prerequisites
- Rust 1.70+ (`rustup` from https://rustup.rs)
- Git
- For Python bindings: Python 3.9+
- For C++ bindings: C++17 compiler

### Setting Up Development Environment

```bash
# Clone the repository
git clone https://github.com/softmata/horus.git
cd horus

# Install HORUS in development mode
./install.sh

# Run tests
cargo test --workspace
```

## How to Contribute

### Reporting Bugs
Use the **Bug Report** template when creating an issue. Include:
- Clear description of the issue
- Steps to reproduce
- Expected vs actual behavior
- HORUS version (`horus --version`)
- OS and language (Rust/Python/C++)

### Suggesting Features
Use the **Feature Request** template. Describe:
- The problem you're trying to solve
- Your proposed solution
- Any alternatives you've considered

### Finding Tasks

Look for issues labeled:
- **good first issue**: Great for newcomers
- **help wanted**: We'd appreciate contributions
- **documentation**: Docs improvements
- **bug**: Bug fixes needed

## Development Workflow

### 1. Fork and Branch
```bash
# Fork the repo on GitHub, then:
git checkout -b feature/your-feature-name
```

### 2. Make Changes
- Follow existing code style
- Add tests for new features
- Update documentation if needed
- Run `cargo fmt` and `cargo clippy`

### 3. Test Your Changes
```bash
# Run all tests
cargo test --workspace

# Test specific component
cargo test -p horus_core
cargo test -p horus_manager

# Run benchmarks (if relevant)
cd benchmarks
cargo run --release --bin [benchmark_name]
```

### 4. Commit Guidelines
Write clear commit messages:
```
Add horus check validation for project configuration

- Validates horus.yaml syntax and structure
- Checks for missing dependencies
- Detects circular path dependencies
- Validates code syntax (Rust/Python/C++)

Closes #123
```

### 5. Submit Pull Request
- Push to your fork
- Create PR against `main` branch
- Fill out the PR template
- Link related issues

## Code Style

### Rust
- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Address `cargo clippy` warnings
- Keep functions focused and documented

### Python
- Follow PEP 8
- Use type hints where possible
- Document public APIs

### C++
- Follow C++17 best practices
- Use RAII for resource management
- Document public interfaces

## Project Structure

```
horus/
├── horus_core/          # Core IPC and messaging
├── horus_manager/       # CLI tool (horus command)
├── horus_macros/        # Rust proc macros
├── horus_library/       # Standard message types
├── horus_py/            # Python bindings
├── horus_cpp/           # C++ framework
├── benchmarks/          # Performance benchmarks
├── docs-site/           # Documentation website
└── examples/            # Example projects
```

## Testing

### Unit Tests
```bash
cargo test --workspace
```

### Integration Tests
```bash
# Framework integration tests
cd horus_manager
cargo test

# CLI tests
cd horus_manager/tests
./run_integration_tests.sh
```

### Performance Tests
```bash
cd benchmarks
cargo run --release --bin ipc_benchmark
```

## Documentation

- Code comments: Document public APIs and complex logic
- Docs site: Update `/docs-site/content/docs/` for user-facing changes
- README updates: Keep READMEs in sync with changes

## Communication

- **GitHub Issues**: For bugs, features, and discussions
- **Pull Requests**: For code contributions
- **Discussions**: For questions and general topics

## Recognition

Contributors will be recognized in:
- Release notes
- CONTRIBUTORS file
- Git commit history

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (Apache-2.0).

---

**Questions?** Open a discussion or reach out in an issue. We're here to help!

Thank you for contributing to HORUS! 🚀
