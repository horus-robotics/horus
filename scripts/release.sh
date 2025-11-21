#!/bin/bash
#
# HORUS Release Helper Script
#
# This script automates the release process:
# 1. Updates version numbers in all necessary files
# 2. Creates a git commit
# 3. Creates and pushes a git tag
# 4. Triggers GitHub Actions to build and publish wheels
#
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.1.6
#

set -e  # Exit on error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Check arguments
if [ $# -eq 0 ]; then
    echo -e "${RED}Error: Version number required${NC}"
    echo ""
    echo "Usage: $0 <version>"
    echo "Example: $0 0.1.6"
    echo ""
    exit 1
fi

NEW_VERSION=$1

# Validate version format (semver)
if ! [[ $NEW_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    echo -e "${RED}Error: Invalid version format${NC}"
    echo "Expected format: X.Y.Z or X.Y.Z-suffix"
    echo "Examples: 0.1.6, 1.0.0, 0.2.0-beta1"
    exit 1
fi

echo -e "${CYAN}╔══════════════════════════════════════╗${NC}"
echo -e "${CYAN}║  HORUS Release Helper                ║${NC}"
echo -e "${CYAN}║  Version: ${NEW_VERSION}                      ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════╝${NC}"
echo ""

# Check we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "horus_py" ]; then
    echo -e "${RED}Error: Must be run from HORUS root directory${NC}"
    exit 1
fi

# Check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    echo -e "${YELLOW}Warning: You have uncommitted changes${NC}"
    git status --short
    echo ""
    read -p "Continue anyway? [y/N]: " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted"
        exit 1
    fi
fi

# Get current version
CURRENT_VERSION=$(grep -m1 '^version' horus_py/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
echo -e "${CYAN}Current version:${NC} $CURRENT_VERSION"
echo -e "${CYAN}New version:${NC}     $NEW_VERSION"
echo ""

# Check if tag already exists
if git rev-parse "v$NEW_VERSION" >/dev/null 2>&1; then
    echo -e "${RED}Error: Tag v$NEW_VERSION already exists${NC}"
    exit 1
fi

# Confirm
read -p "$(echo -e ${YELLOW}?)${NC} Update version and create release? [y/N]: " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted"
    exit 0
fi

echo ""
echo -e "${CYAN}Step 1: Updating version numbers...${NC}"

# Update Cargo.toml files
FILES_TO_UPDATE=(
    "horus/Cargo.toml"
    "horus_core/Cargo.toml"
    "horus_macros/Cargo.toml"
    "horus_library/Cargo.toml"
    "horus_py/Cargo.toml"
)

for file in "${FILES_TO_UPDATE[@]}"; do
    if [ -f "$file" ]; then
        sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$file"
        echo -e "  ${GREEN}✓${NC} Updated $file"
    else
        echo -e "  ${YELLOW}⊘${NC} Skipped $file (not found)"
    fi
done

# Update pyproject.toml
if [ -f "horus_py/pyproject.toml" ]; then
    sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "horus_py/pyproject.toml"
    echo -e "  ${GREEN}✓${NC} Updated horus_py/pyproject.toml"
fi

echo ""
echo -e "${CYAN}Step 2: Creating git commit...${NC}"

# Stage the changes
git add "${FILES_TO_UPDATE[@]}" horus_py/pyproject.toml 2>/dev/null || true

# Show what will be committed
echo ""
git diff --cached --stat
echo ""

# Create commit
git commit -m "Release v$NEW_VERSION

- Bump version to $NEW_VERSION
- Update all Cargo.toml files
- Update pyproject.toml for Python package
"

echo -e "${GREEN}✓${NC} Created commit"

echo ""
echo -e "${CYAN}Step 3: Creating git tag...${NC}"

# Create tag
git tag "v$NEW_VERSION" -m "Release v$NEW_VERSION"
echo -e "${GREEN}✓${NC} Created tag v$NEW_VERSION"

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Release v$NEW_VERSION prepared successfully!${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Next steps:"
echo ""
echo "1. Review the changes:"
echo -e "   ${CYAN}git show HEAD${NC}"
echo ""
echo "2. Push to GitHub to trigger release:"
echo -e "   ${CYAN}git push origin main --tags${NC}"
echo ""
echo "3. Monitor GitHub Actions:"
echo "   - https://github.com/softmata/horus/actions"
echo "   - Build wheels: ~10-15 minutes"
echo "   - Publish to PyPI: automatic after build"
echo ""
echo "4. Verify on PyPI (after ~15 mins):"
echo "   - https://pypi.org/project/horus/"
echo ""
echo "5. Test installation:"
echo -e "   ${CYAN}pip install horus==$NEW_VERSION${NC}"
echo ""
echo -e "${YELLOW}Note: If you need to cancel, run:${NC}"
echo -e "   ${CYAN}git reset --hard HEAD~1${NC}  (undo commit)"
echo -e "   ${CYAN}git tag -d v$NEW_VERSION${NC}   (delete tag)"
echo ""
