# HORUS IDE Integration

This directory contains IDE extensions and plugins for the HORUS robotics framework, providing native development environment support across multiple editors.

## Overview

HORUS IDE integrations provide:
- **Code Intelligence**: Autocomplete, type checking, go-to-definition
- **Build Integration**: Run HORUS commands directly from IDE
- **Debugging**: Native debugging support with breakpoints
- **HORUS-Specific Features**: Topic inspection, node graphs, live dashboard
- **No Configuration Required**: Detects HORUS projects via `horus.yaml`

## Available IDE Support

### Production Ready

**[VSCode Extension](./horus-vscode/)** - Official VSCode extension
- Status: Blueprint and specification complete
- Language Server Protocol (LSP) integration
- Debug Adapter Protocol (DAP) support
- Live dashboard integration
- Topic inspector and node graph visualization

### Planned

**[IntelliJ IDEA / CLion Plugin](./horus-intellij/)** - JetBrains IDE support
- Status: Planned
- Target: Rust developers using JetBrains IDEs
- Integration: Native IntelliJ Platform SDK

**[Vim/Neovim Plugin](./horus-vim/)** - Vim plugin
- Status: Planned
- Target: Terminal-based development
- Integration: LSP client (coc.nvim or native LSP)

**[Emacs Package](./horus-emacs/)** - Emacs integration
- Status: Planned
- Target: Emacs users
- Integration: lsp-mode

## Architecture

All IDE integrations share common components:

### Shared Language Server

Located in `shared/language-server/`, the HORUS Language Server provides:
- Project detection and analysis
- HORUS-specific code completion
- Symbol resolution
- Diagnostics and error reporting

The language server implements the Language Server Protocol (LSP 3.17), making it reusable across all LSP-compatible editors.

### Shared Debug Adapter

Located in `shared/debug-adapter/`, the HORUS Debug Adapter provides:
- Debug session management
- Breakpoint handling
- Variable inspection
- Integration with LLDB/GDB

The debug adapter implements the Debug Adapter Protocol (DAP), making it reusable across all DAP-compatible editors.

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    IDE Clients                               │
│  ┌──────────┐  ┌──────────┐  ┌────────┐  ┌────────────┐   │
│  │ VSCode   │  │ IntelliJ │  │  Vim   │  │   Emacs    │   │
│  └────┬─────┘  └────┬─────┘  └───┬────┘  └─────┬──────┘   │
└───────┼─────────────┼────────────┼──────────────┼──────────┘
        │             │            │              │
        ▼             ▼            ▼              ▼
    (LSP)         (Native)      (LSP)          (LSP)
        │             │            │              │
        └─────────────┴────────────┴──────────────┘
                      │
                      ▼
        ┌─────────────────────────────────┐
        │   HORUS Language Server (Rust)   │
        │  - Project Detection             │
        │  - Code Intelligence             │
        │  - HORUS-Specific Features       │
        └─────────────────────────────────┘
                      │
                      ▼
        ┌─────────────────────────────────┐
        │      HORUS Framework            │
        │  - horus.yaml parsing           │
        │  - Dependency resolution        │
        │  - Runtime integration          │
        └─────────────────────────────────┘
```

## Directory Structure

```
IDE-integration/
├── README.md                    # This file
├── ARCHITECTURE.md              # Shared architecture patterns
│
├── horus-vscode/                # VSCode extension
│   ├── README.md
│   ├── BLUEPRINT.md
│   ├── TECH_STACK.md
│   ├── IMPLEMENTATION_GUIDE.md
│   ├── TESTING_VALIDATION.md
│   ├── package.json             # (to be created)
│   └── src/                     # (to be created)
│
├── horus-intellij/              # IntelliJ IDEA / CLion plugin
│   └── README.md
│
├── horus-vim/                   # Vim/Neovim plugin
│   └── README.md
│
├── horus-emacs/                 # Emacs package
│   └── README.md
│
└── shared/                      # Shared components
    ├── language-server/         # LSP server implementation
    │   ├── Cargo.toml
    │   ├── README.md
    │   └── src/
    │       ├── main.rs
    │       ├── server.rs
    │       ├── project.rs
    │       ├── completion.rs
    │       └── diagnostics.rs
    │
    ├── debug-adapter/           # DAP implementation
    │   ├── Cargo.toml
    │   ├── README.md
    │   └── src/
    │
    ├── docs/                    # Cross-IDE documentation
    │   ├── LSP_PROTOCOL.md      # LSP custom extensions
    │   ├── DAP_PROTOCOL.md      # DAP custom extensions
    │   ├── TOPIC_INSPECTOR.md   # Topic inspection feature
    │   └── NODE_GRAPH.md        # Node graph feature
    │
    └── resources/               # Shared assets
        ├── icons/               # HORUS icons for IDEs
        └── syntax/              # Syntax highlighting definitions
