#!/bin/bash
# HORUS Installation Script
# Installs the HORUS CLI and runtime libraries from source

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${CYAN} HORUS Installation Script${NC}"
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED} Error: Rust is not installed${NC}"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

echo -e "${CYAN}${NC} Detected Rust version: $(rustc --version)"

# Check if C compiler/linker is installed
if ! command -v cc &> /dev/null && ! command -v gcc &> /dev/null; then
    echo -e "${RED} Error: C compiler not found${NC}"
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

echo -e "${CYAN}${NC} Detected C compiler: $(cc --version | head -n1)"

# Check if pkg-config is installed
if ! command -v pkg-config &> /dev/null; then
    echo -e "${RED} Error: pkg-config not found${NC}"
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

echo -e "${CYAN}${NC} Detected pkg-config: $(pkg-config --version)"

# Check if Python is installed (for horus_py)
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version | awk '{print $2}')
    echo -e "${CYAN}${NC} Detected Python: $PYTHON_VERSION"

    # Check if Python version is 3.9+
    PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d. -f1)
    PYTHON_MINOR=$(echo $PYTHON_VERSION | cut -d. -f2)

    if [ "$PYTHON_MAJOR" -ge 3 ] && [ "$PYTHON_MINOR" -ge 9 ]; then
        PYTHON_AVAILABLE=true
    else
        echo -e "${YELLOW}${NC}  Python 3.9+ required for horus_py (found $PYTHON_VERSION)"
        echo -e "  horus_py will be skipped"
        PYTHON_AVAILABLE=false
    fi
else
    echo -e "${YELLOW}${NC}  Python3 not found - horus_py will be skipped"
    PYTHON_AVAILABLE=false
fi

# Check for pip (needed for maturin)
if [ "$PYTHON_AVAILABLE" = true ]; then
    if command -v pip3 &> /dev/null || command -v pip &> /dev/null; then
        echo -e "${CYAN}${NC} Detected pip: $(pip3 --version 2>/dev/null || pip --version)"
    else
        echo -e "${YELLOW}${NC}  pip not found - horus_py will be skipped"
        echo "  Install pip: sudo apt install python3-pip (Debian/Ubuntu)"
        PYTHON_AVAILABLE=false
    fi
fi

echo ""

# Determine installation paths
INSTALL_DIR="$HOME/.cargo/bin"
CACHE_DIR="$HOME/.horus/cache"

echo -e "${CYAN}${NC} Installation paths:"
echo "  CLI binary: $INSTALL_DIR/horus"
echo "  Libraries:  $CACHE_DIR/"
echo ""

# Ask for confirmation
read -p "$(echo -e ${YELLOW}?${NC}) Proceed with installation? [Y/n]: " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]] && [[ ! -z $REPLY ]]; then
    echo -e "${RED}${NC} Installation cancelled"
    exit 0
fi

# Step 1: Build all packages in release mode
echo ""
echo -e "${CYAN} Building HORUS packages (release mode)...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED} Build failed${NC}"
    exit 1
fi

echo -e "${GREEN}${NC} Build completed"
echo ""

# Step 2: Install CLI binary
echo -e "${CYAN}${NC} Installing CLI binary..."

if [ ! -d "$INSTALL_DIR" ]; then
    mkdir -p "$INSTALL_DIR"
fi

cp target/release/horus "$INSTALL_DIR/horus"
chmod +x "$INSTALL_DIR/horus"

echo -e "${GREEN}${NC} CLI installed to $INSTALL_DIR/horus"

# Install sim2d binary
if [ -f "target/release/sim2d" ]; then
    cp target/release/sim2d "$INSTALL_DIR/sim2d"
    chmod +x "$INSTALL_DIR/sim2d"
    echo -e "${GREEN}${NC} sim2d binary installed to $INSTALL_DIR/sim2d"
fi

echo ""

# Step 3: Create cache directory structure
echo -e "${CYAN}${NC} Setting up library cache..."

mkdir -p "$CACHE_DIR"

