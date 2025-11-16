# sim2d Examples

This directory contains example scripts demonstrating sim2d features.

## Prerequisites

1. **HORUS Python bindings installed:**
   ```bash
   cd horus_py
   pip install -e .
   ```

2. **sim2d simulator built:**
   ```bash
   cargo build --package sim2d
   ```

## Available Examples

### dynamic_obstacles.py

Demonstrates dynamic obstacle spawning and removal at runtime.

**Start sim2d first:**
```bash
# Terminal 1: Run simulator
cargo run --package sim2d

# Or with custom config
cargo run --package sim2d -- --world configs/world.yaml
```

**Then run example:**
```bash
# Terminal 2: Run example
cd horus_library/tools/sim2d/examples

# Simple demo (adds 3 obstacles)
python3 dynamic_obstacles.py simple

# Grid demo (creates 3x3 grid of colored circles)
python3 dynamic_obstacles.py grid

# Animation demo (creates and removes obstacles in sequence)
python3 dynamic_obstacles.py animation

# Interactive menu (manual control)
python3 dynamic_obstacles.py interactive
```

**Features demonstrated:**
- Adding rectangular obstacles with custom colors
- Adding circular obstacles
- Removing obstacles by position
- Batch obstacle operations
- Real-time control via HORUS topics

**Message Format:**

The script publishes to `/sim2d/obstacle_cmd` topic:

```python
# Add rectangle
{
    "action": "add",
    "obstacle": {
        "pos": [3.0, 2.0],      # [x, y] in meters
        "shape": "rectangle",
        "size": [1.5, 1.0],     # [width, height]
        "color": [0.8, 0.2, 0.2]  # Optional RGB (0.0-1.0)
    }
}

# Add circle
{
    "action": "add",
    "obstacle": {
        "pos": [-2.0, 4.0],
        "shape": "circle",
        "size": [0.8, 0.8],     # [radius, _]
        "color": [0.2, 0.8, 0.8]
    }
}

# Remove obstacle (10cm position tolerance)
{
    "action": "remove",
    "obstacle": {
        "pos": [3.0, 2.0],      # Position to remove
        "shape": "rectangle",   # Shape doesn't matter
        "size": [0.0, 0.0]
    }
}
```

## Creating Your Own Scripts

```python
from horus import Hub

# Create hub for obstacle commands
obstacle_hub = Hub("/sim2d/obstacle_cmd")

# Add obstacle
cmd = {
    "action": "add",
    "obstacle": {
        "pos": [5.0, 5.0],
        "shape": "circle",
        "size": [1.0, 1.0],
        "color": [1.0, 0.0, 0.0]  # Red
    }
}
obstacle_hub.send(cmd)

# Remove obstacle
cmd = {
    "action": "remove",
    "obstacle": {
        "pos": [5.0, 5.0],
        "shape": "rectangle",
        "size": [0.0, 0.0]
    }
}
obstacle_hub.send(cmd)
```

## Troubleshooting

**"Connected to /sim2d/obstacle_cmd topic" but nothing happens:**
- Make sure sim2d is running first
- Check sim2d terminal for obstacle spawn messages
- Verify HORUS is working: `horus --version`

**Import errors:**
- Install HORUS Python bindings: `cd horus_py && pip install -e .`
- Make sure you're using Python 3.6+

**Obstacles don't appear:**
- Check obstacle position is in view (camera bounds)
- Verify message format matches expected structure
- Enable sim2d logging to see debug messages

**Obstacles don't get removed:**
- Removal uses 10cm position tolerance
- Make sure position matches spawned obstacle (within 0.1m)
- Check sim2d terminal for removal confirmation

## Tips

- **Colors**: Use RGB values between 0.0-1.0 (not 0-255)
- **Positions**: Origin (0, 0) is center of world
- **Sizes**: All measurements in meters
- **Performance**: sim2d can handle 100+ dynamic obstacles
- **Removal tolerance**: 10cm (0.1m) for position matching
