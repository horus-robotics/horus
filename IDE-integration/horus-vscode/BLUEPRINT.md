# HORUS VSCode Extension - Technical Blueprint

## Overview

The HORUS VSCode extension provides native IDE support for HORUS robotics framework development without requiring Cargo.toml or traditional Rust tooling configuration. It integrates directly with the HORUS CLI and runtime to provide autocomplete, type checking, debugging, and live dashboard features.

## Core Objectives

1. **Zero Configuration**: Detect HORUS projects automatically via `horus.yaml`
2. **Native Integration**: Use `horus` commands instead of `cargo` commands
3. **Framework-Aware**: Understand HORUS-specific patterns (nodes, topics, macros)
4. **Live Feedback**: Real-time dashboard integration while code runs
5. **Developer Experience**: Match or exceed Cargo-based Rust development

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    VSCode Extension                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ LSP Client   │  │ Task Provider│  │ Debug Adapter│      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                  │                  │              │
└─────────┼──────────────────┼──────────────────┼──────────────┘
          │                  │                  │
          ▼                  ▼                  ▼
┌─────────────────┐  ┌──────────────┐  ┌──────────────┐
│ HORUS Language  │  │ HORUS CLI    │  │ HORUS Runtime│
│ Server (Rust)   │  │ (horus run)  │  │ (Scheduler)  │
└─────────────────┘  └──────────────┘  └──────────────┘
          │
          ▼
┌─────────────────┐
│ horus.yaml      │
│ Project Files   │
│ HORUS Source    │
└─────────────────┘
```

### Three-Layer Design

**Layer 1: VSCode Extension (TypeScript)**
- User interface and VSCode API integration
- Extension activation and lifecycle
- Webview panels for dashboard/graphs
- Command palette commands
- Status bar integration

**Layer 2: Language Server (Rust)**
- Language Server Protocol implementation
- HORUS project analysis (parse horus.yaml)
- Dependency resolution (HORUS_SOURCE detection)
- Symbol resolution and type inference
- Delegated rust-analyzer integration

**Layer 3: HORUS Integration**
- CLI command execution and output parsing
- Runtime communication (dashboard data)
- Shared memory topic inspection
- Process management and monitoring

## Technology Stack

### VSCode Extension

**Language**: TypeScript 5.x
- Native VSCode API support
- Strong typing for API contracts
- Excellent tooling and ecosystem

**Framework**: VSCode Extension API
- `vscode` - Core VSCode API
- `vscode-languageclient` - LSP client implementation
- `vscode-debugadapter` - Debug protocol implementation

**Build System**: esbuild
- Fast compilation
- Tree shaking for smaller bundle
- TypeScript transpilation

**Testing**:
- `@vscode/test-electron` - Integration tests
- `mocha` - Test runner
- `chai` - Assertions

**Dependencies**:
```json
{
  "vscode": "^1.85.0",
  "vscode-languageclient": "^9.0.0",
  "vscode-debugadapter": "^1.51.0",
  "esbuild": "^0.19.0"
}
```

### Language Server

**Language**: Rust (stable channel)
- Performance for project analysis
- Shared codebase with HORUS core
- Native async support

**Framework**: tower-lsp
- Modern LSP implementation
- Async/await support
- Type-safe LSP message handling

**Dependencies**:
```toml
[dependencies]
tower-lsp = "0.20"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# HORUS dependencies (workspace)
horus_core = { path = "../horus_core" }
horus_manager = { path = "../horus_manager" }
```

**Architecture Pattern**: Actor model via tokio channels
- Separate analysis thread
- Request/response queuing
- Background indexing

### Communication Protocols

**LSP (Language Server Protocol)**
- Standard: LSP 3.17
- Transport: JSON-RPC over stdio
- Messages: textDocument/*, workspace/*, custom HORUS methods

**DAP (Debug Adapter Protocol)**
- Standard: DAP 1.51
- Transport: JSON-RPC over stdio
- Integration: GDB/LLDB via horus CLI wrapper

**HORUS IPC**
- Protocol: Custom JSON over Unix socket
- Purpose: Live dashboard data from running HORUS processes
- Location: `/tmp/horus_ipc_<pid>.sock`

## Implementation Details

### Phase 1: Language Server Protocol

#### Project Detection

**Mechanism**: Workspace root scanning
```rust
// server/src/project.rs
pub struct HorusProject {
    root: PathBuf,
    config: HorusYaml,
    horus_source: PathBuf,
    dependencies: Vec<Dependency>,
}

