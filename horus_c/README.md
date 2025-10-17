# HORUS C API

Minimal C API for integrating hardware drivers with HORUS robotics framework.

## Purpose

This C API exists **solely** for hardware integration - connecting existing C drivers (LiDAR, cameras, robot arms) to HORUS. All core logic should be implemented in Rust nodes.

## Quick Start

```c
#include <horus.h>

int main() {
    // Initialize node
    init("my_driver");

    // Create publisher
    Pub pub = publisher("sensor_data", MSG_LASER_SCAN);

    // Main loop
    while (ok()) {
        LaserScan scan = read_from_hardware();
        send(pub, &scan);
        sleep_ms(100);
    }

    shutdown();
    return 0;
}
```

## Building

```bash
make all          # Build everything
./lidar_driver    # Run example
```

## API Design

**Handle-based for safety:**
- No direct pointers
- Automatic memory management
- Can't crash from C code

**Simple operations:**
- `init()` / `shutdown()`
- `publisher()` / `subscriber()`
- `send()` / `recv()`
- No complex callbacks or vtables

## Examples

### LiDAR Driver
Bridges rplidar or other LiDAR SDKs:
```c
Pub scan_pub = publisher("laser_scan", MSG_LASER_SCAN);
LaserScan scan = lidar_get_scan();
send(scan_pub, &scan);
```

### Camera Driver
Integrates RealSense, USB cameras:
```c
Pub img_pub = publisher("camera/image", MSG_IMAGE);
Image img = camera_capture();
send(img_pub, &img);
```

### Robot Arm
Controls industrial robots:
```c
Sub cmd_sub = subscriber("joint_commands", MSG_JOINT_STATE);
JointState cmd;
if (recv(cmd_sub, &cmd)) {
    robot_move_joints(cmd.positions);
}
```

## Message Types

Standard robotics messages:
- `Twist` - Velocity commands
- `Pose` - Position/orientation
- `LaserScan` - LiDAR data
- `Image` - Camera frames
- `JointState` - Robot joints
- `IMU` - Inertial data
- `PointCloud` - 3D points

## Integration with Rust

C nodes communicate seamlessly with Rust nodes:

```c
// C publishes
send(pub, &laser_scan);
```

```rust
// Rust receives
let scan: LaserScan = sub.recv()?;
```

## Performance

- **Overhead**: ~50ns per message (negligible vs hardware latency)
- **Hardware latency**: 1-100ms (the real bottleneck)
- **Throughput**: Sufficient for any hardware driver

## When to Use

✅ **Use C API for:**
- Vendor SDKs (librealsense, rplidar)
- Proprietary drivers
- Legacy C libraries
- Hardware interfaces

❌ **Don't use C API for:**
- Business logic
- Algorithm implementation
- New development
- Performance-critical code

Write those in Rust for safety and speed.

## License

Same as HORUS core