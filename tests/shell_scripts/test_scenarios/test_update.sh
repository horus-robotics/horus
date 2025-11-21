#!/bin/bash
# Test: update.sh after installation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../helpers/common.sh"

HORUS_ROOT="${HORUS_ROOT:-/horus}"

test_update() {
    start_test "update.sh"

    log_info "Running update.sh..."
    cd "$HORUS_ROOT"

    # Make a fake commit to test update detection
    git config user.email "test@test.com"
    git config user.name "Test User"

    # Answer "N" to "Rebuild anyway?" since we're already up to date
    output=$(echo "N" | bash ./update.sh 2>&1)
    exit_code=$?

    echo "$output"

    # Update should succeed (even if no changes)
    if [ $exit_code -ne 0 ]; then
        log_error "update.sh exited with code $exit_code"
        end_test "update.sh" 1
        return 1
    fi

    # Verify binary still works
    "$HOME/.cargo/bin/horus" --version &> /dev/null
    assert_exit_code 0 $? "horus --version after update" || { end_test "update.sh" 1; return 1; }

    log_success "Update completed successfully"
    end_test "update.sh" 0
    return 0
}

test_update
exit $?
