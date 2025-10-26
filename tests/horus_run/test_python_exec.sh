#!/bin/bash
# Test basic Python execution with horus run

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
TEST_DIR=$(mktemp -d /tmp/horus_test_python_exec_XXXXXX)
trap "rm -rf $TEST_DIR" EXIT

cd "$TEST_DIR"

echo "=== Testing Python Execution with horus run ==="
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

# Test 1: Basic Python file execution
echo "Test 1: Run simple Python file..."
cp /home/lord-patpak/horus/HORUS/tests/horus_run/fixtures/simple_python.py .
if $HORUS run simple_python.py 2>&1 | grep -q "Hello from Python"; then
    pass "Python file executed successfully"
else
    fail "Python file execution" "Did not see expected output"
fi

# Test 2: Python with shebang
echo "Test 2: Python file respects shebang..."
chmod +x simple_python.py
if $HORUS run simple_python.py 2>&1 | grep -q "HORUS run works"; then
    pass "Python with shebang works"
else
    fail "Python shebang" "Execution failed"
fi

# Test 3: Python requires .py extension (expected behavior)
echo "Test 3: Python requires extension..."
cp simple_python.py test_no_ext
OUTPUT=$($HORUS run test_no_ext 2>&1 || true)
if echo "$OUTPUT" | grep -q "Unsupported file type"; then
    pass "Correctly requires .py extension"
else
    fail "Extension requirement" "Should require .py extension"
fi

# Test 4: Passing arguments to Python program
echo "Test 4: Pass arguments to Python program..."
cp /home/lord-patpak/horus/HORUS/tests/horus_run/fixtures/with_args.py .
OUTPUT=$($HORUS run with_args.py -- arg1 arg2 "arg with spaces" 2>&1)
if echo "$OUTPUT" | grep -q "arg1" && echo "$OUTPUT" | grep -q "arg2" && echo "$OUTPUT" | grep -q "arg with spaces"; then
    pass "Arguments passed correctly to Python"
else
    fail "Python arguments" "Arguments not received: $OUTPUT"
fi

# Test 5: Python syntax error handling
echo "Test 5: Handle Python syntax errors..."
cat > syntax_error.py << 'EOF'
def broken(
    print("Missing closing paren"
EOF

if $HORUS run syntax_error.py 2>&1 | grep -q -i "error\|syntax"; then
    pass "Python syntax errors detected"
else
    fail "Python syntax error" "Should have reported syntax error"
fi

# Test 6: Python runtime error handling
echo "Test 6: Handle Python runtime errors..."
cat > runtime_error.py << 'EOF'
#!/usr/bin/env python3
def main():
    x = 1 / 0  # Division by zero
    return 0

if __name__ == "__main__":
    exit(main())
EOF

if ! $HORUS run runtime_error.py 2>&1; then
    pass "Python runtime errors detected"
else
    fail "Python runtime error" "Should have failed with runtime error"
fi

# Test 7: Python imports standard library
echo "Test 7: Python standard library imports..."
cat > with_imports.py << 'EOF'
#!/usr/bin/env python3
import sys
import os
import json

def main():
    print(f"Python {sys.version_info.major}.{sys.version_info.minor}")
    print(f"OS: {os.name}")
    data = json.dumps({"test": "success"})
    print(data)
    return 0

if __name__ == "__main__":
    exit(main())
EOF

if $HORUS run with_imports.py 2>&1 | grep -q "success"; then
    pass "Python standard library imports work"
else
    fail "Python imports" "Standard library import failed"
fi

# Test 8: Python sensor node simulation
echo "Test 8: Run Python sensor node..."
cp /home/lord-patpak/horus/HORUS/tests/horus_run/fixtures/sensor_node.py .
OUTPUT=$($HORUS run sensor_node.py 2>&1)
if echo "$OUTPUT" | grep -q "Sensor Node Starting" && echo "$OUTPUT" | grep -q "Temperature"; then
    pass "Python sensor node executed"
else
    fail "Python sensor node" "Sensor simulation failed"
fi

# Test 9: Python exit code handling
echo "Test 9: Python exit codes..."
cat > exit_code.py << 'EOF'
#!/usr/bin/env python3
import sys
sys.exit(42)
EOF

$HORUS run exit_code.py 2>&1
EXIT_CODE=$?
if [ $EXIT_CODE -eq 42 ]; then
    pass "Python exit codes preserved"
else
    fail "Python exit code" "Expected 42, got $EXIT_CODE"
fi

# Summary
echo ""
echo "================================"
echo "Python Execution Tests Summary"
echo "================================"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All Python execution tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
