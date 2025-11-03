# HORUS VSCode Extension - Implementation Guide

## Getting Started

### Prerequisites

Ensure the following are installed:

```bash
# Check versions
node --version    # Should be >= 18.0.0
npm --version     # Should be >= 9.0.0
rustc --version   # Should be >= 1.75.0
cargo --version   # Should be >= 1.75.0
code --version    # Should be >= 1.85.0
```

Install if missing:

```bash
# Node.js and npm
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# VSCE
npm install -g @vscode/vsce
```

### Initial Setup

Clone and prepare workspace:

```bash
cd /home/lord-patpak/horus/HORUS/horus-vscode

# Initialize npm project
npm init -y

# Install dependencies
npm install --save-dev \
    @types/vscode@^1.85.0 \
    @types/node@^18.0.0 \
    @typescript-eslint/eslint-plugin@^6.0.0 \
    @typescript-eslint/parser@^6.0.0 \
    typescript@^5.0.0 \
    esbuild@^0.19.0 \
    @vscode/test-electron@^2.3.0 \
    mocha@^10.0.0 \
    chai@^4.3.0

npm install --save \
    vscode-languageclient@^9.0.0 \
    vscode-debugadapter@^1.51.0

# Initialize Rust workspace for language server
cargo init server --name horus-language-server
```

## Project Structure Setup

Create the following directory structure:

```bash
mkdir -p {src,src/test/suite,server/src,server/tests,resources,syntaxes,.vscode}
```

Final structure:

```
horus-vscode/
├── package.json                 # Extension manifest
├── tsconfig.json               # TypeScript configuration
├── .eslintrc.json             # ESLint configuration
├── .gitignore
├── README.md
├── CHANGELOG.md
├── src/
│   ├── extension.ts           # Extension entry point
│   ├── languageClient.ts      # LSP client
│   ├── taskProvider.ts        # Task integration
│   ├── debugAdapter.ts        # Debug support
│   ├── dashboard.ts           # Dashboard webview
│   ├── topicInspector.ts      # Topic hover/inspection
│   ├── nodeGraph.ts           # Node graph visualization
│   └── test/
│       ├── runTest.ts         # Test runner
│       └── suite/
│           ├── extension.test.ts
│           └── integration.test.ts
├── server/                    # Language server (Rust)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs           # LSP server entry
│   │   ├── server.rs         # Server implementation
│   │   ├── project.rs        # Project detection/analysis
│   │   ├── completion.rs     # Completion provider
│   │   ├── hover.rs          # Hover provider
│   │   ├── symbols.rs        # Symbol resolution
│   │   ├── topic_inspector.rs
│   │   └── diagnostics.rs
│   └── tests/
│       ├── project_test.rs
│       └── completion_test.rs
├── resources/               # Icons, images
├── syntaxes/               # Syntax highlighting
│   └── horus.tmLanguage.json
└── .vscode/
    ├── launch.json         # Debug configuration
    └── tasks.json          # Build tasks
```

## Implementation Steps

### Step 1: Extension Manifest

Create `package.json`:

