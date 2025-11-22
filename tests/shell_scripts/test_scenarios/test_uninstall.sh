#!/bin/bash
# Test: uninstall.sh

set -e

# Source helpers from mounted location
source "/test_helpers/common.sh"

HORUS_ROOT="${HORUS_ROOT:-/horus}"

test_uninstall() {
    start_test "uninstall.sh"

    log_info "Running uninstall.sh..."
    cd "$HORUS_ROOT"

    # Answer "y" to uninstall prompt, "y" to remove .horus
    output=$(echo -e "y\ny" | bash ./uninstall.sh 2>&1)
    exit_code=$?

    echo "$output"

    if [ $exit_code -ne 0 ]; then
        log_error "uninstall.sh exited with code $exit_code"
        end_test "uninstall.sh" 1
        return 1
    fi

    # Verify removal
    log_info "Verifying uninstallation..."

    # Binary should be gone
    if [ -f "$HOME/.cargo/bin/horus" ]; then
        log_error "Binary still exists after uninstall"
        end_test "uninstall.sh" 1
        return 1
    else
        log_success "Binary removed"
    fi

    # Cache should be gone
    if [ -d "$HOME/.horus/cache" ]; then
        log_error "Cache still exists after uninstall"
        end_test "uninstall.sh" 1
        return 1
    else
        log_success "Cache removed"
    fi

    log_success "Uninstallation verified successfully"
    end_test "uninstall.sh" 0
    return 0
}

test_uninstall
exit $?
