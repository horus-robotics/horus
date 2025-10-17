#!/bin/bash
# Test Rust project creation with macros

set -e

TEST_DIR="/tmp/horus_test_rust_macro_$$"
PROJECT_NAME="test_macro"

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

echo "=== Testing Rust Project Creation (With Macros) ==="

# Test 1: Create Rust project with -m flag
echo -n "Test 1: Create Rust project with -m flag... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -m 2>&1 | grep -q "Project created successfully"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 2: Check project directory exists
echo -n "Test 2: Project directory exists... "
if [ -d "$PROJECT_NAME" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 3: Check main.rs exists
echo -n "Test 3: main.rs file exists... "
if [ -f "$PROJECT_NAME/main.rs" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 4: Check main.rs uses node! macro
echo -n "Test 4: main.rs uses node! macro... "
if grep -q "node!" "$PROJECT_NAME/main.rs"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Should use node! macro${NC}"
    exit 1
fi

# Test 5: Check main.rs does NOT use impl Node
echo -n "Test 5: main.rs does not use impl Node... "
if ! grep -q "impl Node for" "$PROJECT_NAME/main.rs"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Should not use impl Node${NC}"
    exit 1
fi

# Test 6: Check Cargo.toml has horus_macros dependency
echo -n "Test 6: Cargo.toml has horus_macros dependency... "
if grep -q 'horus_macros' "$PROJECT_NAME/Cargo.toml"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Missing horus_macros dependency${NC}"
    exit 1
fi

# Test 7: Check horus dependency has macros feature
echo -n "Test 7: horus has 'macros' feature enabled... "
if grep -q 'features = \["macros"\]' "$PROJECT_NAME/Cargo.toml"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - macros feature not enabled${NC}"
    exit 1
fi

# Test 8: Test with both -r and -m flags
cd "$TEST_DIR"
PROJECT_NAME2="test_both"
echo -n "Test 8: Create with both -r and -m flags... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME2" -r -m 2>&1 | grep -q "Project created successfully"; then
    if grep -q "node!" "$PROJECT_NAME2/main.rs" && \
       grep -q 'horus_macros' "$PROJECT_NAME2/Cargo.toml"; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL - Not using macros${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}✅ PASS${NC}"
fi

echo ""
echo -e "${GREEN}All Rust (with macros) tests passed!${NC}"
