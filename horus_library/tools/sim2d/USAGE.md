# sim2d Usage Guide

Complete guide to using sim2d - HORUS 2D Robot Simulator with image support and headless mode.

## Quick Start

### Basic Usage (GUI Mode)
```bash
# Run with default world
horus sim --2d

# Or directly via cargo
cd horus_library/tools/sim2d
cargo run --release
```

### Headless Mode (No GUI)
```bash
# Perfect for CI/CD, servers, SSH
horus sim --2d --headless

# With custom config
horus sim --2d --headless --world world.yaml --topic robot.cmd_vel
```

## Configuration Formats

### YAML Configuration

**robot.yaml:**
```yaml
width: 0.5        # Robot width in meters
length: 0.8       # Robot length in meters
max_speed: 2.0    # Maximum velocity in m/s
color: [0.2, 0.8, 0.2]  # RGB color [0.0-1.0]
```

**world.yaml:**
```yaml
width: 20.0    # World width in meters
height: 15.0   # World height in meters

obstacles:
  - pos: [5.0, 5.0]     # Center position [x, y]
    size: [2.0, 1.0]    # Dimensions [width, height]
  - pos: [-3.0, -2.0]
    size: [1.5, 1.5]
```

### TOML Configuration

**robot.toml:**
```toml
[robot]
base_width = 0.35
wheel_radius = 0.08
max_linear_velocity = 1.2
```

**world.toml:**
```toml
[world.bounds]
min_x = -20.0
max_x = 20.0
min_y = -20.0
max_y = 20.0

[[obstacles.aabb]]
x = -5.0
y = -1.0
w = 2.0
h = 6.0
```

## Image-Based World Loading

### Load from Floor Plan Image

```bash
# PNG, JPG, or PGM occupancy grid
horus sim --2d --world-image floor_plan.png

# Custom resolution (meters per pixel)
horus sim --2d --world-image map.pgm --resolution 0.05

# Custom threshold (0-255, darker = obstacle)
horus sim --2d --world-image building.jpg --threshold 100

# All together
horus sim --2d \
  --world-image warehouse.png \
  --resolution 0.02 \
  --threshold 128 \
  --topic robot.cmd_vel
```

### Image Format

**Supported formats:**
- PNG (recommended for editing)
- JPEG/JPG
- PGM (ROS standard occupancy grids)

**How it works:**
1. Image converted to grayscale
2. Pixels darker than threshold  obstacles
3. Each obstacle pixel  collision square
4. World size = image_size × resolution

**Example: Creating a test map in Python:**
```python
from PIL import Image, ImageDraw

# 400x400 white background
img = Image.new('L', (400, 400), 255)
draw = ImageDraw.Draw(img)

# Draw black obstacles (walls, objects)
draw.rectangle([100, 100, 150, 300], fill=0)  # Vertical wall
draw.rectangle([250, 50, 350, 100], fill=0)   # Horizontal wall
draw.ellipse([180, 180, 220, 220], fill=0)    # Round obstacle

img.save('test_map.png')
```

Then use it:
```bash
horus sim --2d --world-image test_map.png --resolution 0.05
# Creates 20m × 20m world
```

## Complete CLI Reference

```bash
sim2d [OPTIONS]

Options:
  --robot <FILE>          Robot configuration (YAML/TOML)
  --world <FILE>          World configuration (YAML/TOML)
  --world-image <FILE>    World image (PNG/JPG/PGM) - takes priority over --world
  --resolution <FLOAT>    Image resolution in m/pixel [default: 0.05]
  --threshold <0-255>     Obstacle threshold [default: 128]
  --topic <NAME>          Control topic name [default: robot.cmd_vel]
  --name <NAME>           Robot name [default: robot]
  --headless              Run without GUI (for CI/CD, servers)
  -h, --help              Print help
```

## Usage Patterns

### Pattern 1: Development (GUI)
```bash
# Terminal 1: Visual simulator
horus sim --2d --world-image office.png

# Terminal 2: Control logic
cat > controller.rs << 'EOF'
use horus::prelude::*;
use horus_library::messages::CmdVel;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd: Hub<CmdVel> = Hub::new("cmd_vel")?;

    loop {
        cmd.send(CmdVel::new(1.0, 0.5), None)?;
        std::thread::sleep(Duration::from_millis(100));
    }
}
EOF

horus run controller.rs
```

