#!/bin/bash
# HORUS Installation Verification Script
# Check installation health and diagnose issues

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
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
CHECK="${GREEN}[+]${NC}"
CROSS="${RED}[x]${NC}"
WARN="${YELLOW}[!]${NC}"
INFO="${CYAN}[i]${NC}"

echo -e "${BLUE}========================================${NC}"
echo -e "${CYAN}   HORUS Installation Verification${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

ERRORS=0
WARNINGS=0

#=====================================
# System Requirements
#=====================================
echo -e "${MAGENTA}System Requirements:${NC}"
echo ""

# OS Detection
echo -e "  ${INFO} OS: $OS_TYPE ($OS_DISTRO)"

# Rust
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version | awk '{print $2}')
    RUST_MAJOR=$(echo $RUST_VERSION | cut -d'.' -f1)
    RUST_MINOR=$(echo $RUST_VERSION | cut -d'.' -f2)

    if [ "$RUST_MAJOR" -eq 1 ] && [ "$RUST_MINOR" -ge 70 ]; then
        echo -e "  $CHECK Rust: $RUST_VERSION (>= 1.70 required)"
    else
        echo -e "  $WARN Rust: $RUST_VERSION (< 1.70, please update)"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    echo -e "  $CROSS Rust: Not installed"
    echo -e "      Install from: ${CYAN}https://rustup.rs/${NC}"
    ERRORS=$((ERRORS + 1))
fi

# Cargo
if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version | awk '{print $2}')
    echo -e "  $CHECK Cargo: $CARGO_VERSION"
else
    echo -e "  $CROSS Cargo: Not found"
    ERRORS=$((ERRORS + 1))
fi

# C Compiler
if command -v cc &> /dev/null; then
    CC_NAME=$(cc --version 2>/dev/null | head -n1 | cut -d' ' -f1-3)
    echo -e "  $CHECK C compiler: $CC_NAME"
elif command -v gcc &> /dev/null; then
    GCC_VERSION=$(gcc --version | head -n1)
    echo -e "  $CHECK C compiler: $GCC_VERSION"
elif command -v clang &> /dev/null; then
    CLANG_VERSION=$(clang --version | head -n1)
    echo -e "  $CHECK C compiler: $CLANG_VERSION"
else
    echo -e "  $CROSS C compiler: Not found"
    ERRORS=$((ERRORS + 1))
fi

# pkg-config
if command -v pkg-config &> /dev/null; then
    PKG_VERSION=$(pkg-config --version)
    echo -e "  $CHECK pkg-config: $PKG_VERSION"
else
    echo -e "  $CROSS pkg-config: Not found"
    ERRORS=$((ERRORS + 1))
fi

echo ""

#=====================================
# System Libraries (Full Check)
#=====================================
echo -e "${MAGENTA}System Libraries:${NC}"
echo ""

# Use shared deps.sh if available, otherwise fallback to basic checks
if [ "$DEPS_SOURCED" = true ]; then
    # Use comprehensive check from deps.sh
    print_dep_status
    DEP_FAILURES=$?
    if [ $DEP_FAILURES -gt 0 ]; then
        ERRORS=$((ERRORS + DEP_FAILURES))
    fi
else
    # Fallback: Basic library checks
    declare -a LIBS=(
        "openssl:OpenSSL:required"
        "libudev:udev (device management):linux"
        "alsa:ALSA (audio):linux"
        "wayland-client:Wayland client:linux"
        "wayland-cursor:Wayland cursor:linux"
        "xkbcommon:XKB common:linux"
        "x11:X11:linux"
        "xrandr:Xrandr:linux"
        "xi:Xi (input):linux"
        "xcursor:Xcursor:linux"
        "xinerama:Xinerama:linux"
    )

    for lib_info in "${LIBS[@]}"; do
        IFS=':' read -r lib desc req <<< "$lib_info"

        # Skip linux-only on non-Linux
        if [ "$req" = "linux" ] && [ "$OS_TYPE" != "linux" ]; then
            continue
        fi

        if pkg-config --exists "$lib" 2>/dev/null; then
            VERSION=$(pkg-config --modversion "$lib" 2>/dev/null || echo "installed")
            echo -e "  $CHECK $desc: $VERSION"
        else
            if [ "$req" = "required" ]; then
                echo -e "  $CROSS $desc: Not found (REQUIRED)"
                ERRORS=$((ERRORS + 1))
            else
                echo -e "  $WARN $desc: Not found (may cause build issues)"
                WARNINGS=$((WARNINGS + 1))
            fi
        fi
    done

    # Check for libclang specifically
    if [ "$OS_TYPE" = "macos" ]; then
        if xcode-select -p &>/dev/null; then
            echo -e "  $CHECK libclang: Xcode tools installed"
        else
            echo -e "  $CROSS libclang: Xcode tools not found"
            ERRORS=$((ERRORS + 1))
        fi
    else
        if ldconfig -p 2>/dev/null | grep -q libclang || [ -f /usr/lib/llvm-*/lib/libclang.so ]; then
            echo -e "  $CHECK libclang: installed"
        else
            echo -e "  $WARN libclang: Not found (required for some bindings)"
            WARNINGS=$((WARNINGS + 1))
        fi
    fi
