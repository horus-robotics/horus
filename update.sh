#!/bin/bash
# HORUS Update Script
# Smart update when repo or dependencies change

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${CYAN}🔄 HORUS Update Script${NC}"
echo ""

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo -e "${RED}❌ Error: Not in a git repository${NC}"
    echo "Please run this script from the HORUS repository root"
    exit 1
fi

# Check if there are uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo -e "${YELLOW}⚠  Warning: You have uncommitted changes${NC}"
    echo ""
    git status --short
    echo ""
    read -p "$(echo -e ${YELLOW}?${NC}) Stash changes before updating? [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git stash push -m "Auto-stash before HORUS update at $(date)"
        echo -e "${GREEN}✓${NC} Changes stashed"
        STASHED=true
    else
        echo -e "${YELLOW}→${NC} Continuing with uncommitted changes..."
    fi
    echo ""
fi

# Get current version before update
INSTALL_DIR="$HOME/.cargo/bin"
OLD_VERSION="unknown"
if [ -x "$INSTALL_DIR/horus" ]; then
    OLD_VERSION=$("$INSTALL_DIR/horus" --version 2>/dev/null | awk '{print $2}' || echo "unknown")
fi

echo -e "${CYAN}→${NC} Current version: $OLD_VERSION"
echo ""

# Fetch latest changes
echo -e "${CYAN}→${NC} Fetching latest changes from remote..."
git fetch origin

# Get current branch
CURRENT_BRANCH=$(git branch --show-current)

# Check if there are updates
LOCAL=$(git rev-parse @)
REMOTE=$(git rev-parse @{u} 2>/dev/null || echo "$LOCAL")
BASE=$(git merge-base @ @{u} 2>/dev/null || echo "$LOCAL")

if [ "$LOCAL" = "$REMOTE" ]; then
    echo -e "${GREEN}✓${NC} Already up to date"

    # Check if binary needs rebuilding anyway
    if [ ! -x "$INSTALL_DIR/horus" ]; then
        echo -e "${YELLOW}⚠${NC}  Binary not found, rebuilding..."
        NEEDS_REBUILD=true
    else
        echo ""
        read -p "$(echo -e ${YELLOW}?${NC}) Rebuild anyway? [y/N]: " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            NEEDS_REBUILD=true
        else
            echo -e "${GREEN}✅ Nothing to do!${NC}"
            exit 0
        fi
    fi
elif [ "$LOCAL" = "$BASE" ]; then
    echo -e "${BLUE}→${NC} Updates available on $CURRENT_BRANCH"

    # Show what changed
    echo ""
    echo -e "${CYAN}Recent changes:${NC}"
    git log --oneline --graph --decorate -5 HEAD..@{u}
    echo ""

    read -p "$(echo -e ${YELLOW}?${NC}) Pull and update? [Y/n]: " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        echo -e "${CYAN}→${NC} Pulling latest changes..."
        git pull origin "$CURRENT_BRANCH"
        echo -e "${GREEN}✓${NC} Updated to latest commit"
        NEEDS_REBUILD=true
    else
        echo -e "${RED}✗${NC} Update cancelled"
        exit 0
    fi
else
    echo -e "${YELLOW}⚠${NC}  Branches have diverged!"
    echo "Local and remote have different commits."
    echo ""
    echo "Options:"
    echo "  1. Merge remote changes (git pull)"
    echo "  2. Rebase onto remote (git pull --rebase)"
    echo "  3. Cancel and handle manually"
    echo ""
    read -p "$(echo -e ${YELLOW}?${NC}) Choose option [1-3]: " -n 1 -r
    echo
    case $REPLY in
        1)
            git pull origin "$CURRENT_BRANCH"
            NEEDS_REBUILD=true
            ;;
        2)
            git pull --rebase origin "$CURRENT_BRANCH"
            NEEDS_REBUILD=true
            ;;
        *)
            echo -e "${RED}✗${NC} Update cancelled"
            exit 0
            ;;
    esac
fi

echo ""