### Pattern 2: CI/CD Testing (Headless)
```bash
#!/bin/bash
# test_navigation.sh

# Start headless simulator in background
horus sim --2d --headless --world-image test_map.png &
SIM_PID=$!

# Run navigation test
timeout 30s horus run navigation_test.rs

# Cleanup
kill $SIM_PID
```

### Pattern 3: ROS Map Compatibility
```bash
# Use ROS PGM map directly
horus sim --2d \
  --world-image /path/to/ros/map.pgm \
  --resolution 0.05 \
  --threshold 254
```

### Pattern 4: Multi-Robot Simulation
```bash
# Robot 1
horus sim --2d --topic robot1.cmd_vel --name robot1 &

# Robot 2
horus sim --2d --topic robot2.cmd_vel --name robot2 &

# Control both
horus run multi_robot_controller.rs
```

## Performance Comparison

| Mode | Render | Physics | Memory | Use Case |
|------|--------|---------|--------|----------|
| **GUI** | 60 Hz | 60 Hz | ~150 MB | Development, debugging |
| **Headless** | None | 1000+ Hz | ~30 MB | CI/CD, servers, batch |

## Common Workflows

### Load Existing ROS Map
```bash
# From ROS workspace
horus sim --2d \
  --world-image ~/ros_ws/maps/office.pgm \
  --resolution 0.05 \
  --threshold 254
```

### Quick Prototype with Sketch
1. Draw map in any image editor (Paint, GIMP, etc.)
2. Black = obstacles, White = free space
3. Save as PNG
4. Load: `horus sim --2d --world-image sketch.png`

### Test Navigation Algorithm
```bash
# Headless for faster testing
horus sim --2d --headless --world-image maze.png &

# Run navigation
horus run path_planner.rs

# Results logged to console
```

### Batch Testing Different Maps
```bash
for map in maps/*.png; do
    echo "Testing with $map"
    timeout 60s horus sim --2d --headless --world-image "$map" &
    SIM_PID=$!

    horus run test_suite.rs
    kill $SIM_PID
done
```

## Troubleshooting

### Image Loading Issues

**"Failed to load world from image"**
- Check file path is correct
- Ensure format is PNG, JPG, or PGM
- Verify file permissions

**Too many obstacles / too slow**
- Increase `--resolution` (e.g., 0.1 instead of 0.05)
- Use smaller image
- Simplify map (less detail)

**Robot passes through obstacles**
- Decrease `--threshold` to detect lighter pixels
- Ensure obstacles are dark enough in image
- Check image is not inverted

### Headless Mode Issues

**"Failed to create window" in SSH**
- Use `--headless` flag
- Or use `xvfb-run horus sim --2d`

**Robot doesn't move in headless**
- Headless mode still requires controller
- Check HORUS topics are connected
- Verify control topic name matches

## Tips

**Resolution Guidelines:**
- High detail: 0.01-0.02 m/pixel
- Standard: 0.05 m/pixel (ROS default)
- Low detail: 0.1-0.2 m/pixel

**Threshold Guidelines:**
- Strict (light gray = obstacle): 200
- Standard: 128 (default)
- Permissive (only black = obstacle): 50

**Performance:**
- Larger images = more obstacles = slower physics
- For large maps, use lower resolution
- Headless mode is ~15x faster than GUI

## Examples Directory

See `examples/` for:
- Circle pattern navigation
- Square path following
- Obstacle avoidance
- Multi-robot coordination
- Image map loading

## Next Steps

1. Create your first map: `python create_map.py`
2. Run simulator: `horus sim --2d --world-image map.png`
3. Control robot: `horus run your_controller.rs`
4. Deploy to real robot: Use same HORUS topics!

## Links

- Main README: `../../../README.md`
- HORUS Docs: `../../../docs-site/`
- CLI Reference: See `horus sim --2d --help`
