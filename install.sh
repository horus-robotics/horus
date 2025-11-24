#!/bin/bash
# HORUS Installation Script
# Universal installer that works across all major operating systems
# Uses shared deps.sh for consistent dependency management

set -e  # Exit on error
set -o pipefail  # Fail on pipe errors

# Script version
SCRIPT_VERSION="2.1.0"

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

# Source shared dependency functions (if available)
DEPS_SHARED=false
if [ -f "$SCRIPT_DIR/scripts/deps.sh" ]; then
    source "$SCRIPT_DIR/scripts/deps.sh"
    DEPS_SHARED=true
fi

# Log file for debugging
LOG_FILE="/tmp/horus_install_$(date +%Y%m%d_%H%M%S).log"
exec 2> >(tee -a "$LOG_FILE" >&2)

echo -e "${CYAN}HORUS Installation Script v${SCRIPT_VERSION}${NC}"
echo ""

# Detect operating system
detect_os() {
    local os_type=""
    local os_distro=""

    if [[ "$OSTYPE" == "darwin"* ]]; then
        os_type="macos"
        os_distro="macos"
    elif [[ "$OSTYPE" == "linux"* ]]; then
        os_type="linux"

        # Check for WSL
        if grep -qE "(Microsoft|WSL)" /proc/version 2>/dev/null; then
            os_type="wsl"
        fi

        # Detect Linux distribution
        if [ -f /etc/os-release ]; then
            . /etc/os-release
            os_distro="${ID,,}"

            # Group similar distros
            case "$os_distro" in
                ubuntu|debian|raspbian|pop|mint|elementary)
                    os_distro="debian-based"
                    ;;
                fedora|rhel|centos|rocky|almalinux)
                    os_distro="fedora-based"
                    ;;
                arch|manjaro|endeavouros)
                    os_distro="arch-based"
                    ;;
                opensuse*)
                    os_distro="opensuse"
                    ;;
                alpine)
                    os_distro="alpine"
                    ;;
                void)
                    os_distro="void"
                    ;;
                nixos)
                    os_distro="nixos"
                    ;;
                *)
                    os_distro="unknown"
                    ;;
            esac
        fi
    else
        os_type="unknown"
        os_distro="unknown"
    fi

    echo "$os_type:$os_distro"
}

# Use shared OS detection if available, otherwise use local function
if [ "$DEPS_SHARED" = true ] && [ -n "$OS_TYPE" ]; then
    # OS already detected by deps.sh
    echo -e "${CYAN}[i]${NC} Detected OS: $OS_TYPE ($OS_DISTRO)"
else
    OS_INFO=$(detect_os)
    IFS=':' read -r OS_TYPE OS_DISTRO <<< "$OS_INFO"
    echo -e "${CYAN}[i]${NC} Detected OS: $OS_TYPE ($OS_DISTRO)"
fi

# Auto-install Rust if not present
install_rust() {
    if ! command -v cargo &> /dev/null; then
        echo -e "${YELLOW} Rust is not installed${NC}"
        read -p "$(echo -e ${CYAN}?${NC}) Install Rust automatically? [Y/n]: " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            echo -e "${CYAN} Installing Rust...${NC}"
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"

            if ! command -v cargo &> /dev/null; then
                echo -e "${RED} Failed to install Rust${NC}"
                echo "Please install manually from https://rustup.rs/"
                exit 1
            fi
            echo -e "${GREEN} Rust installed successfully${NC}"
        else
            echo -e "${RED} Rust is required to build HORUS${NC}"
            exit 1
        fi
    fi
}

# Install Rust
install_rust
echo -e "${CYAN}${NC} Detected Rust version: $(rustc --version)"

