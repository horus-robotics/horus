#!/bin/bash
# Complete benchmark suite runner for HORUS

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$BENCH_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Cross-platform shared memory path detection
get_shm_base() {
    case "$(uname -s)" in
        Linux*)  echo "/dev/shm" ;;
        Darwin*) echo "/tmp" ;;
        *)       echo "/tmp" ;;
    esac
}

echo -e "${BLUE} HORUS Complete Benchmark Suite${NC}"
echo "======================================"
echo ""

# Check prerequisites
echo -e "${YELLOW} Checking prerequisites...${NC}"

if ! command -v cargo &> /dev/null; then
    echo -e "${RED} Cargo not found. Please install Rust.${NC}"
    exit 1
fi

if ! command -v python3 &> /dev/null; then
    echo -e "${RED} Python3 not found. Please install Python 3.8+${NC}"
    exit 1
fi

# Check if running with sufficient privileges for CPU config
if [ "$EUID" -ne 0 ]; then
    echo -e "${YELLOW}  Not running as root. Some optimizations will be skipped.${NC}"
    echo "   For best results, run: sudo $0"
else
    # Configure CPU for benchmarks
    echo -e "${YELLOW} Configuring CPU for benchmarks...${NC}"

    # Set performance governor
    if command -v cpupower &> /dev/null; then
        cpupower frequency-set -g performance 2>/dev/null || true
    fi

    # Disable turbo boost
    if [ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]; then
        echo 1 > /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || true
    fi

    echo -e "${GREEN} CPU configured for consistent results${NC}"
fi

# Create results directory
mkdir -p "$RESULTS_DIR"

# Save system info
echo -e "${YELLOW} Recording system information...${NC}"
cat > "$RESULTS_DIR/system_info_$TIMESTAMP.json" << EOF
{
  "timestamp": "$(date -Iseconds)",
  "hostname": "$(hostname)",
  "kernel": "$(uname -r)",
  "cpu": "$(lscpu | grep 'Model name' | cut -d: -f2 | xargs)",
  "cores": $(nproc),
  "memory": "$(free -h | grep Mem | awk '{print $2}')",
  "rust_version": "$(rustc --version)",
  "cargo_version": "$(cargo --version)"
}
EOF

# Build benchmarks
echo -e "${YELLOW}[BUILD] Building benchmarks...${NC}"
cd "$BENCH_DIR"
cargo build --release --benches

# Clear any existing shared memory (cross-platform)
echo -e "${YELLOW}[CLEAN] Cleaning shared memory...${NC}"
SHM_BASE="$(get_shm_base)"
rm -rf "$SHM_BASE/horus_"* "$SHM_BASE/horus" 2>/dev/null || true

# Run benchmarks
echo -e "${BLUE}[RUN] Running benchmark suite...${NC}"
echo ""

# Function to run a single benchmark
run_benchmark() {
    local name=$1
    echo -e "${YELLOW}  Running $name...${NC}"

    if cargo bench --bench "$name" -- --save-baseline "$TIMESTAMP" > "$RESULTS_DIR/${name}_$TIMESTAMP.log" 2>&1; then
        echo -e "${GREEN}     $name completed${NC}"
        return 0
    else
        echo -e "${RED}     $name failed (see logs)${NC}"
        return 1
    fi
}

# Run each benchmark
FAILED=0

run_benchmark "latency" || ((FAILED++))
run_benchmark "throughput" || ((FAILED++))
run_benchmark "comparison" || ((FAILED++))

echo ""

# Check for regressions if baseline exists
if [ -d "target/criterion" ]; then
    echo -e "${YELLOW} Checking for regressions...${NC}"

    if python3 scripts/check_regression.py target/criterion; then
        echo -e "${GREEN} No regressions detected${NC}"
    else
        echo -e "${YELLOW}  Potential regressions found (see report)${NC}"
    fi
fi

# Generate dashboard
echo -e "${YELLOW} Generating visual dashboard...${NC}"

if python3 scripts/generate_dashboard.py; then
    echo -e "${GREEN} Dashboard generated at benchmark-dashboard/index.html${NC}"
else
    echo -e "${RED} Dashboard generation failed${NC}"
fi

# Summary
echo ""
echo "======================================"
echo -e "${BLUE} Benchmark Summary${NC}"
echo "======================================"

if [ -f "target/criterion/reports/index.html" ]; then
    echo -e "${GREEN} Criterion reports available at:${NC}"
    echo "   file://$BENCH_DIR/target/criterion/reports/index.html"
fi

if [ -f "benchmark-dashboard/index.html" ]; then
    echo -e "${GREEN} Visual dashboard available at:${NC}"
    echo "   file://$BENCH_DIR/benchmark-dashboard/index.html"
fi

echo -e "${GREEN} Raw results saved to:${NC}"
echo "   $RESULTS_DIR"

if [ $FAILED -gt 0 ]; then
    echo ""
    echo -e "${RED}  $FAILED benchmark(s) failed${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN} All benchmarks completed successfully!${NC}"

# Open dashboard if possible
if command -v xdg-open &> /dev/null && [ -f "benchmark-dashboard/index.html" ]; then
    echo ""
    read -p "Open dashboard in browser? (y/n) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        xdg-open "benchmark-dashboard/index.html"
    fi
fi