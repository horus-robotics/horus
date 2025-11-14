#!/bin/bash
# HORUS Production Qualification Test Suite
# Run after structural changes to horus_core

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results
PASSED=0
FAILED=0
SKIPPED=0

# Function to print test header
print_header() {
    echo -e "\n${BLUE}================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}================================${NC}\n"
}

# Function to run a test
run_test() {
    local test_name=$1
    local test_cmd=$2
    local timeout=${3:-60}  # Default 60s timeout

    echo -e "${YELLOW}Running: $test_name${NC}"

    if timeout $timeout bash -c "$test_cmd" 2>&1; then
        echo -e "${GREEN} PASSED${NC}: $test_name\n"
        ((PASSED++))
        return 0
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            echo -e "${RED}[TIMEOUT]${NC}: $test_name (exceeded ${timeout}s)\n"
        else
            echo -e "${RED}[FAIL]${NC}: $test_name (exit code: $exit_code)\n"
        fi
        ((FAILED++))
        return 1
    fi
}

# Function to print summary
print_summary() {
    echo -e "\n${BLUE}================================${NC}"
    echo -e "${BLUE}Test Summary${NC}"
    echo -e "${BLUE}================================${NC}"
    echo -e "${GREEN}Passed: $PASSED${NC}"
    echo -e "${RED}Failed: $FAILED${NC}"
    echo -e "${YELLOW}Skipped: $SKIPPED${NC}"
    echo -e "${BLUE}Total: $((PASSED + FAILED + SKIPPED))${NC}\n"

    if [ $FAILED -eq 0 ]; then
        echo -e "${GREEN}All tests passed! ${NC}"
        echo -e "${GREEN}Changes are qualified for production.${NC}\n"
        return 0
    else
        echo -e "${RED}Some tests failed! [FAIL]${NC}"
        echo -e "${RED}Review failures before merging changes.${NC}\n"
        return 1
    fi
}

# Parse command line arguments
CATEGORY="${1:-all}"

# Build all test binaries first
print_header "Building Test Binaries"
cd /home/lord-patpak/horus/HORUS
echo "Building in release mode for accurate testing..."
if ! cargo build --release --bins 2>&1 | grep -E "(Compiling|Finished|error)"; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi
echo -e "${GREEN}Build completed${NC}\n"

# ============================================================================
# 1. API Compatibility Tests
# ============================================================================
if [ "$CATEGORY" = "all" ] || [ "$CATEGORY" = "api" ]; then
    print_header "API Compatibility Tests"

    run_test "Node Creation" \
        "./target/release/test_api_compatibility node_creation"

    run_test "Hub Pub/Sub" \
        "./target/release/test_api_compatibility hub_pubsub"

    run_test "Link Send/Recv" \
        "./target/release/test_api_compatibility link_sendrecv"

    run_test "Scheduler Execution" \
        "./target/release/test_api_compatibility scheduler"

    run_test "Message Types" \
        "./target/release/test_api_compatibility message_types"
fi

# ============================================================================
# 2. IPC Functionality Tests
# ============================================================================
if [ "$CATEGORY" = "all" ] || [ "$CATEGORY" = "ipc" ]; then
    print_header "IPC Functionality Tests"

    run_test "Hub Multi-Process" \
        "./target/release/test_ipc_functionality hub_multiprocess" \
        30

    run_test "Link Single-Process" \
        "./target/release/test_ipc_functionality link_singleprocess"

    run_test "Cross-Process Messaging" \
        "./target/release/test_ipc_functionality cross_process" \
        30

    run_test "Large Messages (1MB)" \
        "./target/release/test_ipc_functionality large_messages"

    run_test "High Frequency (10kHz)" \
        "./target/release/test_ipc_functionality high_frequency" \
        30
fi

# ============================================================================
# 3. Memory Safety Tests
# ============================================================================
if [ "$CATEGORY" = "all" ] || [ "$CATEGORY" = "safety" ]; then
    print_header "Memory Safety Tests"

    run_test "Shared Memory Lifecycle" \
        "./target/release/test_memory_safety shm_lifecycle"

    run_test "Bounds Checking" \
        "./target/release/test_memory_safety bounds_checking"

    run_test "Concurrent Access" \
        "./target/release/test_memory_safety concurrent_access" \
        30

    run_test "Memory Leak Detection" \
        "./target/release/test_memory_safety leak_detection" \
        60

    run_test "Buffer Overflow Protection" \
        "./target/release/test_memory_safety overflow_protection"
fi

# ============================================================================
# 4. Robotics Usage Tests
# ============================================================================
if [ "$CATEGORY" = "all" ] || [ "$CATEGORY" = "robotics" ]; then
    print_header "Robotics Usage Tests"

    run_test "Sensor Data Flow" \
        "./target/release/test_robotics_usage sensor_flow" \
        30

    run_test "Actuator Commands" \
        "./target/release/test_robotics_usage actuator_commands"

    run_test "Control Loop (1kHz)" \
        "./target/release/test_robotics_usage control_loop_1khz" \
        30

    run_test "Transform Broadcasting" \
        "./target/release/test_robotics_usage transforms"

    run_test "State Machine" \
        "./target/release/test_robotics_usage state_machine"
fi

# ============================================================================
# 5. Logging Tests
# ============================================================================
if [ "$CATEGORY" = "all" ] || [ "$CATEGORY" = "logging" ]; then
    print_header "Logging Tests"

    run_test "Log Levels" \
        "./target/release/test_logging log_levels"

    run_test "Message Tracing" \
        "./target/release/test_logging message_tracing"

    run_test "Performance with Logging" \
        "./target/release/test_logging performance" \
        30

    run_test "Context Propagation" \
        "./target/release/test_logging context"
fi

# ============================================================================
# 6. Stress Tests (Optional - long running)
# ============================================================================
if [ "$CATEGORY" = "stress" ]; then
    print_header "Stress Tests (Long Running)"

    run_test "1000+ Topics" \
        "./target/release/test_stress many_topics" \
        120

    run_test "100+ Concurrent Nodes" \
        "./target/release/test_stress many_nodes" \
        120

    run_test "Sustained High Frequency" \
        "./target/release/test_stress sustained_high_freq" \
        300

    run_test "Memory Pressure" \
        "./target/release/test_stress memory_pressure" \
        120

    run_test "Long Running Stability (30min)" \
        "./target/release/test_stress long_running" \
        1800
fi

# Print final summary
print_summary
