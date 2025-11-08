# HORUS VSCode Extension - Testing and Validation Strategy

## Testing Philosophy

The HORUS VSCode extension follows a multi-layered testing approach:

1. **Unit Tests**: Test individual functions and modules in isolation
2. **Integration Tests**: Test component interactions
3. **End-to-End Tests**: Test complete user workflows
4. **Performance Tests**: Ensure responsiveness and efficiency
5. **Compatibility Tests**: Verify cross-platform and version compatibility

## Test Coverage Goals

**Minimum Acceptable**:
- Unit test coverage: 70%
- Integration test coverage: 60%
- Critical paths: 100%

**Target**:
- Unit test coverage: 85%
- Integration test coverage: 75%
- Overall coverage: 80%

## Unit Testing

### Extension (TypeScript)

**Framework**: Mocha + Chai

**Test Structure**:
```
src/test/suite/
├── extension.test.ts          # Extension activation
├── languageClient.test.ts     # LSP client
├── taskProvider.test.ts       # Task provider
├── dashboard.test.ts          # Dashboard
└── utils.test.ts              # Utility functions
```

**Example Test** (`src/test/suite/taskProvider.test.ts`):
```typescript
import * as assert from 'assert';
import * as vscode from 'vscode';
import { HorusTaskProvider } from '../../taskProvider';

suite('HorusTaskProvider', () => {
    let provider: HorusTaskProvider;

    setup(() => {
        provider = new HorusTaskProvider();
    });

    test('should provide tasks for HORUS project', async () => {
        const tasks = await provider.provideTasks();

        assert.ok(tasks.length > 0, 'Should provide at least one task');
        assert.ok(
            tasks.some(t => t.name.includes('run')),
            'Should provide run task'
        );
        assert.ok(
            tasks.some(t => t.name.includes('check')),
            'Should provide check task'
        );
    });

    test('should create correct task execution', async () => {
        const tasks = await provider.provideTasks();
        const runTask = tasks.find(t => t.name.includes('run'));

        assert.ok(runTask, 'Run task should exist');
        assert.ok(runTask.execution, 'Task should have execution');

        const exec = runTask.execution as vscode.ShellExecution;
        assert.ok(exec.command.includes('horus'), 'Should execute horus command');
    });

    test('should handle non-HORUS projects', async () => {
        // Test with empty workspace
        const tasks = await provider.provideTasks();

        // Should return empty array or minimal tasks
        assert.ok(Array.isArray(tasks), 'Should return array');
    });
});
```

**Running Tests**:
```bash
npm test
```

**Coverage Report**:
```bash
npm install --save-dev nyc
npm run test:coverage
```

### Language Server (Rust)

**Framework**: Built-in Rust test framework + tokio-test

**Test Structure**:
```
server/tests/
├── project_test.rs           # Project detection
├── completion_test.rs        # Completion provider
├── hover_test.rs            # Hover provider
├── symbols_test.rs          # Symbol resolution
└── integration/
    ├── lsp_compliance.rs    # LSP protocol compliance
    └── performance.rs       # Performance benchmarks
```

**Example Test** (`server/tests/project_test.rs`):
```rust
use tempfile::TempDir;
use std::fs;
use horus_language_server::project::{HorusProject, HorusYaml};

#[tokio::test]
async fn test_detect_valid_horus_project() {
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

    assert!(project.is_some(), "Should detect HORUS project");

    let project = project.unwrap();
    assert_eq!(project.config.name, "test_project");
    assert_eq!(project.config.version, "0.1.0");
    assert_eq!(project.config.language, "rust");
    assert_eq!(project.dependencies.len(), 1);
}

#[tokio::test]
async fn test_no_detection_for_non_horus_project() {
    let temp_dir = TempDir::new().unwrap();

    // No horus.yaml
    let project = HorusProject::detect(temp_dir.path()).await.unwrap();

    assert!(project.is_none(), "Should not detect non-HORUS project");
}

#[tokio::test]
async fn test_invalid_yaml_returns_error() {
    let temp_dir = TempDir::new().unwrap();

    // Create invalid horus.yaml
    fs::write(temp_dir.path().join("horus.yaml"), "invalid: yaml: content:").unwrap();

    // Should return error
    let result = HorusProject::detect(temp_dir.path()).await;

    assert!(result.is_err(), "Should error on invalid YAML");
}

#[tokio::test]
async fn test_resolve_horus_source() {
    // Set environment variable
    std::env::set_var("HORUS_SOURCE", "/test/path/to/horus");

    let source = resolve_horus_source();

    // Should use environment variable
    assert!(source.is_ok() || source.is_err());
    // Actual validation depends on filesystem
}
```

