# HORUS IDE Integration Architecture

## Design Principles

### 1. Shared Core, IDE-Specific UI

**Core Logic** (Shared):
- Language Server Protocol implementation
- Debug Adapter Protocol implementation
- HORUS project analysis
- Symbol resolution and indexing

**UI Layer** (IDE-Specific):
- Task providers
- Command palette integration
- Dashboard visualization
- Settings panels

### 2. Protocol-Based Communication

All IDE integrations communicate with shared components via standard protocols:

**Language Server Protocol (LSP 3.17)**:
- Text synchronization
- Completion requests
- Hover requests
- Diagnostics
- Custom HORUS methods

**Debug Adapter Protocol (DAP 1.51)**:
- Launch/attach
- Breakpoints
- Variable inspection
- Step control

### 3. Zero Configuration

IDE integrations automatically detect HORUS projects by:
1. Looking for `horus.yaml` in workspace root
2. Resolving `HORUS_SOURCE` environment variable
3. Starting language server when HORUS project detected

No manual configuration files (like Cargo.toml) required.

## Shared Components

### Language Server

**Location**: `shared/language-server/`

**Technology**: Rust + tower-lsp

**Responsibilities**:
- Parse and analyze `horus.yaml`
- Resolve HORUS framework source location
- Index project symbols
- Provide code completion
- Generate diagnostics
- Support custom HORUS features (topic inspection, etc.)

**Communication**:
- Input: stdin (JSON-RPC messages)
- Output: stdout (JSON-RPC responses)
- Transport: Standard I/O pipes

**Lifecycle**:
1. IDE starts language server process
2. Server sends `initialize` request
3. Server responds with capabilities
4. IDE sends `initialized` notification
5. Server indexes project
6. Server responds to LSP requests
7. IDE sends `shutdown` request
8. Server terminates

**Custom LSP Methods**:

```typescript
// Topic inspection
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

method: "horus/topicInfo"
params: TopicInfoParams
result: TopicInfo
```

```typescript
// Node graph data
interface GraphDataParams {}

interface GraphData {
    nodes: NodeInfo[];
    topics: TopicInfo[];
    connections: Connection[];
}

method: "horus/graphData"
params: GraphDataParams
result: GraphData
```

```typescript
// List active nodes
interface ListNodesParams {}

interface NodeInfo {
    name: string;
    publishers: { topic: string; type: string }[];
    subscribers: { topic: string; type: string }[];
    tickRate: number;
    cpuUsage: number;
}

method: "horus/listNodes"
params: ListNodesParams
result: NodeInfo[]
```

### Debug Adapter

**Location**: `shared/debug-adapter/`

**Technology**: Rust + Debug Adapter Protocol

**Responsibilities**:
- Launch HORUS applications with debug symbols
- Manage LLDB/GDB integration
- Handle breakpoints
- Inspect variables
- Control execution flow

**Communication**:
- Input: stdin (JSON-RPC DAP messages)
- Output: stdout (JSON-RPC DAP responses)
- Transport: Standard I/O pipes

**Custom DAP Configuration**:

```json
{
    "type": "horus",
    "request": "launch",
    "program": "${file}",
    "args": [],
    "cwd": "${workspaceFolder}",
    "horusDebug": true
}
```

## IDE-Specific Implementation Guidelines

### VSCode Extension

**Technology**: TypeScript + VSCode API

**Structure**:
```
horus-vscode/
├── package.json           # Extension manifest
├── src/
│   ├── extension.ts       # Entry point
│   ├── languageClient.ts  # LSP client
│   ├── taskProvider.ts    # Task integration
│   └── dashboard.ts       # Dashboard webview
└── out/                   # Compiled output
```

**Integration Pattern**:
```typescript
// Start language server
const client = new LanguageClient(
    'horusLanguageServer',
    serverOptions,
    clientOptions
);

await client.start();

// Send custom request
const topicInfo = await client.sendRequest('horus/topicInfo', {
    topic: 'cmd_vel'
});
```

**Key APIs**:
- `vscode-languageclient`: LSP client
- `vscode.tasks`: Task provider
- `vscode.debug`: Debug adapter
- `vscode.window.createWebviewPanel`: Dashboard

### IntelliJ IDEA / CLion Plugin

**Technology**: Kotlin + IntelliJ Platform SDK

**Structure**:
```
horus-intellij/
├── plugin.xml             # Plugin descriptor
├── build.gradle.kts       # Build configuration
└── src/main/kotlin/
    ├── HorusPlugin.kt
    ├── HorusLspClient.kt
    └── HorusTaskProvider.kt
```