impl HorusProject {
    pub fn detect(workspace_root: &Path) -> Result<Option<Self>> {
        // 1. Look for horus.yaml
        let config_path = workspace_root.join("horus.yaml");
        if !config_path.exists() {
            return Ok(None);
        }

        // 2. Parse horus.yaml
        let config = HorusYaml::parse(&config_path)?;

        // 3. Resolve HORUS_SOURCE
        let horus_source = resolve_horus_source()?;

        // 4. Build dependency graph
        let dependencies = resolve_dependencies(&config, &horus_source)?;

        Ok(Some(HorusProject {
            root: workspace_root.to_path_buf(),
            config,
            horus_source,
            dependencies,
        }))
    }
}
```

**horus.yaml Parsing**:
```rust
#[derive(Debug, Deserialize)]
pub struct HorusYaml {
    pub name: String,
    pub version: String,
    pub language: String,
    pub dependencies: Vec<String>,
    pub author: Option<String>,
    pub description: Option<String>,
}
```

#### HORUS Source Resolution

**Priority Order**:
1. `HORUS_SOURCE` environment variable
2. `~/.horus/source_path` file
3. Common installation paths:
   - `/usr/local/lib/horus`
   - `/opt/horus`
   - `~/.local/share/horus`
4. Fallback: Prompt user

```rust
pub fn resolve_horus_source() -> Result<PathBuf> {
    // Check environment variable
    if let Ok(source) = env::var("HORUS_SOURCE") {
        let path = PathBuf::from(source);
        if validate_horus_source(&path)? {
            return Ok(path);
        }
    }

    // Check config file
    if let Some(home) = dirs::home_dir() {
        let config_file = home.join(".horus/source_path");
        if config_file.exists() {
            let source = fs::read_to_string(config_file)?;
            let path = PathBuf::from(source.trim());
            if validate_horus_source(&path)? {
                return Ok(path);
            }
        }
    }

    // Check common paths
    for common_path in COMMON_INSTALL_PATHS {
        let path = PathBuf::from(common_path);
        if validate_horus_source(&path).unwrap_or(false) {
            return Ok(path);
        }
    }

    bail!("HORUS source not found. Set HORUS_SOURCE or run 'horus install'")
}

fn validate_horus_source(path: &Path) -> Result<bool> {
    // Check for key directories
    Ok(path.join("horus_core").exists() &&
       path.join("horus_library").exists() &&
       path.join("horus").exists())
}
```

#### Symbol Resolution

**Strategy**: Hybrid approach
1. Parse local project files for symbols
2. Index HORUS framework symbols once
3. Cache resolved symbols
4. Delegate complex analysis to rust-analyzer

```rust
pub struct SymbolIndex {
    // Project symbols
    project_symbols: HashMap<String, Symbol>,

    // HORUS framework symbols (cached)
    framework_symbols: Arc<HashMap<String, Symbol>>,

    // External crate symbols (from dependencies)
    dependency_symbols: HashMap<String, Symbol>,
}

impl SymbolIndex {
    pub fn resolve_completion(&self, prefix: &str, context: &Context) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // 1. Check if importing from horus
        if context.is_import && prefix.starts_with("horus") {
            items.extend(self.complete_horus_imports(prefix));
        }

        // 2. Check for Hub/Node/Scheduler methods
        if let Some(type_name) = context.receiver_type {
            items.extend(self.complete_methods(type_name));
        }

        // 3. Check for topic strings
        if context.is_string_literal && context.is_topic_context {
            items.extend(self.complete_topics());
        }

        items
    }

    fn complete_horus_imports(&self, prefix: &str) -> Vec<CompletionItem> {
        self.framework_symbols
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .map(|(name, symbol)| CompletionItem {
                label: name.clone(),
                kind: symbol.kind.into(),
                detail: Some(symbol.signature.clone()),
                documentation: symbol.docs.clone(),
                ..Default::default()
            })
            .collect()
    }
}
```

#### Rust-Analyzer Delegation

**Approach**: Proxy complex queries to rust-analyzer
```rust
pub struct DelegatingAnalyzer {
    rust_analyzer: LanguageClient,
    horus_analyzer: HorusAnalyzer,
}