```

## Development Priorities

### Phase 1: VSCode Extension (Current)
- Complete blueprint and specification
- Implement basic language server
- Create VSCode extension scaffold
- Add task and debug integration
- Implement HORUS-specific features

### Phase 2: Language Server Separation
- Extract language server to `shared/language-server/`
- Make LSP server IDE-agnostic
- Document LSP custom protocol extensions
- Create standalone LSP server binary

### Phase 3: Additional IDE Support
- IntelliJ IDEA / CLion plugin
- Vim/Neovim plugin
- Emacs package
- Evaluate other IDEs based on user demand

## Contributing New IDE Support

To add support for a new IDE:

1. **Create IDE-specific directory**:
   ```bash
   mkdir IDE-integration/horus-<ide-name>
   ```

2. **Use shared language server**:
   - Point to `shared/language-server/` for code intelligence
   - Implement IDE-specific LSP client

3. **Implement IDE-specific features**:
   - Task integration (run/build/check)
   - Debug adapter integration
   - Dashboard/visualization (if supported)

4. **Follow IDE conventions**:
   - VSCode: Extension in TypeScript
   - IntelliJ: Plugin in Kotlin
   - Vim: Plugin in Vimscript/Lua
   - Emacs: Package in Emacs Lisp

5. **Document thoroughly**:
   - README with setup instructions
   - Technical documentation
   - User guide

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed guidelines.

## Feature Parity Matrix

| Feature | VSCode | IntelliJ | Vim | Emacs |
|---------|--------|----------|-----|-------|
| **Core LSP** |
| Autocomplete | Planned | - | - | - |
| Go to Definition | Planned | - | - | - |
| Hover Documentation | Planned | - | - | - |
| Diagnostics | Planned | - | - | - |
| **Build Integration** |
| Run Tasks | Planned | - | - | - |
| Check Tasks | Planned | - | - | - |
| Build Tasks | Planned | - | - | - |
| **Debugging** |
| Breakpoints | Planned | - | - | - |
| Variable Inspection | Planned | - | - | - |
| Call Stack | Planned | - | - | - |
| **HORUS-Specific** |
| Dashboard | Planned | - | - | - |
| Topic Inspector | Planned | - | - | - |
| Node Graph | Planned | - | - | - |
| Live Monitoring | Planned | - | - | - |

## User Documentation

### For Users

**VSCode Users**: See [horus-vscode/README.md](./horus-vscode/README.md)

**Other IDE Users**: Check your IDE's directory for setup instructions (when available)

### For Developers

**Implementing IDE Support**: See [ARCHITECTURE.md](./ARCHITECTURE.md)

**Language Server Development**: See [shared/language-server/README.md](./shared/language-server/README.md)

**Debug Adapter Development**: See [shared/debug-adapter/README.md](./shared/debug-adapter/README.md)

## Installation

### VSCode

```bash
# From VSCode marketplace (when published)
code --install-extension horus.horus-vscode

# From .vsix file
code --install-extension horus-vscode-X.Y.Z.vsix
```

### Other IDEs

Installation instructions will be provided when support is added.

## Requirements

### All IDEs
- HORUS framework installed
- `HORUS_SOURCE` environment variable set (or configured in IDE settings)
- Active HORUS project (contains `horus.yaml`)

### Specific IDEs
- **VSCode**: Version 1.85.0 or later
- **IntelliJ/CLion**: Version 2023.2 or later (planned)
- **Vim/Neovim**: Neovim 0.8+ with LSP support (planned)
- **Emacs**: Version 27.1+ with lsp-mode (planned)

## Support

For issues or questions:
- VSCode Extension: [GitHub Issues](https://github.com/softmata/horus/issues)
- General IDE Support: [HORUS Discussions](https://github.com/softmata/horus/discussions)

## License

All IDE integrations are licensed under the Apache License 2.0, consistent with the HORUS framework.
