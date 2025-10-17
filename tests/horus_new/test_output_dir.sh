#!/bin/bash
# Test custom output directory

set -e

TEST_BASE="/tmp/horus_test_output_$$"
PROJECT_NAME="test_output"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

cleanup() {
    rm -rf "$TEST_BASE"
}

trap cleanup EXIT

echo "=== Testing Custom Output Directory ==="

# Test 1: Custom absolute path
TEST_DIR="$TEST_BASE/custom_dir"
mkdir -p "$TEST_BASE"
echo -n "Test 1: Create project in custom absolute path... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -r -o "$TEST_DIR" 2>&1 | grep -q "Project created successfully"; then
    if [ -d "$TEST_DIR/$PROJECT_NAME" ] && [ -f "$TEST_DIR/$PROJECT_NAME/main.rs" ]; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL - Project not created in correct location${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ FAIL - Project creation failed${NC}"
    exit 1
fi

# Test 2: Nested directory creation
TEST_DIR2="$TEST_BASE/a/b/c"
echo -n "Test 2: Create nested directory structure... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -r -o "$TEST_DIR2" 2>&1 | grep -q "Project created successfully"; then
    if [ -d "$TEST_DIR2/$PROJECT_NAME" ]; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL - Nested directory not created${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 3: Relative path
cd "$TEST_BASE"
mkdir -p relative_test
cd relative_test
echo -n "Test 3: Create project with relative path... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -r -o ./subdir 2>&1 | grep -q "Project created successfully"; then
    if [ -d "./subdir/$PROJECT_NAME" ]; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL - Relative path failed${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 4: Parent directory path
cd "$TEST_BASE"
mkdir -p parent_test/child
cd parent_test/child
echo -n "Test 4: Create project in parent directory... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$PROJECT_NAME" -r -o .. 2>&1 | grep -q "Project created successfully"; then
    if [ -d "../$PROJECT_NAME" ]; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL - Parent directory path failed${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}All output directory tests passed!${NC}"