# Auto-install system dependencies
install_system_deps() {
    local missing_deps=""

    # Check C compiler
    if ! command -v cc &> /dev/null && ! command -v gcc &> /dev/null && ! command -v clang &> /dev/null; then
        missing_deps="compiler"
    fi

    # Check pkg-config
    if ! command -v pkg-config &> /dev/null; then
        missing_deps="$missing_deps pkg-config"
    fi

    # Check for OpenSSL
    if ! pkg-config --exists openssl 2>/dev/null && [ "$OS_TYPE" != "macos" ]; then
        missing_deps="$missing_deps openssl"
    fi

    # Check for libudev (Linux only)
    if [ "$OS_TYPE" = "linux" ] || [ "$OS_TYPE" = "wsl" ]; then
        if ! pkg-config --exists libudev 2>/dev/null; then
            missing_deps="$missing_deps libudev"
        fi
    fi

    # Check for ALSA (Linux only)
    if [ "$OS_TYPE" = "linux" ] || [ "$OS_TYPE" = "wsl" ]; then
        if ! pkg-config --exists alsa 2>/dev/null; then
            missing_deps="$missing_deps alsa"
        fi
    fi

    # Check for libclang (required for OpenCV)
    if ! ldconfig -p 2>/dev/null | grep -q libclang && [ "$OS_TYPE" != "macos" ]; then
        missing_deps="$missing_deps libclang"
    fi

    if [ -n "$missing_deps" ]; then
        echo -e "${YELLOW} Missing dependencies:${missing_deps}${NC}"
        echo ""
        read -p "$(echo -e ${CYAN}?${NC}) Install missing dependencies automatically? [Y/n]: " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            echo -e "${CYAN} Installing dependencies for $OS_DISTRO...${NC}"

            case "$OS_DISTRO" in
                debian-based)
                    sudo apt-get update
                    sudo apt-get install -y build-essential pkg-config libssl-dev libudev-dev libasound2-dev \
                        libclang-dev libopencv-dev libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
                        libxinerama-dev libwayland-dev wayland-protocols libxkbcommon-dev
                    ;;
                fedora-based)
                    sudo dnf groupinstall -y "Development Tools"
                    sudo dnf install -y pkg-config openssl-devel systemd-devel alsa-lib-devel \
                        clang-devel opencv-devel libX11-devel libXrandr-devel libXi-devel \
                        libXcursor-devel libXinerama-devel wayland-devel wayland-protocols-devel \
                        libxkbcommon-devel
                    ;;
                arch-based)
                    sudo pacman -Sy --noconfirm base-devel pkg-config openssl systemd alsa-lib \
                        clang opencv libx11 libxrandr libxi libxcursor libxinerama \
                        wayland wayland-protocols libxkbcommon
                    ;;
                opensuse)
                    sudo zypper install -y -t pattern devel_basis
                    sudo zypper install -y pkg-config libopenssl-devel libudev-devel alsa-devel \
                        clang-devel opencv-devel libX11-devel libXrandr-devel libXi-devel \
                        libXcursor-devel libXinerama-devel wayland-devel wayland-protocols-devel \
                        libxkbcommon-devel
                    ;;
                alpine)
                    sudo apk add --no-cache build-base pkgconfig openssl-dev eudev-dev alsa-lib-dev \
                        clang-dev opencv-dev libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
                        libxinerama-dev wayland-dev wayland-protocols libxkbcommon-dev
                    ;;
                macos)
                    # Check for Xcode Command Line Tools
                    if ! xcode-select -p &> /dev/null; then
                        echo -e "${CYAN} Installing Xcode Command Line Tools...${NC}"
                        xcode-select --install
                        echo "Please wait for Xcode tools to install, then re-run this script"
                        exit 1
                    fi
                    # Install via Homebrew
                    if ! command -v brew &> /dev/null; then
                        echo -e "${YELLOW} Homebrew not found${NC}"
                        echo "Please install from https://brew.sh then re-run this script"
                        exit 1
                    fi
                    brew install pkg-config opencv
                    ;;
                *)
                    echo -e "${YELLOW} Cannot auto-install for $OS_DISTRO${NC}"
                    echo ""
                    echo "Please install manually:"
                    echo "  - C/C++ compiler (gcc, clang, or cc)"
                    echo "  - pkg-config"
                    echo "  - OpenSSL development headers"
                    echo "  - libudev development headers (Linux)"
                    echo "  - ALSA development headers (Linux)"
                    echo "  - libclang development headers"
                    echo "  - OpenCV development headers (optional)"
                    echo ""
                    exit 1
                    ;;
            esac

            echo -e "${GREEN} Dependencies installed${NC}"
        else
            echo -e "${YELLOW} Continuing without installing dependencies${NC}"
            echo "Note: Build may fail if required dependencies are missing"
        fi
    else
        echo -e "${GREEN} All required dependencies found${NC}"
    fi
}

