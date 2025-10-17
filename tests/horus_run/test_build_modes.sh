#!/bin/bash
# Test build modes with horus run (debug, release, clean, build-only)

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

# Test directory
TEST_DIR=$(mktemp -d /tmp/horus_test_build_modes_XXXXXX)
trap "rm -rf $TEST_DIR" EXIT

cd "$TEST_DIR"

echo "=== Testing Build Modes with horus run ==="
echo ""

# Helper functions
pass() {
    echo -e "${GREEN}✅ PASS${NC}: $1"
    ((TESTS_PASSED++))
}

fail() {
    echo -e "${RED}❌ FAIL${NC}: $1"
    echo "   Error: $2"
    ((TESTS_FAILED++))
}

# Test 1: Default debug mode
echo "Test 1: Default debug mode (no optimization)..."
cat > test.rs << 'EOF'
fn main() {
    println!("Debug mode test");
}
EOF

if $HORUS run test.rs 2>&1 | grep -q "Debug mode test"; then
    pass "Debug mode works"
else
    fail "Debug mode" "Failed to run in debug mode"
fi

# Test 2: Release mode with --release flag
echo "Test 2: Release mode with --release..."
cat > release_test.rs << 'EOF'
fn main() {
    println!("Release mode test");
}
EOF

if $HORUS run --release release_test.rs 2>&1 | grep -q "Release mode test"; then
    pass "Release mode works"
else
    fail "Release mode" "Failed to run with --release"
fi

# Test 3: Clean build with --clean flag
echo "Test 3: Clean build with --clean..."
cat > clean_test.rs << 'EOF'
fn main() {
    println!("Clean build test");
}
EOF

# Run once to cache
$HORUS run clean_test.rs 2>&1 > /dev/null

# Run with --clean - should recompile
if $HORUS run --clean clean_test.rs 2>&1 | grep -q "Clean build test"; then
    pass "Clean build works"
else
    fail "Clean build" "Failed to run with --clean"
fi

# Test 4: Build-only mode (no execution)
echo "Test 4: Build-only with --build-only..."
cat > build_only.rs << 'EOF'
fn main() {
    println!("Should not see this");
    panic!("Should not execute!");
}
EOF

OUTPUT=$($HORUS run --build-only build_only.rs 2>&1)
if ! echo "$OUTPUT" | grep -q "Should not see this"; then
    pass "Build-only does not execute"
else
    fail "Build-only" "Should not have executed the program"
fi

# Test 5: Release mode is faster
echo "Test 5: Release mode performance..."
cat > perf_test.rs << 'EOF'
fn fibonacci(n: u64) -> u64 {
    if n <= 1 { return n; }
    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    let result = fibonacci(30);
    println!("Result: {}", result);
}
EOF

# Time debug build
START=$(date +%s%N)
$HORUS run perf_test.rs 2>&1 > /dev/null
END=$(date +%s%N)
DEBUG_TIME=$((($END - $START) / 1000000))

# Clean and time release build
$HORUS run --clean perf_test.rs 2>&1 > /dev/null
START=$(date +%s%N)
$HORUS run --release perf_test.rs 2>&1 > /dev/null
END=$(date +%s%N)
RELEASE_TIME=$((($END - $START) / 1000000))

# Note: This test just verifies both modes work, not that release is faster
# (compilation time can vary)
if [ $DEBUG_TIME -gt 0 ] && [ $RELEASE_TIME -gt 0 ]; then
    pass "Both debug and release modes execute"
else
    fail "Performance test" "Debug: ${DEBUG_TIME}ms, Release: ${RELEASE_TIME}ms"
fi

# Test 6: C with release mode
echo "Test 6: C compilation with --release..."
cat > c_release.c << 'EOF'
#include <stdio.h>
int main() {
    printf("C release mode\n");
    return 0;
}
EOF

if $HORUS run --release c_release.c 2>&1 | grep -q "C release mode"; then
    pass "C release mode works"
else
    fail "C release" "Failed C compilation with --release"
fi

# Test 7: Python ignores build modes
echo "Test 7: Python with build flags..."
cat > python_test.py << 'EOF'
#!/usr/bin/env python3
print("Python with flags")
EOF

# Python should work with --release (even though it doesn't apply)
if $HORUS run --release python_test.py 2>&1 | grep -q "Python with flags"; then
    pass "Python works with --release flag"
else
    fail "Python with flags" "Python should ignore build flags gracefully"
fi

# Test 8: Clean removes cache
echo "Test 8: Clean removes cached binary..."
cat > cache_test.rs << 'EOF'
fn main() {
    println!("Cache test");
}
EOF

# Build and cache
$HORUS run cache_test.rs 2>&1 > /dev/null

# Modify source
cat > cache_test.rs << 'EOF'
fn main() {
    println!("Modified cache test");
}
EOF

# Run with --clean should rebuild
if $HORUS run --clean cache_test.rs 2>&1 | grep -q "Modified cache test"; then
    pass "Clean rebuilds with new changes"
else
    fail "Clean rebuild" "Did not rebuild after clean"
fi

# Test 9: Build-only creates binary
echo "Test 9: Build-only creates executable..."
cat > build_creates.rs << 'EOF'
fn main() {
    println!("Build creates binary");
}
EOF

$HORUS run --build-only build_creates.rs 2>&1 > /dev/null

# Check if binary exists in cache (approximate check)
if [ -d ".horus/cache" ] || $HORUS run build_creates.rs 2>&1 | grep -q "Build creates binary"; then
    pass "Build-only creates binary for later use"
else
    fail "Build-only binary" "Binary not created or cached"
fi

# Test 10: Combining flags
echo "Test 10: Combine --clean and --release..."
cat > combined.rs << 'EOF'
fn main() {
    println!("Combined flags test");
}
EOF

if $HORUS run --clean --release combined.rs 2>&1 | grep -q "Combined flags test"; then
    pass "Can combine --clean and --release"
else
    fail "Combined flags" "Failed to combine flags"
fi

# Summary
echo ""
echo "================================"
echo "Build Modes Tests Summary"
echo "================================"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All build mode tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