# Detect what changed
if [ -n "$NEEDS_REBUILD" ]; then
    echo -e "${CYAN}🔍 Analyzing changes...${NC}"

    CARGO_CHANGED=false
    CODE_CHANGED=false

    # Check if Cargo.toml files changed
    if git diff --name-only HEAD@{1} HEAD 2>/dev/null | grep -q "Cargo.toml"; then
        echo -e "${BLUE}→${NC} Cargo.toml files changed - dependencies may have updated"
        CARGO_CHANGED=true
    fi

    # Check if Rust source code changed
    if git diff --name-only HEAD@{1} HEAD 2>/dev/null | grep -q "\.rs$"; then
        echo -e "${BLUE}→${NC} Rust source files changed"
        CODE_CHANGED=true
    fi

    echo ""

    # Update dependencies if Cargo.toml changed
    if [ "$CARGO_CHANGED" = true ]; then
        echo -e "${CYAN}→${NC} Updating dependencies..."
        cargo update
        echo -e "${GREEN}✓${NC} Dependencies updated"
        echo ""
    fi

    # Rebuild in release mode
    echo -e "${CYAN}🔨 Rebuilding HORUS (release mode)...${NC}"
    echo ""

    # Build with progress
    cargo build --release

    if [ $? -ne 0 ]; then
        echo ""
        echo -e "${RED}❌ Build failed${NC}"
        echo ""
        echo "Troubleshooting:"
        echo "  1. Try recovery install: ./recovery_install.sh"
        echo "  2. Check for compilation errors above"
        echo "  3. Verify dependencies: ./verify.sh"
        exit 1
    fi

    echo ""
    echo -e "${GREEN}✓${NC} Build completed"
    echo ""

    # Install updated binary
    echo -e "${CYAN}→${NC} Installing updated binary..."

    if [ ! -d "$INSTALL_DIR" ]; then
        mkdir -p "$INSTALL_DIR"
    fi

    cp target/release/horus "$INSTALL_DIR/horus"
    chmod +x "$INSTALL_DIR/horus"

    echo -e "${GREEN}✓${NC} Binary installed to $INSTALL_DIR/horus"
    echo ""

    # Update library cache
    echo -e "${CYAN}→${NC} Updating library cache..."

    CACHE_DIR="$HOME/.horus/cache"
    mkdir -p "$CACHE_DIR"

    # Get new version
    NEW_VERSION=$(grep -m1 '^version' horus/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

    # Remove old version if different
    VERSION_FILE="$HOME/.horus/installed_version"
    if [ -f "$VERSION_FILE" ]; then
        OLD_CACHED_VERSION=$(cat "$VERSION_FILE")
        if [ "$OLD_CACHED_VERSION" != "$NEW_VERSION" ]; then
            echo -e "${YELLOW}→${NC} Version changed: $OLD_CACHED_VERSION → $NEW_VERSION"
            echo -e "${CYAN}→${NC} Cleaning old cache..."
            rm -rf "$CACHE_DIR"/*@"$OLD_CACHED_VERSION" 2>/dev/null || true
        fi
    fi

    # Update library files (using same logic as install.sh)
    HORUS_CORE_VERSION=$(grep -m1 '^version' horus_core/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
    HORUS_MACROS_VERSION=$(grep -m1 '^version' horus_macros/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
    HORUS_LIBRARY_VERSION=$(grep -m1 '^version' horus_library/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

    # Update horus_core
    HORUS_CORE_DIR="$CACHE_DIR/horus_core@$HORUS_CORE_VERSION"
    mkdir -p "$HORUS_CORE_DIR/lib"
    cp -r target/release/libhorus_core.* "$HORUS_CORE_DIR/lib/" 2>/dev/null || true
    cp -r target/release/deps/libhorus_core*.rlib "$HORUS_CORE_DIR/lib/" 2>/dev/null || true

    # Update horus
    HORUS_DIR="$CACHE_DIR/horus@$NEW_VERSION"
    mkdir -p "$HORUS_DIR/lib"
    mkdir -p "$HORUS_DIR/target/release"
    cp -r target/release/libhorus*.rlib "$HORUS_DIR/target/release/" 2>/dev/null || true
    cp -r target/release/deps/libhorus_core*.rlib "$HORUS_DIR/target/release/" 2>/dev/null || true

    # Update horus_macros
    HORUS_MACROS_DIR="$CACHE_DIR/horus_macros@$HORUS_MACROS_VERSION"
    mkdir -p "$HORUS_MACROS_DIR/target/release"
    cp -r target/release/libhorus_macros.so "$HORUS_MACROS_DIR/target/release/" 2>/dev/null || true
    cp -r target/release/deps/libhorus_macros*.so "$HORUS_MACROS_DIR/target/release/" 2>/dev/null || true

    # Update horus_library
    HORUS_LIBRARY_DIR="$CACHE_DIR/horus_library@$HORUS_LIBRARY_VERSION"
    mkdir -p "$HORUS_LIBRARY_DIR/target/release"
    cp -r target/release/libhorus_library*.rlib "$HORUS_LIBRARY_DIR/target/release/" 2>/dev/null || true

    echo "$NEW_VERSION" > "$VERSION_FILE"

    echo -e "${GREEN}✓${NC} Libraries updated"
    echo ""

    # Quick verification
    echo -e "${CYAN}🔍 Verifying update...${NC}"

    if [ -x "$INSTALL_DIR/horus" ]; then
        UPDATED_VERSION=$("$INSTALL_DIR/horus" --version 2>/dev/null | awk '{print $2}' || echo "unknown")
        echo -e "${GREEN}✓${NC} New version: $UPDATED_VERSION"
    else
        echo -e "${RED}✗${NC} Binary verification failed"
        exit 1
    fi

    echo ""

    # Run quick smoke test
    if "$INSTALL_DIR/horus" --help &>/dev/null; then
        echo -e "${GREEN}✓${NC} Smoke test passed"
    else
        echo -e "${RED}✗${NC} Smoke test failed"
        echo "  Binary installed but not working correctly"
        echo "  Try: ./recovery_install.sh"
        exit 1
    fi
fi

echo ""
echo -e "${GREEN}✅ Update complete!${NC}"
echo ""

# Show version comparison
if [ "$OLD_VERSION" != "unknown" ] && [ -n "$UPDATED_VERSION" ]; then
    if [ "$OLD_VERSION" != "$UPDATED_VERSION" ]; then
        echo -e "${CYAN}Version:${NC} $OLD_VERSION → $UPDATED_VERSION"
    else
        echo -e "${CYAN}Version:${NC} $UPDATED_VERSION (rebuilt)"
    fi
fi

echo ""
echo -e "${CYAN}Next steps:${NC}"
echo "  • Test your existing projects"
echo "  • Check for breaking changes in commits"
echo "  • Report any issues"

# Restore stashed changes if we stashed them
if [ "$STASHED" = true ]; then
    echo ""
    read -p "$(echo -e ${YELLOW}?${NC}) Restore stashed changes? [Y/n]: " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        git stash pop
        echo -e "${GREEN}✓${NC} Changes restored"
    fi
fi

echo ""