impl DelegatingAnalyzer {
    pub async fn handle_completion(&self, params: CompletionParams) -> Result<CompletionList> {
        // 1. Try HORUS-specific completion
        let horus_items = self.horus_analyzer.complete(&params).await?;

        // 2. Delegate to rust-analyzer for Rust syntax
        let rust_items = self.rust_analyzer.completion(params.clone()).await?;

        // 3. Merge results, prioritizing HORUS items
        Ok(CompletionList {
            is_incomplete: false,
            items: merge_completions(horus_items, rust_items),
        })
    }
}
```

**rust-analyzer Configuration**:
Generate temporary `Cargo.toml` in memory for rust-analyzer:
```rust
pub fn generate_virtual_cargo_toml(project: &HorusProject) -> String {
    format!(
        r#"[package]
name = "{}"
version = "{}"
edition = "2021"

[dependencies]
horus = {{ path = "{}/horus" }}
horus_core = {{ path = "{}/horus_core" }}
horus_library = {{ path = "{}/horus_library" }}
{}

[[bin]]
name = "{}"
path = "main.rs"
"#,
        project.config.name,
        project.config.version,
        project.horus_source.display(),
        project.horus_source.display(),
        project.horus_source.display(),
        format_dependencies(&project.dependencies),
        project.config.name
    )
}
```

Provide to rust-analyzer via custom configuration:
```rust
let ra_config = serde_json::json!({
    "cargo": {
        "buildScripts": {
            "overrideCommand": null
        }
    },
    "checkOnSave": {
        "command": "check",
        "extraEnv": {
            "HORUS_SOURCE": project.horus_source.to_str()
        }
    }
});
```

### Phase 2: Task Integration

#### Task Provider

**VSCode Tasks API**:
```typescript
// extension/src/taskProvider.ts
export class HorusTaskProvider implements vscode.TaskProvider {
    async provideTasks(): Promise<vscode.Task[]> {
        const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
        if (!workspaceFolder) return [];

        // Check for horus.yaml
        const horusYaml = vscode.Uri.joinPath(workspaceFolder.uri, 'horus.yaml');
        try {
            await vscode.workspace.fs.stat(horusYaml);
        } catch {
            return []; // Not a HORUS project
        }

        // Detect available files
        const files = await this.detectRunnableFiles(workspaceFolder);

        return [
            this.createTask('run', 'Run current file', workspaceFolder),
            this.createTask('check', 'Check project for errors', workspaceFolder),
            this.createTask('build', 'Build project (no run)', workspaceFolder),
            this.createTask('dashboard', 'Open dashboard', workspaceFolder),
            this.createTask('sim', 'Start simulator', workspaceFolder),
            ...files.map(f => this.createFileTask(f, workspaceFolder))
        ];
    }

    private createTask(
        command: string,
        description: string,
        folder: vscode.WorkspaceFolder
    ): vscode.Task {
        const execution = new vscode.ShellExecution('horus', [command]);

        const task = new vscode.Task(
            { type: 'horus', command },
            folder,
            `HORUS: ${command}`,
            'horus',
            execution,
            '$horus' // Problem matcher
        );

        task.group = command === 'run' ? vscode.TaskGroup.Build : undefined;
        task.presentationOptions = {
            reveal: vscode.TaskRevealKind.Always,
            panel: vscode.TaskPanelKind.Dedicated,
            clear: true
        };

        return task;
    }

    resolveTask(task: vscode.Task): vscode.Task | undefined {
        // Called when task is executed
        return task;
    }
}
```

#### Problem Matcher

**Parse HORUS/Rust Errors**:
```json
{
  "problemMatcher": {
    "name": "horus",
    "owner": "horus",
    "fileLocation": ["relative", "${workspaceFolder}"],
    "pattern": [
      {
        "regexp": "^error\\[E\\d+\\]: (.*)$",
        "message": 1
      },
      {
        "regexp": "^\\s+-->\\s+(.+):(\\d+):(\\d+)$",
        "file": 1,
        "line": 2,
        "column": 3
      }
    ]
  }
}
```

### Phase 3: Debug Adapter

#### Debug Configuration

**Launch Configuration**:
```typescript
// extension/src/debugConfig.ts
export class HorusDebugConfigProvider implements vscode.DebugConfigurationProvider {
    resolveDebugConfiguration(
        folder: vscode.WorkspaceFolder | undefined,
        config: vscode.DebugConfiguration
    ): vscode.ProviderResult<vscode.DebugConfiguration> {
        // If no configuration, provide default
        if (!config.type && !config.request && !config.name) {
            const editor = vscode.window.activeTextEditor;
            if (!editor || !editor.document.fileName.endsWith('.rs')) {
                return undefined;
            }

            config = {
                type: 'horus',
                name: 'HORUS: Debug Current File',
                request: 'launch',
                program: '${file}',
                args: [],
                cwd: '${workspaceFolder}'
            };
        }

        // Ensure horus debugger is used
        config.horusDebug = true;

        return config;
    }
}
```

#### Debug Adapter Implementation

**Wrapper Around LLDB/GDB**:
```typescript
// extension/src/debugAdapter.ts
export class HorusDebugAdapter implements vscode.DebugAdapter {
    private childProcess?: ChildProcess;
    private lldbAdapter: LLDBAdapter;

