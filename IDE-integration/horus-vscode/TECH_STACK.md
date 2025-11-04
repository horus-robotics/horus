# HORUS VSCode Extension - Technology Stack

## Overview

This document details the specific technologies, libraries, and tools used in the HORUS VSCode extension implementation.

## Core Technologies

### VSCode Extension (TypeScript)

**Runtime**: Node.js 18.x LTS
- Reason: Required by VSCode Extension API
- Benefits: Async/await support, modern JavaScript features
- Version constraint: >= 18.0.0

**Language**: TypeScript 5.x
- Reason: Type safety, better IDE support, compile-time error detection
- Configuration: Strict mode enabled
- Target: ES2022

**Build Tool**: esbuild
- Reason: 100x faster than webpack, simple configuration
- Output: Single bundled file (extension.js)
- Minification: Enabled for production

### Language Server (Rust)

**Rust Version**: 1.75+ (stable channel)
- Reason: Latest stable features, no nightly required
- Edition: 2021

**Core Framework**: tower-lsp 0.20.x
```toml
[dependencies]
tower-lsp = "0.20"
```
- Reason: Most mature Rust LSP implementation
- Features: Async/await, JSON-RPC handling, type-safe protocol
- Alternatives considered: lsp-server (too low-level), lsp-types (just types)

**Async Runtime**: tokio 1.x
```toml
tokio = { version = "1.0", features = ["full"] }
```
- Reason: De facto standard for async Rust
- Features: Multi-threaded runtime, timers, channels
- Configuration: Full feature set for flexibility