# Get version from Cargo.toml
HORUS_VERSION=$(grep -m1 '^version' horus/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_CORE_VERSION=$(grep -m1 '^version' horus_core/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_MACROS_VERSION=$(grep -m1 '^version' horus_macros/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_LIBRARY_VERSION=$(grep -m1 '^version' horus_library/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_CPP_VERSION=$(grep -m1 '^version' horus_cpp/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
HORUS_PY_VERSION=$(grep -m1 '^version' horus_py/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

echo -e "${CYAN}  ${NC} Detected versions:"
echo "    horus: $HORUS_VERSION"
echo "    horus_core: $HORUS_CORE_VERSION"
echo "    horus_macros: $HORUS_MACROS_VERSION"
echo "    horus_library: $HORUS_LIBRARY_VERSION"
echo "    horus_cpp: $HORUS_CPP_VERSION"
echo "    horus_py: $HORUS_PY_VERSION"
echo ""

# Check for version changes
VERSION_FILE="$HOME/.horus/installed_version"
if [ -f "$VERSION_FILE" ]; then
    OLD_VERSION=$(cat "$VERSION_FILE")
    if [ "$OLD_VERSION" != "$HORUS_VERSION" ]; then
        echo -e "${YELLOW}${NC}  Version changed: ${OLD_VERSION}  ${HORUS_VERSION}"
        echo -e "${CYAN}${NC} Cleaning old library versions..."

        # Remove old versioned directories
        rm -rf "$CACHE_DIR/horus@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_core@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_macros@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_library@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_cpp@$OLD_VERSION" 2>/dev/null || true
        rm -rf "$CACHE_DIR/horus_py@$OLD_VERSION" 2>/dev/null || true

        echo -e "${GREEN}${NC} Old versions removed"
        echo ""
    fi
fi

# Step 4: Install horus_core library
echo -e "${CYAN}${NC} Installing horus_core@$HORUS_CORE_VERSION..."
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

echo -e "${GREEN}${NC} Installed horus_core"

# Step 5: Install horus library
echo -e "${CYAN}${NC} Installing horus@$HORUS_VERSION..."
HORUS_DIR="$CACHE_DIR/horus@$HORUS_VERSION"
mkdir -p "$HORUS_DIR/lib"

# Copy compiled libraries
cp -r target/release/libhorus.* "$HORUS_DIR/lib/" 2>/dev/null || true
cp -r target/release/deps/libhorus*.rlib "$HORUS_DIR/lib/" 2>/dev/null || true

# Also copy target/release for Cargo path dependencies
mkdir -p "$HORUS_DIR/target/release"
cp -r target/release/libhorus*.rlib "$HORUS_DIR/target/release/" 2>/dev/null || true
cp -r target/release/deps/libhorus_core*.rlib "$HORUS_DIR/target/release/" 2>/dev/null || true

# CRITICAL: Copy ALL transitive dependencies for Cargo compilation
mkdir -p "$HORUS_DIR/target/release/deps"
echo -e "${CYAN}  ${NC} Bundling transitive dependencies for user projects..."
cp target/release/deps/*.rlib "$HORUS_DIR/target/release/deps/" 2>/dev/null || true
echo -e "${GREEN}${NC} Bundled $(ls target/release/deps/*.rlib 2>/dev/null | wc -l) dependency libraries"

# Copy source Cargo.toml and src for `horus run` Cargo compilation
echo -e "${CYAN}  ${NC} Copying source files for horus run compatibility..."

# Copy workspace Cargo.toml to make cache a valid workspace
cp Cargo.toml "$HORUS_DIR/Cargo.toml" 2>/dev/null || true

# Copy horus crate
mkdir -p "$HORUS_DIR/horus"
cp horus/Cargo.toml "$HORUS_DIR/horus/" 2>/dev/null || true
cp -r horus/src "$HORUS_DIR/horus/" 2>/dev/null || true

# Copy horus_core crate
mkdir -p "$HORUS_DIR/horus_core"
cp horus_core/Cargo.toml "$HORUS_DIR/horus_core/" 2>/dev/null || true
cp -r horus_core/src "$HORUS_DIR/horus_core/" 2>/dev/null || true

# Copy horus_macros crate
mkdir -p "$HORUS_DIR/horus_macros"
cp horus_macros/Cargo.toml "$HORUS_DIR/horus_macros/" 2>/dev/null || true
cp -r horus_macros/src "$HORUS_DIR/horus_macros/" 2>/dev/null || true

# Copy horus_library crate (has lib.rs and subdirectories, not src/)
mkdir -p "$HORUS_DIR/horus_library"
cp horus_library/Cargo.toml "$HORUS_DIR/horus_library/" 2>/dev/null || true
cp horus_library/lib.rs "$HORUS_DIR/horus_library/" 2>/dev/null || true
cp -r horus_library/nodes "$HORUS_DIR/horus_library/" 2>/dev/null || true
cp -r horus_library/messages "$HORUS_DIR/horus_library/" 2>/dev/null || true
cp -r horus_library/traits "$HORUS_DIR/horus_library/" 2>/dev/null || true
cp -r horus_library/algorithms "$HORUS_DIR/horus_library/" 2>/dev/null || true

# Create metadata
cat > "$HORUS_DIR/metadata.json" << EOF
{
  "name": "horus",
  "version": "$HORUS_VERSION",
  "description": "HORUS Framework - Main library",
  "install_type": "source"
}
EOF

echo -e "${GREEN}${NC} Installed horus"

# Step 6: Install horus_macros
echo -e "${CYAN}${NC} Installing horus_macros@$HORUS_MACROS_VERSION..."
HORUS_MACROS_DIR="$CACHE_DIR/horus_macros@$HORUS_MACROS_VERSION"
mkdir -p "$HORUS_MACROS_DIR/lib"

# Copy procedural macro library
cp -r target/release/libhorus_macros.* "$HORUS_MACROS_DIR/lib/" 2>/dev/null || true
cp -r target/release/deps/libhorus_macros*.so "$HORUS_MACROS_DIR/lib/" 2>/dev/null || true

# Also copy to target/release for Cargo
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

echo -e "${GREEN}${NC} Installed horus_macros"

# Step 7: Install horus_library
echo -e "${CYAN}${NC} Installing horus_library@$HORUS_LIBRARY_VERSION..."
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

echo -e "${GREEN}${NC} Installed horus_library"

# Step 8: Install horus_cpp (C++ Framework)
echo -e "${CYAN}${NC} Installing horus_cpp@$HORUS_CPP_VERSION (C++ Framework)..."
HORUS_CPP_DIR="$CACHE_DIR/horus_cpp@$HORUS_CPP_VERSION"
mkdir -p "$HORUS_CPP_DIR/lib"
mkdir -p "$HORUS_CPP_DIR/include"

# Build the C++ library if not already built
if [ ! -f "target/release/libhorus_cpp.so" ] && [ ! -f "target/release/libhorus_cpp.dylib" ]; then
    echo -e "${CYAN}  ${NC} Building C++ framework library..."
    cd horus_cpp
    cargo build --release
    cd ..
fi

# Copy C++ library (both dynamic and static)
cp -r target/release/libhorus_cpp.so "$HORUS_CPP_DIR/lib/" 2>/dev/null || true
cp -r target/release/libhorus_cpp.a "$HORUS_CPP_DIR/lib/" 2>/dev/null || true
cp -r target/release/libhorus_cpp.dylib "$HORUS_CPP_DIR/lib/" 2>/dev/null || true  # macOS

# Copy C++ header files
if [ -f "horus_cpp/include/horus.hpp" ]; then
    cp horus_cpp/include/horus.hpp "$HORUS_CPP_DIR/include/"
fi

# horus.h (internal FFI) is needed by horus.hpp
if [ -f "horus_cpp/include/horus.h" ]; then
    cp horus_cpp/include/horus.h "$HORUS_CPP_DIR/include/"
fi

# Create metadata
cat > "$HORUS_CPP_DIR/metadata.json" << EOF
{
  "name": "horus_cpp",
  "version": "$HORUS_CPP_VERSION",
  "description": "HORUS C++ Framework - Modern C++ API with Node/Scheduler pattern",
  "install_type": "source"
}
EOF

echo -e "${GREEN}${NC} Installed horus_cpp"

# Step 9: Install horus_py (Python bindings)
if [ "$PYTHON_AVAILABLE" = true ]; then
    echo -e "${CYAN}${NC} Installing horus_py@$HORUS_PY_VERSION (Python bindings)..."
    HORUS_PY_DIR="$CACHE_DIR/horus_py@$HORUS_PY_VERSION"
    mkdir -p "$HORUS_PY_DIR"

    # Check if maturin is installed
    if ! command -v maturin &> /dev/null; then
        echo -e "${CYAN}  ${NC} Installing maturin (Python/Rust build tool)..."
        pip3 install maturin --user --quiet

        if [ $? -ne 0 ]; then
            echo -e "${RED}${NC} Failed to install maturin"
            echo -e "${YELLOW}${NC}  Skipping horus_py installation"
            echo -e "  You can install it manually later:"
            echo -e "    ${CYAN}pip install maturin${NC}"
            echo -e "    ${CYAN}cd horus_py && maturin develop --release${NC}"
            PYTHON_AVAILABLE=false
        else
            # Add user bin to PATH for this session
            export PATH="$HOME/.local/bin:$PATH"
            echo -e "${GREEN}${NC} Installed maturin"
        fi
    else
        echo -e "${CYAN}  ${NC} maturin already installed: $(maturin --version)"
    fi

    if [ "$PYTHON_AVAILABLE" = true ]; then
        # Build and install using maturin
        echo -e "${CYAN}  ${NC} Building and installing Python package..."
        cd horus_py

        # Use maturin develop to build and install in development mode
        maturin develop --release --quiet

        if [ $? -eq 0 ]; then
            echo -e "${GREEN}${NC} Built and installed horus_py Python package"

            # Set up proper package structure in cache for HORUS runtime
            echo -e "${CYAN}  ${NC} Setting up package structure in cache..."
            mkdir -p "$HORUS_PY_DIR/lib/horus"

            # Copy the Python wrapper
            cp -r python/horus/__init__.py "$HORUS_PY_DIR/lib/horus/" 2>/dev/null || true

            # Find and copy the compiled extension
            # maturin develop installs it to python/horus/ with .abi3.so extension
            EXTENSION_FOUND=false

            # Check python/horus/ directory (where maturin develop puts it)
            if [ -f "python/horus/_horus.abi3.so" ]; then
                cp python/horus/_horus.abi3.so "$HORUS_PY_DIR/lib/horus/_horus.so"
                EXTENSION_FOUND=true
            elif [ -f "python/horus/_horus.so" ]; then
                cp python/horus/_horus.so "$HORUS_PY_DIR/lib/horus/_horus.so"
                EXTENSION_FOUND=true
            # Check for macOS
            elif [ -f "python/horus/_horus.abi3.dylib" ]; then
                cp python/horus/_horus.abi3.dylib "$HORUS_PY_DIR/lib/horus/_horus.so"
                EXTENSION_FOUND=true
            fi

            if [ "$EXTENSION_FOUND" = false ]; then
                echo -e "${YELLOW}${NC}  Warning: Could not find compiled extension module"
                echo -e "  Expected location: python/horus/_horus.abi3.so"
            else
                echo -e "${GREEN}${NC} Copied compiled extension to cache"
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
                echo -e "${GREEN}${NC} horus_py is importable in Python (system)"
            else
                echo -e "${YELLOW}${NC}  Warning: horus_py built but import test failed (system)"
            fi

            # Test cache installation
            if PYTHONPATH="$HORUS_PY_DIR/lib" python3 -c "import horus" 2>/dev/null; then
                echo -e "${GREEN}${NC} horus_py is importable from cache"
            else
                echo -e "${YELLOW}${NC}  Warning: horus_py not importable from cache"
            fi
        else
            echo -e "${RED}${NC} Failed to build horus_py"
            echo -e "${YELLOW}${NC}  You can try building manually:"
            echo -e "    ${CYAN}cd horus_py && maturin develop --release${NC}"
        fi

        cd ..
    fi
else
    echo -e "${YELLOW}${NC} Skipping horus_py (Python not available)"
fi
echo ""

# Step 10: Copy examples
echo -e "${CYAN}${NC} Installing examples..."
EXAMPLES_DIR="$HORUS_DIR/examples"
mkdir -p "$EXAMPLES_DIR"

# Copy snakesim example
if [ -d "horus_library/apps/snakesim" ]; then
    mkdir -p "$EXAMPLES_DIR/snakesim"
    mkdir -p "$EXAMPLES_DIR/snakesim/snakesim_gui"

    # Copy backend files
    cp horus_library/apps/snakesim/main.rs "$EXAMPLES_DIR/snakesim/" 2>/dev/null || true
    cp horus_library/apps/snakesim/horus.yaml "$EXAMPLES_DIR/snakesim/" 2>/dev/null || true
    cp horus_library/apps/snakesim/README.md "$EXAMPLES_DIR/snakesim/" 2>/dev/null || true

    # Copy GUI source files
    if [ -f "horus_library/apps/snakesim/snakesim_gui/main.rs" ]; then
        cp horus_library/apps/snakesim/snakesim_gui/main.rs "$EXAMPLES_DIR/snakesim/snakesim_gui/" 2>/dev/null || true
    fi
    if [ -f "horus_library/apps/snakesim/snakesim_gui/horus.yaml" ]; then
        cp horus_library/apps/snakesim/snakesim_gui/horus.yaml "$EXAMPLES_DIR/snakesim/snakesim_gui/" 2>/dev/null || true
    fi

    echo -e "${GREEN}${NC} Installed snakesim example with GUI"
fi

echo ""

# Save installed version for future updates
echo "$HORUS_VERSION" > "$VERSION_FILE"

# Migrate old config files from localhost to production
AUTH_CONFIG="$HOME/.horus/auth.json"
if [ -f "$AUTH_CONFIG" ]; then
    if grep -q "localhost" "$AUTH_CONFIG" 2>/dev/null; then
        echo -e "${CYAN}${NC} Migrating registry configuration..."
        # Update localhost URLs to production
        sed -i.bak 's|http://localhost:3001|https://horus-marketplace-api.onrender.com|g' "$AUTH_CONFIG"
        sed -i.bak 's|http://localhost:8080|https://horus-marketplace-api.onrender.com|g' "$AUTH_CONFIG"
        echo -e "${GREEN}${NC} Registry URL updated to production"
        echo ""
    fi
fi

# Step 10: Verify installation
echo -e "${CYAN} Verifying installation...${NC}"

if [ -x "$INSTALL_DIR/horus" ]; then
    echo -e "${GREEN}${NC} CLI binary: OK"
else
    echo -e "${RED}${NC} CLI binary: Missing"
fi

if [ -d "$HORUS_DIR" ]; then
    echo -e "${GREEN}${NC} horus library: OK"
else
    echo -e "${RED}${NC} horus library: Missing"
fi

if [ -d "$HORUS_CORE_DIR" ]; then
    echo -e "${GREEN}${NC} horus_core library: OK"
else
    echo -e "${RED}${NC} horus_core library: Missing"
fi

if [ -d "$HORUS_MACROS_DIR" ]; then
    echo -e "${GREEN}${NC} horus_macros library: OK"
else
    echo -e "${RED}${NC} horus_macros library: Missing"
fi

if [ -d "$HORUS_LIBRARY_DIR" ]; then
    echo -e "${GREEN}${NC} horus_library: OK"
else
    echo -e "${RED}${NC} horus_library: Missing"
fi

if [ -d "$HORUS_CPP_DIR" ]; then
    echo -e "${GREEN}${NC} horus_cpp: OK"
else
    echo -e "${RED}${NC} horus_cpp: Missing"
fi

if [ "$PYTHON_AVAILABLE" = true ]; then
    if [ -d "$HORUS_PY_DIR" ]; then
        echo -e "${GREEN}${NC} horus_py: OK"
    else
        echo -e "${RED}${NC} horus_py: Missing"
    fi
else
    echo -e "${YELLOW}⊘${NC} horus_py: Skipped (Python not available)"
fi

echo ""

# Check if CLI is in PATH
if command -v horus &> /dev/null; then
    echo -e "${GREEN}${NC} 'horus' command is available in PATH"
    echo -e "${CYAN}${NC} Version: $(horus --version 2>/dev/null || echo 'unknown')"
else
    echo -e "${YELLOW}${NC}  'horus' command not found in PATH"
    echo -e "  Add ${CYAN}$INSTALL_DIR${NC} to your PATH:"
    echo -e "  ${CYAN}export PATH=\"\$HOME/.cargo/bin:\$PATH\"${NC}"
    echo ""
    echo -e "  Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
fi

# Step 11: Setup shell completions
echo ""
echo -e "${CYAN}${NC} Setting up shell completions..."

# Detect user's shell
SHELL_NAME=$(basename "$SHELL")
COMPLETION_INSTALLED=false

case "$SHELL_NAME" in
    bash)
        # Try to set up bash completions
        if [ -f ~/.bashrc ]; then
            # Check if completion is already in bashrc
            if ! grep -q "horus completion bash" ~/.bashrc 2>/dev/null; then
                echo "" >> ~/.bashrc
                echo "# HORUS shell completions" >> ~/.bashrc
                echo 'eval "$(horus completion bash)"' >> ~/.bashrc
                echo -e "${GREEN}${NC} Added bash completions to ~/.bashrc"
                COMPLETION_INSTALLED=true
            else
                echo -e "${GREEN}${NC} Bash completions already configured"
                COMPLETION_INSTALLED=true
            fi
        fi
        ;;
    zsh)
        # Try to set up zsh completions
        if [ -f ~/.zshrc ]; then
            if ! grep -q "horus completion zsh" ~/.zshrc 2>/dev/null; then
                echo "" >> ~/.zshrc
                echo "# HORUS shell completions" >> ~/.zshrc
                echo 'eval "$(horus completion zsh)"' >> ~/.zshrc
                echo -e "${GREEN}${NC} Added zsh completions to ~/.zshrc"
                COMPLETION_INSTALLED=true
            else
                echo -e "${GREEN}${NC} Zsh completions already configured"
                COMPLETION_INSTALLED=true
            fi
        fi
        ;;
    fish)
        # Try to set up fish completions
        FISH_COMP_DIR="$HOME/.config/fish/completions"
        if command -v fish &> /dev/null; then
            mkdir -p "$FISH_COMP_DIR"
            if [ -x "$INSTALL_DIR/horus" ]; then
                "$INSTALL_DIR/horus" completion fish > "$FISH_COMP_DIR/horus.fish" 2>/dev/null
                echo -e "${GREEN}${NC} Generated fish completions to $FISH_COMP_DIR/horus.fish"
                COMPLETION_INSTALLED=true
            fi
        fi
        ;;
    *)
        echo -e "${YELLOW}${NC}  Unknown shell: $SHELL_NAME"
        echo -e "  You can manually set up completions later:"
        echo -e "    ${CYAN}horus completion --help${NC}"
        ;;
esac

if [ "$COMPLETION_INSTALLED" = true ]; then
    echo -e "${CYAN}  ℹ${NC}  Shell completions will be active in new terminal sessions"
    echo -e "  To use in this session: ${CYAN}source ~/.${SHELL_NAME}rc${NC} (bash/zsh)"
fi

echo ""
echo -e "${GREEN} HORUS installation complete!${NC}"
echo ""
echo -e "${CYAN}Next steps:${NC}"
echo "  1. Create a new project:"
echo -e "     ${CYAN}horus new my_robot${NC}"
echo ""
echo "  2. Or try the snake game example:"
echo -e "     ${CYAN}cp -r ~/.horus/cache/horus@$HORUS_VERSION/examples/snakesim ~/my_snakesim${NC}"
echo -e "     Terminal 1: ${CYAN}cd ~/my_snakesim && horus run${NC}  (backend with keyboard input)"
echo -e "     Terminal 2: ${CYAN}cd ~/my_snakesim/snakesim_gui && horus run${NC}  (GUI visualization)"
echo -e "     Use Arrow Keys or WASD to control the snake!"
echo ""
echo "  3. Run your project:"
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

# Anonymous install counter (opt-out: export HORUS_NO_TELEMETRY=1)
if [ -z "$HORUS_NO_TELEMETRY" ]; then
    (curl -X POST https://telemetry.horus-registry.dev/count \
         -H "Content-Type: application/json" \
         -d "{\"event\":\"install\",\"os\":\"$(uname -s)\",\"timestamp\":$(date +%s)}" \
         --max-time 3 --silent 2>/dev/null || true) &
fi
