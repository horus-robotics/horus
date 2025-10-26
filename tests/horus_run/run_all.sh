#!/bin/bash
# Run all horus run command tests

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Change to test directory
cd "$(dirname "$0")"

echo -e "${BLUE}${NC}"
echo -e "${BLUE}   HORUS Run Command Test Suite           ${NC}"
echo -e "${BLUE}${NC}"
echo ""

# Check if horus binary exists
if [ ! -f "/home/lord-patpak/horus/HORUS/target/debug/horus" ]; then
    echo -e "${RED} Error: horus binary not found${NC}"
    echo "Please build horus first:"
    echo "  cd /home/lord-patpak/horus/HORUS"
    echo "  cargo build"
    exit 1
fi

# Test counters
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

# Make all test scripts executable
chmod +x *.sh 2>/dev/null

# Function to run a test suite
run_test() {
    local test_file=$1
    local test_name=$2

    if [ ! -f "$test_file" ]; then
        echo -e "${YELLOW}⊘ SKIP: $test_name (file not found)${NC}"
        ((SKIPPED++))
        return
    fi

    echo -e "${BLUE} Running: $test_name ${NC}"
    ((TOTAL++))

    if bash "$test_file"; then
        echo -e "${GREEN} PASSED: $test_name${NC}"
        ((PASSED++))
    else
        echo -e "${RED} FAILED: $test_name${NC}"
        ((FAILED++))
    fi
    echo ""
}

# Run all test suites in logical order
echo -e "${BLUE}=== Basic Execution Tests ===${NC}"
run_test "test_python_exec.sh" "Python Execution"
run_test "test_rust_exec.sh" "Rust Execution"
run_test "test_c_exec.sh" "C Execution"

echo -e "${BLUE}=== Auto-Detection and Build Tests ===${NC}"
run_test "test_autodetect.sh" "Auto-Detection"
run_test "test_build_modes.sh" "Build Modes (Debug/Release/Clean)"

echo -e "${BLUE}=== Dependency and IPC Tests ===${NC}"
run_test "test_dependencies.sh" "Dependency Resolution"
run_test "test_ipc.sh" "IPC and Robotics Applications"

# Print summary
echo -e "${BLUE}${NC}"
echo -e "${BLUE}              Test Summary                 ${NC}"
echo -e "${BLUE}${NC}"
echo -e "${BLUE}${NC}  Total test suites:  $TOTAL"
echo -e "${BLUE}${NC}  ${GREEN}Passed:${NC}            $PASSED"
echo -e "${BLUE}${NC}  ${RED}Failed:${NC}            $FAILED"
if [ $SKIPPED -gt 0 ]; then
    echo -e "${BLUE}${NC}  ${YELLOW}Skipped:${NC}           $SKIPPED"
fi
echo -e "${BLUE}${NC}"
echo ""

# Exit code
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN} ALL TESTS PASSED - horus run is production-ready!${NC}"
    echo ""
    echo "Test coverage:"
    echo "   Language execution (Python, Rust, C)"
    echo "   Auto-detection of main files"
    echo "   Build modes and caching"
    echo "   Dependency resolution"
    echo "   IPC and robotics patterns"
    echo ""
    exit 0
else
    echo -e "${RED} SOME TESTS FAILED - Please review and fix${NC}"
    echo ""
    echo "To debug individual test suites:"
    echo "  bash -x ./test_python_exec.sh"
    echo "  bash -x ./test_rust_exec.sh"
    echo "  bash -x ./test_c_exec.sh"
    echo "  bash -x ./test_autodetect.sh"
    echo "  bash -x ./test_build_modes.sh"
    echo "  bash -x ./test_dependencies.sh"
    echo "  bash -x ./test_ipc.sh"
    echo ""
    exit 1
fi
