#!/bin/bash
# Test project structure validation

set -e

TEST_DIR="/tmp/horus_test_structure_$$"
PROJECT_NAME="test_structure"

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

echo "=== Testing Project Structure ==="

# Create a Rust project for structure testing
/home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -r >/dev/null 2>&1

# Test 1: Root directory exists
echo -n "Test 1: Root project directory... "
if [ -d "$PROJECT_NAME" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL${NC}"
    exit 1
fi

# Test 2: .horus directory exists
echo -n "Test 2: .horus directory... "
if [ -d "$PROJECT_NAME/.horus" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL${NC}"
    exit 1
fi

# Test 3: horus.yaml in root
echo -n "Test 3: horus.yaml in root... "
if [ -f "$PROJECT_NAME/horus.yaml" ]; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL${NC}"
    exit 1
fi

# Test 4: horus.yaml has required fields
echo -n "Test 4: horus.yaml has required fields... "
if grep -q "name:" "$PROJECT_NAME/horus.yaml" && \
   grep -q "version:" "$PROJECT_NAME/horus.yaml" && \
   grep -q "description:" "$PROJECT_NAME/horus.yaml" && \
   grep -q "author:" "$PROJECT_NAME/horus.yaml"; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - Missing required fields${NC}"
    exit 1
fi

# Test 5: horus.yaml project name matches
echo -n "Test 5: horus.yaml project name matches... "
if grep -q "name: $PROJECT_NAME" "$PROJECT_NAME/horus.yaml"; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL - Name mismatch${NC}"
    exit 1
fi

# Test 6: main.rs has valid Rust syntax markers
echo -n "Test 6: main.rs has fn main()... "
if grep -q 'fn main()' "$PROJECT_NAME/main.rs"; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL${NC}"
    exit 1
fi

# Test 7: main.rs has use statements
echo -n "Test 7: main.rs has use horus::prelude... "
if grep -q 'use horus::prelude::' "$PROJECT_NAME/main.rs"; then
    echo -e "${GREEN} PASS${NC}"
else
    echo -e "${RED} FAIL${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}All structure tests passed!${NC}"