    async launch(args: LaunchRequestArguments): Promise<void> {
        // 1. Build with debug symbols using horus
        const buildResult = await this.buildForDebug(args.program);
        if (!buildResult.success) {
            throw new Error(`Build failed: ${buildResult.error}`);
        }

        // 2. Launch via horus run with debug flags
        const executable = buildResult.executable;
        const debugPort = await this.findFreePort();

        this.childProcess = spawn('horus', ['run', '--debug', '--port', String(debugPort), args.program], {
            cwd: args.cwd,
            env: { ...process.env, RUST_BACKTRACE: '1' }
        });

        // 3. Attach LLDB to the spawned process
        await this.lldbAdapter.attach({
            pid: this.childProcess.pid,
            sourceMap: this.generateSourceMap(args.cwd)
        });
    }

    private async buildForDebug(program: string): Promise<BuildResult> {
        return new Promise((resolve) => {
            exec(`horus build --debug ${program}`, (error, stdout, stderr) => {
                if (error) {
                    resolve({ success: false, error: stderr });
                } else {
                    // Parse output to find executable path
                    const match = stdout.match(/Built: (.+)/);
                    resolve({
                        success: true,
                        executable: match ? match[1] : ''
                    });
                }
            });
        });
    }
}
```

### Phase 4: HORUS-Specific Features

#### Live Dashboard Integration

**Architecture**:
```
VSCode Extension
    └── WebView Panel
        └── HTTP Client
            └── HORUS Dashboard Server (localhost:3000)
                └── HORUS Runtime (shared memory)
```

**Implementation**:
```typescript
// extension/src/dashboard.ts
export class DashboardPanel {
    private panel?: vscode.WebviewPanel;
    private dashboardProcess?: ChildProcess;

    async show(): Promise<void> {
        // 1. Start dashboard if not running
        if (!this.dashboardProcess) {
            await this.startDashboard();
        }

        // 2. Create or reveal webview panel
        if (!this.panel) {
            this.panel = vscode.window.createWebviewPanel(
                'horusDashboard',
                'HORUS Dashboard',
                vscode.ViewColumn.Two,
                {
                    enableScripts: true,
                    retainContextWhenHidden: true
                }
            );

            // 3. Load dashboard in iframe
            this.panel.webview.html = this.getDashboardHtml();
        } else {
            this.panel.reveal();
        }
    }

    private getDashboardHtml(): string {
        // Embed dashboard via iframe
        return `
            <!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8">
                <style>
                    body, html, iframe {
                        margin: 0;
                        padding: 0;
                        width: 100%;
                        height: 100vh;
                        border: none;
                    }
                </style>
            </head>
            <body>
                <iframe src="http://localhost:3000"></iframe>
            </body>
            </html>
        `;
    }

    private async startDashboard(): Promise<void> {
        return new Promise((resolve, reject) => {
            this.dashboardProcess = spawn('horus', ['dashboard', '--no-browser'], {
                stdio: 'pipe'
            });

            // Wait for "Dashboard started" message
            this.dashboardProcess.stdout?.on('data', (data: Buffer) => {
                if (data.toString().includes('Dashboard started')) {
                    setTimeout(resolve, 500); // Give it time to bind
                }
            });

            this.dashboardProcess.on('error', reject);
        });
    }
}
```

#### Topic Inspector

**Hover Provider**:
```typescript
// extension/src/topicInspector.ts
export class TopicHoverProvider implements vscode.HoverProvider {
    constructor(private lspClient: LanguageClient) {}

