#!/bin/bash
# HORUS Installation Verification Script v2.0.0
# Comprehensive, systematic, complete verification
# Checks: System, Binaries, Libraries, Cache, Source, Examples, Network, GPU

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Source shared dependency functions
if [ -f "$SCRIPT_DIR/scripts/deps.sh" ]; then
    source "$SCRIPT_DIR/scripts/deps.sh"
    DEPS_SOURCED=true
else
    DEPS_SOURCED=false
    # Fallback OS detection
    OS_TYPE="unknown"
    OS_DISTRO="unknown"
    case "$(uname -s)" in
        Linux*) OS_TYPE="linux" ;;
        Darwin*) OS_TYPE="macos" ;;
    esac
fi

# Symbols
CHECK="${GREEN}✓${NC}"
CROSS="${RED}✗${NC}"
WARN="${YELLOW}!${NC}"
INFO="${CYAN}i${NC}"
SKIP="${WHITE}-${NC}"

# Statistics
ERRORS=0
WARNINGS=0
PASSES=0
SKIPPED=0

# Helper functions
pass() {
    echo -e "  [$CHECK] $1"
    PASSES=$((PASSES + 1))
}

fail() {
    echo -e "  [$CROSS] $1"
    ERRORS=$((ERRORS + 1))
}

warn() {
    echo -e "  [$WARN] $1"
    WARNINGS=$((WARNINGS + 1))
}

info() {
    echo -e "  [$INFO] $1"
}

skip() {
    echo -e "  [$SKIP] $1 (skipped)"
    SKIPPED=$((SKIPPED + 1))
}

section() {
    echo ""
    echo -e "${MAGENTA}━━━ $1 ━━━${NC}"
    echo ""
}

# Paths
INSTALL_DIR="$HOME/.cargo/bin"
HORUS_DIR="$HOME/.horus"
CACHE_DIR="$HORUS_DIR/cache"
TARGET_DIR="$HORUS_DIR/target"

# Shared memory location
if [ "$OS_TYPE" = "linux" ]; then
    SHM_DIR="/dev/shm/horus"
else
    SHM_DIR="/tmp/horus"
fi

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║${NC}       ${WHITE}HORUS Installation Verification v2.0.0${NC}             ${BLUE}║${NC}"
echo -e "${BLUE}║${NC}       ${CYAN}Comprehensive • Systematic • Complete${NC}              ${BLUE}║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

#=====================================
# 1. SYSTEM REQUIREMENTS
#=====================================
section "1. System Requirements"

# OS Detection
info "Operating System: $OS_TYPE"
[ -n "$OS_DISTRO" ] && [ "$OS_DISTRO" != "unknown" ] && info "Distribution: $OS_DISTRO"

# Rust
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version | awk '{print $2}')
    RUST_MAJOR=$(echo $RUST_VERSION | cut -d'.' -f1)
    RUST_MINOR=$(echo $RUST_VERSION | cut -d'.' -f2)

    if [ "$RUST_MAJOR" -eq 1 ] && [ "$RUST_MINOR" -ge 70 ]; then
        pass "Rust $RUST_VERSION (>= 1.70 required)"
    else
        warn "Rust $RUST_VERSION (< 1.70, please update via rustup update)"
    fi
else
    fail "Rust not installed (https://rustup.rs/)"
fi

# Cargo
if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version | awk '{print $2}')
    pass "Cargo $CARGO_VERSION"
else
    fail "Cargo not found"
fi

# C Compiler
CC_FOUND=false
for cc_cmd in cc gcc clang; do
    if command -v $cc_cmd &> /dev/null; then
        CC_NAME=$($cc_cmd --version 2>/dev/null | head -n1 | cut -d' ' -f1-3)
        pass "C compiler: $CC_NAME"
        CC_FOUND=true
        break
    fi
done
[ "$CC_FOUND" = false ] && fail "C compiler not found (install gcc or clang)"

# pkg-config
if command -v pkg-config &> /dev/null; then
    pass "pkg-config $(pkg-config --version)"
else
    fail "pkg-config not found"
fi