```json
{
  "name": "horus-vscode",
  "displayName": "HORUS",
  "description": "HORUS robotics framework support for VSCode",
  "version": "0.1.0",
  "publisher": "horus",
  "repository": {
    "type": "git",
    "url": "https://github.com/horus-robotics/horus"
  },
  "engines": {
    "vscode": "^1.85.0"
  },
  "categories": [
    "Programming Languages",
    "Debuggers",
    "Other"
  ],
  "keywords": [
    "horus",
    "robotics",
    "rust",
    "ros",
    "embedded"
  ],
  "activationEvents": [
    "workspaceContains:horus.yaml",
    "onLanguage:rust"
  ],
  "main": "./out/extension.js",
  "contributes": {
    "languages": [
      {
        "id": "rust",
        "extensions": [".rs"],
        "configuration": "./language-configuration.json"
      }
    ],
    "commands": [
      {
        "command": "horus.run",
        "title": "HORUS: Run Current File",
        "category": "HORUS"
      },
      {
        "command": "horus.check",
        "title": "HORUS: Check Project",
        "category": "HORUS"
      },
      {
        "command": "horus.build",
        "title": "HORUS: Build Project",
        "category": "HORUS"
      },
      {
        "command": "horus.dashboard",
        "title": "HORUS: Open Dashboard",
        "category": "HORUS"
      },
      {
        "command": "horus.sim",
        "title": "HORUS: Start Simulator",
        "category": "HORUS"
      },
      {
        "command": "horus.inspectTopic",
        "title": "HORUS: Inspect Topic",
        "category": "HORUS"
      },
      {
        "command": "horus.showGraph",
        "title": "HORUS: Show Node Graph",
        "category": "HORUS"
      }
    ],
    "taskDefinitions": [
      {
        "type": "horus",
        "required": ["command"],
        "properties": {
          "command": {
            "type": "string",
            "description": "The HORUS command to execute"
          },
          "args": {
            "type": "array",
            "description": "Additional arguments"
          }
        }
      }
    ],
    "debuggers": [
      {
        "type": "horus",
        "label": "HORUS Debug",
        "configurationAttributes": {
          "launch": {
            "required": ["program"],
            "properties": {
              "program": {
                "type": "string",
                "description": "Path to Rust file to debug"
              },
              "args": {
                "type": "array",
                "description": "Program arguments"
              },
              "cwd": {
                "type": "string",
                "description": "Working directory"
              }
            }
          }
        },
        "initialConfigurations": [
          {
            "type": "horus",
            "request": "launch",
            "name": "HORUS: Debug",
            "program": "${file}",
            "args": [],
            "cwd": "${workspaceFolder}"
          }
        ]
      }
    ],
    "configuration": {
      "title": "HORUS",
      "properties": {
        "horus.horusSource": {
          "type": "string",
          "description": "Path to HORUS source directory (overrides HORUS_SOURCE env var)"
        },
        "horus.enableDashboard": {
          "type": "boolean",
          "default": true,
          "description": "Enable integrated dashboard"
        },
        "horus.trace.server": {
          "type": "string",
          "enum": ["off", "messages", "verbose"],
          "default": "off",
          "description": "Language server trace level"
        }
      }
    }
  },
  "scripts": {
    "vscode:prepublish": "npm run compile",
    "compile": "npm run compile:extension && npm run compile:server",
    "compile:extension": "esbuild ./src/extension.ts --bundle --outfile=out/extension.js --external:vscode --format=cjs --platform=node --minify",
    "compile:server": "cd server && cargo build --release && cd .. && mkdir -p bin && cp server/target/release/horus-language-server bin/",
    "watch": "tsc -watch -p ./",
    "pretest": "npm run compile && npm run lint",
    "lint": "eslint src --ext ts",
    "test": "node ./out/test/runTest.js",
    "package": "vsce package"
  }
}
```

### Step 2: TypeScript Configuration

Create `tsconfig.json`:

```json
{
  "compilerOptions": {
    "module": "commonjs",
    "target": "ES2022",
    "outDir": "out",
    "lib": ["ES2022"],
    "sourceMap": true,
    "rootDir": "src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true
  },
  "exclude": ["node_modules", ".vscode-test"]
}
```

Create `.eslintrc.json`:

```json
{
  "root": true,
  "parser": "@typescript-eslint/parser",
  "parserOptions": {
    "ecmaVersion": 2022,
    "sourceType": "module"
  },
  "plugins": ["@typescript-eslint"],
  "extends": [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended"
  ],
  "rules": {
    "@typescript-eslint/no-unused-vars": ["warn", { "argsIgnorePattern": "^_" }],
    "@typescript-eslint/no-explicit-any": "warn"
  }
}
```

### Step 3: Extension Entry Point

Create `src/extension.ts`:

```typescript
import * as vscode from 'vscode';
import { LanguageClientManager } from './languageClient';
import { HorusTaskProvider } from './taskProvider';
import { DashboardPanel } from './dashboard';

let languageClient: LanguageClientManager;

export async function activate(context: vscode.ExtensionContext) {
    console.log('HORUS extension activating...');

    // Check if this is a HORUS project
    if (!await isHorusProject()) {
        console.log('Not a HORUS project, extension inactive');
        return;
    }

    // Initialize language client
    languageClient = new LanguageClientManager(context);
    await languageClient.start();

    // Register task provider
    const taskProvider = new HorusTaskProvider();
    context.subscriptions.push(
        vscode.tasks.registerTaskProvider('horus', taskProvider)
    );

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('horus.run', async () => {
            await executeHorusCommand('run');
        }),
        vscode.commands.registerCommand('horus.check', async () => {
            await executeHorusCommand('check');
        }),
        vscode.commands.registerCommand('horus.build', async () => {
            await executeHorusCommand('build');
        }),
        vscode.commands.registerCommand('horus.dashboard', async () => {
            await DashboardPanel.show(context);
        })
    );

    // Status bar
    const statusBar = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Left,
        100
    );
    statusBar.text = '$(rocket) HORUS: Ready';
    statusBar.show();
    context.subscriptions.push(statusBar);

    console.log('HORUS extension activated');
}

export function deactivate(): Thenable<void> | undefined {
    if (!languageClient) {
        return undefined;
    }
    return languageClient.stop();
}

async function isHorusProject(): Promise<boolean> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return false;
    }

    const horusYaml = vscode.Uri.joinPath(workspaceFolder.uri, 'horus.yaml');
    try {
        await vscode.workspace.fs.stat(horusYaml);
        return true;
    } catch {
        return false;
    }
}

async function executeHorusCommand(command: string): Promise<void> {
    const terminal = vscode.window.createTerminal('HORUS');
    terminal.show();
    terminal.sendText(`horus ${command}`);
}
```