# Check and install system dependencies
# Use shared deps.sh function if available for consistency
if [ "$DEPS_SHARED" = true ]; then
    echo -e "${CYAN}[*]${NC} Checking system dependencies (using shared deps.sh)..."
    MISSING=$(check_all_deps)
    if [ -n "$MISSING" ]; then
        echo -e "${YELLOW}[!]${NC} Missing: $(get_missing_deps_readable)"
        install_system_deps
    else
        echo -e "${GREEN}[+]${NC} All system dependencies found"
    fi
else
    install_system_deps
fi

echo -e "${CYAN}${NC} Detected C compiler: $(cc --version 2>/dev/null | head -n1 || gcc --version 2>/dev/null | head -n1 || clang --version 2>/dev/null | head -n1)"
echo -e "${CYAN}${NC} Detected pkg-config: $(pkg-config --version)"

# Check for required system libraries
echo ""
echo -e "${CYAN}${NC} Checking system dependencies..."

MISSING_LIBS=""

# Core libraries
if ! pkg-config --exists openssl 2>/dev/null; then
    echo -e "${YELLOW}${NC}  OpenSSL development library not found"
    MISSING_LIBS="${MISSING_LIBS} openssl"
fi

if ! pkg-config --exists libudev 2>/dev/null; then
    echo -e "${YELLOW}${NC}  udev development library not found"
    MISSING_LIBS="${MISSING_LIBS} udev"
fi

if ! pkg-config --exists alsa 2>/dev/null; then
    echo -e "${YELLOW}${NC}  ALSA development library not found"
    MISSING_LIBS="${MISSING_LIBS} alsa"
fi

# GUI/Graphics libraries (required for sim2d and dashboard)
if [ "$(uname -s)" = "Linux" ]; then
    if ! pkg-config --exists x11 2>/dev/null; then
        echo -e "${YELLOW}${NC}  X11 development library not found"
        MISSING_LIBS="${MISSING_LIBS} x11"
    fi

    if ! pkg-config --exists xrandr 2>/dev/null; then
        echo -e "${YELLOW}${NC}  Xrandr development library not found"
        MISSING_LIBS="${MISSING_LIBS} xrandr"
    fi

    if ! pkg-config --exists xi 2>/dev/null; then
        echo -e "${YELLOW}${NC}  Xi (X11 Input) development library not found"
        MISSING_LIBS="${MISSING_LIBS} xi"
    fi

    if ! pkg-config --exists xcursor 2>/dev/null; then
        echo -e "${YELLOW}${NC}  Xcursor development library not found"
        MISSING_LIBS="${MISSING_LIBS} xcursor"
    fi

    if ! pkg-config --exists wayland-client 2>/dev/null; then
        echo -e "${YELLOW}${NC}  Wayland development library not found"
        MISSING_LIBS="${MISSING_LIBS} wayland"
    fi

    if ! pkg-config --exists xkbcommon 2>/dev/null; then
        echo -e "${YELLOW}${NC}  xkbcommon development library not found"
        MISSING_LIBS="${MISSING_LIBS} xkbcommon"
    fi
fi

# Optional but recommended libraries
OPTIONAL_MISSING=""

if ! pkg-config --exists libv4l2 2>/dev/null; then
    echo -e "${YELLOW}${NC}  libv4l2 not found (optional - needed for camera support)"
    OPTIONAL_MISSING="${OPTIONAL_MISSING} libv4l2"
fi

if ! pkg-config --exists fontconfig 2>/dev/null; then
    echo -e "${YELLOW}${NC}  fontconfig not found (optional - improves text rendering)"
    OPTIONAL_MISSING="${OPTIONAL_MISSING} fontconfig"
fi

# Hardware driver libraries (optional - for real hardware access)
HARDWARE_MISSING=""

# Check for RealSense camera support
if ! pkg-config --exists realsense2 2>/dev/null; then
    echo -e "${YELLOW}${NC}  librealsense2 not found (optional - for RealSense depth cameras)"
    HARDWARE_MISSING="${HARDWARE_MISSING} realsense"
fi

# Check for CAN utilities (useful for debugging SocketCAN)
if ! command -v cansend &> /dev/null; then
    echo -e "${YELLOW}${NC}  can-utils not found (optional - for CAN bus debugging)"
    HARDWARE_MISSING="${HARDWARE_MISSING} can-utils"
fi