# cmake (needed for some deps)
if command -v cmake &> /dev/null; then
    CMAKE_VERSION=$(cmake --version | head -n1 | awk '{print $3}')
    pass "cmake $CMAKE_VERSION"
else
    warn "cmake not found (may be needed for some builds)"
fi

# Python3 (for horus_py)
if command -v python3 &> /dev/null; then
    PY_VERSION=$(python3 --version 2>&1 | awk '{print $2}')
    pass "Python $PY_VERSION"
else
    info "Python3 not found (optional for horus_py)"
fi

#=====================================
# 2. SYSTEM LIBRARIES
#=====================================
section "2. System Libraries"

if [ "$DEPS_SOURCED" = true ]; then
    print_dep_status
    DEP_FAILURES=$?
    if [ $DEP_FAILURES -gt 0 ]; then
        ERRORS=$((ERRORS + DEP_FAILURES))
    fi
else
    # Fallback: Basic library checks
    declare -a LIBS=(
        "openssl:OpenSSL:required"
        "libudev:udev:linux"
        "alsa:ALSA:linux"
        "wayland-client:Wayland:linux"
        "xkbcommon:XKB:linux"
        "x11:X11:linux"
        "vulkan:Vulkan:optional"
    )

    for lib_info in "${LIBS[@]}"; do
        IFS=':' read -r lib desc req <<< "$lib_info"

        [ "$req" = "linux" ] && [ "$OS_TYPE" != "linux" ] && continue

        if pkg-config --exists "$lib" 2>/dev/null; then
            VERSION=$(pkg-config --modversion "$lib" 2>/dev/null || echo "installed")
            pass "$desc $VERSION"
        else
            if [ "$req" = "required" ]; then
                fail "$desc not found (REQUIRED)"
            elif [ "$req" = "optional" ]; then
                info "$desc not found (optional)"
            else
                warn "$desc not found (may affect build)"
            fi
        fi
    done
fi

#=====================================
# 3. HORUS BINARIES
#=====================================
section "3. HORUS Binaries"

# Main horus binary
if [ -x "$INSTALL_DIR/horus" ]; then
    if "$INSTALL_DIR/horus" --version &>/dev/null; then
        VERSION=$("$INSTALL_DIR/horus" --version 2>/dev/null | awk '{print $2}')
        pass "horus binary v$VERSION"

        # Check PATH
        if command -v horus &>/dev/null; then
            WHICH_HORUS=$(which horus)
            if [ "$WHICH_HORUS" = "$INSTALL_DIR/horus" ]; then
                pass "horus in PATH (correct location)"
            else
                warn "horus in PATH but different location: $WHICH_HORUS"
            fi
        else
            warn "horus not in PATH (add $INSTALL_DIR to PATH)"
        fi
    else
        fail "horus binary installed but not working"
    fi
else
    fail "horus binary not found at $INSTALL_DIR/horus"
fi

# sim2d binary
if [ -x "$INSTALL_DIR/sim2d" ]; then
    if "$INSTALL_DIR/sim2d" --help &>/dev/null; then
        pass "sim2d binary installed"
    else
        fail "sim2d binary installed but not working"
    fi
else
    fail "sim2d binary not found at $INSTALL_DIR/sim2d"
fi

# sim3d binary
if [ -x "$INSTALL_DIR/sim3d" ]; then
    if "$INSTALL_DIR/sim3d" --help &>/dev/null; then
        pass "sim3d binary installed"
    else
        fail "sim3d binary installed but not working"
    fi
else
    fail "sim3d binary not found at $INSTALL_DIR/sim3d"
fi

# horus_router binary (optional)
if [ -x "$INSTALL_DIR/horus_router" ]; then
    pass "horus_router binary installed"
else
    info "horus_router binary not found (optional)"
fi

#=====================================
# 4. HORUS LIBRARY CACHE
#=====================================
section "4. Library Cache"

