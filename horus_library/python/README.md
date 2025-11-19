# HORUS Library - Python Bindings

Standard robotics messages, nodes, and algorithms for HORUS (Python).

## Package Structure

This package mirrors the Rust `horus_library` structure:

```
horus_library/                  (Python package)
├── messages/                   # Standard message types
│   ├── __init__.py            # Geometry, Control, Sensor, etc.
│   └── (re-exports from _library.so)
├── nodes/                      # Reusable robotics nodes (future)
│   └── __init__.py
├── algorithms/                 # Common algorithms (future)
│   └── __init__.py
├── _library.abi3.so           # Compiled Rust extension
└── __init__.py                # Package root (re-exports everything)
```

## Installation

Built automatically with the main HORUS installation.

## Usage

### Recommended: Via main `horus` package

```python
import horus

# All library types available at top level
pose = horus.Pose2D(x=1.0, y=2.0, theta=0.5)
cmd = horus.CmdVel(linear=1.0, angular=0.5)
scan = horus.LaserScan()
```

### Alternative: Organized submodule imports

```python
from horus_library.messages import Pose2D, CmdVel, LaserScan
# from horus_library.nodes import ...  # Future
# from horus_library.algorithms import ...  # Future
```

## Available Messages

See [Message Types Reference](https://docs.horus-registry.dev/messages) for complete documentation.

## Cross-Language Compatibility

All message types are binary-compatible with Rust, enabling seamless Python ↔ Rust communication.