fi

echo ""

#=====================================
# HORUS Installation
#=====================================
echo -e "${MAGENTA}HORUS Installation:${NC}"
echo ""

INSTALL_DIR="$HOME/.cargo/bin"
HORUS_DIR="$HOME/.horus"

# Binary
if [ -x "$INSTALL_DIR/horus" ]; then
    if "$INSTALL_DIR/horus" --version &>/dev/null; then
        VERSION=$("$INSTALL_DIR/horus" --version 2>/dev/null | awk '{print $2}')
        echo -e "  $CHECK Binary: v$VERSION at $INSTALL_DIR/horus"

        # Check if it's the one in PATH
        if command -v horus &>/dev/null; then
            WHICH_HORUS=$(which horus)
            if [ "$WHICH_HORUS" = "$INSTALL_DIR/horus" ]; then
                echo -e "  $CHECK In PATH: Yes (correct binary)"
            else
                echo -e "  $WARN In PATH: Yes (different binary: $WHICH_HORUS)"
                WARNINGS=$((WARNINGS + 1))
            fi
        else
            echo -e "  $WARN In PATH: No"
            echo -e "      Add to shell profile: ${CYAN}export PATH=\"\$HOME/.cargo/bin:\$PATH\"${NC}"
            WARNINGS=$((WARNINGS + 1))
        fi
    else
        echo -e "  $CROSS Binary: Installed but not working"
        ERRORS=$((ERRORS + 1))
    fi
else
    echo -e "  $CROSS Binary: Not installed"
    ERRORS=$((ERRORS + 1))
fi

echo ""

# Core Libraries
echo -e "${MAGENTA}Core Libraries:${NC}"
echo ""

CACHE_DIR="$HORUS_DIR/cache"
if [ -d "$CACHE_DIR" ]; then
    # Get installed version
    VERSION_FILE="$HORUS_DIR/installed_version"
    if [ -f "$VERSION_FILE" ]; then
        INSTALLED_VERSION=$(cat "$VERSION_FILE")
        echo -e "  ${INFO} Installed version: $INSTALLED_VERSION"
    fi

    # Check each component
    declare -a COMPONENTS=(
        "horus:Main library"
        "horus_core:Runtime core"
        "horus_macros:Proc macros"
        "horus_library:Standard library"
        "horus_c:C bindings"
        "horus_py:Python bindings"
    )

    for comp_info in "${COMPONENTS[@]}"; do
        IFS=':' read -r comp desc <<< "$comp_info"

        if ls "$CACHE_DIR"/${comp}@* 1>/dev/null 2>&1; then
            COMP_DIR=$(ls -d "$CACHE_DIR"/${comp}@* 2>/dev/null | head -n1)
            COMP_VERSION=$(basename "$COMP_DIR" | sed "s/${comp}@//")
            echo -e "  $CHECK $desc: v$COMP_VERSION"
        else
            if [ "$comp" = "horus_c" ] || [ "$comp" = "horus_py" ]; then
                echo -e "  $INFO $desc: Not installed (optional)"
            else
                echo -e "  $CROSS $desc: Not installed"
                ERRORS=$((ERRORS + 1))
            fi
        fi
    done
else
    echo -e "  $CROSS Library cache not found: $CACHE_DIR"
    ERRORS=$((ERRORS + 1))
fi

echo ""

#=====================================
# Functionality Tests
#=====================================
echo -e "${MAGENTA}Functionality Tests:${NC}"
echo ""

if [ -x "$INSTALL_DIR/horus" ]; then
    # Test --version
    if "$INSTALL_DIR/horus" --version &>/dev/null; then
        echo -e "  $CHECK Command: --version"
    else
        echo -e "  $CROSS Command: --version failed"
        ERRORS=$((ERRORS + 1))
    fi

    # Test --help
    if "$INSTALL_DIR/horus" --help &>/dev/null; then
        echo -e "  $CHECK Command: --help"
    else
        echo -e "  $CROSS Command: --help failed"
        ERRORS=$((ERRORS + 1))
    fi

    # Test key subcommands exist
    declare -a SUBCOMMANDS=("new" "run" "dashboard" "pkg" "env" "auth" "version")
    ALL_CMDS_OK=true
    for subcmd in "${SUBCOMMANDS[@]}"; do
        if "$INSTALL_DIR/horus" "$subcmd" --help &>/dev/null; then
            :  # Success, do nothing
        else
            echo -e "  $CROSS Command: horus $subcmd --help failed"
            ERRORS=$((ERRORS + 1))
            ALL_CMDS_OK=false
        fi
    done
    if [ "$ALL_CMDS_OK" = true ]; then
        echo -e "  $CHECK All subcommands: Accessible"
    fi

    # Test cargo build in HORUS source (if we're in the repo)
    if [ -f "Cargo.toml" ] && grep -q "horus_manager" "Cargo.toml" 2>/dev/null; then
        echo ""
        echo -e "  ${INFO} Running build verification..."

        # Test cargo check (fast, no warnings)
        if cargo check --quiet 2>&1 | grep -q "error:"; then
            echo -e "  $CROSS Build: cargo check failed"
            ERRORS=$((ERRORS + 1))
        else
            WARNING_COUNT=$(cargo check 2>&1 | grep "warning:" | grep -v "profiles for the non root" | wc -l)
            if [ "$WARNING_COUNT" -eq 0 ]; then
                echo -e "  $CHECK Build: cargo check passes (0 warnings)"
            else
                echo -e "  $WARN Build: cargo check has $WARNING_COUNT warning(s)"
                WARNINGS=$((WARNINGS + 1))
            fi
        fi

        # Verify binary still works after build
        if [ -x "./target/debug/horus" ]; then
            if ./target/debug/horus --version &>/dev/null; then
                echo -e "  $CHECK Binary: Debug build functional"
            else
                echo -e "  $CROSS Binary: Debug build not working"
                ERRORS=$((ERRORS + 1))
            fi
        fi
    fi