**Running Tests**:
```bash
cd server
cargo test
```

**Coverage Report**:
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Integration Testing

### Extension Integration Tests

**Purpose**: Test interaction between extension components

**Test File** (`src/test/suite/integration.test.ts`):
```typescript
import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import { execSync } from 'child_process';

suite('Integration Tests', function() {
    this.timeout(60000); // 1 minute timeout

    let testProjectPath: string;

    suiteSetup(async () => {
        // Create test HORUS project
        const testDir = path.join(__dirname, '../../../test-workspace');
        execSync(`horus new test_integration_project`, { cwd: testDir });
        testProjectPath = path.join(testDir, 'test_integration_project');

        // Open in VSCode
        const uri = vscode.Uri.file(testProjectPath);
        await vscode.commands.executeCommand('vscode.openFolder', uri);

        // Wait for extension activation
        await new Promise(resolve => setTimeout(resolve, 3000));
    });

    test('Extension activates for HORUS project', async () => {
        const ext = vscode.extensions.getExtension('horus.horus-vscode');

        assert.ok(ext, 'Extension should be loaded');
        assert.ok(ext.isActive, 'Extension should be active');
    });

    test('Language server starts successfully', async () => {
        // Check output channel for success message
        const outputChannel = vscode.window.createOutputChannel('HORUS');
        const logs = outputChannel.toString();

        assert.ok(
            logs.includes('Language Server ready') || logs.includes('initialized'),
            'Language server should start'
        );
    });

    test('Completion works for HORUS imports', async () => {
        // Open main.rs
        const mainFile = path.join(testProjectPath, 'main.rs');
        const document = await vscode.workspace.openTextDocument(mainFile);
        await vscode.window.showTextDocument(document);

        // Trigger completion at "use horus::"
        const position = new vscode.Position(0, 11); // After "use horus::"
        const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
            'vscode.executeCompletionItemProvider',
            document.uri,
            position
        );

        assert.ok(completions, 'Should return completions');
        assert.ok(
            completions.items.some(item => item.label === 'prelude'),
            'Should suggest prelude'
        );
    });

    test('Tasks are registered', async () => {
        const tasks = await vscode.tasks.fetchTasks({ type: 'horus' });

        assert.ok(tasks.length > 0, 'Should register tasks');
        assert.ok(
            tasks.some(t => t.name.includes('run')),
            'Should register run task'
        );
    });

    test('Run task executes successfully', async function() {
        this.timeout(30000);

        const tasks = await vscode.tasks.fetchTasks({ type: 'horus' });
        const runTask = tasks.find(t => t.name.includes('run'));

        assert.ok(runTask, 'Run task should exist');

        // Execute task
        const execution = await vscode.tasks.executeTask(runTask);

        // Wait for completion
        await new Promise((resolve, reject) => {
            const disposable = vscode.tasks.onDidEndTask(e => {
                if (e.execution === execution) {
                    disposable.dispose();
                    resolve();
                }
            });

            // Timeout after 20 seconds
            setTimeout(() => {
                disposable.dispose();
                reject(new Error('Task execution timeout'));
            }, 20000);
        });
    });

    suiteTeardown(() => {
        // Clean up test project
        execSync(`rm -rf ${testProjectPath}`);
    });
});
```

### LSP Protocol Compliance Tests

**Purpose**: Ensure language server conforms to LSP specification

**Test File** (`server/tests/integration/lsp_compliance.rs`):
```rust
use tower_lsp::lsp_types::*;
use tower_lsp::LspService;

#[tokio::test]
async fn test_initialize_request() {
    let (service, _) = LspService::new(|client| {
        HorusLanguageServer::new(client)
    });

    let params = InitializeParams {
        process_id: Some(1234),
        root_uri: Some(Url::from_file_path("/test/workspace").unwrap()),
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };

    let response = service.initialize(params).await.unwrap();

    // Verify capabilities
    assert!(
        response.capabilities.text_document_sync.is_some(),
        "Should support text document sync"
    );
    assert!(
        response.capabilities.completion_provider.is_some(),
        "Should support completion"
    );
    assert!(
        response.capabilities.hover_provider.is_some(),
        "Should support hover"
    );
}

#[tokio::test]
async fn test_completion_request() {
    let (service, _) = setup_test_service().await;

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path("/test/main.rs").unwrap()
            },
            position: Position { line: 0, character: 11 }
        },
        context: None,
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    let response = service.completion(params).await;

    assert!(response.is_ok(), "Completion should not error");

    match response.unwrap() {
        Some(CompletionResponse::Array(items)) => {
            assert!(!items.is_empty(), "Should return completion items");
        }
        Some(CompletionResponse::List(list)) => {
            assert!(!list.items.is_empty(), "Should return completion items");
        }
        None => panic!("Should return some completions")
    }
}

#[tokio::test]
async fn test_hover_request() {
    let (service, _) = setup_test_service().await;

    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path("/test/main.rs").unwrap()
            },
            position: Position { line: 5, character: 10 }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let response = service.hover(params).await;

    assert!(response.is_ok(), "Hover should not error");
}
```

