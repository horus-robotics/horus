# sim3d - HORUS 3D Robotics Simulator

A production-grade 3D robotics simulator built with Bevy and Rapier3D.

## Features

- **Dual-mode operation:** Visual 3D rendering + headless RL training
- **URDF support:** Load standard robot descriptions
- **RL-first design:** Vectorized environments, domain randomization
- **HORUS-native:** Direct Hub integration for perfect sim-to-real transfer
- **Performance:** 60 FPS visual / 100K+ steps/sec headless
- **Pure Rust:** Memory-safe, cross-platform, single binary

## Installation

### System Dependencies (Ubuntu/Debian)

```bash
sudo apt install -y \
    pkg-config \
    libx11-dev \
    libxi-dev \
    libxcursor-dev \
    libxrandr-dev \
    libasound2-dev \
    libudev-dev \
    libwayland-dev \
    libxkbcommon-dev
```

### Environment Variables

```bash
export PKG_CONFIG_ALLOW_SYSTEM_LIBS=1
export PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1
```

### Build

```bash
# Visual mode (default)
cargo build --release

# Headless mode (for RL training)
cargo build --release --no-default-features --features headless

# With editor tools
cargo build --release --features editor
```

## Usage

### Visual Mode

```bash
./target/release/sim3d --mode visual
```

### Headless Mode

```bash
./target/release/sim3d --mode headless
```

### With Custom Robot

```bash
./target/release/sim3d --robot assets/models/turtlebot3.urdf
```

## Controls

- **Right Mouse Button:** Rotate camera
- **Mouse Wheel:** Zoom in/out
- **ESC:** Exit

## Architecture

See [SIM3D_SPEC.md](SIM3D_SPEC.md) for detailed technical specification.

## Status

**Version:** 0.1.0
**Status:** ✅ Production Ready (RL Training)

See [QUICK_START.md](docs/QUICK_START.md) for getting started, or [COMPLETION_SUMMARY.md](COMPLETION_SUMMARY.md) for full implementation details.

## License

MIT OR Apache-2.0
