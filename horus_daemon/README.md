# HORUS Daemon - Remote Execution Server

A lightweight HTTP server that enables remote code deployment and execution for HORUS robotics framework.

## Overview

The HORUS daemon runs on your robot and accepts code deployments from your development machine over WiFi/network, eliminating the need for manual SSH/SCP workflows.

## Architecture

```
Laptop (CLI)                    Robot (Daemon)
    |                               |
    | 1. Package code as tar.gz     |
    |------------------------------>|
    |                               | 2. Extract to /tmp/horus/
    |                               | 3. Compile if needed (Rust/C)
    |                               | 4. Execute binary/script
    |                               | 5. Return PID & deployment ID
    |<------------------------------|
    | 6. Show success message       |
```

## Features

### Core Functionality (Implemented )

- **HTTP Server**: Axum-based server on port 8080
- **Code Upload**: Accepts tar.gz archives via POST `/deploy`
- **Auto-extraction**: Unpacks to `/tmp/horus/deploy-{uuid}/`
- **Multi-Language Support**:
  - **Python**: Direct execution with `python3`
  - **Rust**: Compilation with `rustc` (if available)
  - **C**: Compilation with `gcc` (if available)
- **Health Check**: GET `/health` endpoint
- **Deployment Tracking**: Returns deployment ID and PID
- **Process Execution**: Spawns deployed code as separate process

## Installation

### On Robot (Raspberry Pi, Jetson, etc.)

```bash
# Build the daemon
cd /path/to/HORUS
cargo build --release -p horus_daemon

# Run the daemon
./target/release/horus_daemon
```

### On Development Machine

```bash
# Build CLI with remote support
cargo build --release -p horus_manager

# Or install globally
cargo install --path horus_manager

# Deploy code
horus run --remote <ROBOT> <file>
```

## Usage

### Start Daemon

```bash
# Start on default port 8080
./target/debug/horus_daemon

# Expected output:
#  HORUS daemon listening on 0.0.0.0:8080
# 📡 Ready to receive deployments
```

### Deploy Code

```bash
# Deploy specific file
horus run --remote localhost:8080 robot_node.py
horus run -R localhost:8080 robot_node.py  # Short form

# Auto-detect main.py in current directory
cd my_project/
horus run -R robot

# Deploy with full URL
horus run -R http://192.168.1.100:8080 vision.py
```

### URL Formats Supported

- `localhost:8080` → `http://localhost:8080/deploy`
- `192.168.1.100` → `http://192.168.1.100:8080/deploy`
- `robot` → `http://robot:8080/deploy`
- `http://robot:8080` → `http://robot:8080/deploy`

## API Reference

### `GET /health`

Health check endpoint.

**Response:**
```
OK
```

### `POST /deploy`

Deploy and execute code.

**Request:**
- Content-Type: `application/gzip`
- Body: tar.gz archive containing source files (.py, .rs, .c)

**Response:**
```json
{
  "deployment_id": "323e53aa-fc2e-4955-a9d3-b6b1bc1fd461",
  "status": "running",
  "pid": 48854,
  "message": "Successfully deployed and started /tmp/horus/deploy-.../main.rs"
}
```

**Error Response:**
```json
{
  "error": "Compilation failed: rustc: command not found"
}
```

## File Detection

The daemon automatically finds and handles the entry point:

1. Looks for `main.*` (main.py, main.rs, or main.c - highest priority)
2. Falls back to first source file found (.py, .rs, or .c)
3. **Python (.py)**: Executes directly with `python3`
4. **Rust (.rs)**: Compiles with `rustc --edition 2021`, then executes
5. **C (.c)**: Compiles with `gcc`, then executes
6. Returns compilation errors to CLI if build fails

## Directory Structure

```
horus_daemon/
├── Cargo.toml           # Dependencies (axum, tokio, tar, etc.)
├── src/
│   ├── main.rs         # HTTP server setup
│   └── deploy.rs       # Upload, extract, execute logic
└── README.md           # This file
```

## Testing

Manual testing:

```bash
# Terminal 1: Start daemon
./target/debug/horus_daemon

# Terminal 2: Test with curl
cd /tmp
echo 'print("Hello from remote!")' > test.py
tar czf test.tar.gz test.py
curl -X POST http://localhost:8080/deploy \
  -H "Content-Type: application/gzip" \
  --data-binary @test.tar.gz

# Terminal 2: Test with CLI
horus run --remote localhost:8080 test.py
```

## Troubleshooting

### Port already in use

```bash
# Find and kill process on port 8080
lsof -ti:8080 | xargs kill -9

# Or use fuser
fuser -k 8080/tcp
```

### Deployment fails

```bash
# Check daemon logs
RUST_LOG=debug ./target/debug/horus_daemon

# Verify Python is available
which python3

# Verify rustc is available (for Rust deployments)
which rustc

# Verify gcc is available (for C deployments)
which gcc
```

### Connection refused

```bash
# Check daemon is running
curl http://localhost:8080/health

# Check firewall
sudo ufw allow 8080
```

## Implementation Notes

### Why HTTP instead of SSH?

- Simpler automation (no key management)
- Better for CI/CD pipelines
- Cross-platform (Windows, macOS, Linux)
- Easy debugging (curl, browser)

### Why Not a Daemon Normally?

Daemons are avoided in robotics because they add latency to real-time paths. This daemon is **safe** because:

1. **Not in message path**: Nodes communicate directly via HORUS IPC (85-167ns)
2. **Deployment only**: Only handles uploads, not runtime communication
3. **Optional**: Local execution works without daemon

### Performance

- **Upload**: ~50ms for typical Python file
- **Extraction**: ~10ms for small archives
- **Startup**: ~100ms to spawn Python process
- **Total**: ~200ms deployment time (acceptable for development)

## Development

### Building from Source

```bash
cd horus_daemon
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Dependencies

- `axum` - HTTP server framework
- `tokio` - Async runtime
- `tar` - Archive extraction
- `uuid` - Deployment ID generation
- `serde` - JSON serialization

## Security Considerations

** Warning**: This daemon is intended for development/testing only.

For production use, consider:
- Adding API key authentication
- Enabling HTTPS/TLS
- Rate limiting
- Input validation
- Sandboxing deployed code

## License

Part of the HORUS robotics framework.
