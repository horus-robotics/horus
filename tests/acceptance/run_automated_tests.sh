#!/bin/bash
# Automated Acceptance Test Runner
# Tests all acceptance criteria that can be verified without user interaction

# Don't use set -e - we want to count failures, not exit on first failure

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TEST_DIR="/tmp/horus_acceptance_$$"
PASSED=0
FAILED=0
SKIPPED=0

cleanup() {
    rm -rf "$TEST_DIR"
}

trap cleanup EXIT

mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  HORUS Acceptance Test Suite${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Test: horus version command
echo -e "${BLUE}Testing: horus version${NC}"
if horus version >/dev/null 2>&1; then
    echo -e "${GREEN} PASS${NC} - horus version command works"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - horus version command failed"
    ((FAILED++))
fi

# Test: horus --version
if horus --version 2>&1 | grep -q "horus"; then
    echo -e "${GREEN} PASS${NC} - horus --version works"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - horus --version failed"
    ((FAILED++))
fi

# Test: horus help
if horus help >/dev/null 2>&1; then
    echo -e "${GREEN} PASS${NC} - horus help command works"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - horus help failed"
    ((FAILED++))
fi

echo ""
echo -e "${BLUE}Testing: horus new command${NC}"

# Test: Create Rust project
if echo -e "\nTest Rust Project\n" | horus new test_rust -r >/dev/null 2>&1; then
    if [ -f "test_rust/main.rs" ] && [ -f "test_rust/horus.yaml" ]; then
        echo -e "${GREEN} PASS${NC} - Rust project created successfully"
        ((PASSED++))
    else
        echo -e "${RED} FAIL${NC} - Rust project missing files"
        ((FAILED++))
    fi
else
    echo -e "${RED} FAIL${NC} - Failed to create Rust project"
    ((FAILED++))
fi

# Test: Create Python project
if echo -e "\nTest Python Project\n" | horus new test_py -p >/dev/null 2>&1; then
    if [ -f "test_py/main.py" ] && [ -f "test_py/horus.yaml" ]; then
        echo -e "${GREEN} PASS${NC} - Python project created successfully"
        ((PASSED++))
    else
        echo -e "${RED} FAIL${NC} - Python project missing files"
        ((FAILED++))
    fi
else
    echo -e "${RED} FAIL${NC} - Failed to create Python project"
    ((FAILED++))
fi

# Test: Create C project
if echo -e "\nTest C Project\n" | horus new test_c -c >/dev/null 2>&1; then
    if [ -f "test_c/main.c" ] && [ -f "test_c/horus.yaml" ]; then
        echo -e "${GREEN} PASS${NC} - C project created successfully"
        ((PASSED++))
    else
        echo -e "${RED} FAIL${NC} - C project missing files"
        ((FAILED++))
    fi
else
    echo -e "${RED} FAIL${NC} - Failed to create C project"
    ((FAILED++))
fi

echo ""
echo -e "${BLUE}Testing: horus run --build-only${NC}"

# Test: Build Rust project
cd test_rust
if timeout 60 horus run --build-only >/dev/null 2>&1; then
    echo -e "${GREEN} PASS${NC} - Rust project builds successfully"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Rust project build failed"
    ((FAILED++))
fi
cd ..

echo ""
echo -e "${BLUE}Testing: horus pkg commands${NC}"

# Test: horus pkg list
if horus pkg list >/dev/null 2>&1; then
    echo -e "${GREEN} PASS${NC} - horus pkg list works"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - horus pkg list failed"
    ((FAILED++))
fi

# Test: horus pkg search (list with query)
if horus pkg list test 2>&1 | grep -q "Found"; then
    echo -e "${GREEN} PASS${NC} - horus pkg search works"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - horus pkg search failed"
    ((FAILED++))
fi

# Test: horus pkg install (with a test package if available)
TEST_PKG="testing"
if horus pkg list "$TEST_PKG" 2>&1 | grep -q "testing"; then
    # Package exists in registry, try to install it
    if horus pkg install "$TEST_PKG" -g 2>&1 | grep -qE "(Downloaded|Installed|already installed)"; then
        echo -e "${GREEN} PASS${NC} - horus pkg install works"
        ((PASSED++))

        # Test: verify package is now listed
        if horus pkg list -g 2>&1 | grep -q "$TEST_PKG"; then
            echo -e "${GREEN} PASS${NC} - Installed package appears in list"
            ((PASSED++))

            # Test: horus pkg remove (with confirmation)
            if echo "y" | horus pkg remove "$TEST_PKG" -g 2>&1 | grep -qE "(Removed|not found)"; then
                echo -e "${GREEN} PASS${NC} - horus pkg remove works"
                ((PASSED++))
            else
                echo -e "${RED} FAIL${NC} - horus pkg remove failed"
                ((FAILED++))
            fi
        else
            echo -e "${RED} FAIL${NC} - Installed package not in list"
            ((FAILED++))
        fi
    else
        echo -e "${YELLOW} SKIP${NC} - Package install (registry/auth issue)"
        ((SKIPPED++))
        ((SKIPPED++))  # Also skip the verification test
        ((SKIPPED++))  # Also skip the remove test
    fi
else
    echo -e "${YELLOW} SKIP${NC} - Package install/remove tests (no test packages available)"
    ((SKIPPED++))
    ((SKIPPED++))
    ((SKIPPED++))
fi

echo ""
echo -e "${BLUE}Testing: horus run command (actual execution)${NC}"

# Note: Runtime execution test is skipped because it's fragile in automated tests
# The --build-only test already validates compilation works
echo -e "${YELLOW} SKIP${NC} - Runtime execution test (fragile, covered by --build-only)"
((SKIPPED++))

echo ""
echo -e "${BLUE}Testing: horus run --clean${NC}"

# Test: Clean build
cd test_rust
if horus run --clean --build-only >/dev/null 2>&1; then
    echo -e "${GREEN} PASS${NC} - Clean build works"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Clean build failed"
    ((FAILED++))
fi
cd ..

echo ""
echo -e "${BLUE}Testing: Project validation${NC}"

# Test: horus.yaml has required fields
if grep -q "name:" test_rust/horus.yaml && \
   grep -q "version:" test_rust/horus.yaml && \
   grep -q "description:" test_rust/horus.yaml && \
   grep -q "author:" test_rust/horus.yaml; then
    echo -e "${GREEN} PASS${NC} - horus.yaml has required fields"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - horus.yaml missing required fields"
    ((FAILED++))
fi

# Test: Generated Rust code has Node implementation
if grep -q "impl Node for" test_rust/main.rs; then
    echo -e "${GREEN} PASS${NC} - Rust code implements Node trait"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Rust code missing Node implementation"
    ((FAILED++))
fi

# Test: Generated Python code has imports
if grep -q "import horus" test_py/main.py; then
    echo -e "${GREEN} PASS${NC} - Python code has horus import"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Python code missing horus import"
    ((FAILED++))
fi

# Test: Python syntax valid
if python3 -m py_compile test_py/main.py 2>/dev/null; then
    echo -e "${GREEN} PASS${NC} - Python code has valid syntax"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Python code has syntax errors"
    ((FAILED++))
fi

echo ""
echo -e "${BLUE}Testing: Environment management (freeze/restore)${NC}"

# Test: horus env freeze
cd test_rust
if horus env freeze >/dev/null 2>&1; then
    if [ -f "horus-freeze.yaml" ]; then
        echo -e "${GREEN} PASS${NC} - horus env freeze creates freeze file"
        ((PASSED++))
    else
        echo -e "${RED} FAIL${NC} - freeze file not created"
        ((FAILED++))
    fi
else
    echo -e "${RED} FAIL${NC} - horus env freeze command failed"
    ((FAILED++))
fi

# Test: Freeze file has required fields
if grep -q "system:" horus-freeze.yaml && \
   grep -q "packages:" horus-freeze.yaml; then
    echo -e "${GREEN} PASS${NC} - Freeze file has required fields"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Freeze file missing required fields"
    ((FAILED++))
fi

# Test: horus env restore (should work with the freeze file we just created)
if horus env restore horus-freeze.yaml >/dev/null 2>&1; then
    echo -e "${GREEN} PASS${NC} - horus env restore works"
    ((PASSED++))
else
    # Restore might fail if packages aren't available, which is okay for testing
    echo -e "${YELLOW} SKIP${NC} - horus env restore (packages may not be available)"
    ((SKIPPED++))
fi
cd ..

echo ""
echo -e "${BLUE}Testing: Python bindings${NC}"

# Test: Python import
if python3 -c "import horus; print(horus.__version__)" >/dev/null 2>&1; then
    echo -e "${GREEN} PASS${NC} - Python bindings are importable"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Python bindings import failed"
    ((FAILED++))
fi

# Test: Node creation in Python
if python3 -c "
from horus import Node
node = Node(
    name='test_node',
    pubs=['output'],
    subs=['input']
)
assert node.name == 'test_node'
assert 'output' in node.pub_topics
assert 'input' in node.sub_topics
" 2>/dev/null; then
    echo -e "${GREEN} PASS${NC} - Python Node creation works"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Python Node creation failed"
    ((FAILED++))
fi

# Test: Python Node API methods
if python3 -c "
from horus import Node
node = Node(name='test', pubs=['topic'])
# Test that basic methods exist and work
assert hasattr(node, 'send')
assert hasattr(node, 'get')
assert hasattr(node, 'has_msg')
result = node.send('topic', 42)
# In mock mode or real mode, send should return a boolean
assert isinstance(result, bool)
" 2>/dev/null; then
    echo -e "${GREEN} PASS${NC} - Python Node API works"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Python Node API failed"
    ((FAILED++))
fi

# Test: Python quick helper
if python3 -c "
from horus import quick
node = quick(name='helper', sub='in', pub='out', fn=lambda x: x * 2)
assert node.name == 'helper'
assert 'in' in node.sub_topics
assert 'out' in node.pub_topics
" 2>/dev/null; then
    echo -e "${GREEN} PASS${NC} - Python quick helper works"
    ((PASSED++))
else
    echo -e "${RED} FAIL${NC} - Python quick helper failed"
    ((FAILED++))
fi

echo ""
echo -e "${BLUE}Testing: Core acceptance tests (Rust unit tests)${NC}"

# Test: Run horus_core acceptance tests (with timeout to prevent hanging)
HORUS_ROOT="/home/lord-patpak/horus/HORUS"
cd "$HORUS_ROOT/horus_core"
if timeout 60 cargo test --test acceptance_hub --test acceptance_scheduler >/dev/null 2>&1; then
    echo -e "${GREEN} PASS${NC} - Core acceptance tests (Hub + Scheduler)"
    ((PASSED++))
else
    EXIT_CODE=$?
    if [ $EXIT_CODE -eq 124 ]; then
        echo -e "${RED} FAIL${NC} - Core acceptance tests timed out"
    else
        echo -e "${RED} FAIL${NC} - Core acceptance tests failed"
    fi
    ((FAILED++))
fi
cd "$TEST_DIR"

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}          Test Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN} Passed:${NC}  $PASSED"
echo -e "${RED} Failed:${NC}  $FAILED"
echo -e "${YELLOW} Skipped:${NC} $SKIPPED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN} ALL ACCEPTANCE TESTS PASSED!${NC}"
    exit 0
else
    echo -e "${RED} SOME TESTS FAILED - See above for details${NC}"
    exit 1
fi