**Serialization**: serde + serde_json + serde_yaml
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
```
- Reason: Industry standard for Rust serialization
- Usage: LSP messages (JSON), horus.yaml parsing (YAML)

## Extension Dependencies

### Production Dependencies

**vscode-languageclient** (^9.0.0)
```typescript
import { LanguageClient } from 'vscode-languageclient/node';
```
- Purpose: LSP client implementation
- Features: Automatic reconnection, middleware support
- Protocol: LSP 3.17

**vscode-debugadapter** (^1.51.0)
```typescript
import { DebugSession } from 'vscode-debugadapter';
```
- Purpose: Debug adapter protocol implementation
- Features: Breakpoints, stepping, variable inspection

### Development Dependencies

**@types/vscode** (^1.85.0)
- TypeScript definitions for VSCode API
- Ensures type safety for extension development

**@types/node** (^18.x)
- Node.js type definitions
- Required for file system, process operations

**@typescript-eslint/eslint-plugin** (^6.0.0)
- Linting for TypeScript code
- Rules: Recommended + strict

**prettier** (^3.0.0)
- Code formatting
- Configuration: 2-space indent, single quotes

**mocha** (^10.0.0)
- Test framework
- Integration with @vscode/test-electron

**@vscode/test-electron** (^2.3.0)
- VSCode extension testing framework
- Launches VSCode instance for integration tests

## Language Server Dependencies

### Core LSP

**tower-lsp** (0.20.x)
```toml
[dependencies]
tower-lsp = "0.20"
```
- Provides: LspService, LanguageServer trait, request handlers
- Protocol support: Full LSP 3.17 specification

**lsp-types** (0.94.x)
```toml
lsp-types = "0.94"
```
- Automatically included by tower-lsp
- Provides: All LSP type definitions

### Project Analysis

**syn** (2.0.x)
```toml
syn = { version = "2.0", features = ["full", "parsing"] }
```
- Purpose: Rust syntax tree parsing
- Usage: Analyze Rust source files for symbols
- Features: Full syntax support, macro expansion

**quote** (1.0.x)
```toml
quote = "1.0"
```
- Purpose: Code generation
- Usage: Generate completions, macro expansions

**proc-macro2** (1.0.x)
```toml
proc-macro2 = "1.0"
```
- Purpose: Token manipulation
- Usage: Parse and analyze macro invocations

### File System and I/O

**walkdir** (2.4.x)
```toml
walkdir = "2.4"
```
- Purpose: Recursive directory traversal
- Usage: Index project files

**globset** (0.4.x)
```toml
globset = "0.4"
```
- Purpose: Pattern matching
- Usage: .gitignore support, file filtering

**notify** (6.1.x)
```toml
notify = "6.1"
```
- Purpose: File system watching
- Usage: Detect file changes for re-indexing

### Concurrency and Parallelism

**tokio** (1.x) - Already listed above

**rayon** (1.8.x)
```toml
rayon = "1.8"
```
- Purpose: Data parallelism
- Usage: Parallel file parsing, indexing

**dashmap** (5.5.x)
```toml
dashmap = "5.5"
```
- Purpose: Concurrent HashMap
- Usage: Thread-safe symbol cache

### Logging and Diagnostics

**tracing** (0.1.x)
```toml
tracing = "0.1"
```
- Purpose: Structured logging
- Features: Spans, events, levels

**tracing-subscriber** (0.3.x)
```toml
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```
- Purpose: Log consumption and formatting
- Features: Environment-based filtering, JSON output

### Utilities

**anyhow** (1.0.x)
```toml
anyhow = "1.0"
```
- Purpose: Error handling
- Usage: Simplify error propagation

**thiserror** (1.0.x)
```toml
thiserror = "1.0"
```
- Purpose: Custom error types
- Usage: Domain-specific errors

**once_cell** (1.19.x)
```toml
once_cell = "1.19"
```
- Purpose: Lazy initialization
- Usage: Static caches, global state

**dirs** (5.0.x)
```toml
dirs = "5.0"
```
- Purpose: Platform-specific directories
- Usage: Find home directory, config paths

## HORUS Integration

### Workspace Dependencies

**horus_core**
```toml
horus_core = { path = "../horus_core" }
```
- Purpose: Core HORUS types and utilities
- Usage: Understand HORUS message types, node structure

**horus_manager**
```toml
horus_manager = { path = "../horus_manager" }
```
- Purpose: HORUS project management
- Usage: Parse horus.yaml, resolve dependencies

**horus_macros** (analysis only)
```toml
horus_macros = { path = "../horus_macros" }
```
- Purpose: Understand macro expansion
- Usage: Provide macro completion, documentation

## Build and Packaging

### Extension Packaging

**@vscode/vsce** (^2.22.0)
```bash
npm install -g @vscode/vsce
```
- Purpose: Package and publish extensions
- Commands: `vsce package`, `vsce publish`

**esbuild** (^0.19.0)
```json
{
  "scripts": {
    "compile": "esbuild ./src/extension.ts --bundle --outfile=out/extension.js --external:vscode --format=cjs --platform=node"
  }
}
```
- Configuration: Bundle, minify, external vscode module

### Language Server Build

**cargo**
```bash
cargo build --release --manifest-path=server/Cargo.toml
```
- Output: `server/target/release/horus-language-server`
- Optimization: LTO enabled, strip symbols

**Post-build**: Copy binary to extension
```bash
mkdir -p bin
cp server/target/release/horus-language-server bin/
```

## Development Tools

### VSCode Extensions for Development

**Rust Analyzer**
- Purpose: Rust language support while developing server
- Install: `code --install-extension rust-lang.rust-analyzer`

**ESLint**
- Purpose: TypeScript linting
- Install: `code --install-extension dbaeumer.vscode-eslint`

**Prettier**
- Purpose: Code formatting
- Install: `code --install-extension esbenp.prettier-vscode`

### CLI Tools

**rustfmt**
```bash
rustup component add rustfmt
```
- Purpose: Rust code formatting
- Usage: `cargo fmt --all`

**clippy**
```bash
rustup component add clippy
```
- Purpose: Rust linting
- Usage: `cargo clippy --all-targets`

**cargo-watch**
```bash
cargo install cargo-watch
```
- Purpose: Auto-rebuild on file changes
- Usage: `cargo watch -x build`

## Testing Infrastructure

### Extension Tests

**Mocha** (^10.0.0)
```json
{
  "scripts": {
    "test": "node ./out/test/runTest.js"
  }
}
```
- Test runner configuration
- Integration with VSCode test environment

**@vscode/test-electron** (^2.3.0)
```typescript
import { runTests } from '@vscode/test-electron';
```
- Downloads and runs VSCode instance
- Loads extension in test mode

**chai** (^4.3.0)
```typescript
import { expect } from 'chai';
```
- Assertion library
- Fluent syntax

### Language Server Tests

**tokio-test** (^0.4.0)
```toml
[dev-dependencies]
tokio-test = "0.4"
```
- Testing utilities for async code
- Mock timers, assertions

**insta** (^1.34.0)
```toml
[dev-dependencies]
insta = "1.34"
```
- Snapshot testing
- Compare LSP responses with expected output

**criterion** (^0.5.0)
```toml
[dev-dependencies]
criterion = "0.5"
```
- Benchmarking framework
- Measure performance regressions

## Performance Considerations

### Language Server Optimization

**Compilation Flags** (Cargo.toml):
```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
```
- Reason: Maximum performance for LSP responsiveness
- Trade-off: Longer compile time

**Caching Strategy**:
```rust
use dashmap::DashMap;
use once_cell::sync::Lazy;

static SYMBOL_CACHE: Lazy<DashMap<PathBuf, Vec<Symbol>>> =
    Lazy::new(|| DashMap::new());
```
- In-memory caching of parsed symbols
- Thread-safe concurrent access

### Extension Optimization

**Bundle Size Reduction**:
```json
{
  "esbuild": {
    "minify": true,
    "treeShaking": true,
    "external": ["vscode"]
  }
}
```
- Result: ~200KB bundled extension

**Lazy Loading**:
```typescript
// Only load dashboard when requested
let dashboardModule: typeof import('./dashboard');

