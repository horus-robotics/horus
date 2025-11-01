# Changelog

All notable changes to HORUS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Enhanced verify.sh**: Now includes comprehensive build verification
  - Tests all subcommands (`new`, `run`, `dashboard`, `pkg`, `env`, `auth`, `version`)
  - Runs `cargo check` to verify zero warnings
  - Validates debug binary functionality
  - Detects and reports codebase health issues
- **Link Single-Slot Optimization**: 29% performance improvement over Hub in 1P1C scenarios
  - Median latency: 312ns (624 cycles @ 2GHz)
  - P95 latency: 444ns, P99 latency: 578ns
  - Burst throughput: 6.05 MHz (6M+ msg/s)
  - Bandwidth: Up to 369 MB/s for burst messages
  - Production-validated with 6.2M+ test messages, zero corruptions

### Changed
- **Code Quality**: Eliminated all compiler warnings across the workspace
  - Removed unused code and dead imports
  - Marked future features with `#[allow(dead_code)]`
  - Updated Bevy deprecated APIs (Camera2dBundle → Camera2d, SpriteBundle → Sprite)
  - Fixed privacy warnings in sim2d
- **UI/UX**: Replaced all Unicode emojis with ASCII equivalents
  - Shell scripts now use `[+]`, `[x]`, `[!]`, `[i]`, `[*]`, `[#]`, `[>]` instead of Unicode symbols
  - Improved terminal compatibility across different platforms
  - Preserved React icon components in horus-marketplace
- **Documentation**: Major documentation improvements
  - **Positioned HORUS as production ROS2 competitor** with direct comparison table
  - Clarified `horus run` single-file design as feature, not limitation
  - Updated README.md with ROS2 performance comparison (50-500x faster IPC)
  - Enhanced CLI reference docs with examples
  - Added complete snakesim demo to docs-site examples
  - Added guidance for multi-crate workspace projects (use `cargo` directly)
  - Fixed installation docs: "Full reinstall" now includes `git pull` before `./install.sh`
- **Snakesim Architecture**: Restructured to proper single-file HORUS project
  - Merged multi-crate structure into single main.rs file
  - Now compatible with `horus run` command
  - GUI remains as separate binary (snakesim_gui) due to eframe event loop
  - GUI features: 20x20 grid, bright green snake with animated eyes, smooth 200ms updates
  - Pre-built GUI binary included in examples installation

### Fixed
- Implemented missing `--clean` flag functionality in `horus run`
- Updated deprecated Bevy APIs in sim2d (0.15 compatibility)
- Fixed workspace conflict by excluding `.horus/` directory from workspace members
- Removed debug code (`dbg!` statements) from snakesim_gui
- **Fixed `horus run` to work for regular users without HORUS source code**
  - Modified `find_horus_source_dir()` to fall back to `~/.horus/cache/` when source not found
  - Updated Cargo.toml generation to detect cache vs source installations
  - Modified install.sh to copy workspace Cargo.toml and source files to cache
  - Added examples directory (`~/.horus/cache/horus@0.1.0/examples/`) with snakesim
  - Users can now run HORUS projects with only the installed packages
  - Verified end-to-end: users can copy examples and run them successfully

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
- Production-grade latency: 312ns-481ns (Link/Hub SPSC/MPMC)
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

**Performance Benchmarks (x86_64, cross-core):**
- **Link (SPSC)**: 312ns median, 6M+ msg/s, 369 MB/s burst bandwidth
- **Hub (MPMC)**: 481ns median, flexible pub/sub architecture
- Link is 29% faster than Hub in 1P1C scenarios
- Production-validated with 6.2M+ test messages, zero corruptions

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
- [HORUS Repository](https://github.com/horus-robotics/horus)
- [HORUS Documentation](https://docs.horus-registry.dev)