if [ ! -z "$MISSING_LIBS" ]; then
    echo ""
    echo -e "${RED} Missing REQUIRED system libraries!${NC}"
    echo ""
    echo "Please install the following packages:"
    echo ""
    echo -e "${CYAN}Ubuntu/Debian/Raspberry Pi OS:${NC}"
    echo "  sudo apt update"
    echo "  sudo apt install -y build-essential pkg-config \\"
    echo "    libssl-dev libudev-dev libasound2-dev \\"
    echo "    libx11-dev libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev \\"
    echo "    libwayland-dev wayland-protocols libxkbcommon-dev \\"
    echo "    libvulkan-dev libfontconfig-dev libfreetype-dev \\"
    echo "    libv4l-dev"
    echo ""
    echo -e "${CYAN}Fedora/RHEL/CentOS:${NC}"
    echo "  sudo dnf groupinstall \"Development Tools\""
    echo "  sudo dnf install -y pkg-config openssl-devel systemd-devel alsa-lib-devel \\"
    echo "    libX11-devel libXrandr-devel libXi-devel libXcursor-devel libXinerama-devel \\"
    echo "    wayland-devel wayland-protocols-devel libxkbcommon-devel \\"
    echo "    vulkan-devel fontconfig-devel freetype-devel \\"
    echo "    libv4l-devel"
    echo ""
    echo -e "${CYAN}Arch Linux:${NC}"
    echo "  sudo pacman -S base-devel pkg-config openssl systemd alsa-lib \\"
    echo "    libx11 libxrandr libxi libxcursor libxinerama \\"
    echo "    wayland wayland-protocols libxkbcommon \\"
    echo "    vulkan-icd-loader fontconfig freetype2 \\"
    echo "    v4l-utils"
    echo ""
    echo -e "${CYAN}macOS:${NC}"
    echo "  xcode-select --install"
    echo "  brew install pkg-config"
    echo ""

    # Platform-specific notes
    if grep -q "Raspberry Pi" /proc/cpuinfo 2>/dev/null || grep -q "BCM" /proc/cpuinfo 2>/dev/null; then
        echo -e "${CYAN}Raspberry Pi detected - Additional packages:${NC}"
        echo "  sudo apt install -y libraspberrypi-dev i2c-tools python3-smbus"
        echo ""
        echo -e "${CYAN}Enable hardware interfaces (I2C, SPI, Serial):${NC}"
        echo "  sudo raspi-config"
        echo "  # Navigate to: Interface Options → I2C → Enable"
        echo "  # Navigate to: Interface Options → SPI → Enable"
        echo "  # Navigate to: Interface Options → Serial Port → Enable"
        echo ""
    fi

    if [ -f "/etc/nv_tegra_release" ] || grep -q "tegra" /proc/cpuinfo 2>/dev/null; then
        echo -e "${CYAN}NVIDIA Jetson detected - Additional packages:${NC}"
        echo "  sudo apt install -y nvidia-jetpack"
        echo "  # For GPU acceleration, ensure CUDA toolkit is installed"
        echo ""
    fi

    exit 1
fi

echo -e "${GREEN}${NC} All required system dependencies found"

if [ ! -z "$OPTIONAL_MISSING" ]; then
    echo -e "${YELLOW}${NC} Some optional dependencies missing (camera/font support may be limited)"
fi

if [ ! -z "$HARDWARE_MISSING" ]; then
    echo -e "${CYAN}${NC}  Optional hardware driver packages available:"
    echo ""
    if [[ "$HARDWARE_MISSING" == *"realsense"* ]]; then
        echo -e "  ${CYAN}RealSense Depth Cameras:${NC}"
        echo "    Ubuntu/Debian: sudo apt install -y librealsense2-dev librealsense2-utils"
        echo "    See: https://github.com/IntelRealSense/librealsense/blob/master/doc/distribution_linux.md"
        echo ""
    fi
    if [[ "$HARDWARE_MISSING" == *"can-utils"* ]]; then
        echo -e "  ${CYAN}CAN Bus Debugging Tools:${NC}"
        echo "    Ubuntu/Debian: sudo apt install -y can-utils"
        echo "    Usage: candump can0, cansend can0 123#DEADBEEF"
        echo ""
    fi
    echo -e "  ${CYAN}Note:${NC} Hardware features are optional. You can install these later if needed."
    echo ""
fi

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

# Check for pip (needed for Python bindings)
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

