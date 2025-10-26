#!/bin/bash
# Test Rust project creation (without macros)

set -e

TEST_DIR="/tmp/horus_test_rust_$$"
PROJECT_NAME="test_rs"

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

echo "=== Testing Rust Project Creation (No Macros) ==="

# Test 1: Create Rust project
echo -n "Test 1: Create Rust project with -r flag... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -r 2>&1 | grep -q "Project created successfully"; then
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

# Test 3: Check main.rs exists
echo -n "Test 3: main.rs file exists... "
if [ -f "$PROJECT_NAME/main.rs" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - main.rs not found${NC}"
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

# Test 5: Check main.rs uses Node trait (not macros)
echo -n "Test 5: main.rs uses impl Node... "
if grep -q "impl Node for" "$PROJECT_NAME/main.rs"; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - Should use impl Node${NC}"
    exit 1
fi

# Test 6: Check main.rs does NOT use macros
echo -n "Test 6: main.rs does not use node! macro... "
if ! grep -q "node!" "$PROJECT_NAME/main.rs"; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - Should not use macros${NC}"
    exit 1
fi

# Test 7: Check .horus directory exists
echo -n "Test 7: .horus directory exists... "
if [ -d "$PROJECT_NAME/.horus" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - .horus directory not created${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}All Rust (no macros) tests passed!${NC}"