    async provideHover(
        document: vscode.TextDocument,
        position: vscode.Position
    ): Promise<vscode.Hover | undefined> {
        const range = document.getWordRangeAtPosition(position, /"[^"]+"/);
        if (!range) return undefined;

        const topicName = document.getText(range).slice(1, -1);

        // Check if this is a topic string (in Hub::new, pub, sub context)
        const context = this.getContext(document, position);
        if (!context.isTopicContext) return undefined;

        // Query HORUS runtime for topic info
        const info = await this.queryTopicInfo(topicName);
        if (!info) return undefined;

        const markdown = new vscode.MarkdownString();
        markdown.appendMarkdown(`**Topic**: \`${topicName}\`\n\n`);
        markdown.appendMarkdown(`**Type**: ${info.messageType}\n\n`);
        markdown.appendMarkdown(`**Publishers**: ${info.publishers.length}\n\n`);
        markdown.appendMarkdown(`**Subscribers**: ${info.subscribers.length}\n\n`);

        if (info.currentValue) {
            markdown.appendCodeblock(JSON.stringify(info.currentValue, null, 2), 'json');
        }

        markdown.isTrusted = true;
        return new vscode.Hover(markdown, range);
    }

    private async queryTopicInfo(topic: string): Promise<TopicInfo | undefined> {
        // Send custom LSP request
        return this.lspClient.sendRequest('horus/topicInfo', { topic });
    }
}
```

**Language Server Side**:
```rust
// server/src/topic_inspector.rs
pub async fn handle_topic_info(topic: &str) -> Result<TopicInfo> {
    // 1. Check if HORUS runtime is active
    let runtime = HorusRuntime::connect().await?;

    // 2. Query topic metadata
    let metadata = runtime.get_topic_metadata(topic).await?;

    // 3. Read current value from shared memory
    let value = match read_topic_value(topic, &metadata.type_info) {
        Ok(v) => Some(v),
        Err(_) => None
    };

    Ok(TopicInfo {
        name: topic.to_string(),
        message_type: metadata.type_name,
        publishers: metadata.publishers,
        subscribers: metadata.subscribers,
        current_value: value,
        message_rate_hz: metadata.rate,
    })
}

fn read_topic_value(topic: &str, type_info: &TypeInfo) -> Result<serde_json::Value> {
    // Access shared memory topic
    let shm_path = format!("/dev/shm/horus/topics/{}", topic);
    let data = std::fs::read(&shm_path)?;

    // Deserialize based on type
    deserialize_message(&data, type_info)
}
```

#### Node Graph Visualization

**TreeView Provider**:
```typescript
// extension/src/nodeGraph.ts
export class NodeGraphProvider implements vscode.TreeDataProvider<GraphNode> {
    private _onDidChangeTreeData = new vscode.EventEmitter<GraphNode | undefined>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    constructor(private lspClient: LanguageClient) {}

    async getChildren(element?: GraphNode): Promise<GraphNode[]> {
        if (!element) {
            // Root level - show all nodes
            const nodes = await this.lspClient.sendRequest<NodeInfo[]>('horus/listNodes', {});
            return nodes.map(n => new GraphNode(n.name, 'node', n));
        } else if (element.type === 'node') {
            // Show publishers and subscribers
            const pubs = element.data.publishers.map(p =>
                new GraphNode(`PUB: ${p.topic}`, 'publisher', p)
            );
            const subs = element.data.subscribers.map(s =>
                new GraphNode(`SUB: ${s.topic}`, 'subscriber', s)
            );
            return [...pubs, ...subs];
        }
        return [];
    }

    getTreeItem(element: GraphNode): vscode.TreeItem {
        const item = new vscode.TreeItem(
            element.label,
            element.type === 'node' ? vscode.TreeItemCollapsibleState.Collapsed : vscode.TreeItemCollapsibleState.None
        );

        item.contextValue = element.type;
        item.tooltip = element.getTooltip();

        if (element.type === 'publisher' || element.type === 'subscriber') {
            item.command = {
                command: 'horus.inspectTopic',
                title: 'Inspect Topic',
                arguments: [element.data.topic]
            };
        }

        return item;
    }
}
```

**Graph Visualization WebView**:
```typescript
// extension/src/graphView.ts
export class GraphView {
    async show(): Promise<void> {
        const panel = vscode.window.createWebviewPanel(
            'horusGraph',
            'HORUS Node Graph',
            vscode.ViewColumn.Two,
            { enableScripts: true }
        );

        // Get graph data from LSP
        const graphData = await this.lspClient.sendRequest('horus/graphData', {});

        panel.webview.html = this.getGraphHtml(graphData);
    }

