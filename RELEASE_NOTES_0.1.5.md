# HORUS v0.1.5

## Release Notes

This release brings version consistency across the entire HORUS ecosystem, updates to the core framework, and improvements to external tooling.

## What's Changed

### Version Synchronization
- All core packages updated to v0.1.5
- Python bindings synchronized to v0.1.5
- C++ API updated to v0.1.5
- Documentation site updated to v0.1.5
- External tools (marketplace, discord bot) updated to v0.1.5

### Core Packages
- `horus` - Main CLI tool
- `horus_core` - Core IPC and messaging library
- `horus_library` - Standard robotics nodes and messages
- `horus_py` - Python bindings
- `horus_cpp` - C++ API bindings
- `horus_manager` - Package manager
- `horus_router` - Message routing
- `horus_macros` - Procedural macros

### Python Packages
- `horus` - Python bindings (PyPI)
- `horus.library` - Standard robotics messages
- `sim3d_rl` - 3D simulation RL environments

### Documentation
- Updated all version badges to v0.1.5
- Updated installation path examples
- Updated CLI version references
- Synchronized API documentation versions

### External Ecosystem
- **horus-marketplace**: Registry and package repository updated to v0.1.5
- **horus-discord-bot**: Community bot updated to display v0.1.5-alpha

## Installation

### From Source
```bash
git clone https://github.com/your-org/HORUS
cd HORUS
./install.sh
```

### Verify Installation
```bash
horus --version  # Should show 0.1.5
```

## Compatibility

### Binary Compatibility
All message types remain binary-compatible with previous 0.1.x releases. Shared memory IPC between different language bindings (Rust, Python, C++) continues to work seamlessly.

### API Compatibility
This release maintains API compatibility with v0.1.4. No breaking changes have been introduced.

## Upgrade Guide

If upgrading from v0.1.4 or earlier:

1. Pull the latest changes:
   ```bash
   git pull origin main
   ```

2. Run the installation script:
   ```bash
   ./install.sh
   ```

3. Verify the upgrade:
   ```bash
   horus --version
   ```

4. Update your project dependencies in `horus.yaml`:
   ```yaml
   dependencies:
     horus_core: "^0.1.5"
     horus_library: "^0.1.5"
   ```

## Package Locations

After installation, packages are located in:
- Rust: `~/.horus/cache/horus_core-0.1.5/`
- Python: `~/.horus/cache/horus_py-0.1.5/`
- C++: `~/.horus/cache/horus_cpp-0.1.5/`
- Library: `~/.horus/cache/horus_library-0.1.5/`

## Known Issues

None reported for this release.

## Contributors

Thank you to all contributors who made this release possible.

## Links

- Documentation: https://code.claude.com/docs
- Marketplace: https://horus-marketplace-api.onrender.com
- Discord: Join our community
- Issues: Report bugs on GitHub

---

**Full Changelog**: https://github.com/your-org/HORUS/compare/v0.1.4...v0.1.5
