# HORUS Debug Adapter

A shared Debug Adapter Protocol (DAP) implementation for HORUS that provides debugging capabilities across all IDE integrations.

## Overview

The HORUS Debug Adapter is a standalone Rust binary that implements DAP 1.51, providing:

- **Launch/Attach**: Start HORUS applications with debug symbols
- **Breakpoints**: Set breakpoints in HORUS nodes
- **Step Control**: Step through, step over, step into
- **Variable Inspection**: Inspect HORUS topics, node state, and variables
- **Call Stack**: Navigate call stack with HORUS context
- **LLDB/GDB Integration**: Low-level debugger integration

## Architecture

The debug adapter communicates with IDEs via standard input/output using JSON-RPC:

```
IDE Debug UI (VSCode/IntelliJ/Vim)
         |
         | JSON-RPC over stdin/stdout
         ▼
HORUS Debug Adapter (Rust)
         |
         | MI Protocol
         ▼
LLDB/GDB Debugger
         |
         ▼
HORUS Application Process
```

## Technology Stack

**Core Dependencies**:
- `dap-rs`: Debug Adapter Protocol implementation
- `tokio`: Async runtime
- `serde/serde_json`: JSON-RPC serialization

**Debugger Integration**:
- `lldb-sys`: LLDB bindings (primary)
- `gdb-command`: GDB fallback support

## Project Structure

```
debug-adapter/
├── Cargo.toml              # Rust project manifest
├── README.md               # This file
└── src/
    ├── main.rs             # Entry point
    ├── adapter.rs          # DAP adapter implementation
    ├── debugger/           # Debugger backends
    │   ├── mod.rs
    │   ├── lldb.rs         # LLDB integration
    │   └── gdb.rs          # GDB integration
    ├── horus_integration/  # HORUS-specific debugging
    │   ├── mod.rs
    │   ├── topic_watch.rs  # Watch topic values
    │   └── node_state.rs   # Node state inspection
    └── utils.rs
```

## DAP Capabilities

### Standard DAP Features

```json
{
    "supportsConfigurationDoneRequest": true,
    "supportsFunctionBreakpoints": true,
    "supportsConditionalBreakpoints": true,
    "supportsHitConditionalBreakpoints": true,
    "supportsEvaluateForHovers": true,
    "supportsStepBack": false,
    "supportsSetVariable": true,
    "supportsRestartFrame": true,
    "supportsGotoTargetsRequest": true,
    "supportsStepInTargetsRequest": true,
    "supportsCompletionsRequest": true,
    "supportsModulesRequest": true,
    "supportsExceptionOptions": true,
    "supportsValueFormattingOptions": true,
    "supportsExceptionInfoRequest": true,
    "supportTerminateDebuggee": true,
    "supportSuspendDebuggee": true,
    "supportsDelayedStackTraceLoading": true,
    "supportsLoadedSourcesRequest": true,
    "supportsLogPoints": true,
    "supportsTerminateThreadsRequest": true,
    "supportsSetExpression": true,
    "supportsDataBreakpoints": false,
    "supportsReadMemoryRequest": true,
    "supportsWriteMemoryRequest": true,
    "supportsDisassembleRequest": true
}
```

### Custom HORUS Features

See [../docs/DAP_PROTOCOL.md](../docs/DAP_PROTOCOL.md) for detailed protocol documentation.

**Topic Watches**: Monitor HORUS topic values in real-time during debugging

**Node State Inspection**: View internal state of HORUS nodes

**Publisher/Subscriber Tracking**: See which nodes are communicating during execution

## Launch Configuration

### VSCode launch.json

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "horus",
            "request": "launch",
            "name": "Debug HORUS Node",
            "program": "${file}",
            "args": [],
            "cwd": "${workspaceFolder}",
            "horusDebug": true,
            "stopOnEntry": false,
            "environment": [],
            "externalConsole": false,
            "MIMode": "lldb"
        }
    ]
}
```

### IntelliJ Run Configuration

```kotlin
HorusDebugConfiguration(
    name = "Debug HORUS Node",
    programPath = "\${file}",
    workingDirectory = "\${workspaceFolder}",
    debuggerType = DebuggerType.LLDB
)
```

## Building

```bash
cd shared/debug-adapter
cargo build --release
```

The binary will be at `target/release/horus-debug-adapter`.

## Running Standalone

```bash
# Debug adapter communicates via stdin/stdout
# Typically launched by IDE, but can test manually:
./target/release/horus-debug-adapter
```

## IDE Integration

### VSCode

```typescript
const debugAdapterDescriptor = new vscode.DebugAdapterExecutable(
    'horus-debug-adapter',
    []
);

vscode.debug.registerDebugAdapterDescriptorFactory('horus', {
    createDebugAdapterDescriptor(session: vscode.DebugSession) {
        return debugAdapterDescriptor;
    }
});
```

### IntelliJ

```kotlin
class HorusDebugRunner : ProgramRunner<RunnerSettings> {
    override fun execute(environment: ExecutionEnvironment) {
        val debugProcess = HorusDebugProcess(
            commandLine = "horus-debug-adapter"
        )
        // ...
    }
}
```

## Debugger Backend Selection

The debug adapter automatically selects the best available debugger:

1. **LLDB** (preferred): Modern, better Rust support
2. **GDB**: Fallback if LLDB unavailable

Users can override via launch configuration:

```json
{
    "MIMode": "lldb"  // or "gdb"
}
```

## HORUS-Specific Debugging

### Topic Value Watches

Set watches on HORUS topics to see published values during debugging:

```rust
// In debugger, add watch expression:
topic("cmd_vel")
```

This resolves to the latest message on the `cmd_vel` topic.

### Node State Inspection

View internal state of HORUS nodes:

```rust
// Watch expression:
node("controller").state
```

### Publisher/Subscriber Tracking

See communication graph during execution:

```rust
// Watch expression:
topic("cmd_vel").publishers  // List of publishing nodes
topic("cmd_vel").subscribers // List of subscribing nodes
```

## Development

### Running Tests

```bash
cargo test
```

### Logging

Set `RUST_LOG` for verbose output:

```bash
RUST_LOG=horus_debug_adapter=debug horus-debug-adapter
```

## Implementation Status

- [ ] Basic DAP server scaffold
- [ ] LLDB integration
- [ ] GDB integration
- [ ] Breakpoint handling
- [ ] Step control
- [ ] Variable inspection
- [ ] Call stack
- [ ] Topic watches
- [ ] Node state inspection
- [ ] Publisher/subscriber tracking

## Troubleshooting

### LLDB Not Found

Install LLDB:

```bash
# Ubuntu/Debian
sudo apt install lldb

# macOS (via Xcode Command Line Tools)
xcode-select --install

# Arch Linux
sudo pacman -S lldb
```

### Breakpoints Not Hitting

Ensure debug symbols are enabled in Cargo.toml:

```toml
[profile.dev]
debug = true

[profile.release]
debug = true
```

### Permission Denied on Linux

LLDB may require ptrace permissions:

```bash
echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope
```

## Contributing

See [../../ARCHITECTURE.md](../../ARCHITECTURE.md) for architectural guidelines.

## License

Apache License 2.0 - consistent with HORUS framework
