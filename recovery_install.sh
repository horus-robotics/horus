#!/bin/bash
# HORUS Recovery Installation Script
# Nuclear option: Fix broken installations, detect issues, fresh start

set +e  # Don't exit on error - we're diagnosing problems

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

echo -e "${MAGENTA}🔧 HORUS Recovery Installation Script${NC}"
echo -e "${YELLOW}═════════════════════════════════════${NC}"
echo ""
echo "This script will:"
echo "  • Detect system dependencies and issues"
echo "  • Clean all build artifacts and caches"
echo "  • Fresh installation from scratch"
echo "  • Comprehensive verification"
echo ""

read -p "$(echo -e ${YELLOW}?${NC}) Continue with recovery installation? [y/N]: " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${GREEN}✓${NC} Recovery cancelled"
    exit 0
fi

echo ""
echo -e "${CYAN}═══ PHASE 1: System Diagnostics ═══${NC}"
echo ""

# Track errors
ERRORS=0
WARNINGS=0

# Check Rust installation
echo -e "${INFO} Checking Rust toolchain..."
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version | awk '{print $2}')
    echo -e "  $CHECK Rust: $RUST_VERSION"

    # Check minimum version (1.70)
    RUST_MAJOR=$(echo $RUST_VERSION | cut -d'.' -f1)
    RUST_MINOR=$(echo $RUST_VERSION | cut -d'.' -f2)

    if [ "$RUST_MAJOR" -eq 1 ] && [ "$RUST_MINOR" -lt 70 ]; then
        echo -e "  $WARN Rust version is old (< 1.70)"
        echo -e "      Update with: rustup update"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    echo -e "  $CROSS Rust not found"
    echo -e "      Install from: https://rustup.rs/"
    ERRORS=$((ERRORS + 1))
fi

# Check Cargo
if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version | awk '{print $2}')
    echo -e "  $CHECK Cargo: $CARGO_VERSION"
else
    echo -e "  $CROSS Cargo not found"
    ERRORS=$((ERRORS + 1))
fi

echo ""

# Check C compiler
echo -e "${INFO} Checking C compiler..."
if command -v cc &> /dev/null; then
    CC_VERSION=$(cc --version | head -n1)
    echo -e "  $CHECK C compiler: $CC_VERSION"
elif command -v gcc &> /dev/null; then
    GCC_VERSION=$(gcc --version | head -n1)
    echo -e "  $CHECK GCC: $GCC_VERSION"
else
    echo -e "  $CROSS C compiler not found"
    echo ""
    echo "  Install build tools:"
    echo -e "    ${CYAN}Ubuntu/Debian:${NC}"
    echo "      sudo apt update && sudo apt install build-essential"
    echo -e "    ${CYAN}Fedora/RHEL:${NC}"
    echo "      sudo dnf groupinstall \"Development Tools\""
    echo -e "    ${CYAN}macOS:${NC}"
    echo "      xcode-select --install"
    ERRORS=$((ERRORS + 1))
fi

echo ""

# Check pkg-config
echo -e "${INFO} Checking system dependencies..."
if command -v pkg-config &> /dev/null; then
    echo -e "  $CHECK pkg-config: $(pkg-config --version)"
else
    echo -e "  $CROSS pkg-config not found"
    echo "      Required for building system library dependencies"
    ERRORS=$((ERRORS + 1))
fi

# Check for common system libraries
declare -a LIBS=("openssl" "udev")
declare -a MISSING_LIBS=()

for lib in "${LIBS[@]}"; do
    if pkg-config --exists "$lib" 2>/dev/null; then
        VERSION=$(pkg-config --modversion "$lib" 2>/dev/null || echo "unknown")
        echo -e "  $CHECK lib$lib: $VERSION"
    else
        echo -e "  $WARN lib$lib: not found (optional)"
        MISSING_LIBS+=("$lib")
        WARNINGS=$((WARNINGS + 1))
    fi
done

