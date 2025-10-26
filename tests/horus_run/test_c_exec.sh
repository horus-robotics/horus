#!/bin/bash
# Test basic C execution with horus run

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
TEST_DIR=$(mktemp -d /tmp/horus_test_c_exec_XXXXXX)
trap "rm -rf $TEST_DIR" EXIT

cd "$TEST_DIR"

echo "=== Testing C Execution with horus run ==="
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

# Test 1: Basic C file compilation and execution
echo "Test 1: Run simple C file..."
cp /home/lord-patpak/horus/HORUS/tests/horus_run/fixtures/simple_c.c .
if $HORUS run simple_c.c 2>&1 | grep -q "Hello from C"; then
    pass "C file compiled and executed"
else
    fail "C file execution" "Did not see expected output"
fi

# Test 2: C with standard library
echo "Test 2: C with standard library functions..."
cat > with_stdlib.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main() {
    char *str = malloc(100);
    strcpy(str, "Memory allocation works");
    printf("%s\n", str);
    free(str);
    return 0;
}
EOF

if $HORUS run with_stdlib.c 2>&1 | grep -q "Memory allocation works"; then
    pass "C standard library works"
else
    fail "C stdlib" "Standard library functions failed"
fi

# Test 3: C compilation error handling
echo "Test 3: Handle C compilation errors..."
cat > compile_error.c << 'EOF'
#include <stdio.h>

int main() {
    printf("Missing semicolon")
    return 0;
}
EOF

if $HORUS run compile_error.c 2>&1 | grep -q -i "error"; then
    pass "C compilation errors detected"
else
    fail "C compile error" "Should have reported compilation error"
fi

# Test 4: C with command-line arguments
echo "Test 4: Pass arguments to C program..."
cat > with_args.c << 'EOF'
#include <stdio.h>

int main(int argc, char *argv[]) {
    printf("Program: %s\n", argv[0]);
    printf("Argument count: %d\n", argc - 1);

    for (int i = 1; i < argc; i++) {
        printf("Arg %d: %s\n", i, argv[i]);
    }

    return 0;
}
EOF

OUTPUT=$($HORUS run with_args.c -- arg1 arg2 "arg with spaces" 2>&1)
if echo "$OUTPUT" | grep -q "arg1" && echo "$OUTPUT" | grep -q "arg2" && echo "$OUTPUT" | grep -q "arg with spaces"; then
    pass "Arguments passed to C program"
else
    fail "C arguments" "Arguments not received correctly"
fi

# Test 5: C exit code
echo "Test 5: C exit codes..."
cat > exit_code.c << 'EOF'
#include <stdlib.h>

int main() {
    return 42;
}
EOF

$HORUS run exit_code.c 2>&1
EXIT_CODE=$?
if [ $EXIT_CODE -eq 42 ]; then
    pass "C exit codes preserved"
else
    fail "C exit code" "Expected 42, got $EXIT_CODE"
fi

# Test 6: C with multiple source files (single compilation unit)
echo "Test 6: C with multiple functions..."
cat > multi_functions.c << 'EOF'
#include <stdio.h>

int add(int a, int b) {
    return a + b;
}

int multiply(int a, int b) {
    return a * b;
}

int main() {
    printf("5 + 3 = %d\n", add(5, 3));
    printf("5 * 3 = %d\n", multiply(5, 3));
    return 0;
}
EOF

OUTPUT=$($HORUS run multi_functions.c 2>&1)
if echo "$OUTPUT" | grep -q "5 + 3 = 8" && echo "$OUTPUT" | grep -F "5 * 3 = 15"; then
    pass "C with multiple functions works"
else
    fail "C multi-function" "Function calls failed"
fi

# Test 7: C math library
echo "Test 7: C math library linking..."
cat > with_math.c << 'EOF'
#include <stdio.h>
#include <math.h>

int main() {
    double result = sqrt(16.0);
    printf("sqrt(16) = %.1f\n", result);

    result = pow(2.0, 3.0);
    printf("pow(2,3) = %.1f\n", result);

    return 0;
}
EOF

OUTPUT=$($HORUS run with_math.c 2>&1)
if echo "$OUTPUT" | grep -q "sqrt(16) = 4.0" && echo "$OUTPUT" | grep -q "pow(2,3) = 8.0"; then
    pass "C math library linked correctly"
else
    fail "C math library" "Math functions failed: $OUTPUT"
fi

# Test 8: C cached execution
echo "Test 8: Cached C binary..."
cat > cache_test.c << 'EOF'
#include <stdio.h>
int main() {
    printf("Cache test\n");
    return 0;
}
EOF

# First run to compile
$HORUS run cache_test.c 2>&1 > /dev/null

# Second run should be faster (cached)
START=$(date +%s%N)
$HORUS run cache_test.c 2>&1 > /dev/null
END=$(date +%s%N)
DURATION=$((($END - $START) / 1000000))

if [ $DURATION -lt 3000 ]; then  # Less than 3 seconds indicates cache
    pass "C binary cached correctly"
else
    fail "C caching" "Second run took ${DURATION}ms (expected < 3000ms)"
fi

# Test 9: C with environment variables
echo "Test 9: C environment variables..."
cat > env_test.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>

int main() {
    char *val = getenv("HORUS_TEST_VAR");
    if (val != NULL) {
        printf("Got env var: %s\n", val);
    } else {
        printf("No env var found\n");
    }
    return 0;
}
EOF

OUTPUT=$(HORUS_TEST_VAR="test_value" $HORUS run env_test.c 2>&1)
if echo "$OUTPUT" | grep -q "test_value"; then
    pass "C environment variables work"
else
    fail "C env vars" "Environment variable not passed"
fi

# Summary
echo ""
echo "================================"
echo "C Execution Tests Summary"
echo "================================"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All C execution tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
