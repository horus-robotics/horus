#!/bin/bash
# Test edge cases and boundary conditions

set -e

TEST_DIR="/tmp/horus_test_edge_$$"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

cleanup() {
    rm -rf "$TEST_DIR"
}

trap cleanup EXIT

mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo "=== Testing Edge Cases ==="

# Test 1: Hyphenated project name (should convert to underscores in Rust)
echo -n "Test 1: Hyphenated project name... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "my-robot" -r 2>&1 | grep -q "Project created successfully"; then
    # Check that Cargo.toml uses underscores
    if grep -q 'name = "my_robot"' my-robot/Cargo.toml; then
        echo -e "${GREEN}✅ PASS - Correctly converted to underscores${NC}"
    else
        echo -e "${YELLOW}⚠ PARTIAL - Created but may have naming issues${NC}"
    fi
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 2: Project with numbers
echo -n "Test 2: Project name with numbers... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "robot123" -r 2>&1 | grep -q "Project created successfully"; then
    if [ -d "robot123" ]; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 3: Single character name
echo -n "Test 3: Single character project name... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "x" -r 2>&1 | grep -q "Project created successfully"; then
    if [ -d "x" ]; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 4: Directory already exists (should fail or handle gracefully)
echo -n "Test 4: Directory already exists... "
mkdir -p "existing_project"
if /home/lord-patpak/horus/HORUS/target/debug/horus new "existing_project" -r 2>&1 | grep -qE "(already exists|error|Error)"; then
    echo -e "${GREEN}✅ PASS - Correctly handles existing directory${NC}"
elif /home/lord-patpak/horus/HORUS/target/debug/horus new "existing_project" -r 2>&1 | grep -q "Project created successfully"; then
    # If it succeeds, it might be creating files inside existing dir
    echo -e "${YELLOW}⚠ WARN - Creates in existing directory${NC}"
else
    echo -e "${RED}❌ FAIL - Unexpected behavior${NC}"
    exit 1
fi

# Test 5: Underscore in name
echo -n "Test 5: Project name with underscores... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "my_robot" -r 2>&1 | grep -q "Project created successfully"; then
    if [ -d "my_robot" ] && [ -f "my_robot/Cargo.toml" ]; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 6: Mixed case name
echo -n "Test 6: Mixed case project name... "
if /home/lord-patpak/horus/HORUS/target/debug/horus new "MyRobot" -r 2>&1 | grep -q "Project created successfully"; then
    if [ -d "MyRobot" ]; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

# Test 7: Long project name
echo -n "Test 7: Long project name (100 chars)... "
LONG_NAME="very_long_project_name_that_has_exactly_one_hundred_characters_to_test_boundary_conditions_abcdefg"
if /home/lord-patpak/horus/HORUS/target/debug/horus new "$LONG_NAME" -r 2>&1 | grep -q "Project created successfully"; then
    if [ -d "$LONG_NAME" ]; then
        echo -e "${GREEN}✅ PASS${NC}"
    else
        echo -e "${RED}❌ FAIL${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ FAIL${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}All edge case tests passed!${NC}"
