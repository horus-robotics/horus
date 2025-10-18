# Changelog

All notable changes to HORUS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Known Issues
- Dashboard TUI mode is incomplete
- Remote deployment lacks versioning and rollback features
- Python bindings missing type hints
- C bindings support minimal operations only

## [0.1.0-alpha] - 2024-10-18

### Added

#### Core Framework
- Lock-free, zero-copy shared memory pub/sub system (Hub)
- Priority-based scheduler with deterministic execution (0-255 priority levels)
- Node trait with init/tick/shutdown lifecycle
- Production-grade latency: 366ns-2.8us for 16B-120KB messages
- POSIX shared memory implementation with cache-line alignment
- Built-in Ctrl+C handling with proper cleanup
- Comprehensive logging with IPC timing metrics

#### CLI Tool
- `horus new` - Create projects with Rust, Python, or C templates
- `horus run` - Smart build and execution with release mode support
- `horus pkg` - Package installation, removal, and search
- `horus auth` - GitHub OAuth authentication
- `horus publish` - Package publishing to registry
- `horus env` - Environment freeze and restore
- `horus dashboard` - Web dashboard and TUI monitoring

#### Package Registry and Marketplace
- Full-featured registry backend with Axum and SQLite
- Package upload/download with version management
- GitHub OAuth authentication for publishers
- Package search and metadata APIs
- Documentation serving (local and external)
- Import resolution system for dependency management
- Environment freeze/restore for reproducible builds
- Web-based marketplace frontend with Next.js
- Category filtering and package browsing
- Package documentation viewer
- Developer portal for package management

#### Language Bindings
- Python bindings via PyO3 FFI
- C bindings for hardware driver integration
- Multi-language project templates

#### Examples and Tools
- SnakeSim: Multi-node game demo with keyboard input, game logic, and GUI
- Sim2D: 2D physics simulator with Bevy visualization and Rapier2D
- Template projects for all supported languages

#### Developer Tools
- GitHub Actions CI/CD with multi-OS testing (Ubuntu 20.04, 22.04)
- Automated benchmarking with PR comments
- rustfmt and clippy enforcement
- Multi-language test scripts

#### Documentation
- Comprehensive README with quick start guide
- CONTRIBUTING.md with development workflow
- Contributor License Agreement (CLA)
- Documentation website with MDX pages
- API documentation for core components

### Changed
- N/A (initial release)

### Deprecated
- N/A (initial release)

### Removed
- N/A (initial release)

### Fixed
- N/A (initial release)

### Security
- Memory-safe messaging with Rust ownership system
- Fixed-size message structures prevent buffer overflows
- Process isolation with proper shared memory permissions
- GitHub OAuth for package publishing authentication

## Release Notes

### v0.1.0-alpha - Alpha Release

This is the initial alpha release of HORUS. The core framework is production-grade with proven sub-microsecond latency, but ecosystem tools are still in development.

**What Works:**
- Core pub/sub messaging (Hub)
- Priority-based scheduling
- Multi-language support (Rust, Python, C)
- Project creation and execution
- Basic monitoring and logging
- Package registry backend with full CRUD operations
- Marketplace web frontend for browsing packages
- GitHub OAuth authentication for publishing

**What's Incomplete:**
- Dashboard TUI mode (web dashboard works)
- Remote deployment versioning and rollback
- Python type hints
- Advanced C bindings

**Performance Benchmarks:**
- 16B messages: 366ns
- 304B IMU data: 543ns
- 1.5KB LaserScan: 1.58us
- 120KB PointCloud: 2.8us

**Breaking Changes:**
- None (initial release)

**Migration Guide:**
- None (initial release)

## Version History

- [0.1.0-alpha] - 2024-10-18 - Initial alpha release

---

## Unreleased Changes

Track unreleased changes here as development continues:

### To Be Added
- Finish dashboard TUI mode
- Add versioning and rollback to remote deployment
- Python type hints for bindings
- Extended C API for advanced use cases
- Performance profiling tools
- Automated integration tests
- Docker development environment
- Video tutorials and guides

### To Be Fixed
- Stabilize real-time dashboard updates
- Improve error messages for build failures
- Handle edge cases in shared memory cleanup

---

## Change Categories

Changes are grouped using the following categories:

- **Added** - New features
- **Changed** - Changes to existing functionality
- **Deprecated** - Soon-to-be removed features
- **Removed** - Removed features
- **Fixed** - Bug fixes
- **Security** - Security improvements

## Links

- [Keep a Changelog](https://keepachangelog.com/)
- [Semantic Versioning](https://semver.org/)
- [HORUS Repository](https://github.com/lord-patpak/horus)
- [HORUS Documentation](https://docs.horus-registry.dev)