### Step 4: Language Client Setup

Create `src/languageClient.ts`:

```typescript
import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

export class LanguageClientManager {
    private client?: LanguageClient;

    constructor(private context: vscode.ExtensionContext) {}

    async start(): Promise<void> {
        const serverPath = this.getServerPath();

        const serverOptions: ServerOptions = {
            run: {
                command: serverPath,
                transport: TransportKind.stdio
            },
            debug: {
                command: serverPath,
                transport: TransportKind.stdio,
                options: {
                    env: {
                        ...process.env,
                        RUST_LOG: 'debug'
                    }
                }
            }
        };

        const clientOptions: LanguageClientOptions = {
            documentSelector: [
                { scheme: 'file', language: 'rust' }
            ],
            synchronize: {
                fileEvents: vscode.workspace.createFileSystemWatcher('**/*.rs')
            },
            initializationOptions: {
                horusSource: this.getHorusSource()
            }
        };

        this.client = new LanguageClient(
            'horusLanguageServer',
            'HORUS Language Server',
            serverOptions,
            clientOptions
        );

        await this.client.start();
    }

    async stop(): Promise<void> {
        if (this.client) {
            await this.client.stop();
        }
    }

    private getServerPath(): string {
        const serverBinary = process.platform === 'win32'
            ? 'horus-language-server.exe'
            : 'horus-language-server';

        return path.join(this.context.extensionPath, 'bin', serverBinary);
    }

    private getHorusSource(): string | undefined {
        // Check configuration
        const config = vscode.workspace.getConfiguration('horus');
        const configuredSource = config.get<string>('horusSource');
        if (configuredSource) {
            return configuredSource;
        }

        // Check environment variable
        return process.env.HORUS_SOURCE;
    }
}
```

### Step 5: Language Server Implementation

Create `server/Cargo.toml`:

```toml
[package]
name = "horus-language-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "horus-language-server"
path = "src/main.rs"

[dependencies]
# LSP
tower-lsp = "0.20"
lsp-types = "0.94"

# Async runtime
tokio = { version = "1.0", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Utilities
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dirs = "5.0"
walkdir = "2.4"

# HORUS dependencies
horus_core = { path = "../../horus_core" }
horus_manager = { path = "../../horus_manager" }

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.8"
```

Create `server/src/main.rs`:

```rust
use tower_lsp::{LspService, Server};
use tracing_subscriber::{fmt, EnvFilter};

mod server;
mod project;
mod completion;
mod symbols;

use server::HorusLanguageServer;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("HORUS Language Server starting...");

    // Create LSP service
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        HorusLanguageServer::new(client)
    });

    // Start server
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

Create `server/src/server.rs`:

```rust
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::project::HorusProject;

pub struct HorusLanguageServer {
    client: Client,
    project: Option<HorusProject>,
}

