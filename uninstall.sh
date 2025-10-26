#!/bin/bash
# HORUS Uninstallation Script
# Removes HORUS CLI and runtime libraries

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}  HORUS Uninstallation Script${NC}"
echo ""

# Determine installation paths
INSTALL_DIR="$HOME/.cargo/bin"
CACHE_DIR="$HOME/.horus/cache"
HORUS_DIR="$HOME/.horus"

echo -e "${YELLOW}${NC}  This will remove:"
echo "  • CLI binary:     $INSTALL_DIR/horus"
echo "  • Libraries:      $CACHE_DIR/"
echo "  • Cache:          $HORUS_DIR/"
echo "  • Shared memory:  /dev/shm/horus/"
echo ""

# Ask for confirmation
read -p "$(echo -e ${YELLOW}?${NC}) Are you sure you want to uninstall HORUS? [y/N]: " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${GREEN}${NC} Uninstallation cancelled"
    exit 0
fi

echo ""
echo -e "${CYAN}${NC} Uninstalling HORUS..."
echo ""

# Remove CLI binary
if [ -f "$INSTALL_DIR/horus" ]; then
    rm -f "$INSTALL_DIR/horus"
    echo -e "${GREEN}${NC} Removed CLI binary"
else
    echo -e "${YELLOW}${NC}  CLI binary not found (already removed?)"
fi

# Remove global cache
if [ -d "$CACHE_DIR" ]; then
    # List what will be removed
    echo -e "${CYAN}${NC} Removing cached libraries:"
    for pkg in "$CACHE_DIR"/*; do
        if [ -d "$pkg" ]; then
            echo "  • $(basename "$pkg")"
        fi
    done

    rm -rf "$CACHE_DIR"
    echo -e "${GREEN}${NC} Removed library cache"
else
    echo -e "${YELLOW}${NC}  Library cache not found (already removed?)"
fi

# Ask about removing entire .horus directory
if [ -d "$HORUS_DIR" ]; then
    echo ""
    echo -e "${YELLOW}${NC}  The .horus directory still exists and may contain:"
    echo "  • User settings and configuration"
    echo "  • Authentication credentials"
    echo "  • Registry data"
    echo ""
    read -p "$(echo -e ${YELLOW}?${NC}) Remove entire ~/.horus directory? [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$HORUS_DIR"
        echo -e "${GREEN}${NC} Removed ~/.horus directory"
    else
        echo -e "${CYAN}${NC} Kept ~/.horus directory"
    fi
fi

# Clean up shared memory files
SHM_DIR="/dev/shm/horus"
SHM_LOGS="/dev/shm/horus_logs"

if [ -d "$SHM_DIR" ] || [ -f "$SHM_LOGS" ]; then
    echo ""
    echo -e "${CYAN}${NC} Cleaning shared memory files..."

    if [ -d "$SHM_DIR" ]; then
        # List what's being removed
        if [ -d "$SHM_DIR/topics" ] && [ "$(ls -A $SHM_DIR/topics 2>/dev/null)" ]; then
            echo "  • Removing topic files in /dev/shm/horus/topics/"
        fi
        if [ -d "$SHM_DIR/heartbeats" ] && [ "$(ls -A $SHM_DIR/heartbeats 2>/dev/null)" ]; then
            echo "  • Removing heartbeat files in /dev/shm/horus/heartbeats/"
        fi
        rm -rf "$SHM_DIR" 2>/dev/null || true
    fi

    if [ -f "$SHM_LOGS" ]; then
        echo "  • Removing log buffer at /dev/shm/horus_logs"
        rm -f "$SHM_LOGS" 2>/dev/null || true
    fi

    echo -e "${GREEN}${NC} Cleaned shared memory"
fi

echo ""
echo -e "${GREEN} HORUS uninstalled successfully${NC}"
echo ""
echo -e "${CYAN}Note:${NC} Project-local .horus directories (in your projects) were not removed."
echo "      You can manually delete them if needed."
echo ""