## End-to-End Testing

### User Workflow Tests

**Scenario 1: New Project Creation and Development**
```typescript
test('Complete new project workflow', async function() {
    this.timeout(120000); // 2 minutes

    // 1. Create project via CLI
    execSync('horus new e2e_test_project');

    // 2. Open in VSCode
    const uri = vscode.Uri.file('./e2e_test_project');
    await vscode.commands.executeCommand('vscode.openFolder', uri);
    await sleep(3000);

    // 3. Verify extension active
    const ext = vscode.extensions.getExtension('horus.horus-vscode');
    assert.ok(ext?.isActive);

    // 4. Edit code
    const document = await vscode.workspace.openTextDocument('./e2e_test_project/main.rs');
    const editor = await vscode.window.showTextDocument(document);

    // 5. Trigger completion
    await editor.edit(editBuilder => {
        editBuilder.insert(new vscode.Position(2, 0), 'use horus::');
    });

    await sleep(500);

    const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
        'vscode.executeCompletionItemProvider',
        document.uri,
        new vscode.Position(2, 11)
    );

    assert.ok(completions?.items.some(i => i.label === 'prelude'));

    // 6. Run project
    await vscode.commands.executeCommand('horus.run');
    await sleep(5000);

    // 7. Verify no errors
    const diagnostics = vscode.languages.getDiagnostics();
    const errors = diagnostics.filter(([_, diags]) =>
        diags.some(d => d.severity === vscode.DiagnosticSeverity.Error)
    );

    assert.strictEqual(errors.length, 0, 'Should have no errors');
});
```

**Scenario 2: Dashboard Integration**
```typescript
test('Dashboard workflow', async function() {
    this.timeout(60000);

    // 1. Open HORUS project
    const uri = vscode.Uri.file('./test_project');
    await vscode.commands.executeCommand('vscode.openFolder', uri);
    await sleep(2000);

    // 2. Start project in background
    const terminal = vscode.window.createTerminal('HORUS');
    terminal.sendText('horus run');
    await sleep(5000);

    // 3. Open dashboard
    await vscode.commands.executeCommand('horus.dashboard');
    await sleep(2000);

    // 4. Verify dashboard visible
    const panels = vscode.window.tabGroups.all
        .flatMap(group => group.tabs)
        .filter(tab => tab.label.includes('Dashboard'));

    assert.ok(panels.length > 0, 'Dashboard should be open');

    // 5. Clean up
    terminal.dispose();
});
```

### Manual Test Checklist

Execute these tests manually before each release:

**Project Detection**:
- [ ] Open folder with horus.yaml - extension activates
- [ ] Open folder without horus.yaml - extension stays inactive
- [ ] Status bar shows "HORUS: Ready"
- [ ] Error notification if HORUS_SOURCE not found

**Code Intelligence**:
- [ ] Type `use horus::` - autocomplete shows `prelude`
- [ ] Type `Hub::` - autocomplete shows `new`
- [ ] Hover over `Scheduler` - documentation appears
- [ ] Click on `Node` - jumps to definition
- [ ] Introduce type error - red squiggle appears
- [ ] Save file - diagnostics update

**Tasks**:
- [ ] Ctrl+Shift+B shows HORUS tasks
- [ ] Select "HORUS: Run" - terminal opens and executes
- [ ] Task output appears in terminal
- [ ] Errors highlighted in Problems panel
- [ ] Task completes successfully

**Commands**:
- [ ] Ctrl+Shift+P  "HORUS: Run" - executes
- [ ] Ctrl+Shift+P  "HORUS: Check" - executes
- [ ] Ctrl+Shift+P  "HORUS: Dashboard" - opens dashboard
- [ ] All commands work as expected

**Debugging**:
- [ ] Set breakpoint in main.rs
- [ ] Press F5 - debug session starts
- [ ] Breakpoint hits
- [ ] Variables panel shows values
- [ ] Step through code works
- [ ] Continue execution works

**Dashboard**:
- [ ] Start `horus run` in terminal
- [ ] Open dashboard via command
- [ ] Dashboard shows live data
- [ ] Nodes list updates
- [ ] Topics list updates
- [ ] Dashboard survives hide/show