    private getGraphHtml(data: GraphData): string {
        return `
            <!DOCTYPE html>
            <html>
            <head>
                <script src="https://d3js.org/d3.v7.min.js"></script>
                <style>
                    body { margin: 0; }
                    svg { width: 100vw; height: 100vh; }
                    .node { fill: #2196F3; stroke: #fff; }
                    .topic { fill: #4CAF50; stroke: #fff; }
                    .link { stroke: #999; stroke-opacity: 0.6; }
                </style>
            </head>
            <body>
                <svg id="graph"></svg>
                <script>
                    const data = ${JSON.stringify(data)};
                    // D3.js force-directed graph rendering
                    // ... (implementation details)
                </script>
            </body>
            </html>
        `;
    }
}
```

## Validation and Testing Strategy

### Unit Testing

**Language Server Tests**:
```rust
// server/tests/project_detection.rs
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
    assert_eq!(project.dependencies.len(), 1);
}

#[tokio::test]
async fn test_resolve_horus_source() {
    // Set environment variable
    env::set_var("HORUS_SOURCE", "/test/path/to/horus");

    let source = resolve_horus_source().await;
    // Should fail if path doesn't exist, but we're testing resolution logic
    assert!(source.is_err() || source.unwrap().ends_with("horus"));
}
```

**Extension Tests**:
```typescript
// extension/src/test/suite/taskProvider.test.ts
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('HORUS Task Provider', () => {
    test('Detects HORUS project', async () => {
        const workspaceFolder = vscode.workspace.workspaceFolders![0];
        const provider = new HorusTaskProvider();

        const tasks = await provider.provideTasks();

        assert.ok(tasks.length > 0);
        assert.ok(tasks.some(t => t.name.includes('run')));
        assert.ok(tasks.some(t => t.name.includes('check')));
    });

    test('Creates correct task execution', async () => {
        const provider = new HorusTaskProvider();
        const tasks = await provider.provideTasks();

        const runTask = tasks.find(t => t.name.includes('run'));
        assert.ok(runTask);
        assert.strictEqual(runTask.execution.commandLine, 'horus run');
    });
});
```

### Integration Testing

**End-to-End Workflow Tests**:
```typescript
// extension/src/test/suite/integration.test.ts
suite('Integration Tests', () => {
    test('Complete development workflow', async function() {
        this.timeout(30000); // 30 second timeout

        // 1. Create new HORUS project via CLI
        await exec('horus new test_integration_project');

        // 2. Open in VSCode
        const uri = vscode.Uri.file('./test_integration_project');
        await vscode.commands.executeCommand('vscode.openFolder', uri);

        // Wait for extension activation
        await sleep(2000);

        // 3. Verify language server started
        const extension = vscode.extensions.getExtension('horus.horus-vscode');
        assert.ok(extension?.isActive);

        // 4. Open main.rs
        const document = await vscode.workspace.openTextDocument('./test_integration_project/main.rs');
        await vscode.window.showTextDocument(document);

        // 5. Trigger completion at import line
        const position = new vscode.Position(0, 10); // After "use horus::"
        const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
            'vscode.executeCompletionItemProvider',
            document.uri,
            position
        );

        assert.ok(completions);
        assert.ok(completions.items.some(item => item.label === 'prelude'));

        // 6. Run task
        const tasks = await vscode.tasks.fetchTasks({ type: 'horus' });
        const runTask = tasks.find(t => t.name.includes('run'));
        assert.ok(runTask);

        const execution = await vscode.tasks.executeTask(runTask);

        // Wait for task completion
        await new Promise(resolve => {
            vscode.tasks.onDidEndTask(e => {
                if (e.execution === execution) resolve();
            });
        });
    });
});
```

### LSP Protocol Compliance Testing

**Use LSP Test Suite**:
```rust
// server/tests/lsp_compliance.rs
use tower_lsp::lsp_types::*;
use tower_lsp::LspService;

#[tokio::test]
async fn test_initialize_request() {
    let (service, _) = LspService::new(|client| HorusLanguageServer::new(client));

    let params = InitializeParams {
        process_id: Some(1234),
        root_uri: Some(Url::from_file_path("/test/workspace").unwrap()),
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };

    let response = service.initialize(params).await.unwrap();

    assert!(response.capabilities.text_document_sync.is_some());
    assert!(response.capabilities.completion_provider.is_some());
    assert!(response.capabilities.hover_provider.is_some());
}

#[tokio::test]
async fn test_completion_request() {
    let (service, _) = setup_test_service().await;

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path("/test/workspace/main.rs").unwrap()
            },
            position: Position { line: 5, character: 15 }
        },
        context: None,
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    let response = service.completion(params).await.unwrap();

    match response {
        Some(CompletionResponse::Array(items)) => {
            assert!(!items.is_empty());
        }
        _ => panic!("Expected completion items")
    }
}
```

### Performance Testing

**Benchmarks**:
```rust
// server/benches/symbol_resolution.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_symbol_resolution(c: &mut Criterion) {
    let project = setup_test_project();
    let index = SymbolIndex::build(&project).unwrap();

    c.bench_function("resolve_horus_import", |b| {
        b.iter(|| {
            index.resolve_completion(
                black_box("horus::prelude::"),
                black_box(&CompletionContext::import())
            )
        });
    });

    c.bench_function("resolve_method", |b| {
        b.iter(|| {
            index.resolve_completion(
                black_box("Hub::"),
                black_box(&CompletionContext::method("Hub"))
            )
        });
    });
}

