#!/bin/bash
# HORUS Network Transport Benchmark Script
# Compares latency across different transport backends

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HORUS_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║          HORUS Network Transport Benchmark Suite                 ║"
echo "╚══════════════════════════════════════════════════════════════════╝"
echo ""

# Check system capabilities
echo "System Information:"
echo "  Kernel: $(uname -r)"
echo "  CPU: $(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2 | xargs || echo 'Unknown')"
echo ""

# Check for io_uring support
KERNEL_MAJOR=$(uname -r | cut -d. -f1)
KERNEL_MINOR=$(uname -r | cut -d. -f2)

echo "Low-Latency Feature Detection:"

if [ "$KERNEL_MAJOR" -ge 5 ] && [ "$KERNEL_MINOR" -ge 1 ] || [ "$KERNEL_MAJOR" -ge 6 ]; then
    echo "  ✓ io_uring: Supported (kernel >= 5.1)"
else
    echo "  ✗ io_uring: Not supported (requires kernel 5.1+)"
fi

echo "  ✓ Batch UDP (sendmmsg/recvmmsg): Available on all Linux"
echo ""

# Run benchmarks
echo "Running Network Transport Benchmarks..."
echo "(This may take a few minutes)"
echo ""

cd "$HORUS_ROOT"

# Build in release mode first
echo "Building benchmarks in release mode..."
cargo build --release --package horus_benchmarks 2>/dev/null

# Run the transport comparison benchmark
echo ""
echo "=== Transport Comparison (Shared Memory vs UDP) ==="
cargo bench --bench network_transport -- "transport_comparison" --noplot 2>&1 | grep -E "(shared_memory|udp_loopback|time:)"

# Run io_uring checks
echo ""
echo "=== io_uring Backend Status ==="
cargo bench --bench network_transport -- "io_uring_availability" --noplot 2>&1 | grep -E "(io_uring|Available|Expected)"

echo ""
echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║          Expected Latency Comparison                             ║"
echo "╠══════════════════════════════════════════════════════════════════╣"
echo "║ Transport           │ Latency    │ Notes                         ║"
echo "╠══════════════════════════════════════════════════════════════════╣"
echo "║ Shared Memory       │ ~250ns     │ Fastest, local only           ║"
echo "║ io_uring            │ 2-3µs      │ Async I/O, Linux 5.1+         ║"
echo "║ Batch UDP           │ 3-5µs      │ sendmmsg, Linux               ║"
echo "║ Standard UDP        │ 5-10µs     │ Cross-platform                ║"
echo "╚══════════════════════════════════════════════════════════════════╝"
echo ""
echo "For detailed benchmarks, run:"
echo "  cargo bench --bench network_transport"