**Integration Pattern**:
```kotlin
// LSP client integration
class HorusLspClient : LanguageServerWrapper() {
    override fun getLanguageServerName() = "HORUS"

    override fun createServerDefinition(): LanguageServerDefinition {
        return RawCommandServerDefinition("horus-language-server")
    }
}
```

**Key APIs**:
- `com.intellij.openapi.project`: Project management
- `com.redhat.devtools.lsp4ij`: LSP support
- `com.intellij.execution`: Run configurations
- `com.intellij.xdebugger`: Debug support

### Vim/Neovim Plugin

**Technology**: Lua (Neovim) or VimScript (Vim)

**Structure**:
```
horus-vim/
├── plugin/horus.vim       # Plugin initialization
├── autoload/horus.vim     # Autoload functions
└── lua/horus/
    ├── init.lua
    └── lsp.lua            # LSP configuration
```

**Integration Pattern (Neovim with built-in LSP)**:
```lua
-- Setup HORUS language server
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

configs.horus_ls = {
    default_config = {
        cmd = { 'horus-language-server' },
        filetypes = { 'rust' },
        root_dir = lspconfig.util.root_pattern('horus.yaml'),
        settings = {},
    },
}

lspconfig.horus_ls.setup{}
```

**Integration Pattern (Vim with coc.nvim)**:
```json
{
  "languageserver": {
    "horus": {
      "command": "horus-language-server",
      "filetypes": ["rust"],
      "rootPatterns": ["horus.yaml"],
      "settings": {}
    }
  }
}
```

**Key Dependencies**:
- Neovim: Built-in LSP client
- Vim: coc.nvim or vim-lsp

### Emacs Package

**Technology**: Emacs Lisp

**Structure**:
```
horus-emacs/
├── horus.el               # Main package
├── horus-lsp.el           # LSP configuration
└── horus-mode.el          # Major mode
```

**Integration Pattern**:
```elisp
;; Register HORUS language server
(require 'lsp-mode)

(lsp-register-client
 (make-lsp-client
  :new-connection (lsp-stdio-connection "horus-language-server")
  :major-modes '(rust-mode)
  :server-id 'horus-ls
  :activation-fn (lsp-activate-on "horus.yaml")))
```

**Key Dependencies**:
- `lsp-mode`: LSP client
- `rust-mode`: Rust editing support

## Feature Implementation Patterns

### Code Completion

**Flow**:
1. User types in editor
2. IDE sends `textDocument/completion` to language server
3. Language server analyzes context
4. Language server returns completion items
5. IDE displays completion popup

**Language Server Logic**:
```rust
async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
    let position = params.text_document_position;

    // Analyze context
    let context = self.analyze_context(&position)?;

    // Generate completions
    let items = match context {
        CompletionContext::Import => self.complete_imports(),
        CompletionContext::Type => self.complete_types(),
        CompletionContext::Topic => self.complete_topics(),
        _ => vec![]
    };

    Ok(Some(CompletionResponse::Array(items)))
}
```

### Topic Inspector

**Flow**:
1. User hovers over topic string
2. IDE sends `textDocument/hover` to language server
3. Language server identifies topic string
4. Language server sends custom `horus/topicInfo` request
5. Language server formats response as markdown
6. IDE displays hover popup

**IDE-Specific Implementation**:

**VSCode**:
```typescript
class TopicHoverProvider implements vscode.HoverProvider {
    async provideHover(document, position) {
        const range = document.getWordRangeAtPosition(position, /"[^"]+"/);
        if (!range) return;

        const topic = document.getText(range).slice(1, -1);
        const info = await this.lspClient.sendRequest('horus/topicInfo', { topic });

        return new vscode.Hover(formatTopicInfo(info), range);
    }
}
```

**Neovim**:
```lua
vim.lsp.handlers['textDocument/hover'] = function(_, result, ctx)
    -- Standard LSP hover handling
    -- Topic info is included in hover response
    if result and result.contents then
        vim.lsp.util.open_floating_preview(result.contents)
    end
end
```

### Dashboard Integration

**VSCode** (WebView):
```typescript
const panel = vscode.window.createWebviewPanel(
    'horusDashboard',
    'HORUS Dashboard',
    vscode.ViewColumn.Two,
    { enableScripts: true }
);

panel.webview.html = `
    <iframe src="http://localhost:3000"></iframe>
`;
```

**IntelliJ** (Tool Window):
```kotlin
class HorusDashboardToolWindow : ToolWindowFactory {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val content = toolWindow.contentManager.factory
            .createContent(JBCefBrowser("http://localhost:3000").component, "", false)
        toolWindow.contentManager.addContent(content)
    }
}
```

**Vim/Emacs** (Terminal/Browser):
```lua
-- Open dashboard in browser
vim.fn.system('horus dashboard')
```