if [ -d "$CACHE_DIR" ]; then
    pass "Cache directory exists: $CACHE_DIR"

    # Check installed version
    VERSION_FILE="$HORUS_DIR/installed_version"
    if [ -f "$VERSION_FILE" ]; then
        INSTALLED_VERSION=$(cat "$VERSION_FILE")
        info "Installed version: $INSTALLED_VERSION"
    else
        warn "Version file missing: $VERSION_FILE"
    fi

    # Core components (REQUIRED)
    declare -a CORE_COMPONENTS=(
        "horus:Main library"
        "horus_core:Runtime core"
        "horus_macros:Proc macros"
        "horus_library:Standard library"
    )

    for comp_info in "${CORE_COMPONENTS[@]}"; do
        IFS=':' read -r comp desc <<< "$comp_info"

        if ls "$CACHE_DIR"/${comp}@* 1>/dev/null 2>&1; then
            COMP_DIR=$(ls -d "$CACHE_DIR"/${comp}@* 2>/dev/null | head -n1)
            COMP_VERSION=$(basename "$COMP_DIR" | sed "s/${comp}@//")

            # Check for source files
            if [ -d "$COMP_DIR/src" ]; then
                pass "$desc v$COMP_VERSION (with source)"
            else
                warn "$desc v$COMP_VERSION (source missing)"
            fi

            # Check for Cargo.toml
            if [ ! -f "$COMP_DIR/Cargo.toml" ]; then
                warn "$desc missing Cargo.toml"
            fi
        else
            fail "$desc not cached"
        fi
    done

    # Additional components
    declare -a EXTRA_COMPONENTS=(
        "horus_manager:Manager"
        "horus_router:Router"
        "horus_py:Python bindings"
    )

    for comp_info in "${EXTRA_COMPONENTS[@]}"; do
        IFS=':' read -r comp desc <<< "$comp_info"

        if ls "$CACHE_DIR"/${comp}@* 1>/dev/null 2>&1; then
            COMP_DIR=$(ls -d "$CACHE_DIR"/${comp}@* 2>/dev/null | head -n1)
            COMP_VERSION=$(basename "$COMP_DIR" | sed "s/${comp}@//")
            pass "$desc v$COMP_VERSION"
        else
            info "$desc not cached (optional)"
        fi
    done
else
    fail "Cache directory missing: $CACHE_DIR"
fi

#=====================================
# 5. PRE-COMPILED DEPENDENCIES
#=====================================
section "5. Pre-compiled Dependencies"