criterion_group!(benches, bench_symbol_resolution);
criterion_main!(benches);
```

**Performance Targets**:
- Project indexing: < 1 second for typical project
- Completion response: < 50ms
- Hover response: < 100ms
- Memory usage: < 100MB for language server

### Manual Testing Checklist

**Project Detection**:
- [ ] Extension activates when opening folder with `horus.yaml`
- [ ] Extension does not activate for non-HORUS projects
- [ ] Status bar shows "HORUS: Ready" when active
- [ ] Error shown if HORUS_SOURCE not found

**Code Intelligence**:
- [ ] Autocomplete works for `use horus::prelude::*`
- [ ] Autocomplete shows Hub, Scheduler, Node types
- [ ] Go to definition works for HORUS types
- [ ] Hover shows documentation for HORUS functions
- [ ] Error squiggles appear for type errors
- [ ] Macro expansion shows in diagnostics

**Tasks**:
- [ ] "HORUS: Run" task appears in task list
- [ ] Running task executes `horus run` successfully
- [ ] Task output appears in terminal
- [ ] Problem matcher highlights errors correctly
- [ ] Quick fix suggestions work for compile errors

**Debugging**:
- [ ] Debug configuration auto-generated
- [ ] Pressing F5 starts debug session
- [ ] Breakpoints hit correctly
- [ ] Variable inspection works
- [ ] Call stack shows HORUS scheduler frames

**Dashboard**:
- [ ] Dashboard panel opens via command
- [ ] Shows live data when `horus run` is active
- [ ] Updates in real-time
- [ ] Survives webview hide/show

**Topic Inspector**:
- [ ] Hover over topic string shows info
- [ ] Topic info includes current value
- [ ] Topic info shows publishers/subscribers
- [ ] Click opens detailed view

**Node Graph**:
- [ ] Tree view shows all nodes
- [ ] Nodes expand to show pubs/subs
- [ ] Graph visualization renders correctly
- [ ] Nodes are clickable (go to definition)

## Build and Distribution

### Development Build

**Prerequisites**:
```bash
# Install Node.js and npm
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install VSCE (VSCode Extension packaging tool)
npm install -g @vscode/vsce
```

**Build Steps**:
```bash
cd horus-vscode

# Install dependencies
npm install

# Build language server
cd server
cargo build --release
cd ..

# Build extension
npm run compile

# Package extension
vsce package
```

### Extension Packaging

**package.json Configuration**:
```json
{
  "name": "horus-vscode",
  "displayName": "HORUS",
  "description": "HORUS robotics framework support for VSCode",
  "version": "0.1.0",
  "publisher": "horus",
  "engines": {
    "vscode": "^1.85.0"
  },
  "categories": ["Programming Languages", "Debuggers"],
  "activationEvents": [
    "workspaceContains:horus.yaml",
    "onLanguage:rust"
  ],
  "main": "./out/extension.js",
  "contributes": {
    "languages": [{
      "id": "rust",
      "extensions": [".rs"]
    }],
    "commands": [
      {
        "command": "horus.run",
        "title": "HORUS: Run"
      },
      {
        "command": "horus.check",
        "title": "HORUS: Check"
      },
      {
        "command": "horus.dashboard",
        "title": "HORUS: Open Dashboard"
      }
    ],
    "taskDefinitions": [{
      "type": "horus"
    }],
    "debuggers": [{
      "type": "horus",
      "label": "HORUS Debug"
    }]
  }
}
```

### Distribution

**VSCode Marketplace**:
1. Create publisher account at https://marketplace.visualstudio.com/
2. Generate Personal Access Token
3. Publish:
```bash
vsce publish -p <token>
```

**GitHub Releases**:
```bash
# Tag version
git tag v0.1.0
git push origin v0.1.0

# Create GitHub release with .vsix file attached
gh release create v0.1.0 horus-vscode-0.1.0.vsix
```

**Installation**:
```bash
# From .vsix file
code --install-extension horus-vscode-0.1.0.vsix

