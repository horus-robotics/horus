#!/bin/bash
# Common testing utilities for shell script tests

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Test state
TESTS_PASSED=0
TESTS_FAILED=0
TEST_START_TIME=""

# Logging
log_info() {
    echo -e "${CYAN}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $*"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

# Test utilities
start_test() {
    local test_name="$1"
    TEST_START_TIME=$(date +%s)
    log_info "Starting test: $test_name"
}

end_test() {
    local test_name="$1"
    local status="$2"
    local end_time=$(date +%s)
    local duration=$((end_time - TEST_START_TIME))

    if [ "$status" -eq 0 ]; then
        log_success "$test_name completed in ${duration}s"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "$test_name failed in ${duration}s"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

# Assertions
assert_exit_code() {
    local expected="$1"
    local actual="${2:-$?}"
    local msg="${3:-Exit code mismatch}"

    if [ "$actual" -eq "$expected" ]; then
        log_success "$msg (exit code: $expected)"
        return 0
    else
        log_error "$msg (expected: $expected, got: $actual)"
        return 1
    fi
}

assert_command_exists() {
    local cmd="$1"
    if command -v "$cmd" &> /dev/null; then
        log_success "Command exists: $cmd"
        return 0
    else
        log_error "Command not found: $cmd"
        return 1
    fi
}

assert_file_exists() {
    local file="$1"
    if [ -f "$file" ]; then
        log_success "File exists: $file"
        return 0
    else
        log_error "File not found: $file"
        return 1
    fi
}

assert_dir_exists() {
    local dir="$1"
    if [ -d "$dir" ]; then
        log_success "Directory exists: $dir"
        return 0
    else
        log_error "Directory not found: $dir"
        return 1
    fi
}

assert_contains() {
    local haystack="$1"
    local needle="$2"
    if echo "$haystack" | grep -q "$needle"; then
        log_success "Output contains: $needle"
        return 0
    else
        log_error "Output does not contain: $needle"
        return 1
    fi
}

assert_not_contains() {
    local haystack="$1"
    local needle="$2"
    if echo "$haystack" | grep -q "$needle"; then
        log_error "Output should not contain: $needle"
        return 1
    else
        log_success "Output does not contain: $needle"
        return 0
    fi
}

# Run a command and capture output
run_command() {
    local output
    local exit_code
    output=$(eval "$@" 2>&1)
    exit_code=$?
    echo "$output"
    return $exit_code
}

# Run a script with timeout
run_script_with_timeout() {
    local script="$1"
    local timeout="${2:-600}"  # Default 10 minutes
    local output
    local exit_code

    log_info "Running $script with ${timeout}s timeout..."
    output=$(timeout "$timeout" bash "$script" 2>&1)
    exit_code=$?

    if [ $exit_code -eq 124 ]; then
        log_error "Script timed out after ${timeout}s"
        return 124
    fi

    echo "$output"
    return $exit_code
}

# Print test summary
print_summary() {
    local total=$((TESTS_PASSED + TESTS_FAILED))
    echo ""
    echo "========================================"
    echo "Test Summary"
    echo "========================================"
    echo "Total:  $total"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    echo "========================================"

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}All tests passed!${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed!${NC}"
        return 1
    fi
}

# Cleanup helpers
cleanup_horus() {
    log_info "Cleaning up HORUS installation..."
    rm -rf ~/.cargo/bin/horus ~/.horus /dev/shm/horus* 2>/dev/null || true
}

# Wait for a condition with timeout
wait_for() {
    local condition="$1"
    local timeout="${2:-30}"
    local interval="${3:-1}"
    local elapsed=0

    while ! eval "$condition" &> /dev/null; do
        sleep "$interval"
        elapsed=$((elapsed + interval))
        if [ $elapsed -ge $timeout ]; then
            log_error "Timeout waiting for: $condition"
            return 1
        fi
    done
    log_success "Condition met: $condition"
    return 0
}
