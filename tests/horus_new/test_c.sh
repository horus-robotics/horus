#!/bin/bash
# Test C project creation

set -e

TEST_DIR="/tmp/horus_test_c_$$"
PROJECT_NAME="test_c"

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

echo "=== Testing C Project Creation ==="

# Test 1: Create C project
echo -n "Test 1: Create C project with -c flag... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -c 2>&1 | grep -q "Project created successfully"; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL${NC}"
    exit 1
fi

# Test 2: Check project directory exists
echo -n "Test 2: Project directory exists... "
if [ -d "$PROJECT_NAME" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL${NC}"
    exit 1
fi

# Test 3: Check main.c exists
echo -n "Test 3: main.c file exists... "
if [ -f "$PROJECT_NAME/main.c" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - main.c not found${NC}"
    exit 1
fi

# Test 4: Check horus.yaml exists
echo -n "Test 4: horus.yaml exists... "
if [ -f "$PROJECT_NAME/horus.yaml" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL${NC}"
    exit 1
fi

# Test 5: Check .horus directory exists
echo -n "Test 5: .horus directory exists... "
if [ -d "$PROJECT_NAME/.horus" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - .horus directory not created${NC}"
    exit 1
fi

# Test 6: Check no Cargo.toml exists (C project)
echo -n "Test 6: No Cargo.toml for C project... "
if [ ! -f "$PROJECT_NAME/Cargo.toml" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - Unexpected Cargo.toml${NC}"
    exit 1
fi

# Test 7: Check no main.rs exists
echo -n "Test 7: No main.rs for C project... "
if [ ! -f "$PROJECT_NAME/main.rs" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - Unexpected main.rs${NC}"
    exit 1
fi

# Test 8: Check no main.py exists
echo -n "Test 8: No main.py for C project... "
if [ ! -f "$PROJECT_NAME/main.py" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - Unexpected main.py${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}All C project tests passed!${NC}"