impl HorusLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            project: None,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for HorusLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        tracing::info!("Initialize request received");

        // Detect HORUS project
        if let Some(root_uri) = params.root_uri {
            let root_path = root_uri.to_file_path().ok();
            if let Some(path) = root_path {
                match HorusProject::detect(&path).await {
                    Ok(Some(project)) => {
                        tracing::info!("Detected HORUS project: {}", project.config.name);
                        // Store project (requires interior mutability)
                    }
                    Ok(None) => {
                        tracing::warn!("Not a HORUS project");
                    }
                    Err(e) => {
                        tracing::error!("Failed to detect project: {}", e);
                    }
                }
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![":".to_string(), ".".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "HORUS Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Server initialized");
        self.client
            .log_message(MessageType::INFO, "HORUS Language Server ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        tracing::debug!("Completion request: {:?}", params);

        // TODO: Implement completion logic

        Ok(Some(CompletionResponse::Array(vec![])))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        tracing::debug!("Hover request: {:?}", params);

        // TODO: Implement hover logic

        Ok(None)
    }
}
```

### Step 6: Project Detection

Create `server/src/project.rs`:

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct HorusYaml {
    pub name: String,
    pub version: String,
    pub language: String,
    pub dependencies: Vec<String>,
}

pub struct HorusProject {
    pub root: PathBuf,
    pub config: HorusYaml,
    pub horus_source: PathBuf,
}

impl HorusProject {
    pub async fn detect(workspace_root: &Path) -> Result<Option<Self>> {
        // Look for horus.yaml
        let config_path = workspace_root.join("horus.yaml");
        if !config_path.exists() {
            return Ok(None);
        }

        // Parse horus.yaml
        let config_content = fs::read_to_string(&config_path)
            .context("Failed to read horus.yaml")?;
        let config: HorusYaml = serde_yaml::from_str(&config_content)
            .context("Failed to parse horus.yaml")?;

        // Resolve HORUS source
        let horus_source = resolve_horus_source()?;

        Ok(Some(Self {
            root: workspace_root.to_path_buf(),
            config,
            horus_source,
        }))
    }
}

fn resolve_horus_source() -> Result<PathBuf> {
    // Check environment variable
    if let Ok(source) = std::env::var("HORUS_SOURCE") {
        let path = PathBuf::from(source);
        if validate_horus_source(&path) {
            return Ok(path);
        }
    }

    // Check config file
    if let Some(home) = dirs::home_dir() {
        let config_file = home.join(".horus/source_path");
        if config_file.exists() {
            if let Ok(source) = fs::read_to_string(config_file) {
                let path = PathBuf::from(source.trim());
                if validate_horus_source(&path) {
                    return Ok(path);
                }
            }
        }
    }

    anyhow::bail!("HORUS source not found. Set HORUS_SOURCE environment variable")
}

fn validate_horus_source(path: &Path) -> bool {
    path.join("horus_core").exists() &&
    path.join("horus_library").exists() &&
    path.join("horus").exists()
}
```

### Step 7: Build and Test

Build extension:

```bash
# Install dependencies
npm install

# Build extension
npm run compile

# Package extension
npm run package
```

Install and test:

```bash
# Install locally
code --install-extension horus-vscode-0.1.0.vsix

# Test in VSCode
# 1. Open HORUS project
# 2. Check status bar shows "HORUS: Ready"
# 3. Try completion in .rs file
# 4. Run HORUS: Run command
```

## Testing Implementation

### Unit Tests

Create `server/tests/project_test.rs`:

```rust
use tempfile::TempDir;
use std::fs;

#[tokio::test]
async fn test_detect_horus_project() {
    let temp_dir = TempDir::new().unwrap();

    // Create horus.yaml
    let config = r#"
name: test_project
version: 0.1.0
language: rust
dependencies:
  - horus@0.1.0
"#;
    fs::write(temp_dir.path().join("horus.yaml"), config).unwrap();

    // Detect project
    let project = HorusProject::detect(temp_dir.path()).await.unwrap();
    assert!(project.is_some());

    let project = project.unwrap();
    assert_eq!(project.config.name, "test_project");
}
```

### Integration Tests

Create `src/test/suite/extension.test.ts`:

```typescript
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Extension Test Suite', () => {
    vscode.window.showInformationMessage('Running tests...');

    test('Extension should activate', async () => {
        const ext = vscode.extensions.getExtension('horus.horus-vscode');
        assert.ok(ext);

        await ext.activate();
        assert.strictEqual(ext.isActive, true);
    });

    test('Should register commands', async () => {
        const commands = await vscode.commands.getCommands(true);
        assert.ok(commands.includes('horus.run'));
        assert.ok(commands.includes('horus.check'));
        assert.ok(commands.includes('horus.dashboard'));
    });
});
```

Run tests:

```bash
npm test
```

## Validation Checklist

Before release, validate:

- [ ] Extension activates on HORUS projects
- [ ] Language server starts successfully
- [ ] Completion works for basic HORUS types
- [ ] Tasks appear in task list
- [ ] Commands work from command palette
- [ ] No errors in Output panel
- [ ] Extension bundles correctly
- [ ] All tests pass

## Next Steps

After basic implementation:

1. Implement completion logic
2. Add hover provider
3. Implement dashboard integration
4. Add debug adapter
5. Create topic inspector
6. Build node graph visualization
7. Write comprehensive tests
8. Create user documentation
9. Publish to marketplace
