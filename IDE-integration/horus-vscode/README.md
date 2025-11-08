# HORUS VSCode Extension

Official VSCode extension for the HORUS robotics framework, providing native IDE support without requiring Cargo.toml configuration.

## Documentation

This directory contains the complete blueprint for implementing the HORUS VSCode extension:

- **[BLUEPRINT.md](./BLUEPRINT.md)** - Complete technical specification and architecture
- **[TECH_STACK.md](./TECH_STACK.md)** - Technology choices and dependencies
- **[IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md)** - Step-by-step implementation instructions
- **[TESTING_VALIDATION.md](./TESTING_VALIDATION.md)** - Testing strategy and validation procedures

## Quick Overview

The HORUS VSCode extension provides:

1. **Language Server Protocol Support**
   - Autocomplete for HORUS types and functions
   - Type checking without Cargo.toml
   - Go to definition
   - Hover documentation

2. **Build Integration**
   - Run HORUS commands directly from VSCode
   - Task provider for build/run/check
   - Problem matcher for error highlighting

3. **Debug Support**
   - Integrated debugging via Debug Adapter Protocol
   - Breakpoints in HORUS applications
   - Variable inspection

4. **HORUS-Specific Features**
   - Live dashboard integration
   - Topic inspector (hover over topic strings)
   - Node graph visualization
   - Real-time performance monitoring

## Architecture

```
Extension (TypeScript)
    ├── Language Client  Language Server (Rust)
    ├── Task Provider  HORUS CLI
    ├── Debug Adapter  LLDB/GDB
    └── Dashboard Panel  HORUS Dashboard
```

## Development Status

**Current**: Planning and specification phase

**Next Steps**:
1. Set up project structure
2. Implement basic language server
3. Create extension scaffold
4. Add task integration
5. Implement dashboard
6. Add debugging support
7. Create topic inspector
8. Build node graph visualization

## Getting Started

See [IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md) for detailed setup and implementation instructions.

## Contributing

This extension is part of the HORUS robotics framework. For contribution guidelines, see the main [HORUS repository](https://github.com/softmata/horus).

## License

Apache License 2.0 - See main HORUS repository for details.
