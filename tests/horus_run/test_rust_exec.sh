#!/bin/bash
# Test basic Rust execution with horus run

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
TEST_DIR=$(mktemp -d /tmp/horus_test_rust_exec_XXXXXX)
trap "rm -rf $TEST_DIR" EXIT

cd "$TEST_DIR"

echo "=== Testing Rust Execution with horus run ==="
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

# Test 1: Basic Rust file compilation and execution
echo "Test 1: Run simple Rust file..."
cp /home/lord-patpak/horus/HORUS/tests/horus_run/fixtures/simple_rust.rs .
if $HORUS run simple_rust.rs 2>&1 | grep -q "Hello from Rust"; then
    pass "Rust file compiled and executed"
else
    fail "Rust file execution" "Did not see expected output"
fi

# Test 2: Rust file cached (second run faster)
echo "Test 2: Cached Rust execution..."
# Run once to cache
$HORUS run simple_rust.rs 2>&1 > /dev/null

# Run again - should use cache
START=$(date +%s%N)
$HORUS run simple_rust.rs 2>&1 > /dev/null
END=$(date +%s%N)
DURATION=$((($END - $START) / 1000000))

if [ $DURATION -lt 5000 ]; then  # Less than 5 seconds indicates cache hit
    pass "Rust binary cached correctly"
else
    fail "Rust caching" "Second run took ${DURATION}ms (expected < 5000ms)"
fi

# Test 3: Rust with dependencies (std)
echo "Test 3: Rust with standard library..."
cat > with_std.rs << 'EOF'
use std::collections::HashMap;
use std::io::Write;

fn main() {
    let mut map = HashMap::new();
    map.insert("test", "success");

    println!("HashMap test: {}", map.get("test").unwrap());
    writeln!(std::io::stdout(), "IO test: success").unwrap();
}
EOF

if $HORUS run with_std.rs 2>&1 | grep -q "success"; then
    pass "Rust with std library works"
else
    fail "Rust std library" "Standard library usage failed"
fi

# Test 4: Rust compilation error handling
echo "Test 4: Handle Rust compilation errors..."
cat > compile_error.rs << 'EOF'
fn main() {
    let x: i32 = "not an integer";  // Type mismatch
    println!("{}", x);
}
EOF

if $HORUS run compile_error.rs 2>&1 | grep -q -i "error\|mismatched"; then
    pass "Rust compilation errors detected"
else
    fail "Rust compile error" "Should have reported compilation error"
fi

# Test 5: Rust panic handling
echo "Test 5: Handle Rust panics..."
cat > panic.rs << 'EOF'
fn main() {
    panic!("Test panic");
}
EOF

if ! $HORUS run panic.rs 2>&1; then
    pass "Rust panics handled"
else
    fail "Rust panic" "Should have failed on panic"
fi

# Test 6: Rust with command-line arguments
echo "Test 6: Pass arguments to Rust program..."
cat > with_args.rs << 'EOF'
fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("Program: {}", args[0]);

    for (i, arg) in args.iter().skip(1).enumerate() {
        println!("Arg {}: {}", i + 1, arg);
    }
}
EOF

OUTPUT=$($HORUS run with_args.rs -- test1 test2 "arg with spaces" 2>&1)
if echo "$OUTPUT" | grep -q "test1" && echo "$OUTPUT" | grep -q "test2" && echo "$OUTPUT" | grep -q "arg with spaces"; then
    pass "Arguments passed to Rust program"
else
    fail "Rust arguments" "Arguments not received correctly"
fi

# Test 7: Rust exit code
echo "Test 7: Rust exit codes..."
cat > exit_code.rs << 'EOF'
fn main() {
    std::process::exit(42);
}
EOF

$HORUS run exit_code.rs 2>&1
EXIT_CODE=$?
if [ $EXIT_CODE -eq 42 ]; then
    pass "Rust exit codes preserved"
else
    fail "Rust exit code" "Expected 42, got $EXIT_CODE"
fi

# Test 8: Rust with environment variables
echo "Test 8: Rust environment variables..."
cat > env_test.rs << 'EOF'
fn main() {
    if let Ok(val) = std::env::var("HORUS_TEST_VAR") {
        println!("Got env var: {}", val);
    } else {
        println!("No env var found");
    }
}
EOF

OUTPUT=$(HORUS_TEST_VAR="test_value" $HORUS run env_test.rs 2>&1)
if echo "$OUTPUT" | grep -q "test_value"; then
    pass "Rust environment variables work"
else
    fail "Rust env vars" "Environment variable not passed"
fi

# Test 9: Rust debug vs release output
echo "Test 9: Rust produces output..."
cat > output_test.rs << 'EOF'
fn main() {
    for i in 1..=5 {
        println!("Line {}", i);
    }
}
EOF

OUTPUT=$($HORUS run output_test.rs 2>&1)
LINE_COUNT=$(echo "$OUTPUT" | grep -c "Line")
if [ $LINE_COUNT -eq 5 ]; then
    pass "Rust output complete"
else
    fail "Rust output" "Expected 5 lines, got $LINE_COUNT"
fi

# Test 10: Rust file with Unicode
echo "Test 10: Rust with Unicode characters..."
cat > unicode.rs << 'EOF'
fn main() {
    println!("Hello 世界 ");
    println!("Тест Русский");
    println!("Test émojis:   ");
}
EOF

if $HORUS run unicode.rs 2>&1 | grep -q ""; then
    pass "Rust Unicode handling works"
else
    fail "Rust Unicode" "Unicode output failed"
fi

# Summary
echo ""
echo "================================"
echo "Rust Execution Tests Summary"
echo "================================"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All Rust execution tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
