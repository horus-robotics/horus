#!/bin/bash
# HORUS Shell Script Test Runner
# Tests installation scripts in clean Docker containers

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default settings
DEFAULT_DISTRO="ubuntu-22.04"
AVAILABLE_DISTROS=("ubuntu-22.04" "ubuntu-24.04" "debian-12" "fedora-39")
AVAILABLE_TESTS=("install" "verify" "uninstall" "full-flow")

# Usage
usage() {
    echo "Usage: $0 [test] [distro] [options]"
    echo ""
    echo "Tests:"
    echo "  install       - Test install.sh"
    echo "  verify        - Test verify.sh"
    echo "  uninstall     - Test uninstall.sh"
    echo "  full-flow     - Test complete installation flow"
    echo "  all           - Run all tests (default)"
    echo ""
    echo "Distros:"
    echo "  ubuntu-22.04  - Ubuntu 22.04 LTS (default)"
    echo "  ubuntu-24.04  - Ubuntu 24.04 LTS"
    echo "  debian-12     - Debian 12 (Bookworm)"
    echo "  fedora-39     - Fedora 39"
    echo "  all-distros   - Test on all distributions"
    echo ""
    echo "Options:"
    echo "  --keep        - Keep container after test (for debugging)"
    echo "  --no-cache    - Rebuild Docker images from scratch"
    echo "  --verbose     - Show detailed output"
    echo ""
    echo "Examples:"
    echo "  $0                          # Run all tests on Ubuntu 22.04"
    echo "  $0 install                  # Test install.sh only"
    echo "  $0 install ubuntu-24.04     # Test install.sh on Ubuntu 24.04"
    echo "  $0 full-flow debian-12      # Full flow on Debian 12"
    echo "  $0 all all-distros          # Run all tests on all distros"
    exit 1
}

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

log_section() {
    echo -e "\n${BLUE}====================================${NC}"
    echo -e "${BLUE}$*${NC}"
    echo -e "${BLUE}====================================${NC}\n"
}

# Build Docker image for distro
build_image() {
    local distro="$1"
    local no_cache="${2:-false}"

    log_info "Building Docker image for $distro..."

    local dockerfile="$SCRIPT_DIR/dockerfiles/${distro}.Dockerfile"
    if [ ! -f "$dockerfile" ]; then
        log_error "Dockerfile not found: $dockerfile"
        return 1
    fi

    local cache_flag=""
    if [ "$no_cache" = "true" ]; then
        cache_flag="--no-cache"
    fi

    docker build $cache_flag \
        -t "horus-shell-test:${distro}" \
        -f "$dockerfile" \
        "$SCRIPT_DIR/dockerfiles/" || return 1

    log_success "Image built: horus-shell-test:${distro}"
}

# Run test in container
run_test_in_container() {
    local test_name="$1"
    local distro="$2"
    local keep_container="${3:-false}"

    local container_name="horus-test-${test_name}-${distro}-$$"
    local test_script="$SCRIPT_DIR/test_scenarios/test_${test_name}.sh"

    if [ ! -f "$test_script" ]; then
        log_error "Test script not found: $test_script"
        return 1
    fi

    log_info "Starting container: $container_name"

    # Run container with HORUS source mounted
    local rm_flag=""
    if [ "$keep_container" != "true" ]; then
        rm_flag="--rm"
    fi

    # Run test in container
    docker run $rm_flag \
        --name "$container_name" \
        -v "$REPO_ROOT:/horus:ro" \
        -v "$SCRIPT_DIR/helpers:/test_helpers:ro" \
        -v "$test_script:/test.sh:ro" \
        -e HORUS_ROOT=/horus \
        -w /home/testuser \
        "horus-shell-test:${distro}" \
        bash /test.sh

    local exit_code=$?

    if [ $exit_code -eq 0 ]; then
        log_success "Test passed: $test_name on $distro"
    else
        log_error "Test failed: $test_name on $distro (exit code: $exit_code)"

        if [ "$keep_container" = "true" ]; then
            log_info "Container kept for debugging: $container_name"
            log_info "Inspect with: docker exec -it $container_name bash"
        fi
    fi

    return $exit_code
}

# Run full installation flow
run_full_flow() {
    local distro="$1"
    local keep_container="${2:-false}"

    log_section "Running Full Installation Flow on $distro"

    local tests=("install" "verify" "uninstall")
    local container_name="horus-test-full-flow-${distro}-$$"

    log_info "Starting persistent container: $container_name"

    # Start container
    docker run -d \
        --name "$container_name" \
        -v "$REPO_ROOT:/horus:ro" \
        -v "$SCRIPT_DIR/helpers:/test_helpers:ro" \
        -v "$SCRIPT_DIR/test_scenarios:/test_scenarios:ro" \
        -e HORUS_ROOT=/horus \
        -w /home/testuser \
        "horus-shell-test:${distro}" \
        sleep 3600

    local overall_status=0

    # Run each test in sequence
    for test in "${tests[@]}"; do
        log_info "Running test: $test"

        docker exec "$container_name" \
            bash "/test_scenarios/test_${test}.sh"

        if [ $? -ne 0 ]; then
            log_error "Test failed: $test"
            overall_status=1
            break
        fi

        log_success "Test passed: $test"
    done

    # Cleanup
    if [ "$keep_container" != "true" ]; then
        docker rm -f "$container_name" &> /dev/null || true
    else
        log_info "Container kept for debugging: $container_name"
    fi

    if [ $overall_status -eq 0 ]; then
        log_success "Full flow completed successfully on $distro"
    else
        log_error "Full flow failed on $distro"
    fi

    return $overall_status
}

# Main execution
main() {
    local test_type="${1:-all}"
    local distro="${2:-$DEFAULT_DISTRO}"
    local keep_container=false
    local no_cache=false

    # Parse options
    shift 2 2>/dev/null || shift $# 2>/dev/null

    while [ $# -gt 0 ]; do
        case "$1" in
            --keep)
                keep_container=true
                ;;
            --no-cache)
                no_cache=true
                ;;
            --verbose)
                set -x
                ;;
            --help|-h)
                usage
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
        shift
    done

    # Check Docker is available
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed or not in PATH"
        echo "Install Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi

    log_section "HORUS Shell Script Tests"

    # Handle "all-distros" option
    if [ "$distro" = "all-distros" ]; then
        local overall_status=0

        for d in "${AVAILABLE_DISTROS[@]}"; do
            log_section "Testing on $d"

            build_image "$d" "$no_cache" || { overall_status=1; continue; }

            if [ "$test_type" = "all" ]; then
                run_full_flow "$d" "$keep_container" || overall_status=1
            elif [ "$test_type" = "full-flow" ]; then
                run_full_flow "$d" "$keep_container" || overall_status=1
            else
                run_test_in_container "$test_type" "$d" "$keep_container" || overall_status=1
            fi
        done

        exit $overall_status
    fi

    # Single distro testing
    build_image "$distro" "$no_cache" || exit 1

    if [ "$test_type" = "all" ] || [ "$test_type" = "full-flow" ]; then
        run_full_flow "$distro" "$keep_container"
        exit $?
    else
        run_test_in_container "$test_type" "$distro" "$keep_container"
        exit $?
    fi
}

# Run main
main "$@"