# From marketplace (after publishing)
code --install-extension horus.horus-vscode
```

## Maintenance and Updates

### Version Management

**Semantic Versioning**:
- MAJOR: Breaking changes to LSP protocol or configuration
- MINOR: New features (e.g., new commands, visualizations)
- PATCH: Bug fixes, performance improvements

**Update Mechanism**:
- VSCode auto-updates extensions from marketplace
- Language server bundled with extension (no separate install)
- HORUS framework compatibility checked on activation

### Compatibility Matrix

| Extension Version | Min VSCode | Min HORUS | Max HORUS |
|-------------------|------------|-----------|-----------|
| 0.1.x             | 1.85.0     | 0.1.0     | 0.1.x     |
| 0.2.x             | 1.85.0     | 0.1.0     | 0.2.x     |
| 1.0.x             | 1.90.0     | 1.0.0     | 1.x.x     |

**Version Check**:
```rust
// server/src/version.rs
pub fn check_horus_compatibility(horus_version: &str) -> Result<()> {
    let min_version = Version::parse("0.1.0")?;
    let max_version = Version::parse("0.2.0")?;
    let current = Version::parse(horus_version)?;

    if current < min_version {
        bail!("HORUS {} is too old. Minimum version: {}", current, min_version);
    }

    if current >= max_version {
        warn!("HORUS {} may not be fully supported. Recommended: < {}", current, max_version);
    }

    Ok(())
}
```

### Logging and Diagnostics

**Extension Logs**:
```typescript
// extension/src/logging.ts
export class Logger {
    private outputChannel: vscode.OutputChannel;

    constructor() {
        this.outputChannel = vscode.window.createOutputChannel('HORUS');
    }

    info(message: string): void {
        const timestamp = new Date().toISOString();
        this.outputChannel.appendLine(`[${timestamp}] INFO: ${message}`);
    }

    error(message: string, error?: Error): void {
        const timestamp = new Date().toISOString();
        this.outputChannel.appendLine(`[${timestamp}] ERROR: ${message}`);
        if (error) {
            this.outputChannel.appendLine(error.stack || error.message);
        }
    }
}
```

**Language Server Logs**:
```rust
// server/src/main.rs
use tracing::{info, error};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    info!("HORUS Language Server starting...");

    // Start server
    let (service, socket) = LspService::new(|client| {
        HorusLanguageServer::new(client)
    });

    Server::new(stdin(), stdout(), socket).serve(service).await;
}
```

**Diagnostic Collection**:
```typescript
// Help users report issues
export async function collectDiagnostics(): Promise<string> {
    const diagnostics = {
        vscodeVersion: vscode.version,
        extensionVersion: getExtensionVersion(),
        horusVersion: await getHorusVersion(),
        horusSource: process.env.HORUS_SOURCE,
        platform: process.platform,
        logs: await getRecentLogs(100)
    };

    return JSON.stringify(diagnostics, null, 2);
}
```

## Future Enhancements

### Phase 5: Advanced Features

**Performance Profiler**:
- Integrated flamegraph visualization
- Node execution timing breakdown
- IPC latency tracking
- Memory usage analysis

**Parameter Tuning UI**:
- Visual sliders for numeric parameters
- Real-time parameter updates
- Parameter history and undo
- Export parameter sets

**Remote Development**:
- Deploy to robot directly from VSCode
- Remote debugging via SSH tunnel
- Live code sync to robot
- Remote file system access

**AI Integration**:
- Code completion via Copilot/Codeium
- Natural language to HORUS code
- Error explanation and fixes
- Architecture suggestions

### Cross-IDE Support

**JetBrains Plugin** (IntelliJ IDEA, CLion):
- Kotlin-based plugin
- Similar LSP integration
- Native IntelliJ UI components

**Vim/Neovim**:
- Lua plugin for Neovim
- VimScript for classic Vim
- LSP client via coc.nvim or native LSP

**Emacs**:
- Emacs Lisp package
- lsp-mode integration
- Company-mode completion

## Success Metrics

**Adoption Metrics**:
- VSCode Marketplace installs
- Active users (monthly)
- User ratings and reviews

**Performance Metrics**:
- Language server startup time
- Completion latency (p50, p95, p99)
- Memory footprint
- CPU usage

**Quality Metrics**:
- Bug report rate
- Issue resolution time
- Test coverage
- User satisfaction surveys

**Target KPIs**:
- 1000+ installs in first month
- 4.5+ star rating
- < 100ms completion latency (p95)
- > 80% test coverage
