#!/bin/bash
# Test auto-detection of main files with horus run

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# HORUS binary
HORUS="/home/lord-patpak/horus/HORUS/target/debug/horus"

# Base test directory
BASE_TEST_DIR=$(mktemp -d /tmp/horus_test_autodetect_XXXXXX)
trap "rm -rf $BASE_TEST_DIR" EXIT

echo "=== Testing Auto-Detection with horus run ==="
echo ""

# Helper functions
pass() {
    echo -e "${GREEN} PASS${NC}: $1"
    ((TESTS_PASSED++))
}

fail() {
    echo -e "${RED} FAIL${NC}: $1"
    echo "   Error: $2"
    ((TESTS_FAILED++))
}

# Test 1: Auto-detect main.py
echo "Test 1: Auto-detect main.py..."
TEST_DIR="${BASE_TEST_DIR}/test_mainpy"
mkdir -p "${TEST_DIR}"
cd "${TEST_DIR}"

cat > main.py << 'EOF'
#!/usr/bin/env python3
print("Auto-detected main.py")
EOF

if $HORUS run 2>&1 | grep -q "Auto-detected main.py"; then
    pass "Auto-detected main.py"
else
    fail "main.py auto-detect" "Did not find main.py"
fi

# Test 2: Auto-detect main.rs
echo "Test 2: Auto-detect main.rs..."
TEST_DIR="${BASE_TEST_DIR}/test_mainrs"
mkdir -p "${TEST_DIR}"
cd "${TEST_DIR}"

cat > main.rs << 'EOF'
fn main() {
    println!("Auto-detected main.rs");
}
EOF

if $HORUS run 2>&1 | grep -q "Auto-detected main.rs"; then
    pass "Auto-detected main.rs"
else
    fail "main.rs auto-detect" "Did not find main.rs"
fi

# Test 3: Auto-detect main.c
echo "Test 3: Auto-detect main.c..."
TEST_DIR="${BASE_TEST_DIR}/test_mainc"
mkdir -p "${TEST_DIR}"
cd "${TEST_DIR}"

cat > main.c << 'EOF'
#include <stdio.h>
int main() {
    printf("Auto-detected main.c\n");
    return 0;
}
EOF

if $HORUS run 2>&1 | grep -q "Auto-detected main.c"; then
    pass "Auto-detected main.c"
else
    fail "main.c auto-detect" "Did not find main.c"
fi

# Test 4: Auto-detect src/main.rs
echo "Test 4: Auto-detect src/main.rs..."
TEST_DIR="${BASE_TEST_DIR}/test_src_mainrs"
mkdir -p "${TEST_DIR}/src"
cd "${TEST_DIR}"

cat > src/main.rs << 'EOF'
fn main() {
    println!("Auto-detected src/main.rs");
}
EOF

if $HORUS run 2>&1 | grep -q "Auto-detected src/main.rs"; then
    pass "Auto-detected src/main.rs"
else
    fail "src/main.rs auto-detect" "Did not find src/main.rs"
fi

# Test 5: Auto-detect src/main.py
echo "Test 5: Auto-detect src/main.py..."
TEST_DIR="${BASE_TEST_DIR}/test_src_mainpy"
mkdir -p "${TEST_DIR}/src"
cd "${TEST_DIR}"

cat > src/main.py << 'EOF'
#!/usr/bin/env python3
print("Auto-detected src/main.py")
EOF

if $HORUS run 2>&1 | grep -q "Auto-detected src/main.py"; then
    pass "Auto-detected src/main.py"
else
    fail "src/main.py auto-detect" "Did not find src/main.py"
fi

# Test 6: Single file fallback (only one .py)
echo "Test 6: Single file fallback..."
TEST_DIR="${BASE_TEST_DIR}/test_single_file"
mkdir -p "${TEST_DIR}"
cd "${TEST_DIR}"

cat > only_file.py << 'EOF'
#!/usr/bin/env python3
print("Only Python file found")
EOF

if $HORUS run 2>&1 | grep -q "Only Python file found"; then
    pass "Single file fallback works"
else
    fail "Single file fallback" "Did not run only file"
fi

# Test 7: Prefer main.rs over other .rs files
echo "Test 7: Prefer main.rs over other files..."
TEST_DIR="${BASE_TEST_DIR}/test_prefer_main"
mkdir -p "${TEST_DIR}"
cd "${TEST_DIR}"

cat > main.rs << 'EOF'
fn main() {
    println!("Preferred main.rs");
}
EOF

cat > other.rs << 'EOF'
fn main() {
    println!("Other file");
}
EOF

OUTPUT=$($HORUS run 2>&1)
if echo "$OUTPUT" | grep -q "Preferred main.rs" && ! echo "$OUTPUT" | grep -q "Other file"; then
    pass "Prefers main.rs over other .rs files"
else
    fail "main.rs preference" "Did not prefer main.rs"
fi

# Test 8: Error when no suitable file
echo "Test 8: Error when no suitable file found..."
TEST_DIR="${BASE_TEST_DIR}/test_no_file"
mkdir -p "${TEST_DIR}"
cd "${TEST_DIR}"

touch README.md
touch data.txt

if $HORUS run 2>&1 | grep -q -i "error\|not found\|no.*file"; then
    pass "Error shown when no suitable file"
else
    fail "No file error" "Should show error when no runnable file"
fi

# Test 9: Run from subdirectory
echo "Test 9: Run directory with main file..."
TEST_DIR="${BASE_TEST_DIR}/test_run_dir"
mkdir -p "${TEST_DIR}/subdir"
cd "${TEST_DIR}"

cat > subdir/main.py << 'EOF'
#!/usr/bin/env python3
print("Running from subdirectory")
EOF

if $HORUS run subdir/ 2>&1 | grep -q "Running from subdirectory"; then
    pass "Run directory finds main file"
else
    fail "Directory run" "Did not find main in directory"
fi

# Test 10: Ambiguous files - multiple mains
echo "Test 10: Handle multiple main files..."
TEST_DIR="${BASE_TEST_DIR}/test_multiple_mains"
mkdir -p "${TEST_DIR}"
cd "${TEST_DIR}"

cat > main.py << 'EOF'
#!/usr/bin/env python3
print("Python main")
EOF

cat > main.rs << 'EOF'
fn main() {
    println!("Rust main");
}
EOF

# Should pick one (likely the first alphabetically or by priority)
# Just verify it runs without error
if $HORUS run 2>&1 | grep -q -E "Python main|Rust main"; then
    pass "Handles multiple main files"
else
    fail "Multiple mains" "Failed with multiple main files"
fi

# Test 11: Cargo.toml project
echo "Test 11: Auto-detect Cargo.toml project..."
TEST_DIR="${BASE_TEST_DIR}/test_cargo_project"
mkdir -p "${TEST_DIR}/src"
cd "${TEST_DIR}"

cat > Cargo.toml << 'EOF'
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
EOF

cat > src/main.rs << 'EOF'
fn main() {
    println!("Cargo project detected");
}
EOF

if $HORUS run 2>&1 | grep -q "Cargo project detected"; then
    pass "Auto-detects Cargo.toml project"
else
    fail "Cargo project" "Did not detect Cargo project"
fi

# Summary
echo ""
echo "================================"
echo "Auto-Detection Tests Summary"
echo "================================"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All auto-detection tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
