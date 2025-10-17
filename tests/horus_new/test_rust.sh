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
    echo -e "${RED}❌ FAIL - main.rs not found${NC}"
    exit 1
fi

# Test 4: Check Cargo.toml exists
echo -n "Test 4: Cargo.toml exists... "
if [ -f "$PROJECT_NAME/Cargo.toml" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Cargo.toml not found${NC}"
    exit 1
fi

# Test 5: Check horus.yaml exists
echo -n "Test 5: horus.yaml exists... "
if [ -f "$PROJECT_NAME/horus.yaml" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 6: Validate Cargo.toml is valid TOML
echo -n "Test 6: Cargo.toml is valid TOML... "
if cargo metadata --manifest-path "$PROJECT_NAME/Cargo.toml" --no-deps >/dev/null 2>&1 || \
   grep -q '^\[package\]' "$PROJECT_NAME/Cargo.toml"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Invalid Cargo.toml${NC}"
    exit 1
fi

# Test 7: Check main.rs uses Node trait (not macros)
echo -n "Test 7: main.rs uses impl Node... "
if grep -q "impl Node for" "$PROJECT_NAME/main.rs"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Should use impl Node${NC}"
    exit 1
fi

# Test 8: Check main.rs does NOT use macros
echo -n "Test 8: main.rs does not use node! macro... "
if ! grep -q "node!" "$PROJECT_NAME/main.rs"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Should not use macros${NC}"
    exit 1
fi

# Test 9: Check Cargo.toml has horus dependency
echo -n "Test 9: Cargo.toml has horus dependency... "
if grep -q '^horus = ' "$PROJECT_NAME/Cargo.toml"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Missing horus dependency${NC}"
    exit 1
fi

# Test 10: Check Cargo.toml does NOT have horus_macros
echo -n "Test 10: Cargo.toml has no horus_macros dependency... "
if ! grep -q 'horus_macros' "$PROJECT_NAME/Cargo.toml"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Should not have horus_macros${NC}"
    exit 1
fi

# Test 11: Check .horus directory
echo -n "Test 11: .horus directory structure... "
if [ -d "$PROJECT_NAME/.horus" ] && \
   [ -f "$PROJECT_NAME/.horus/env.toml" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}All Rust (no macros) tests passed!${NC}"
