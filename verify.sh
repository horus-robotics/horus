#!/bin/bash
# HORUS Installation Verification Script
# Check installation health and diagnose issues

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Symbols
CHECK="${GREEN}✓${NC}"
CROSS="${RED}✗${NC}"
WARN="${YELLOW}⚠${NC}"
INFO="${CYAN}→${NC}"

echo -e "${BLUE}════════════════════════════════════${NC}"
echo -e "${CYAN}   HORUS Installation Verification${NC}"
echo -e "${BLUE}════════════════════════════════════${NC}"
echo ""

ERRORS=0
WARNINGS=0

#=====================================
# System Requirements
#=====================================
echo -e "${MAGENTA}System Requirements:${NC}"
echo ""

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
    CC_NAME=$(cc --version | head -n1 | awk '{print $1}')
    echo -e "  $CHECK C compiler: $CC_NAME"
elif command -v gcc &> /dev/null; then
    echo -e "  $CHECK C compiler: GCC"
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
# System Libraries
#=====================================
echo -e "${MAGENTA}System Libraries:${NC}"
echo ""

# Check common libraries
declare -a LIBS=(
    "openssl:OpenSSL"
    "udev:udev (Linux device management)"
)

for lib_info in "${LIBS[@]}"; do
    IFS=':' read -r lib desc <<< "$lib_info"

    if pkg-config --exists "$lib" 2>/dev/null; then
        VERSION=$(pkg-config --modversion "$lib" 2>/dev/null || echo "installed")
        echo -e "  $CHECK $desc: $VERSION"
    else
        echo -e "  $WARN $desc: Not found (optional for some features)"
        WARNINGS=$((WARNINGS + 1))
    fi
done

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

    # List available commands
    COMMANDS=$("$INSTALL_DIR/horus" --help 2>/dev/null | grep -E "^  [a-z]" | awk '{print $1}' | sort)
    if [ -n "$COMMANDS" ]; then
        echo -e "  $CHECK Available commands:"
        echo "$COMMANDS" | while read -r cmd; do
            echo -e "      - $cmd"
        done
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
echo -e "${BLUE}════════════════════════════════════${NC}"
echo -e "${CYAN}Summary:${NC}"
echo ""

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "  ${GREEN}✅ Perfect! Everything looks good.${NC}"
    echo ""
    echo -e "  HORUS is properly installed and ready to use."
    EXIT_CODE=0
elif [ $ERRORS -eq 0 ]; then
    echo -e "  ${YELLOW}⚠  $WARNINGS warning(s) found${NC}"
    echo ""
    echo -e "  HORUS is installed but with minor issues."
    echo -e "  Review warnings above for optional improvements."
    EXIT_CODE=1
else
    echo -e "  ${RED}❌ $ERRORS error(s), $WARNINGS warning(s) found${NC}"
    echo ""
    echo -e "  HORUS installation has problems."
    echo ""
    echo -e "  ${CYAN}Recommended actions:${NC}"
    echo -e "    1. Review errors above"
    echo -e "    2. Run: ${CYAN}./recovery_install.sh${NC} to fix"
    echo -e "    3. Or reinstall: ${CYAN}./install.sh${NC}"
    EXIT_CODE=2
fi

echo -e "${BLUE}════════════════════════════════════${NC}"
echo ""

exit $EXIT_CODE