# Build with automatic retry and error recovery
build_with_recovery() {
    local max_retries=3
    local retry=0

    # Define packages to build (excludes dev-only: benchmarks, tests, horus_py)
    # horus_py is installed separately from PyPI, not built from source
    local BUILD_PACKAGES=(
        "horus"
        "horus_core"
        "horus_macros"
        "horus_manager"
        "horus_library"
        "sim2d"
        "sim3d"
    )

    # Build command with explicit package selection (faster, skips benchmarks/tests)
    local BUILD_CMD="cargo build --release"
    for pkg in "${BUILD_PACKAGES[@]}"; do
        BUILD_CMD="$BUILD_CMD -p $pkg"
    done

    while [ $retry -lt $max_retries ]; do
        echo ""
        echo -e "${CYAN} Building HORUS packages (attempt $((retry + 1))/$max_retries)...${NC}"
        echo -e "${CYAN}   Packages: ${BUILD_PACKAGES[*]}${NC}"
        echo -e "${CYAN}   Skipping: benchmarks, horus_py (installed from PyPI), tanksim, horus_router${NC}"

        # Clean build on retry
        if [ $retry -gt 0 ]; then
            echo -e "${CYAN} Cleaning previous build artifacts...${NC}"
            cargo clean
        fi

        # Try building only required packages
        if $BUILD_CMD 2>&1 | tee -a "$LOG_FILE"; then
            echo -e "${GREEN} Build completed successfully${NC}"
            return 0
        else
            ((retry++))

            if [ $retry -lt $max_retries ]; then
                echo -e "${YELLOW} Build failed, attempting recovery...${NC}"

                # Common fixes for build failures
                echo -e "${CYAN} Updating cargo index...${NC}"
                cargo update

                # Fix potential permission issues
                if grep -q "permission denied" "$LOG_FILE"; then
                    echo -e "${CYAN} Fixing permissions...${NC}"
                    chmod -R u+rwx target/ 2>/dev/null || true
                    chmod -R u+rwx ~/.cargo/ 2>/dev/null || true
                fi

                # Clear cargo cache if download failed
                if grep -q "failed to download\|failed to fetch" "$LOG_FILE"; then
                    echo -e "${CYAN} Clearing cargo cache...${NC}"
                    rm -rf ~/.cargo/registry/cache
                    rm -rf ~/.cargo/registry/index
                fi

                sleep 2
            fi
        fi
    done

    echo -e "${RED} Build failed after $max_retries attempts${NC}"
    echo -e "${YELLOW} Check the log file for details: $LOG_FILE${NC}"
    echo ""
    echo "Troubleshooting steps:"
    echo "  1. Try: cargo clean && rm -rf ~/.horus/cache && ./install.sh"
    echo "  2. Check if you have enough disk space: df -h"
    echo "  3. Try updating Rust: rustup update stable"
    echo "  4. Report issue: https://github.com/softmata/horus/issues"
    return 1
}

# Step 1: Build all packages
if ! build_with_recovery; then
    exit 1
