#!/bin/bash
# Test Python project creation

set -e

TEST_DIR="/tmp/horus_test_python_$$"
PROJECT_NAME="test_py"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    rm -rf "$TEST_DIR"
}

trap cleanup EXIT

# Create test directory
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo "=== Testing Python Project Creation ==="

# Test 1: Create Python project
echo -n "Test 1: Create Python project with -p flag... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -p 2>&1 | grep -q "Project created successfully"; then
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
    echo -e "${RED}❌ FAIL - Directory not created${NC}"
    exit 1
fi

# Test 3: Check main.py exists
echo -n "Test 3: main.py file exists... "
if [ -f "$PROJECT_NAME/main.py" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - main.py not found${NC}"
    exit 1
fi

# Test 4: Check horus.yaml exists
echo -n "Test 4: horus.yaml exists... "
if [ -f "$PROJECT_NAME/horus.yaml" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - horus.yaml not found${NC}"
    exit 1
fi

# Test 5: Check .horus directory structure
echo -n "Test 5: .horus directory structure... "
if [ -d "$PROJECT_NAME/.horus" ] && \
   [ -d "$PROJECT_NAME/.horus/bin" ] && \
   [ -d "$PROJECT_NAME/.horus/lib" ] && \
   [ -d "$PROJECT_NAME/.horus/include" ] && \
   [ -f "$PROJECT_NAME/.horus/env.toml" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - .horus structure incomplete${NC}"
    exit 1
fi

# Test 6: Validate Python syntax
echo -n "Test 6: Python syntax is valid... "
if python3 -m py_compile "$PROJECT_NAME/main.py" 2>/dev/null; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Invalid Python syntax${NC}"
    exit 1
fi

# Test 7: Check main.py contains expected imports
echo -n "Test 7: main.py contains horus import... "
if grep -q "import horus" "$PROJECT_NAME/main.py"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Missing horus import${NC}"
    exit 1
fi

# Test 8: Check horus.yaml has correct project name
echo -n "Test 8: horus.yaml has correct name... "
if grep -q "name: $PROJECT_NAME" "$PROJECT_NAME/horus.yaml"; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Incorrect project name in horus.yaml${NC}"
    exit 1
fi

# Test 9: Check no Cargo.toml exists (Python project)
echo -n "Test 9: No Cargo.toml for Python project... "
if [ ! -f "$PROJECT_NAME/Cargo.toml" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
else
    echo -e "${RED}❌ FAIL - Unexpected Cargo.toml${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}All Python tests passed!${NC}"