if [ ${#MISSING_LIBS[@]} -gt 0 ]; then
    echo ""
    echo -e "  ${YELLOW}Missing optional libraries:${NC}"
    echo -e "    ${CYAN}Ubuntu/Debian:${NC}"
    echo "      sudo apt install libssl-dev libudev-dev"
    echo -e "    ${CYAN}Fedora/RHEL:${NC}"
    echo "      sudo dnf install openssl-devel systemd-devel"
fi

echo ""

# Check for conflicting HORUS installations
echo -e "${INFO} Checking for existing HORUS installations..."
INSTALL_DIR="$HOME/.cargo/bin"
HORUS_BINARY="$INSTALL_DIR/horus"

if [ -f "$HORUS_BINARY" ]; then
    OLD_VERSION=$("$HORUS_BINARY" --version 2>/dev/null | awk '{print $2}' || echo "broken")
    echo -e "  $WARN Found existing binary: $OLD_VERSION"
    WARNINGS=$((WARNINGS + 1))
else
    echo -e "  $CHECK No existing binary found"
fi

# Check for multiple horus binaries in PATH
HORUS_COUNT=$(compgen -c | grep -c "^horus$" 2>/dev/null || echo "0")
if [ "$HORUS_COUNT" -gt 1 ]; then
    echo -e "  $WARN Multiple 'horus' commands found in PATH"
    which -a horus 2>/dev/null | while read -r path; do
        echo "      - $path"
    done
    WARNINGS=$((WARNINGS + 1))
fi

echo ""

# Check cargo cache
echo -e "${INFO} Checking cargo cache..."
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
CARGO_CACHE_SIZE=$(du -sh "$CARGO_HOME/registry" 2>/dev/null | awk '{print $1}' || echo "0")
echo -e "  ${INFO} Cargo cache size: $CARGO_CACHE_SIZE"

# Check HORUS cache
echo -e "${INFO} Checking HORUS cache..."
HORUS_DIR="$HOME/.horus"
if [ -d "$HORUS_DIR" ]; then
    HORUS_CACHE_SIZE=$(du -sh "$HORUS_DIR" 2>/dev/null | awk '{print $1}' || echo "0")
    echo -e "  ${INFO} HORUS cache size: $HORUS_CACHE_SIZE"

    # List installed components
    CACHE_DIR="$HORUS_DIR/cache"
    if [ -d "$CACHE_DIR" ]; then
        echo -e "  ${INFO} Installed components:"
        ls -1 "$CACHE_DIR" 2>/dev/null | while read -r component; do
            echo "      - $component"
        done
    fi
else
    echo -e "  $CHECK No HORUS cache found"
fi

echo ""
echo -e "${CYAN}═══ Diagnostic Summary ═══${NC}"
echo -e "  Errors:   ${RED}$ERRORS${NC}"
echo -e "  Warnings: ${YELLOW}$WARNINGS${NC}"
echo ""

if [ $ERRORS -gt 0 ]; then
    echo -e "${RED}❌ Critical errors found!${NC}"
    echo "Please fix the errors above before proceeding."
    echo ""
    exit 1
fi

if [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}⚠  Warnings found${NC}"
    echo "You can continue, but some features may not work."
    echo ""
    read -p "$(echo -e ${YELLOW}?${NC}) Continue anyway? [y/N]: " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${GREEN}✓${NC} Recovery cancelled"
        exit 0
    fi
    echo ""
fi

echo -e "${CYAN}═══ PHASE 2: Cleanup ═══${NC}"
echo ""

# Clean build artifacts
echo -e "${INFO} Cleaning build artifacts..."
if [ -d "target" ]; then
    TARGET_SIZE=$(du -sh target 2>/dev/null | awk '{print $1}' || echo "unknown")
    echo -e "  ${INFO} Removing target/ ($TARGET_SIZE)..."
    rm -rf target
    echo -e "  $CHECK Build artifacts removed"
else
    echo -e "  $CHECK No build artifacts found"
fi

# Clean cargo cache for HORUS
echo -e "${INFO} Cleaning HORUS from cargo cache..."
if [ -d "$CARGO_HOME/registry" ]; then
    find "$CARGO_HOME/registry" -type d -name "*horus*" -exec rm -rf {} + 2>/dev/null || true
    echo -e "  $CHECK Cargo cache cleaned"
fi

# Ask about HORUS cache
if [ -d "$HORUS_DIR/cache" ]; then
    echo ""
    read -p "$(echo -e ${YELLOW}?${NC}) Remove ~/.horus/cache (installed libraries)? [Y/n]: " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        rm -rf "$HORUS_DIR/cache"
        echo -e "  $CHECK HORUS cache removed"
    else
        echo -e "  ${INFO} Keeping HORUS cache"
    fi
fi

# Ask about user config
if [ -d "$HORUS_DIR/config" ]; then
    echo ""
    read -p "$(echo -e ${YELLOW}?${NC}) Remove ~/.horus/config (user settings)? [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$HORUS_DIR/config"
        echo -e "  $CHECK User config removed"
    else
        echo -e "  ${INFO} Keeping user config"
    fi
fi

# Remove old binary
if [ -f "$HORUS_BINARY" ]; then
    rm -f "$HORUS_BINARY"
    echo -e "  $CHECK Old binary removed"
fi

echo ""
echo -e "${GREEN}✓${NC} Cleanup complete"
echo ""

echo -e "${CYAN}═══ PHASE 3: Fresh Installation ═══${NC}"
echo ""

# Exit on error for installation phase
set -e

# Run the standard install script
if [ -f "./install.sh" ]; then
    echo -e "${INFO} Running install.sh..."
    echo ""
    bash ./install.sh
    INSTALL_SUCCESS=$?
else
    echo -e "${RED}❌ install.sh not found${NC}"
    echo "Make sure you're in the HORUS repository root"
    exit 1
fi

echo ""

if [ $INSTALL_SUCCESS -eq 0 ]; then
    echo -e "${CYAN}═══ PHASE 4: Verification ═══${NC}"
    echo ""

    # Comprehensive verification
    ALL_OK=true

    # Test binary
    echo -e "${INFO} Testing HORUS binary..."
    if [ -x "$INSTALL_DIR/horus" ]; then
        if "$INSTALL_DIR/horus" --version &>/dev/null; then
            VERSION=$("$INSTALL_DIR/horus" --version | awk '{print $2}')
            echo -e "  $CHECK Binary works: v$VERSION"
        else
            echo -e "  $CROSS Binary exists but doesn't run"
            ALL_OK=false
        fi
    else
        echo -e "  $CROSS Binary not installed"
        ALL_OK=false
    fi

    # Test basic commands
    echo -e "${INFO} Testing basic commands..."
    if "$INSTALL_DIR/horus" --help &>/dev/null; then
        echo -e "  $CHECK --help works"
    else
        echo -e "  $CROSS --help failed"
        ALL_OK=false
    fi

    # Check if in PATH
    echo -e "${INFO} Checking PATH..."
    if command -v horus &>/dev/null; then
        WHICH_HORUS=$(which horus)
        echo -e "  $CHECK 'horus' found in PATH"
        echo -e "      → $WHICH_HORUS"
    else
        echo -e "  $WARN 'horus' not in PATH"
        echo -e "      Add to ~/.bashrc or ~/.zshrc:"
        echo -e "      ${CYAN}export PATH=\"\$HOME/.cargo/bin:\$PATH\"${NC}"
        ALL_OK=false
    fi

    # Check library cache
    echo -e "${INFO} Checking installed libraries..."
    VERSION_FILE="$HOME/.horus/installed_version"
    if [ -f "$VERSION_FILE" ]; then
        INSTALLED_VERSION=$(cat "$VERSION_FILE")
        echo -e "  $CHECK Installed version: $INSTALLED_VERSION"
    else
        echo -e "  $WARN Version file not found"
    fi

    CACHE_DIR="$HOME/.horus/cache"
    EXPECTED_LIBS=("horus" "horus_core" "horus_macros" "horus_library")
    for lib in "${EXPECTED_LIBS[@]}"; do
        if ls "$CACHE_DIR"/${lib}@* 1>/dev/null 2>&1; then
            echo -e "  $CHECK $lib installed"
        else
            echo -e "  $CROSS $lib missing"
            ALL_OK=false
        fi
    done

    echo ""

    if [ "$ALL_OK" = true ]; then
        echo -e "${GREEN}✅ Recovery installation successful!${NC}"
        echo ""
        echo -e "${CYAN}Next steps:${NC}"
        echo "  1. Test creating a project:"
        echo -e "     ${CYAN}horus new test_project${NC}"
        echo ""
        echo "  2. Run it:"
        echo -e "     ${CYAN}cd test_project && horus run${NC}"
    else
        echo -e "${RED}❌ Some verification checks failed${NC}"
        echo ""
        echo "Installation completed but with issues."
        echo "Try the following:"
        echo "  1. Check the errors above"
        echo "  2. Ensure ~/.cargo/bin is in your PATH"
        echo "  3. Try running: horus --version"
        echo "  4. Report the issue with full output"
    fi
else
    echo -e "${RED}❌ Installation failed${NC}"
    echo ""
    echo "Please check the errors above and try again."
    exit 1
fi

echo ""