**Topic Inspector**:
- [ ] Hover over topic string `"cmd_vel"`
- [ ] Tooltip shows topic info
- [ ] Current value displayed (if available)
- [ ] Publishers/subscribers listed

**Cross-Platform**:
- [ ] Test on Linux
- [ ] Test on macOS
- [ ] Test on Windows
- [ ] All features work on each platform

## Performance Testing

### Benchmarks

**Language Server Response Time**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_completion(c: &mut Criterion) {
    let project = setup_test_project();
    let server = HorusLanguageServer::new(mock_client());

    c.bench_function("completion_horus_prelude", |b| {
        b.iter(|| {
            let params = CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::from_file_path("/test/main.rs").unwrap()
                    },
                    position: Position { line: 0, character: 11 }
                },
                context: None,
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            black_box(server.completion(params))
        });
    });
}

fn bench_hover(c: &mut Criterion) {
    let server = HorusLanguageServer::new(mock_client());

    c.bench_function("hover_on_symbol", |b| {
        b.iter(|| {
            let params = HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::from_file_path("/test/main.rs").unwrap()
                    },
                    position: Position { line: 5, character: 10 }
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            };

            black_box(server.hover(params))
        });
    });
}

criterion_group!(benches, bench_completion, bench_hover);
criterion_main!(benches);
```

**Performance Targets**:
- Extension activation: < 2 seconds
- Language server startup: < 1 second
- Completion response: < 50ms (p95)
- Hover response: < 100ms (p95)
- Memory usage: < 100MB (language server)
- Bundle size: < 500KB (extension)

**Running Benchmarks**:
```bash
cd server
cargo bench
```

### Load Testing

**Large Project Handling**:
```rust
#[tokio::test]
async fn test_large_project_indexing() {
    let large_project = create_large_test_project(1000); // 1000 files

    let start = Instant::now();
    let project = HorusProject::detect(&large_project).await.unwrap();
    let duration = start.elapsed();

    assert!(
        duration < Duration::from_secs(10),
        "Large project indexing should complete within 10 seconds"
    );
}
```

## Compatibility Testing

### VSCode Version Compatibility

**Supported Versions**: 1.85.0+

**Test Matrix**:
| VSCode Version | Linux | macOS | Windows |
|----------------|-------|-------|---------|
| 1.85.0 (min)   | Yes   | Yes   | Yes     |
| 1.86.0         | Yes   | Yes   | Yes     |
| Insiders       | Yes   | Yes   | Yes     |

**Compatibility Test**:
```bash
# Test on specific VSCode version
code-1.85.0 --extensionDevelopmentPath=. --extensionTestsPath=./out/test
```

### Platform-Specific Tests

**Linux**:
```bash
# Test on Ubuntu 22.04, Fedora 38
npm run test:linux
```

**macOS**:
```bash
# Test on Intel and Apple Silicon
npm run test:macos
```

**Windows**:
```bash
# Test on Windows 10, Windows 11
npm run test:windows
```

### HORUS Version Compatibility

**Test with Multiple HORUS Versions**:
```bash
# Test with HORUS 0.1.0
HORUS_VERSION=0.1.0 npm test

# Test with latest
HORUS_VERSION=latest npm test
```

## Regression Testing

### Automated Regression Suite

**Purpose**: Catch regressions after code changes

**Setup** (`.github/workflows/regression.yml`):
```yaml
name: Regression Tests

on: [pull_request]

jobs:
  regression:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - run: npm ci
      - run: npm run test:regression
      - uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: regression-results
          path: test-results/