export async function showDashboard() {
    if (!dashboardModule) {
        dashboardModule = await import('./dashboard');
    }
    return dashboardModule.show();
}
```
- Faster activation time
- Lower memory usage

## Security Considerations

### Input Validation

**LSP Messages**:
```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CustomRequest {
    topic: String,
}
```
- Strict deserialization
- Reject unknown fields

**File Paths**:
```rust
fn validate_path(path: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize()?;

    // Ensure path is within workspace
    if !canonical.starts_with(&workspace_root) {
        bail!("Path outside workspace");
    }

    Ok(canonical)
}
```
- Prevent directory traversal
- Canonicalize all paths

### Sandboxing

**Process Isolation**:
- Language server runs in separate process
- Communication only via stdio
- No shared memory

**Resource Limits**:
```typescript
const serverOptions: ServerOptions = {
    command: serverPath,
    options: {
        env: process.env,
        maxBuffer: 10 * 1024 * 1024, // 10MB
    }
};
```

## Compatibility

### Platform Support

**Operating Systems**:
- Linux (x86_64, aarch64)
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

**VSCode Versions**:
- Minimum: 1.85.0
- Tested: 1.85.0 - 1.86.0
- Target: Latest stable

**HORUS Versions**:
- Minimum: 0.1.0
- Recommended: Latest patch version
- Breaking changes: Communicated via changelog

### Node.js Versions

**Supported**:
- 18.x LTS (minimum)
- 20.x LTS (recommended)
- 21.x Current (supported)

**Not Supported**:
- < 18.0.0 (missing features)

### Rust Versions

**Minimum Supported Rust Version (MSRV)**: 1.75.0
- Reason: Uses latest stable features
- Policy: Update MSRV when necessary, document in CHANGELOG

## Dependency Management

### Update Policy

**Semver Compliance**:
- MAJOR: Update when breaking changes needed
- MINOR: Update for new features
- PATCH: Update for bug fixes

**Security Updates**:
- Critical: Update immediately
- High: Update within 1 week
- Medium/Low: Update in next release

**Dependency Audit**:
```bash
# NPM audit
npm audit

# Cargo audit
cargo install cargo-audit
cargo audit
```

### Lock Files

**package-lock.json**: Committed
- Reason: Reproducible builds
- Update: On dependency changes

**Cargo.lock**: Committed
- Reason: Reproducible builds
- Update: On dependency changes

## Continuous Integration

### GitHub Actions Workflow

**.github/workflows/ci.yml**:
```yaml
name: CI

on: [push, pull_request]

jobs:
  test-extension:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 18
      - run: npm ci
      - run: npm test

  test-server:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --manifest-path=server/Cargo.toml

  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - uses: actions-rs/toolchain@v1
      - run: npm ci
      - run: cargo build --release --manifest-path=server/Cargo.toml
      - run: npm run package
      - uses: actions/upload-artifact@v4
        with:
          name: vsix
          path: '*.vsix'
```

### Pre-commit Hooks

**husky + lint-staged**:
```json
{
  "husky": {
    "hooks": {
      "pre-commit": "lint-staged"
    }
  },
  "lint-staged": {
    "*.ts": ["eslint --fix", "prettier --write"],
    "*.rs": ["cargo fmt --", "cargo clippy --"]
  }
}
```

## Documentation

### Code Documentation

**TypeScript**:
```typescript
/**
 * Provides HORUS task integration for VSCode.
 *
 * @remarks
 * Detects `horus.yaml` and generates tasks for run, check, build commands.
 *
 * @example
 * ```typescript
 * const provider = new HorusTaskProvider();
 * context.subscriptions.push(
 *     vscode.tasks.registerTaskProvider('horus', provider)
 * );
 * ```
 */
export class HorusTaskProvider implements vscode.TaskProvider {
    // ...
}
```
- TSDoc format
- Examples included

**Rust**:
```rust
/// Resolves the HORUS source directory.
///
/// # Search Order
///
/// 1. `HORUS_SOURCE` environment variable
/// 2. `~/.horus/source_path` configuration file
/// 3. Common installation paths (`/usr/local/lib/horus`, etc.)
///
/// # Errors
///
/// Returns `Err` if HORUS source cannot be found or is invalid.
///
/// # Example
///
/// ```
/// let source = resolve_horus_source()?;
/// println!("HORUS source: {}", source.display());
/// ```
pub fn resolve_horus_source() -> Result<PathBuf> {
    // ...
}
```
- Rustdoc format
- Examples tested by `cargo test --doc`

### API Documentation

**Generated Documentation**:
```bash
# TypeScript API docs
npm run docs # Uses TypeDoc

# Rust API docs
cargo doc --open --manifest-path=server/Cargo.toml
```

## Monitoring and Telemetry

### Error Reporting

**Sentry Integration** (Optional):
```typescript
import * as Sentry from '@sentry/node';

Sentry.init({
    dsn: process.env.SENTRY_DSN,
    environment: 'production',
    beforeSend(event) {
        // Remove sensitive data
        return event;
    }
});
```

### Usage Analytics

**Privacy-First**:
- No personal data collected
- Opt-in only
- Anonymous usage statistics

**Metrics Collected**:
- Extension activation count
- Command usage frequency
- Average completion latency
- Error rates

## Licensing

**Extension License**: Apache-2.0
**Language Server License**: Apache-2.0
**Dependencies**: All permissive licenses (Apache-2.0, BSD)

**License Compliance Check**:
```bash
# NPM
npm install -g license-checker
license-checker --summary

# Cargo
cargo install cargo-license
cargo license --manifest-path=server/Cargo.toml
```
