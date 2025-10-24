#!/bin/bash
# HORUS Installation Script
# Installs the HORUS CLI and runtime libraries from source

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}🚀 HORUS Installation Script${NC}"
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Error: Rust is not installed${NC}"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

echo -e "${CYAN}→${NC} Detected Rust version: $(rustc --version)"

# Check if C compiler/linker is installed
if ! command -v cc &> /dev/null && ! command -v gcc &> /dev/null; then
    echo -e "${RED}❌ Error: C compiler not found${NC}"
    echo ""
    echo "HORUS requires a C compiler/linker to build native code."
    echo ""
    echo "Install build tools for your system:"
    echo ""
    echo -e "${CYAN}Ubuntu/Debian:${NC}"
    echo "  sudo apt update && sudo apt install build-essential pkg-config libudev-dev libssl-dev libasound2-dev"
    echo ""
    echo -e "${CYAN}Fedora/RHEL/CentOS:${NC}"
    echo "  sudo dnf groupinstall \"Development Tools\""
    echo "  sudo dnf install pkg-config systemd-devel openssl-devel alsa-lib-devel"
    echo ""
    echo -e "${CYAN}Arch Linux:${NC}"
    echo "  sudo pacman -S base-devel pkg-config systemd openssl alsa-lib"
    echo ""
    echo -e "${CYAN}macOS:${NC}"
    echo "  xcode-select --install"
    echo "  brew install pkg-config"
    echo ""
    exit 1
fi

echo -e "${CYAN}→${NC} Detected C compiler: $(cc --version | head -n1)"

# Check if pkg-config is installed
if ! command -v pkg-config &> /dev/null; then
    echo -e "${RED}❌ Error: pkg-config not found${NC}"
    echo ""
    echo "HORUS requires pkg-config to build system library dependencies."
    echo ""
    echo "Install pkg-config for your system:"
    echo ""
    echo -e "${CYAN}Ubuntu/Debian:${NC}"
    echo "  sudo apt install pkg-config libudev-dev libssl-dev libasound2-dev"
    echo ""
    echo -e "${CYAN}Fedora/RHEL/CentOS:${NC}"
    echo "  sudo dnf install pkg-config systemd-devel openssl-devel alsa-lib-devel"
    echo ""
    echo -e "${CYAN}Arch Linux:${NC}"
    echo "  sudo pacman -S pkg-config systemd openssl alsa-lib"
    echo ""
    echo -e "${CYAN}macOS:${NC}"
    echo "  brew install pkg-config"
    echo ""
    exit 1
fi

echo -e "${CYAN}→${NC} Detected pkg-config: $(pkg-config --version)"

# Check if Python is installed (for horus_py)
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version | awk '{print $2}')
    echo -e "${CYAN}→${NC} Detected Python: $PYTHON_VERSION"

    # Check if Python version is 3.9+
    PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d. -f1)
    PYTHON_MINOR=$(echo $PYTHON_VERSION | cut -d. -f2)

    if [ "$PYTHON_MAJOR" -ge 3 ] && [ "$PYTHON_MINOR" -ge 9 ]; then
        PYTHON_AVAILABLE=true
    else
        echo -e "${YELLOW}⚠${NC}  Python 3.9+ required for horus_py (found $PYTHON_VERSION)"
        echo -e "  horus_py will be skipped"
        PYTHON_AVAILABLE=false
    fi
else
    echo -e "${YELLOW}⚠${NC}  Python3 not found - horus_py will be skipped"
    PYTHON_AVAILABLE=false
fi

# Check for pip (needed for maturin)
if [ "$PYTHON_AVAILABLE" = true ]; then
    if command -v pip3 &> /dev/null || command -v pip &> /dev/null; then
        echo -e "${CYAN}→${NC} Detected pip: $(pip3 --version 2>/dev/null || pip --version)"
    else
        echo -e "${YELLOW}⚠${NC}  pip not found - horus_py will be skipped"
        echo "  Install pip: sudo apt install python3-pip (Debian/Ubuntu)"
        PYTHON_AVAILABLE=false
    fi
fi

echo ""

# Determine installation paths
INSTALL_DIR="$HOME/.cargo/bin"
CACHE_DIR="$HOME/.horus/cache"

