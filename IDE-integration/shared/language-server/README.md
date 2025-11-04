# HORUS Language Server

A shared Language Server Protocol (LSP) implementation for HORUS that provides code intelligence across all IDE integrations.

## Overview

The HORUS Language Server is a standalone Rust binary that implements LSP 3.17, providing:

- **Project Detection**: Automatically detects HORUS projects via `horus.yaml`
- **Code Intelligence**: Autocomplete, go-to-definition, hover documentation
- **HORUS-Specific Features**: Topic inspection, node graph data, live system monitoring
- **Diagnostics**: Real-time error checking and warnings
- **Symbol Resolution**: Navigate HORUS framework and project symbols

## Architecture

The language server communicates with IDEs via standard input/output using JSON-RPC 2.0:

```
IDE (VSCode/IntelliJ/Vim/Emacs)
         |
         | JSON-RPC over stdin/stdout
         ▼
HORUS Language Server (Rust)
         |
         ▼
HORUS Framework + Project Files
```

## Technology Stack

**Core Dependencies**:
- `tower-lsp 0.20`: LSP server framework
- `tokio 1.0`: Async runtime
- `serde/serde_json`: JSON-RPC serialization
- `serde_yaml`: Parse `horus.yaml` configuration

**HORUS Integration**:
- `syn`: Rust syntax parsing
- `ra_ap_syntax`: Advanced Rust analysis (same as rust-analyzer)

## Project Structure

```
language-server/
├── Cargo.toml              # Rust project manifest
├── README.md               # This file
└── src/
    ├── main.rs             # Entry point, stdio setup
    ├── server.rs           # LSP server implementation
    ├── backend.rs          # Request handlers
    ├── project.rs          # HORUS project detection/analysis
    ├── completion.rs       # Code completion logic
    ├── hover.rs            # Hover documentation
    ├── diagnostics.rs      # Error checking
    ├── symbols.rs          # Symbol indexing
    └── horus_integration/  # HORUS-specific features
        ├── mod.rs
        ├── topic_info.rs   # Topic inspection
        ├── graph_data.rs   # Node graph generation
        └── live_monitor.rs # Runtime monitoring
```

## LSP Capabilities

### Standard LSP Features

```rust
ServerCapabilities {
    text_document_sync: Some(TextDocumentSyncCapability::Kind(
        TextDocumentSyncKind::INCREMENTAL,
    )),
    completion_provider: Some(CompletionOptions {
        trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
        ..Default::default()
    }),
    hover_provider: Some(HoverProviderCapability::Simple(true)),
    definition_provider: Some(OneOf::Left(true)),
    references_provider: Some(OneOf::Left(true)),
    document_symbol_provider: Some(OneOf::Left(true)),
    workspace_symbol_provider: Some(OneOf::Left(true)),
    ..Default::default()
}
```

### Custom HORUS Methods

See [../docs/LSP_PROTOCOL.md](../docs/LSP_PROTOCOL.md) for detailed protocol documentation.

**`horus/topicInfo`**: Get information about a specific topic
```typescript
interface TopicInfoParams {
    topic: string;
}

interface TopicInfo {
    name: string;
    messageType: string;
    publishers: string[];
    subscribers: string[];
    currentValue?: any;
    messageRate?: number;
}
```

**`horus/graphData`**: Get node graph data for visualization
```typescript
interface GraphData {
    nodes: NodeInfo[];
    topics: TopicInfo[];
    connections: Connection[];
}
```

**`horus/listNodes`**: List all active nodes in running system
```typescript
interface NodeInfo {
    name: string;
    publishers: { topic: string; type: string }[];
    subscribers: { topic: string; type: string }[];
    tickRate: number;
    cpuUsage: number;
}
```

## Building

```bash
cd shared/language-server
cargo build --release
```

The binary will be at `target/release/horus-language-server`.

## Running Standalone

```bash
# Language server communicates via stdin/stdout
# Typically launched by IDE, but can test manually:
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
    ./target/release/horus-language-server
```

## IDE Integration

### VSCode

The VSCode extension launches the language server automatically:

```typescript
const serverOptions: ServerOptions = {
    command: 'horus-language-server',
    args: []
};

const client = new LanguageClient(
    'horusLanguageServer',
    'HORUS Language Server',
    serverOptions,
    clientOptions
);

await client.start();
```

### Neovim

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

configs.horus_ls = {
    default_config = {
        cmd = { 'horus-language-server' },
        filetypes = { 'rust' },
        root_dir = lspconfig.util.root_pattern('horus.yaml'),
    },
}

lspconfig.horus_ls.setup{}
```

### Emacs

```elisp
(lsp-register-client
 (make-lsp-client
  :new-connection (lsp-stdio-connection "horus-language-server")
  :major-modes '(rust-mode)
  :server-id 'horus-ls
  :activation-fn (lsp-activate-on "horus.yaml")))
```

## Development

### Running Tests

```bash
cargo test
```

### Logging

Set `RUST_LOG` for verbose output:

```bash
RUST_LOG=horus_language_server=debug horus-language-server
```

IDE extensions typically configure logging via LSP initialization options:

```typescript
{
    trace: {
        server: 'verbose'
    }
}
```

## Implementation Status

- [ ] Basic LSP server scaffold
- [ ] Project detection via `horus.yaml`
- [ ] HORUS_SOURCE resolution
- [ ] Dependency parsing
- [ ] Code completion
- [ ] Hover documentation
- [ ] Go-to-definition
- [ ] Symbol indexing
- [ ] Diagnostics
- [ ] Custom HORUS methods
- [ ] Runtime integration

## Contributing

See [../../ARCHITECTURE.md](../../ARCHITECTURE.md) for architectural guidelines.

## License

Apache License 2.0 - consistent with HORUS framework
