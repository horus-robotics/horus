# sim2d Configuration Files

This directory contains example configuration files for customizing the sim2d simulator.

## Quick Start

### Use Default Settings
```bash
horus sim 2d
# Or: cargo run --release
```

### Load Custom Robot
```bash
horus sim 2d --robot configs/robot.yaml
# Or: cargo run --release -- --robot configs/robot.yaml
```

### Load Custom World
```bash
horus sim 2d --world configs/world.yaml
# Or: cargo run --release -- --world configs/world.yaml
```

### Load Both
```bash
horus sim 2d --robot configs/robot.yaml --world configs/world.yaml
```

### Load World from Image (Occupancy Grid)
```bash
horus sim 2d --world_image map.png --resolution 0.05 --threshold 128
```

## File Formats

Both **YAML** and **TOML** formats are supported. The simulator auto-detects based on file extension.

## Robot Configuration

### Fields

| Field | Type | Unit | Description |
|-------|------|------|-------------|
| `width` | float | meters | Robot width (perpendicular to forward direction) |
| `length` | float | meters | Robot length (along forward direction) |
| `max_speed` | float | m/s | Maximum linear velocity |
| `color` | [float; 3] | RGB 0.0-1.0 | Visual color in simulator |

### Example (YAML)

```yaml
# robot.yaml
width: 0.5        # 0.5m wide
length: 0.8       # 0.8m long
max_speed: 2.0    # 2 m/s maximum
color: [0.2, 0.8, 0.2]  # Green [R, G, B]
```

### Example (TOML)

```toml
# robot.toml
width = 0.5
length = 0.8
max_speed = 2.0
color = [0.2, 0.8, 0.2]  # Green
```

### Color Examples

```yaml
# Common colors (RGB format, values 0.0 to 1.0)
color: [0.2, 0.8, 0.2]   # Green (default)
color: [0.2, 0.2, 0.8]   # Blue
color: [0.8, 0.2, 0.2]   # Red
color: [0.8, 0.8, 0.2]   # Yellow
color: [0.8, 0.2, 0.8]   # Magenta
color: [0.2, 0.8, 0.8]   # Cyan
color: [0.5, 0.5, 0.5]   # Gray
```

## World Configuration

### Fields

| Field | Type | Unit | Description |
|-------|------|------|-------------|
| `width` | float | meters | World width |
| `height` | float | meters | World height |
| `obstacles` | array | - | List of rectangular obstacles |

### Obstacle Fields

| Field | Type | Unit | Description |
|-------|------|------|-------------|
| `pos` | [float; 2] | meters | Center position [x, y] |
| `size` | [float; 2] | meters | Dimensions [width, height] |

### Example (YAML)

```yaml
# world.yaml
width: 20.0    # 20m x 15m world
height: 15.0

obstacles:
  - pos: [5.0, 5.0]      # Obstacle at (5, 5)
    size: [2.0, 1.0]     # 2m wide, 1m tall

  - pos: [-3.0, -2.0]    # Another obstacle
    size: [1.5, 1.5]     # Square obstacle

  - pos: [0.0, 7.0]      # Wall-like obstacle
    size: [3.0, 0.5]     # Long and thin
```

### Example (TOML)

```toml
# world.toml
width = 20.0
height = 15.0

[[obstacles]]
pos = [5.0, 5.0]
size = [2.0, 1.0]

[[obstacles]]
pos = [-3.0, -2.0]
size = [1.5, 1.5]
```

## World from Image

You can load a world from an **occupancy grid image** (PNG, JPG, or PGM format):

```bash
horus sim 2d --world_image my_map.png --resolution 0.05 --threshold 128
```

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `--world_image` | - | Path to image file (PNG/JPG/PGM) |
| `--resolution` | 0.05 | Meters per pixel |
| `--threshold` | 128 | Obstacle threshold (0-255, darker = obstacle) |

### How It Works

1. Image is converted to grayscale
2. Pixels darker than `threshold` become obstacles
3. Each pixel becomes a small square obstacle
4. World size = image dimensions × resolution