else
    echo -e "  $WARN Cannot run functionality tests (binary not working)"
fi

echo ""

#=====================================
# Optional Features
#=====================================
echo -e "${MAGENTA}Optional Features:${NC}"
echo ""

# Python bindings
if python3 -c "import sys; sys.path.insert(0, '$HORUS_DIR/cache'); import horus" 2>/dev/null; then
    PY_VERSION=$(python3 --version | awk '{print $2}')
    echo -e "  $CHECK Python bindings: Working (Python $PY_VERSION)"
else
    echo -e "  $INFO Python bindings: Not installed or not working"
fi

# C bindings
if [ -f "$HORUS_DIR/cache/horus_c@"*/lib/libhorus_c.so ] || [ -f "$HORUS_DIR/cache/horus_c@"*/lib/libhorus_c.dylib ]; then
    echo -e "  $CHECK C bindings: Installed"
else
    echo -e "  $INFO C bindings: Not installed"
fi

echo ""

#=====================================
# Disk Usage
#=====================================
echo -e "${MAGENTA}Disk Usage:${NC}"
echo ""

if [ -d "$HORUS_DIR" ]; then
    HORUS_SIZE=$(du -sh "$HORUS_DIR" 2>/dev/null | awk '{print $1}')
    echo -e "  ${INFO} ~/.horus: $HORUS_SIZE"

    if [ -d "$CACHE_DIR" ]; then
        CACHE_SIZE=$(du -sh "$CACHE_DIR" 2>/dev/null | awk '{print $1}')
        echo -e "  ${INFO} Library cache: $CACHE_SIZE"
    fi
fi

CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
if [ -d "$CARGO_HOME/registry" ]; then
    REGISTRY_SIZE=$(du -sh "$CARGO_HOME/registry" 2>/dev/null | awk '{print $1}')
    echo -e "  ${INFO} Cargo cache: $REGISTRY_SIZE"
fi

echo ""

#=====================================
# Summary
#=====================================
echo -e "${BLUE}========================================${NC}"
echo -e "${CYAN}Summary:${NC}"
echo ""

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "  ${GREEN}Perfect! Everything looks good.${NC}"
    echo ""
    echo -e "  HORUS is properly installed and ready to use."
    EXIT_CODE=0
elif [ $ERRORS -eq 0 ]; then
    echo -e "  ${YELLOW}$WARNINGS warning(s) found${NC}"
    echo ""
    echo -e "  HORUS is installed but with minor issues."
    echo -e "  Review warnings above for optional improvements."
    EXIT_CODE=1
else
    echo -e "  ${RED}$ERRORS error(s), $WARNINGS warning(s) found${NC}"
    echo ""
    echo -e "  HORUS installation has problems."
    echo ""
    echo -e "  ${CYAN}Recommended actions:${NC}"

    # Check if it's a system dependency issue
    if [ "$DEPS_SOURCED" = true ]; then
        MISSING_DEPS=$(check_all_deps)
        if [ -n "$MISSING_DEPS" ]; then
            echo -e "    1. Install missing system dependencies:"
            echo -e "       ${CYAN}./install.sh${NC}  (will auto-install deps)"
            echo ""
            echo -e "    Or manually install:"
            case "$OS_DISTRO" in
                debian-based)
                    echo -e "       ${CYAN}sudo apt-get install $(get_packages_for_os)${NC}"
                    ;;
                fedora-based)
                    echo -e "       ${CYAN}sudo dnf install $(get_packages_for_os)${NC}"
                    ;;
                arch-based)
                    echo -e "       ${CYAN}sudo pacman -S $(get_packages_for_os)${NC}"
                    ;;
            esac
            echo ""
        fi
    fi

    echo -e "    - Run: ${CYAN}cargo clean && rm -rf ~/.horus/cache && ./install.sh${NC}"
    EXIT_CODE=2
fi

echo -e "${BLUE}========================================${NC}"
echo ""

exit $EXIT_CODE
