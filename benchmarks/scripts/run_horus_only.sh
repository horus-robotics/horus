#!/bin/bash
# Quick HORUS-only benchmark runner

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$BENCH_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE} HORUS Quick Benchmark Suite${NC}"
echo "=============================="
echo ""

# Create results directory
mkdir -p "$RESULTS_DIR"

# Build
echo -e "${YELLOW}Building benchmarks...${NC}"
cd "$BENCH_DIR"
cargo build --release --benches

# Clear shared memory (cross-platform)
echo -e "${YELLOW}Cleaning shared memory...${NC}"
case "$(uname -s)" in
    Linux*)  SHM_BASE="/dev/shm" ;;
    Darwin*) SHM_BASE="/tmp" ;;
    *)       SHM_BASE="/tmp" ;;
esac
rm -rf "$SHM_BASE/horus_"* "$SHM_BASE/horus" 2>/dev/null || true

echo ""
echo -e "${BLUE}Running benchmarks...${NC}"

# Run latency benchmark
echo -e "${YELLOW}1. Latency benchmarks${NC}"
cargo bench --bench latency -- \
    --warm-up-time 2 \
    --measurement-time 3 \
    --save-baseline "$TIMESTAMP" 2>&1 | tee "$RESULTS_DIR/latency_$TIMESTAMP.log"

echo ""

# Run throughput benchmark
echo -e "${YELLOW}2. Throughput benchmarks${NC}"
cargo bench --bench throughput -- \
    --warm-up-time 2 \
    --measurement-time 3 \
    --save-baseline "$TIMESTAMP" 2>&1 | tee "$RESULTS_DIR/throughput_$TIMESTAMP.log"

echo ""
echo -e "${GREEN} Benchmarks completed!${NC}"
echo ""

# Summary
echo -e "${BLUE}Results Summary:${NC}"
echo "================"

# Extract key metrics
echo ""
echo "Latency (Hub):"
grep -E "hub_latency.*time:" "$RESULTS_DIR/latency_$TIMESTAMP.log" | head -5

echo ""
echo "Throughput (Hub):"
grep -E "throughput.*time:" "$RESULTS_DIR/throughput_$TIMESTAMP.log" | head -5

echo ""
echo -e "${GREEN}Full results saved to: $RESULTS_DIR${NC}"
echo -e "${GREEN}Criterion reports: file://$BENCH_DIR/target/criterion/reports/index.html${NC}"