fi
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
HORUS_PY_VERSION=$(grep -m1 '^version' horus_py/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

echo -e "${CYAN}  ${NC} Detected versions:"
echo "    horus: $HORUS_VERSION"
echo "    horus_core: $HORUS_CORE_VERSION"
echo "    horus_macros: $HORUS_MACROS_VERSION"
echo "    horus_library: $HORUS_LIBRARY_VERSION"
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

# Step 8: Install horus_py (Python bindings) - Optional
if [ "$PYTHON_AVAILABLE" = true ]; then
    echo -e "${CYAN}${NC} Installing horus_py@$HORUS_PY_VERSION (Python bindings - optional)..."

    # Try to install from PyPI (pre-built wheel)
    echo -e "${CYAN}  ${NC} Attempting to install from PyPI..."

    # Try pip install with --user flag first
    if pip3 install horus --user --quiet 2>/dev/null; then
        echo -e "${GREEN}${NC} Installed horus Python package from PyPI"

        # Verify installation
        if python3 -c "import horus; print(f'horus {horus.__version__}')" 2>/dev/null; then
            INSTALLED_VERSION=$(python3 -c "import horus; print(horus.__version__)" 2>/dev/null)
            echo -e "${GREEN}${NC} Python bindings working (version: $INSTALLED_VERSION)"
        else
            echo -e "${YELLOW}⊘${NC} Python bindings installed but import failed"
        fi
    else
        # If pip install fails, it's optional - don't build from source
        echo -e "${YELLOW}⊘${NC} Python bindings: Not available on PyPI yet (optional)"
        echo ""
        echo -e "  ${CYAN}Python bindings will be available after the first release.${NC}"
        echo -e "  For now, you can:"
        echo -e "    1. Wait for the next release (recommended)"
        echo -e "    2. Build from source (developers only):"
        echo -e "       ${CYAN}cd horus_py && pip install maturin && maturin develop --release${NC}"
        echo ""
    fi
else
    echo -e "${YELLOW}⊘${NC} Skipping horus_py (Python not available)"
fi
echo ""

# Step 10: Copy examples
echo -e "${CYAN}${NC} Installing examples..."
EXAMPLES_DIR="$HORUS_DIR/examples"
mkdir -p "$EXAMPLES_DIR"

# Copy snakesim example (complete directory structure)
if [ -d "horus_library/apps/snakesim" ]; then
    echo -e "${CYAN}  ${NC} Installing Snake Game example..."

    # Copy entire snakesim directory to preserve structure
    cp -r horus_library/apps/snakesim "$EXAMPLES_DIR/" 2>/dev/null || true

    # Clean up unnecessary files from the copied example
    rm -rf "$EXAMPLES_DIR/snakesim/.horus" 2>/dev/null || true
    rm -rf "$EXAMPLES_DIR/snakesim/.gitignore" 2>/dev/null || true
    rm -rf "$EXAMPLES_DIR/snakesim/snakesim_gui/.horus" 2>/dev/null || true
    rm -rf "$EXAMPLES_DIR/snakesim/snakesim_gui/.claude" 2>/dev/null || true

    # Verify the copy
    if [ -f "$EXAMPLES_DIR/snakesim/main.rs" ] && [ -f "$EXAMPLES_DIR/snakesim/snakesim_gui/main.rs" ]; then
        echo -e "${GREEN}${NC} Installed Snake Game example"
        echo -e "${CYAN}     ${NC} Backend: main.rs + horus.yaml"
        echo -e "${CYAN}     ${NC} GUI: snakesim_gui/main.rs + horus.yaml"
    else
        echo -e "${YELLOW}${NC}  Warning: Snake Game example may be incomplete"
    fi
else
    echo -e "${YELLOW}${NC}  Snake Game example not found in source"
fi

# Copy wallesim example (WALL-E 3D simulation)
if [ -d "horus_library/apps/wallesim" ]; then
    echo -e "${CYAN}  ${NC} Installing WALL-E 3D Simulation example..."

    # Copy entire wallesim directory to preserve structure
    cp -r horus_library/apps/wallesim "$EXAMPLES_DIR/" 2>/dev/null || true

    # Clean up unnecessary files from the copied example
    rm -rf "$EXAMPLES_DIR/wallesim/.horus" 2>/dev/null || true
    rm -rf "$EXAMPLES_DIR/wallesim/.gitignore" 2>/dev/null || true
    rm -rf "$EXAMPLES_DIR/wallesim/.claude" 2>/dev/null || true

    # Verify the copy
    if [ -f "$EXAMPLES_DIR/wallesim/world.yaml" ] && [ -f "$EXAMPLES_DIR/wallesim/models/walle/walle.urdf" ]; then
        echo -e "${GREEN}${NC} Installed WALL-E 3D Simulation example"
        echo -e "${CYAN}     ${NC} World: world.yaml (Axiom cargo bay)"
        echo -e "${CYAN}     ${NC} Robot: models/walle/walle.urdf"
    else
        echo -e "${YELLOW}${NC}  Warning: WALL-E simulation example may be incomplete"
    fi
else
    echo -e "${YELLOW}${NC}  WALL-E simulation example not found in source"
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
echo "  2. Or try the Snake Game example:"
echo -e "     ${CYAN}cp -r ~/.horus/cache/horus@${HORUS_VERSION}/examples/snakesim ~/my_snakesim${NC}"
echo -e "     ${CYAN}cd ~/my_snakesim${NC}"
echo ""
echo -e "     Terminal 1 (Backend): ${CYAN}horus run${NC}"
echo -e "     Terminal 2 (GUI): ${CYAN}cd snakesim_gui && horus run${NC}"
echo ""
echo -e "     Use ${CYAN}Arrow Keys${NC} or ${CYAN}WASD${NC} to control the snake!"
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