echo -e "${CYAN}→${NC} Installation paths:"
echo "  CLI binary: $INSTALL_DIR/horus"
echo "  Libraries:  $CACHE_DIR/"
echo ""

# Ask for confirmation
read -p "$(echo -e ${YELLOW}?${NC}) Proceed with installation? [Y/n]: " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]] && [[ ! -z $REPLY ]]; then
    echo -e "${RED}✗${NC} Installation cancelled"
    exit 0
fi

# Step 1: Build all packages in release mode
echo ""
echo -e "${CYAN}🔨 Building HORUS packages (release mode)...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Build completed"
echo ""

# Step 2: Install CLI binary
echo -e "${CYAN}→${NC} Installing CLI binary..."

if [ ! -d "$INSTALL_DIR" ]; then
    mkdir -p "$INSTALL_DIR"
fi

cp target/release/horus "$INSTALL_DIR/horus"
chmod +x "$INSTALL_DIR/horus"

echo -e "${GREEN}✓${NC} CLI installed to $INSTALL_DIR/horus"
echo ""

# Step 3: Create cache directory structure
echo -e "${CYAN}→${NC} Setting up library cache..."

mkdir -p "$CACHE_DIR"

# Get version from Cargo.toml
HORUS_VERSION=$(grep -m1 '^version' horus/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_CORE_VERSION=$(grep -m1 '^version' horus_core/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_MACROS_VERSION=$(grep -m1 '^version' horus_macros/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_LIBRARY_VERSION=$(grep -m1 '^version' horus_library/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_C_VERSION=$(grep -m1 '^version' horus_c/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_PY_VERSION=$(grep -m1 '^version' horus_py/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

echo -e "${CYAN}  →${NC} Detected versions:"
echo "    horus: $HORUS_VERSION"
echo "    horus_core: $HORUS_CORE_VERSION"
echo "    horus_macros: $HORUS_MACROS_VERSION"
echo "    horus_library: $HORUS_LIBRARY_VERSION"
echo "    horus_c: $HORUS_C_VERSION"
echo "    horus_py: $HORUS_PY_VERSION"
echo ""

# Check for version changes
VERSION_FILE="$HOME/.horus/installed_version"
if [ -f "$VERSION_FILE" ]; then
    OLD_VERSION=$(cat "$VERSION_FILE")
    if [ "$OLD_VERSION" != "$HORUS_VERSION" ]; then
        echo -e "${YELLOW}⚠${NC}  Version changed: ${OLD_VERSION} → ${HORUS_VERSION}"
        echo -e "${CYAN}→${NC} Cleaning old library versions..."

        # Remove old versioned directories
        rm -rf "$CACHE_DIR/horus@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_core@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_macros@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_library@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_c@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_py@$OLD_VERSION" 2>/dev/null || true

        echo -e "${GREEN}✓${NC} Old versions removed"
        echo ""
    fi
fi

# Step 4: Install horus_core library
echo -e "${CYAN}→${NC} Installing horus_core@$HORUS_CORE_VERSION..."
HORUS_CORE_DIR="$CACHE_DIR/horus_core@$HORUS_CORE_VERSION"
mkdir -p "$HORUS_CORE_DIR/lib"

# Copy compiled libraries
cp -r target/release/libhorus_core.* "$HORUS_CORE_DIR/lib/" 2>/dev/null || true
cp -r target/release/deps/libhorus_core*.rlib "$HORUS_CORE_DIR/lib/" 2>/dev/null || true

# Create metadata
cat > "$HORUS_CORE_DIR/metadata.json" << EOF
{
  "name": "horus_core",
  "version": "$HORUS_CORE_VERSION",
  "description": "HORUS Core - Runtime and scheduler",
  "install_type": "source"
}
EOF

echo -e "${GREEN}✓${NC} Installed horus_core"

# Step 5: Install horus library
echo -e "${CYAN}→${NC} Installing horus@$HORUS_VERSION..."
HORUS_DIR="$CACHE_DIR/horus@$HORUS_VERSION"
mkdir -p "$HORUS_DIR/lib"

# Copy compiled libraries
cp -r target/release/libhorus.* "$HORUS_DIR/lib/" 2>/dev/null || true
cp -r target/release/deps/libhorus*.rlib "$HORUS_DIR/lib/" 2>/dev/null || true

# Also copy target/release for rustc linking
mkdir -p "$HORUS_DIR/target/release"
cp -r target/release/libhorus*.rlib "$HORUS_DIR/target/release/" 2>/dev/null || true
cp -r target/release/deps/libhorus_core*.rlib "$HORUS_DIR/target/release/" 2>/dev/null || true

# CRITICAL: Copy ALL transitive dependencies for rustc compilation
mkdir -p "$HORUS_DIR/target/release/deps"
echo -e "${CYAN}  →${NC} Bundling transitive dependencies for user projects..."
cp target/release/deps/*.rlib "$HORUS_DIR/target/release/deps/" 2>/dev/null || true
echo -e "${GREEN}✓${NC} Bundled $(ls target/release/deps/*.rlib 2>/dev/null | wc -l) dependency libraries"

# Create metadata
cat > "$HORUS_DIR/metadata.json" << EOF
{
  "name": "horus",
  "version": "$HORUS_VERSION",
  "description": "HORUS Framework - Main library",
  "install_type": "source"
}
EOF

echo -e "${GREEN}✓${NC} Installed horus"

# Step 6: Install horus_macros
echo -e "${CYAN}→${NC} Installing horus_macros@$HORUS_MACROS_VERSION..."
HORUS_MACROS_DIR="$CACHE_DIR/horus_macros@$HORUS_MACROS_VERSION"
mkdir -p "$HORUS_MACROS_DIR/lib"

# Copy procedural macro library
cp -r target/release/libhorus_macros.* "$HORUS_MACROS_DIR/lib/" 2>/dev/null || true
cp -r target/release/deps/libhorus_macros*.so "$HORUS_MACROS_DIR/lib/" 2>/dev/null || true

# Also copy to target/release for rustc
mkdir -p "$HORUS_MACROS_DIR/target/release"
cp -r target/release/libhorus_macros.so "$HORUS_MACROS_DIR/target/release/" 2>/dev/null || true
cp -r target/release/deps/libhorus_macros*.so "$HORUS_MACROS_DIR/target/release/" 2>/dev/null || true

# Create metadata
cat > "$HORUS_MACROS_DIR/metadata.json" << EOF
{
  "name": "horus_macros",
  "version": "$HORUS_MACROS_VERSION",
  "description": "HORUS Macros - Procedural macros for simplified node creation",
  "install_type": "source"
}
EOF

echo -e "${GREEN}✓${NC} Installed horus_macros"

# Step 7: Install horus_library
echo -e "${CYAN}→${NC} Installing horus_library@$HORUS_LIBRARY_VERSION..."
HORUS_LIBRARY_DIR="$CACHE_DIR/horus_library@$HORUS_LIBRARY_VERSION"
mkdir -p "$HORUS_LIBRARY_DIR/lib"

# Copy compiled libraries
cp -r target/release/libhorus_library.* "$HORUS_LIBRARY_DIR/lib/" 2>/dev/null || true
cp -r target/release/deps/libhorus_library*.rlib "$HORUS_LIBRARY_DIR/lib/" 2>/dev/null || true

# Also copy to target/release
mkdir -p "$HORUS_LIBRARY_DIR/target/release"
cp -r target/release/libhorus_library*.rlib "$HORUS_LIBRARY_DIR/target/release/" 2>/dev/null || true

# Create metadata
cat > "$HORUS_LIBRARY_DIR/metadata.json" << EOF
{
  "name": "horus_library",
  "version": "$HORUS_LIBRARY_VERSION",
  "description": "HORUS Standard Library - Common messages and nodes",
  "install_type": "source"
}
EOF

echo -e "${GREEN}✓${NC} Installed horus_library"

# Step 8: Install horus_c (C bindings)
echo -e "${CYAN}→${NC} Installing horus_c@$HORUS_C_VERSION..."
HORUS_C_DIR="$CACHE_DIR/horus_c@$HORUS_C_VERSION"
mkdir -p "$HORUS_C_DIR/lib"
mkdir -p "$HORUS_C_DIR/include"

# Copy C library (both dynamic and static)
cp -r target/release/libhorus_c.so "$HORUS_C_DIR/lib/" 2>/dev/null || true
cp -r target/release/libhorus_c.a "$HORUS_C_DIR/lib/" 2>/dev/null || true
cp -r target/release/libhorus_c.dylib "$HORUS_C_DIR/lib/" 2>/dev/null || true  # macOS

# Copy header file (generated by cbindgen)
if [ -f "horus_c/horus.h" ]; then
    cp horus_c/horus.h "$HORUS_C_DIR/include/"
elif [ -f "target/horus.h" ]; then
    cp target/horus.h "$HORUS_C_DIR/include/"
fi

# Create metadata
cat > "$HORUS_C_DIR/metadata.json" << EOF
{
  "name": "horus_c",
  "version": "$HORUS_C_VERSION",
  "description": "HORUS C API - C bindings for hardware integration",
  "install_type": "source"
}
EOF

echo -e "${GREEN}✓${NC} Installed horus_c"

# Step 9: Install horus_py (Python bindings)
if [ "$PYTHON_AVAILABLE" = true ]; then
    echo -e "${CYAN}→${NC} Installing horus_py@$HORUS_PY_VERSION (Python bindings)..."
    HORUS_PY_DIR="$CACHE_DIR/horus_py@$HORUS_PY_VERSION"
    mkdir -p "$HORUS_PY_DIR"

    # Check if maturin is installed
    if ! command -v maturin &> /dev/null; then
        echo -e "${CYAN}  →${NC} Installing maturin (Python/Rust build tool)..."
        pip3 install maturin --user --quiet

        if [ $? -ne 0 ]; then
            echo -e "${RED}✗${NC} Failed to install maturin"
            echo -e "${YELLOW}⚠${NC}  Skipping horus_py installation"
            echo -e "  You can install it manually later:"
            echo -e "    ${CYAN}pip install maturin${NC}"
            echo -e "    ${CYAN}cd horus_py && maturin develop --release${NC}"
            PYTHON_AVAILABLE=false
        else
            # Add user bin to PATH for this session
            export PATH="$HOME/.local/bin:$PATH"
            echo -e "${GREEN}✓${NC} Installed maturin"
        fi
    else
        echo -e "${CYAN}  →${NC} maturin already installed: $(maturin --version)"
    fi

    if [ "$PYTHON_AVAILABLE" = true ]; then
        # Build and install using maturin
        echo -e "${CYAN}  →${NC} Building and installing Python package..."
        cd horus_py

        # Use maturin develop to build and install in development mode
        maturin develop --release --quiet

        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✓${NC} Built and installed horus_py Python package"

            # Set up proper package structure in cache for HORUS runtime
            echo -e "${CYAN}  →${NC} Setting up package structure in cache..."
            mkdir -p "$HORUS_PY_DIR/lib/horus"

            # Copy the Python wrapper
            cp -r python/horus/__init__.py "$HORUS_PY_DIR/lib/horus/" 2>/dev/null || true

            # Find and copy the compiled extension with correct name
            # maturin builds it as either libhorus_py.so or horus_py.so, but we need _horus.so
            if [ -f "target/release/libhorus_py.so" ]; then
                cp target/release/libhorus_py.so "$HORUS_PY_DIR/lib/horus/_horus.so"
            elif [ -f "target/release/horus_py.so" ]; then
                cp target/release/horus_py.so "$HORUS_PY_DIR/lib/horus/_horus.so"
            elif [ -f "target/release/libhorus_py.dylib" ]; then
                cp target/release/libhorus_py.dylib "$HORUS_PY_DIR/lib/horus/_horus.so"
            else
                echo -e "${YELLOW}⚠${NC}  Warning: Could not find compiled extension module"
            fi

            # Create metadata
            cat > "$HORUS_PY_DIR/metadata.json" << PYEOF
{
  "name": "horus_py",
  "version": "$HORUS_PY_VERSION",
  "description": "HORUS Python bindings - Python API for HORUS framework",
  "install_type": "source"
}
PYEOF

            # Test both installations: pip-installed and cache
            if python3 -c "import horus" 2>/dev/null; then
                echo -e "${GREEN}✓${NC} horus_py is importable in Python (system)"
            else
                echo -e "${YELLOW}⚠${NC}  Warning: horus_py built but import test failed (system)"
            fi

            # Test cache installation
            if PYTHONPATH="$HORUS_PY_DIR/lib" python3 -c "import horus" 2>/dev/null; then
                echo -e "${GREEN}✓${NC} horus_py is importable from cache"
            else
                echo -e "${YELLOW}⚠${NC}  Warning: horus_py not importable from cache"
            fi
        else
            echo -e "${RED}✗${NC} Failed to build horus_py"
            echo -e "${YELLOW}⚠${NC}  You can try building manually:"
            echo -e "    ${CYAN}cd horus_py && maturin develop --release${NC}"
        fi

        cd ..
    fi
else
    echo -e "${YELLOW}→${NC} Skipping horus_py (Python not available)"
fi
echo ""

# Save installed version for future updates
echo "$HORUS_VERSION" > "$VERSION_FILE"

# Step 10: Verify installation
echo -e "${CYAN}🔍 Verifying installation...${NC}"

if [ -x "$INSTALL_DIR/horus" ]; then
    echo -e "${GREEN}✓${NC} CLI binary: OK"
else
    echo -e "${RED}✗${NC} CLI binary: Missing"
fi

if [ -d "$HORUS_DIR" ]; then
    echo -e "${GREEN}✓${NC} horus library: OK"
else
    echo -e "${RED}✗${NC} horus library: Missing"
fi

if [ -d "$HORUS_CORE_DIR" ]; then
    echo -e "${GREEN}✓${NC} horus_core library: OK"
else
    echo -e "${RED}✗${NC} horus_core library: Missing"
fi

if [ -d "$HORUS_MACROS_DIR" ]; then
    echo -e "${GREEN}✓${NC} horus_macros library: OK"
else
    echo -e "${RED}✗${NC} horus_macros library: Missing"
fi

if [ -d "$HORUS_LIBRARY_DIR" ]; then
    echo -e "${GREEN}✓${NC} horus_library: OK"
else
    echo -e "${RED}✗${NC} horus_library: Missing"
fi

if [ -d "$HORUS_C_DIR" ]; then
    echo -e "${GREEN}✓${NC} horus_c: OK"
else
    echo -e "${RED}✗${NC} horus_c: Missing"
fi

if [ "$PYTHON_AVAILABLE" = true ]; then
    if [ -d "$HORUS_PY_DIR" ]; then
        echo -e "${GREEN}✓${NC} horus_py: OK"
    else
        echo -e "${RED}✗${NC} horus_py: Missing"
    fi
else
    echo -e "${YELLOW}⊘${NC} horus_py: Skipped (Python not available)"
fi

echo ""

# Check if CLI is in PATH
if command -v horus &> /dev/null; then
    echo -e "${GREEN}✓${NC} 'horus' command is available in PATH"
    echo -e "${CYAN}→${NC} Version: $(horus --version 2>/dev/null || echo 'unknown')"
else
    echo -e "${YELLOW}⚠${NC}  'horus' command not found in PATH"
    echo -e "  Add ${CYAN}$INSTALL_DIR${NC} to your PATH:"
    echo -e "  ${CYAN}export PATH=\"\$HOME/.cargo/bin:\$PATH\"${NC}"
    echo ""
    echo -e "  Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
fi

echo ""
echo -e "${GREEN}✅ HORUS installation complete!${NC}"
echo ""
echo -e "${CYAN}Next steps:${NC}"
echo "  1. Create a new project:"
echo -e "     ${CYAN}horus new my_robot${NC}"
echo ""
echo "  2. Run your project:"
echo -e "     ${CYAN}cd my_robot${NC}"
echo -e "     ${CYAN}horus run${NC}"
echo ""

if [ "$PYTHON_AVAILABLE" = true ]; then
    echo -e "${CYAN}Python bindings:${NC}"
    echo "  Try the Python API:"
    echo -e "     ${CYAN}python3 -c 'import horus; print(horus.__doc__)'${NC}"
    echo ""
fi

echo -e "For help: ${CYAN}horus --help${NC}"
echo ""