**Example:**
- Image: 400×300 pixels
- Resolution: 0.05 m/pixel
- Result: 20m × 15m world

### Creating Occupancy Grid Images

**Black = Obstacle, White = Free space**

```
Pixel value < threshold → Obstacle
Pixel value ≥ threshold → Free space
```

You can create maps using:
- Image editors (GIMP, Photoshop, Paint)
- SLAM tools (ROS gmapping, cartographer)
- Drawing programs (draw walls in black on white background)

## Coordinate System

- **Origin (0, 0)**: Center of the world
- **+X**: Right
- **+Y**: Up
- **Robot starts**: At origin (0, 0) facing up (+Y direction)

```
        +Y (up)
         ↑
         |
-X ←-----+----→ +X (right)
         |
         ↓
        -Y (down)
```

## Example Scenarios

### Small Indoor Robot
```yaml
# small_robot.yaml
width: 0.3
length: 0.4
max_speed: 1.0
color: [0.2, 0.2, 0.8]  # Blue
```

### Large Outdoor Robot
```yaml
# large_robot.yaml
width: 1.2
length: 1.8
max_speed: 5.0
color: [0.8, 0.5, 0.2]  # Orange
```

### Warehouse Environment
```yaml
# warehouse.yaml
width: 50.0
height: 30.0

obstacles:
  # Storage racks
  - pos: [10.0, 10.0]
    size: [2.0, 8.0]
  - pos: [10.0, -10.0]
    size: [2.0, 8.0]
  - pos: [-10.0, 10.0]
    size: [2.0, 8.0]
  - pos: [-10.0, -10.0]
    size: [2.0, 8.0]

  # Loading dock wall
  - pos: [20.0, 0.0]
    size: [0.5, 20.0]
```

### Simple Maze
```yaml
# maze.yaml
width: 10.0
height: 10.0

obstacles:
  # Horizontal walls
  - pos: [0.0, 2.0]
    size: [6.0, 0.3]
  - pos: [0.0, -2.0]
    size: [6.0, 0.3]

  # Vertical walls
  - pos: [2.0, 0.0]
    size: [0.3, 3.0]
  - pos: [-2.0, 0.0]
    size: [0.3, 3.0]
```

## Advanced Usage

### Custom Control Topic
```bash
horus sim 2d --topic /my_robot/cmd_vel
```

### Headless Mode (No GUI)
```bash
horus sim 2d --headless
```

### All Options Combined
```bash
horus sim 2d \
  --robot configs/robot.yaml \
  --world configs/world.yaml \
  --topic /robot/cmd_vel \
  --name my_robot
```

## Controlling the Robot

The simulator listens for `CmdVel` messages on the control topic (default: `/robot/cmd_vel`).

**From another terminal:**
```bash
# Run a simple driver node
cargo run -p simple_driver

# Or publish commands directly using HORUS API
```

**CmdVel message format:**
```rust
CmdVel {
    linear: 1.0,    // Forward velocity (m/s)
    angular: 0.5,   // Angular velocity (rad/s)
}
```

## Tips

1. **Start simple**: Use default configs first, then customize
2. **Test configs**: Run `horus sim 2d --robot your_robot.yaml` to verify
3. **Visualize**: The GUI shows your robot and obstacles in real-time
4. **Iterate**: Adjust and reload - no compilation needed!
5. **Version control**: Keep your custom configs in your project repo

## Troubleshooting

### Robot not visible
- Check `color` values are between 0.0 and 1.0
- Ensure `width` and `length` are reasonable (> 0.1m)

### Obstacles not showing
- Verify `pos` coordinates are within world bounds
- Check `size` values are positive

### Config file not loading
- Check file extension (`.yaml` or `.toml`)
- Verify YAML/TOML syntax (indentation matters in YAML)
- Look for error messages in terminal output

### Robot doesn't move
- Check HORUS topic connection
- Verify control topic name matches your publisher
- Ensure `max_speed` is reasonable (> 0)

## See Also

- [sim2d source code](../src/main.rs) - Full implementation details
- [HORUS documentation](../../../../docs-site/) - Framework docs
- [Example driver](../../../../horus_library/apps/) - Sample control code
