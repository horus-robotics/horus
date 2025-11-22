#!/bin/bash
# Test: install.sh on clean system

set -e

# Source helpers from mounted location
source "/test_helpers/common.sh"

HORUS_ROOT="${HORUS_ROOT:-/horus}"

test_install() {
    start_test "install.sh"

    log_info "Simulating fresh system - no Rust, no dependencies"

    # Verify Rust is NOT installed (clean state)
    if command -v rustc &> /dev/null; then
        log_warn "Rust already installed, this is not a clean system"
    fi

    # Install Rust first (scripts expect user to do this manually)
    log_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
    assert_command_exists "rustc"
    assert_command_exists "cargo"

    # Run install script with auto-confirm
    log_info "Running install.sh..."
    cd "$HORUS_ROOT"

    # Answer "Y" to installation prompt
    output=$(echo "Y" | bash ./install.sh 2>&1)
    exit_code=$?

    echo "$output"

    # Check if install succeeded
    if [ $exit_code -ne 0 ]; then
        log_error "install.sh exited with code $exit_code"
        echo "$output" | tail -20
        end_test "install.sh" 1
        return 1
    fi

    # Verify installation
    log_info "Verifying installation..."

    # Check binary exists
    assert_file_exists "$HOME/.cargo/bin/horus" || { end_test "install.sh" 1; return 1; }

    # Check binary is executable
    if [ -x "$HOME/.cargo/bin/horus" ]; then
        log_success "Binary is executable"
    else
        log_error "Binary is not executable"
        end_test "install.sh" 1
        return 1
    fi

    # Check binary works
    "$HOME/.cargo/bin/horus" --version &> /dev/null
    assert_exit_code 0 $? "horus --version" || { end_test "install.sh" 1; return 1; }

    # Check library cache exists
    assert_dir_exists "$HOME/.horus/cache" || { end_test "install.sh" 1; return 1; }

    # Check core libraries installed
    for lib in horus horus_core horus_macros horus_library; do
        if ls "$HOME/.horus/cache/${lib}@"* 1> /dev/null 2>&1; then
            log_success "Library installed: $lib"
        else
            log_error "Library missing: $lib"
            end_test "install.sh" 1
            return 1
        fi
    done

    # Check version file
    assert_file_exists "$HOME/.horus/installed_version" || { end_test "install.sh" 1; return 1; }

    # Test basic command
    log_info "Testing basic commands..."
    "$HOME/.cargo/bin/horus" --help &> /dev/null
    assert_exit_code 0 $? "horus --help" || { end_test "install.sh" 1; return 1; }

    log_success "Installation verified successfully"
    end_test "install.sh" 0
    return 0
}

# Run the test
test_install
exit $?