if [ -d "$TARGET_DIR" ]; then
    pass "Target directory exists: $TARGET_DIR"

    # Check for release deps
    RELEASE_DEPS="$TARGET_DIR/release/deps"
    if [ -d "$RELEASE_DEPS" ]; then
        # Count rlib files
        RLIB_COUNT=$(ls "$RELEASE_DEPS"/*.rlib 2>/dev/null | wc -l)
        if [ "$RLIB_COUNT" -gt 0 ]; then
            pass "Pre-compiled rlibs: $RLIB_COUNT files"
        else
            warn "No pre-compiled rlib files found"
        fi

        # Count rmeta files
        RMETA_COUNT=$(ls "$RELEASE_DEPS"/*.rmeta 2>/dev/null | wc -l)
        if [ "$RMETA_COUNT" -gt 0 ]; then
            pass "Pre-compiled rmeta: $RMETA_COUNT files"
        else
            warn "No pre-compiled rmeta files found"
        fi

        # Check for HORUS-specific libs
        HORUS_RLIBS=$(ls "$RELEASE_DEPS"/libhorus*.rlib 2>/dev/null | wc -l)
        if [ "$HORUS_RLIBS" -gt 0 ]; then
            pass "HORUS library artifacts: $HORUS_RLIBS files"
        else
            warn "HORUS library artifacts not found"
        fi
    else
        warn "Release deps directory missing"
    fi

    # Check fingerprints
    FINGERPRINT_DIR="$TARGET_DIR/release/.fingerprint"
    if [ -d "$FINGERPRINT_DIR" ]; then
        FP_COUNT=$(ls -d "$FINGERPRINT_DIR"/horus* 2>/dev/null | wc -l)
        if [ "$FP_COUNT" -gt 0 ]; then
            pass "Build fingerprints: $FP_COUNT HORUS entries"
        else
            warn "No HORUS fingerprints found"
        fi
    else
        warn "Fingerprint directory missing"
    fi
else
    fail "Target directory missing: $TARGET_DIR"
    info "Run install.sh to create pre-compiled dependencies"
fi

#=====================================
# 6. SHARED MEMORY
#=====================================
section "6. Shared Memory"

# Check if shared memory directory is accessible
if [ "$OS_TYPE" = "linux" ]; then
    if [ -d "/dev/shm" ]; then
        pass "Shared memory available: /dev/shm"

        # Check permissions
        if [ -w "/dev/shm" ]; then
            pass "Shared memory writable"
        else
            fail "Shared memory not writable"
        fi

        # Check if HORUS session can be created
        TEST_SHM="/dev/shm/horus_verify_test_$$"
        if mkdir -p "$TEST_SHM" 2>/dev/null; then
            pass "Can create HORUS sessions"
            rmdir "$TEST_SHM" 2>/dev/null
        else
            fail "Cannot create HORUS sessions in /dev/shm"
        fi

        # Show existing HORUS sessions
        if [ -d "$SHM_DIR" ]; then
            SESSION_COUNT=$(ls -d "$SHM_DIR"/* 2>/dev/null | wc -l)
            info "Active HORUS sessions: $SESSION_COUNT"
        fi
    else
        fail "/dev/shm not available"
    fi
else
    # macOS fallback
    if [ -d "/tmp" ] && [ -w "/tmp" ]; then
        pass "Shared memory fallback: /tmp"
    else
        fail "/tmp not writable for shared memory"
    fi
fi

#=====================================
# 7. COMMAND FUNCTIONALITY
#=====================================
section "7. Command Functionality"

if command -v horus &>/dev/null || [ -x "$INSTALL_DIR/horus" ]; then
    HORUS_CMD="${INSTALL_DIR}/horus"

    # Core commands
    declare -a COMMANDS=(
        "--version:Version check"
        "--help:Help system"
        "new --help:Project creation"
        "run --help:Project runner"
        "pkg --help:Package manager"
        "env --help:Environment"
        "dashboard --help:Dashboard"
        "auth --help:Authentication"
        "sim2d --help:2D Simulator"
        "sim3d --help:3D Simulator"
        "monitor --help:Monitor"
    )

    ALL_OK=true
    for cmd_info in "${COMMANDS[@]}"; do
        IFS=':' read -r cmd desc <<< "$cmd_info"

        if $HORUS_CMD $cmd &>/dev/null; then
            pass "horus $cmd"
        else
            fail "horus $cmd failed"
            ALL_OK=false
        fi
    done
else
    skip "Command tests (horus not available)"
fi

#=====================================
# 8. EXAMPLES & TEMPLATES
#=====================================
section "8. Examples & Templates"

# Check for examples in cache
if [ -d "$CACHE_DIR" ]; then
    # snakesim
    if ls "$CACHE_DIR"/snakesim* 1>/dev/null 2>&1; then
        pass "snakesim example available"
    else
        info "snakesim example not cached"
    fi

    # wallesim
    if ls "$CACHE_DIR"/wallesim* 1>/dev/null 2>&1; then
        pass "wallesim example available"
    else
        info "wallesim example not cached"
    fi

    # simple_driver
    if ls "$CACHE_DIR"/simple_driver* 1>/dev/null 2>&1; then
        pass "simple_driver example available"
    else
        info "simple_driver example not cached"
    fi
fi

# Check for templates
TEMPLATES_DIR="$HORUS_DIR/templates"
if [ -d "$TEMPLATES_DIR" ]; then
    TEMPLATE_COUNT=$(ls "$TEMPLATES_DIR" 2>/dev/null | wc -l)
    if [ "$TEMPLATE_COUNT" -gt 0 ]; then
        pass "Project templates: $TEMPLATE_COUNT available"
    else
        info "No project templates found"
    fi
else
    info "Templates directory not found"
fi

#=====================================
# 9. SHELL COMPLETIONS
#=====================================
section "9. Shell Completions"

# Bash completions
if [ -f "$HOME/.bash_completion.d/horus" ] || [ -f "/etc/bash_completion.d/horus" ] || [ -f "$HORUS_DIR/completions/horus.bash" ]; then
    pass "Bash completions installed"
else
    info "Bash completions not installed (optional)"
fi

# Zsh completions
if [ -f "$HOME/.zsh/completions/_horus" ] || [ -f "/usr/share/zsh/site-functions/_horus" ] || [ -f "$HORUS_DIR/completions/horus.zsh" ]; then
    pass "Zsh completions installed"
else
    info "Zsh completions not installed (optional)"
fi

# Fish completions
if [ -f "$HOME/.config/fish/completions/horus.fish" ] || [ -f "/usr/share/fish/completions/horus.fish" ] || [ -f "$HORUS_DIR/completions/horus.fish" ]; then
    pass "Fish completions installed"
else
    info "Fish completions not installed (optional)"
fi

#=====================================
# 10. GPU & GRAPHICS (for sim3d)
#=====================================
section "10. GPU & Graphics (sim3d)"

# Check Vulkan
if command -v vulkaninfo &>/dev/null; then
    if vulkaninfo --summary 2>/dev/null | grep -q "GPU"; then
        GPU_NAME=$(vulkaninfo --summary 2>/dev/null | grep "GPU" | head -n1 | sed 's/.*GPU.*: //')
        pass "Vulkan available: $GPU_NAME"
    else
        warn "Vulkan installed but no GPU detected"
    fi
elif pkg-config --exists vulkan 2>/dev/null; then
    info "Vulkan SDK installed (vulkaninfo not available)"
else
    info "Vulkan not detected (required for sim3d graphics)"
fi

# Check for display
if [ -n "$DISPLAY" ] || [ -n "$WAYLAND_DISPLAY" ]; then
    if [ -n "$WAYLAND_DISPLAY" ]; then
        pass "Display available: Wayland ($WAYLAND_DISPLAY)"
    else
        pass "Display available: X11 ($DISPLAY)"
    fi
else
    info "No display detected (headless mode only)"
fi

# Check OpenGL (fallback renderer)
if command -v glxinfo &>/dev/null; then
    GL_VERSION=$(glxinfo 2>/dev/null | grep "OpenGL version" | head -n1 | sed 's/.*: //')
    if [ -n "$GL_VERSION" ]; then
        pass "OpenGL: $GL_VERSION"
    else
        info "OpenGL: Unable to query version"
    fi
else
    info "glxinfo not available (OpenGL status unknown)"
fi

#=====================================
# 11. NETWORK & REGISTRY
#=====================================
section "11. Network & Registry"

# Check network connectivity to crates.io (for cargo)
if curl -s --connect-timeout 5 https://crates.io/api/v1/crates/serde 2>/dev/null | grep -q "serde"; then
    pass "crates.io accessible"
else
    warn "crates.io not accessible (offline mode required)"
fi

# Check HORUS registry (if configured)
REGISTRY_URL="https://registry.horus.dev"
if curl -s --connect-timeout 5 "$REGISTRY_URL" 2>/dev/null | head -c 100 | grep -qi "horus\|registry\|html"; then
    pass "HORUS registry accessible"
else
    info "HORUS registry not accessible (local-only mode)"
fi

# Check GitHub (for updates)
if curl -s --connect-timeout 5 https://api.github.com/rate_limit &>/dev/null; then
    pass "GitHub API accessible"
else
    info "GitHub API not accessible"
fi

#=====================================
# 12. DISK USAGE
#=====================================
section "12. Disk Usage"

if [ -d "$HORUS_DIR" ]; then
    HORUS_SIZE=$(du -sh "$HORUS_DIR" 2>/dev/null | awk '{print $1}')
    info "~/.horus total: $HORUS_SIZE"

    if [ -d "$CACHE_DIR" ]; then
        CACHE_SIZE=$(du -sh "$CACHE_DIR" 2>/dev/null | awk '{print $1}')
        info "  Library cache: $CACHE_SIZE"
    fi

    if [ -d "$TARGET_DIR" ]; then
        TARGET_SIZE=$(du -sh "$TARGET_DIR" 2>/dev/null | awk '{print $1}')
        info "  Pre-compiled: $TARGET_SIZE"
    fi
fi

CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
if [ -d "$CARGO_HOME/registry" ]; then
    REGISTRY_SIZE=$(du -sh "$CARGO_HOME/registry" 2>/dev/null | awk '{print $1}')
    info "Cargo registry cache: $REGISTRY_SIZE"
fi

# Check available disk space
AVAILABLE=$(df -h "$HOME" 2>/dev/null | tail -1 | awk '{print $4}')
info "Available disk space: $AVAILABLE"

#=====================================
# 13. BUILD VERIFICATION (if in repo)
#=====================================
section "13. Build Verification"

if [ -f "$SCRIPT_DIR/Cargo.toml" ] && grep -q "horus_manager" "$SCRIPT_DIR/Cargo.toml" 2>/dev/null; then
    info "Running in HORUS source repository"

    # Quick cargo check
    info "Running cargo check..."
    if cargo check --quiet 2>&1 | grep -q "error:"; then
        fail "cargo check has errors"
    else
        pass "cargo check passes"
    fi

    # Check for local debug build
    if [ -x "$SCRIPT_DIR/target/debug/horus" ]; then
        if "$SCRIPT_DIR/target/debug/horus" --version &>/dev/null; then
            pass "Debug build functional"
        else
            warn "Debug build exists but not working"
        fi
    fi

    # Check for local release build
    if [ -x "$SCRIPT_DIR/target/release/horus" ]; then
        if "$SCRIPT_DIR/target/release/horus" --version &>/dev/null; then
            pass "Release build functional"
        else
            warn "Release build exists but not working"
        fi
    fi
else
    skip "Build verification (not in HORUS repo)"
fi

#=====================================
# SUMMARY
#=====================================
echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║${NC}                      ${WHITE}VERIFICATION SUMMARY${NC}                   ${BLUE}║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

TOTAL=$((PASSES + ERRORS + WARNINGS))

echo -e "  ${GREEN}Passed:${NC}   $PASSES"
echo -e "  ${RED}Errors:${NC}   $ERRORS"
echo -e "  ${YELLOW}Warnings:${NC} $WARNINGS"
echo -e "  ${WHITE}Skipped:${NC}  $SKIPPED"
echo ""

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "  ${GREEN}════════════════════════════════════════${NC}"
    echo -e "  ${GREEN}  ✓ HORUS installation is PERFECT!${NC}"
    echo -e "  ${GREEN}════════════════════════════════════════${NC}"
    echo ""
    echo -e "  Ready to use: ${CYAN}horus new my_project${NC}"
    EXIT_CODE=0
elif [ $ERRORS -eq 0 ]; then
    echo -e "  ${YELLOW}════════════════════════════════════════${NC}"
    echo -e "  ${YELLOW}  ! HORUS is functional with warnings${NC}"
    echo -e "  ${YELLOW}════════════════════════════════════════${NC}"
    echo ""
    echo -e "  Review warnings above for optional improvements."
    EXIT_CODE=1
else
    echo -e "  ${RED}════════════════════════════════════════${NC}"
    echo -e "  ${RED}  ✗ HORUS installation has ERRORS${NC}"
    echo -e "  ${RED}════════════════════════════════════════${NC}"
    echo ""
    echo -e "  ${CYAN}Recommended fixes:${NC}"
    echo ""
    echo -e "    1. Clean reinstall:"
    echo -e "       ${CYAN}./install.sh${NC}"
    echo ""
    echo -e "    2. If system dependencies missing:"
    if [ "$OS_TYPE" = "linux" ]; then
        echo -e "       ${CYAN}sudo apt-get install gcc libc6-dev pkg-config${NC}"
        echo -e "       ${CYAN}sudo apt-get install libssl-dev libudev-dev libasound2-dev${NC}"
    elif [ "$OS_TYPE" = "macos" ]; then
        echo -e "       ${CYAN}xcode-select --install${NC}"
        echo -e "       ${CYAN}brew install openssl pkg-config${NC}"
    fi
    echo ""
    echo -e "    3. If binaries not working:"
    echo -e "       ${CYAN}cargo clean && cargo build --release${NC}"
    echo ""
    EXIT_CODE=2
fi

echo ""
echo -e "  Verification completed at $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

exit $EXIT_CODE