### Node Graph Visualization

**VSCode** (WebView with D3.js):
```typescript
const graphData = await client.sendRequest('horus/graphData', {});
panel.webview.html = getGraphHtml(graphData);
```

**IntelliJ** (Custom Swing Component):
```kotlin
class NodeGraphPanel : JPanel() {
    fun renderGraph(data: GraphData) {
        // Custom graph rendering with Swing
    }
}
```

**Vim/Emacs** (ASCII Art or External Tool):
```lua
-- Display graph as ASCII art or open external viewer
local graph = vim.fn.systemlist('horus graph --format=ascii')
vim.api.nvim_buf_set_lines(buf, 0, -1, false, graph)
```

## Testing Strategy

### Language Server Testing

**Unit Tests** (Rust):
```rust
#[tokio::test]
async fn test_completion_horus_imports() {
    let server = create_test_server();
    let response = server.completion(test_params()).await.unwrap();

    assert!(response.is_some());
    let items = response.unwrap();
    assert!(items.iter().any(|i| i.label == "prelude"));
}
```

**Integration Tests** (Protocol Testing):
```rust
#[tokio::test]
async fn test_lsp_protocol_compliance() {
    let (stdin, stdout) = UnixStream::pair().unwrap();
    let server = spawn_server(stdin, stdout);

    send_initialize_request(&server);
    assert_eq!(server.capabilities.completion_provider, Some(...));
}
```

### IDE Extension Testing

**VSCode**:
```typescript
suite('Extension Tests', () => {
    test('Activates on HORUS project', async () => {
        const ext = vscode.extensions.getExtension('horus.horus-vscode');
        await ext.activate();
        assert.ok(ext.isActive);
    });
});
```

**IntelliJ**:
```kotlin
class HorusPluginTest : BasePlatformTestCase() {
    fun testPluginActivation() {
        val plugin = PluginManager.getPlugin(PluginId.getId("com.horus.intellij"))
        assertNotNull(plugin)
    }
}
```

## Performance Considerations

### Language Server Optimization

**Caching**:
- Cache parsed symbols
- Cache HORUS framework index
- Invalidate on file changes only

**Incremental Analysis**:
- Only re-analyze changed files
- Use dependency graph for efficient updates

**Memory Management**:
- Limit cache size
- Clear unused symbols periodically
- Use weak references where appropriate

### IDE Extension Optimization

**Lazy Loading**:
- Load dashboard only when requested
- Defer heavy computations
- Use background threads/workers

**Bundle Size**:
- Tree-shake unused dependencies
- Minify production builds
- Compress assets

## Cross-IDE Compatibility

### File Handling

All IDEs must:
- Watch `horus.yaml` for changes
- Detect new/deleted `.rs` files
- Handle multi-root workspaces (if applicable)

### Settings Synchronization

Settings naming convention:
- `horus.horusSource`: HORUS source path
- `horus.enableDashboard`: Dashboard toggle
- `horus.trace.server`: Language server logging level

### Error Reporting

Consistent error messages across IDEs:
- "HORUS source not found. Set HORUS_SOURCE environment variable."
- "Not a HORUS project. Missing horus.yaml in workspace root."
- "Language server failed to start. Check HORUS installation."

## Migration Path

### Phase 1: VSCode (Standalone)
- Implement all features in VSCode extension
- Language server embedded in extension

### Phase 2: Extract Language Server
- Move language server to `shared/language-server/`
- Make VSCode extension use standalone server
- Document LSP custom protocol

### Phase 3: Additional IDEs
- Implement IntelliJ plugin using shared server
- Implement Vim plugin using shared server
- Document IDE-specific integration patterns

### Phase 4: Maintenance
- Maintain feature parity across IDEs
- Shared bug fixes in language server
- IDE-specific UI improvements

## Contributing Guidelines

### Adding New IDE Support

1. **Evaluate Feasibility**:
   - Does IDE support LSP or similar protocol?
   - Is there active user demand?
   - Are maintainers available?

2. **Create Directory**:
   - `mkdir IDE-integration/horus-<ide-name>`
   - Follow naming convention

3. **Implement Core Features**:
   - LSP client integration
   - Task/command execution
   - Settings panel

4. **Optional Advanced Features**:
   - Debug adapter integration
   - Dashboard visualization
   - Custom UI components

5. **Document Thoroughly**:
   - README with installation
   - Technical architecture
   - User guide

6. **Test Comprehensively**:
   - Unit tests
   - Integration tests
   - Manual testing checklist

7. **Maintain**:
   - Fix IDE-specific bugs
   - Keep up with IDE API changes
   - Respond to user issues
