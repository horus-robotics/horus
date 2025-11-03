# HORUS C++ Message Library

This directory contains C++ message definitions that are binary-compatible with the Rust message types defined in `horus_library/messages/`.

## Structure

```
cpp/
└── include/
    └── horus/
        ├── messages.hpp          # Single include for all messages
        └── messages/
            ├── geometry.hpp      # Geometric types (Twist, Pose2D, Vector3, Quaternion, Transform)
            └── sensor.hpp        # Sensor types (LaserScan, IMU, Odometry, Range, BatteryState)
```

## Usage

### In your C++ code:

```cpp
#include <horus/messages.hpp>

using namespace horus::messages;

// Create a twist message
Twist cmd = Twist::new_2d(1.0, 0.5);  // 1 m/s forward, 0.5 rad/s rotation

// Create a laser scan
LaserScan scan;
scan.ranges[0] = 5.2;  // 5.2 meters at index 0

// Create an IMU message
Imu imu;
imu.set_orientation_from_euler(0.0, 0.0, 1.57);  // 90 degrees yaw
```

### Message Types

#### Geometry (`messages/geometry.hpp`)
- `Vector3` - 3D vector
- `Point3` - 3D point
- `Quaternion` - 3D rotation
- `Twist` - Linear and angular velocity
- `Pose2D` - 2D position and orientation
- `Transform` - 3D transformation

#### Sensors (`messages/sensor.hpp`)
- `LaserScan` - 2D lidar scan (360 points)
- `Imu` - Inertial measurement unit data
- `Odometry` - Combined pose + velocity estimate
- `Range` - Single distance sensor reading
- `BatteryState` - Battery monitoring data

## Binary Compatibility

These structures are designed to be binary-compatible with Rust:

- All fields use standard layout
- Matching field types (`float` = `f32`, `double` = `f64`)
- Matching field order
- No virtual functions or inheritance
- No padding issues (same alignment)

## Integration with horus_cpp

The `horus_cpp` FFI layer uses these message types:

```cpp
// In horus_cpp/include/horus.h
#include <horus/messages.hpp>

// Use the types directly in FFI
bool send(HorusPub pub, const horus::messages::Twist* data);
bool recv(HorusSub sub, horus::messages::Twist* data);
```

## Adding New Message Types

1. Define the Rust struct in `horus_library/messages/*.rs`
2. Create matching C++ struct in `cpp/include/horus/messages/*.hpp`
3. Ensure binary layout compatibility
4. Add to `messages.hpp` for convenience

## Notes

- All message types have `timestamp` fields (uint64_t nanoseconds since epoch)
- All messages have `update_timestamp()` method to set current time
- Geometry types use `double` (f64) for precision
- Sensor arrays use fixed sizes for shared memory safety
