#!/bin/bash
# Test: verify.sh after installation

set -e

# Source helpers from mounted location
source "/test_helpers/common.sh"

HORUS_ROOT="${HORUS_ROOT:-/horus}"

test_verify() {
    start_test "verify.sh"

    log_info "Running verify.sh..."
    cd "$HORUS_ROOT"

    output=$(bash ./verify.sh 2>&1)
    exit_code=$?

    echo "$output"

    # verify.sh should pass (exit 0) for perfect installation
    # exit 1 for warnings, exit 2 for errors
    if [ $exit_code -eq 0 ]; then
        log_success "verify.sh: Perfect installation"
    elif [ $exit_code -eq 1 ]; then
        log_warn "verify.sh: Warnings found"
        assert_contains "$output" "warning"
    else
        log_error "verify.sh: Errors found (exit code: $exit_code)"
        end_test "verify.sh" 1
        return 1
    fi

    # Check that verify found the installation
    assert_contains "$output" "Binary:" || { end_test "verify.sh" 1; return 1; }
    assert_contains "$output" "horus library:" || { end_test "verify.sh" 1; return 1; }

    end_test "verify.sh" 0
    return 0
}

test_verify
exit $?
