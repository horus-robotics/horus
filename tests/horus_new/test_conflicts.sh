#!/bin/bash
# Test flag conflicts

set -e

TEST_DIR="/tmp/horus_test_conflicts_$$"
PROJECT_NAME="test_conflict"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

cleanup() {
    rm -rf "$TEST_DIR"
}

trap cleanup EXIT

mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo "=== Testing Flag Conflicts ==="

# Test 1: Python + Rust conflict
echo -n "Test 1: Python + Rust flags conflict... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -p -r 2>&1 | grep -qE "(conflict|cannot be used with|error)"; then
    echo -e "${GREEN}✅ PASS - Correctly rejected${NC}"
else
    echo -e "${RED}❌ FAIL - Should reject conflicting flags${NC}"
    exit 1
fi

# Test 2: Python + C conflict
echo -n "Test 2: Python + C flags conflict... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -p -c 2>&1 | grep -qE "(conflict|cannot be used with|error)"; then
    echo -e "${GREEN}✅ PASS - Correctly rejected${NC}"
else
    echo -e "${RED}❌ FAIL - Should reject conflicting flags${NC}"
    exit 1
fi

# Test 3: Rust + C conflict
echo -n "Test 3: Rust + C flags conflict... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -r -c 2>&1 | grep -qE "(conflict|cannot be used with|error)"; then
    echo -e "${GREEN}✅ PASS - Correctly rejected${NC}"
else
    echo -e "${RED}❌ FAIL - Should reject conflicting flags${NC}"
    exit 1
fi

# Test 4: All three languages conflict
echo -n "Test 4: All three language flags conflict... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -p -r -c 2>&1 | grep -qE "(conflict|cannot be used with|error)"; then
    echo -e "${GREEN}✅ PASS - Correctly rejected${NC}"
else
    echo -e "${RED}❌ FAIL - Should reject conflicting flags${NC}"
    exit 1
fi

# Test 5: Macro + Python conflict
echo -n "Test 5: Macro + Python flags conflict... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -p -m 2>&1 | grep -qE "(conflict|cannot be used with|error)"; then
    echo -e "${GREEN}✅ PASS - Correctly rejected${NC}"
else
    echo -e "${RED}❌ FAIL - Should reject conflicting flags${NC}"
    exit 1
fi

# Test 6: Macro + C conflict
echo -n "Test 6: Macro + C flags conflict... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -c -m 2>&1 | grep -qE "(conflict|cannot be used with|error)"; then
    echo -e "${GREEN}✅ PASS - Correctly rejected${NC}"
else
    echo -e "${RED}❌ FAIL - Should reject conflicting flags${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}All conflict tests passed!${NC}"