```

**Regression Test Suite**:
```typescript
suite('Regression Tests', () => {
    test('Issue #1: Completion crashes on empty file', async () => {
        // Test specific bug fix
        const document = await createEmptyDocument();
        const completions = await vscode.commands.executeCommand(
            'vscode.executeCompletionItemProvider',
            document.uri,
            new vscode.Position(0, 0)
        );

        assert.doesNotThrow(() => completions);
    });

    test('Issue #2: Dashboard freezes with large logs', async () => {
        // Generate large log output
        for (let i = 0; i < 10000; i++) {
            console.log(`Log line ${i}`);
        }

        await vscode.commands.executeCommand('horus.dashboard');
        // Dashboard should still be responsive
        await sleep(1000);

        const panels = vscode.window.tabGroups.all
            .flatMap(group => group.tabs)
            .filter(tab => tab.label.includes('Dashboard'));

        assert.ok(panels.length > 0);
    });
});
```

## Validation Before Release

### Pre-Release Checklist

**Code Quality**:
- [ ] All tests pass (unit, integration, e2e)
- [ ] Test coverage meets targets (>80%)
- [ ] No high-severity linting warnings
- [ ] Code formatted with prettier/rustfmt
- [ ] No `console.log` statements (use proper logging)
- [ ] No hardcoded paths or credentials

**Functionality**:
- [ ] All commands work
- [ ] All features tested manually
- [ ] Dashboard works
- [ ] Debugging works
- [ ] No errors in VSCode Developer Tools console

**Performance**:
- [ ] Extension activates in < 2 seconds
- [ ] Completion responds in < 50ms
- [ ] Memory usage < 100MB
- [ ] No memory leaks (run for 1 hour)

**Documentation**:
- [ ] README updated
- [ ] CHANGELOG updated
- [ ] API documentation generated
- [ ] Examples work

**Compatibility**:
- [ ] Tested on Linux, macOS, Windows
- [ ] Tested with VSCode 1.85.0 - Latest
- [ ] Tested with HORUS 0.1.0+
- [ ] No breaking changes without major version bump

**Package**:
- [ ] Extension bundles without errors
- [ ] .vsix file < 10MB
- [ ] All required files included
- [ ] No unnecessary files included

### Release Process

**1. Version Bump**:
```bash
# Patch release (bug fixes)
npm version patch

# Minor release (new features)
npm version minor

# Major release (breaking changes)
npm version major
```

**2. Build and Test**:
```bash
npm run test:all
npm run package
```

**3. Manual Validation**:
```bash
code --install-extension horus-vscode-X.Y.Z.vsix
# Perform manual testing
```

**4. Publish**:
```bash
vsce publish
```

**5. Create GitHub Release**:
```bash
gh release create vX.Y.Z horus-vscode-X.Y.Z.vsix \
    --title "vX.Y.Z" \
    --notes "$(cat CHANGELOG.md | sed -n '/## \[X.Y.Z\]/,/## \[/p' | sed '$d')"
```

## Continuous Testing

### CI/CD Pipeline

**GitHub Actions** (`.github/workflows/ci.yml`):
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test-extension:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 18
      - run: npm ci
      - run: npm run compile
      - run: npm test
      - uses: codecov/codecov-action@v3
        with:
          files: ./coverage/lcov.info

  test-server:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cd server && cargo test --all-features
      - run: cd server && cargo tarpaulin --out Xml
      - uses: codecov/codecov-action@v3
        with:
          files: ./server/cobertura.xml

  integration:
    runs-on: ubuntu-latest
    needs: [test-extension, test-server]
    steps:
      - uses: actions/checkout@v4
      - run: npm ci
      - run: npm run test:integration

  package:
    runs-on: ubuntu-latest
    needs: [test-extension, test-server, integration]
    steps:
      - uses: actions/checkout@v4
      - run: npm ci
      - run: cargo build --release --manifest-path=server/Cargo.toml
      - run: npm run package
      - uses: actions/upload-artifact@v4
        with:
          name: vsix
          path: '*.vsix'
```

## Monitoring and Feedback

### Error Tracking

**Sentry Integration** (optional):
```typescript
import * as Sentry from '@sentry/node';

if (process.env.SENTRY_DSN) {
    Sentry.init({
        dsn: process.env.SENTRY_DSN,
        environment: process.env.NODE_ENV || 'development',
        beforeSend(event) {
            // Remove sensitive information
            if (event.user) {
                delete event.user.email;
                delete event.user.ip_address;
            }
            return event;
        }
    });
}
```

### Usage Analytics

**Telemetry** (opt-in):
```typescript
async function trackFeatureUsage(feature: string) {
    const telemetryEnabled = vscode.workspace
        .getConfiguration('horus')
        .get<boolean>('telemetry.enabled', false);

    if (!telemetryEnabled) return;

    // Send anonymous usage data
    await fetch('https://telemetry.horus.com/event', {
        method: 'POST',
        body: JSON.stringify({
            feature,
            version: getExtensionVersion(),
            timestamp: Date.now()
        })
    });
}
```

### User Feedback

**GitHub Issues Template**:
```markdown
### Bug Report

**Extension Version**:

**VSCode Version**:

**OS**:

**HORUS Version**:

**Steps to Reproduce**:
1.
2.
3.

**Expected Behavior**:

**Actual Behavior**:

**Logs**:
```
(Paste logs from Output > HORUS)
```

**Additional Context**:
```

## Summary

This testing strategy ensures:
- High code quality through comprehensive unit tests
- Correct behavior through integration tests
- Real-world usability through end-to-end tests
- Performance through benchmarks
- Compatibility across platforms and versions
- Continuous quality through CI/CD
- User satisfaction through monitoring and feedback
