# HORUS C++ API

Modern C++ interface for HORUS robotics framework with 40+ message types.

## Overview

The HORUS C++ API provides:
- **Type-safe messaging** - Template-based publishers and subscribers
- **40+ message types** - Geometry, sensors, vision, perception, navigation, control, diagnostics
- **RAII resource management** - Automatic initialization and cleanup
- **Node framework** - Build modular robotics applications  
- **Shared memory IPC** - Zero-copy communication between processes

For C-only projects, use the underlying C API (horus.h). This C++ wrapper provides a safer, more ergonomic interface.

## Installation

```bash
cd /path/to/HORUS
./install.sh
```

Headers install to `~/.horus/cache/horus_cpp@VERSION/include/`

## Quick Start - Basic Example

```cpp
#include <horus.hpp>

int main() {
    // Initialize HORUS system
    horus::System sys("my_robot");

    // Create publisher and subscriber
    horus::Publisher<horus::Twist> cmd_pub("cmd_vel");
    horus::Subscriber<horus::Twist> cmd_sub("cmd_vel");

    // Send velocity command
    horus::Twist cmd = horus::Twist::new_2d(1.0, 0.5);  // 1 m/s forward, 0.5 rad/s turn
    cmd_pub.send(cmd);

    // Receive data
    horus::Twist received;
    if (cmd_sub.recv(received)) {
        horus::Log::info("Received velocity command");
    }

    return 0;
}
```

## Compilation

```bash
g++ -std=c++17 my_robot.cpp \
    -I~/.horus/cache/horus_cpp@VERSION/include \
    -I~/.horus/cache/horus_c@VERSION/include \
    -L~/.horus/cache/horus_cpp@VERSION/lib \
    -L~/.horus/cache/horus_c@VERSION/lib \
    -lhorus_cpp -lhorus_c -lpthread -lrt \
    -o my_robot
```

##  Message Library

### Geometry Messages

```cpp
// Twist - Velocity command
horus::Twist vel = horus::Twist::new_2d(1.0, 0.5);
vel.linear[0] = 1.0;   // Forward velocity (m/s)
vel.angular[2] = 0.5;  // Angular velocity (rad/s)

// Pose2D - Robot position  
horus::Pose2D pose(5.0, 3.0, 1.57);  // x, y, theta
double dist = pose.distance_to(other_pose);

// Quaternion - 3D orientation
horus::Quaternion q = horus::Quaternion::from_euler(0, 0, 1.57);  // 90° yaw

// Transform - 3D pose
horus::Transform tf(1.0, 2.0, 3.0, 0, 0, 0, 1);  // Position + orientation
```

**Available**: `Twist`, `Pose2D`, `Vector3`, `Point3`, `Quaternion`, `Transform`

### Sensor Messages

```cpp
// LaserScan - Lidar data
horus::LaserScan scan;
scan.ranges[0] = 5.2f;  // Range at 0°
size_t valid_points = scan.valid_count();
float min_range = scan.min_range();

// IMU - Inertial measurement
horus::Imu imu;
imu.set_orientation_from_euler(0, 0, 1.57);
imu.angular_velocity[2] = 0.5;  // rad/s

// Odometry - Combined pose + velocity
horus::Odometry odom;
odom.pose = horus::Pose2D(10.0, 5.0, 0.0);
odom.twist = horus::Twist::new_2d(1.0, 0.0);

// BatteryState - Power monitoring
horus::BatteryState battery;
battery.voltage = 24.5f;
battery.percentage = 0.85f;
```

**Available**: `LaserScan`, `Imu`, `Odometry`, `Range`, `BatteryState`

### Vision Messages

```cpp
// CameraInfo - Camera calibration
horus::CameraInfo cam = horus::CameraInfo::create(
    640, 480,      // width, height
    525.0, 525.0,  // focal lengths
    320.0, 240.0   // principal point
);

// Detection - Object detection
horus::RegionOfInterest bbox(100, 150, 80, 120);
horus::Detection det("person", 0.95f, bbox);

// DetectionArray - Multiple detections
horus::DetectionArray detections;
detections.add_detection(det);
```

**Available**: `Image`, `CompressedImage`, `CameraInfo`, `RegionOfInterest`, `Detection`, `DetectionArray`, `StereoInfo`

### Perception Messages

```cpp
// PointCloud - 3D point cloud
horus::Point3 points[3] = {
    horus::Point3(1.0, 2.0, 3.0),
    horus::Point3(4.0, 5.0, 6.0),
    horus::Point3(7.0, 8.0, 9.0)
};
horus::PointCloud cloud = horus::PointCloud::create_xyz(points, 3);

// BoundingBox3D - 3D object detection
horus::BoundingBox3D bbox(
    horus::Point3(0, 0, 0),  // center
    horus::Vector3(2, 4, 6)  // size
);
bbox.set_label("car");
double volume = bbox.volume();
```

**Available**: `PointCloud`, `PointField`, `BoundingBox3D`, `BoundingBoxArray3D`, `DepthImage`, `PlaneDetection`, `PlaneArray`

### Navigation Messages

```cpp
// Goal - Navigation target
horus::Goal goal(
    horus::Pose2D(10.0, 5.0, 0.0),  // target pose
    0.1,  // position tolerance (m)
    0.1   // angle tolerance (rad)
);
goal.timeout_seconds = 30.0;

// Path - Waypoints
horus::Path path;
path.add_waypoint(horus::Waypoint(horus::Pose2D(0, 0, 0)));
path.add_waypoint(horus::Waypoint(horus::Pose2D(5, 0, 0)));

// OccupancyGrid - 2D map
horus::OccupancyGrid grid;
grid.init(100, 100, 0.05f, horus::Pose2D(0, 0, 0));  // 5m x 5m
grid.set_occupancy(50, 50, 100);  // Mark center as occupied
```

**Available**: `Goal`, `GoalStatus`, `GoalResult`, `Waypoint`, `Path`, `OccupancyGrid`, `CostMap`, `VelocityObstacle`, `PathPlan`

### Control Messages

```cpp
// MotorCommand - Direct motor control
horus::MotorCommand motor = horus::MotorCommand::velocity(1, 10.0);

// DifferentialDriveCommand - Two-wheeled robot
horus::DifferentialDriveCommand drive = horus::DifferentialDriveCommand::from_twist(
    1.0, 0.5,   // linear, angular velocity
    0.3, 0.05   // wheel_base, wheel_radius
);

// PidConfig - PID controller gains
horus::PidConfig pid = horus::PidConfig::pd(2.0, 0.5);

// JointCommand - Multi-joint control
horus::JointCommand joints;
joints.add_position("shoulder", 1.57);
joints.add_velocity("elbow", 0.5);
```

**Available**: `MotorCommand`, `DifferentialDriveCommand`, `ServoCommand`, `PidConfig`, `TrajectoryPoint`, `JointCommand`

### Diagnostics Messages

```cpp
// Heartbeat - Node alive signal
horus::Heartbeat hb = horus::Heartbeat::create("robot_node", 42);
hb.update(123.45);  // Update uptime

// Status - System status
horus::Status status = horus::Status::warn(100, "Low battery warning");
status.set_component("power_monitor");

// EmergencyStop - Safety signal
horus::EmergencyStop estop = horus::EmergencyStop::engage("Obstacle detected");

// ResourceUsage - System monitoring
horus::ResourceUsage resources;
resources.cpu_percent = 45.2f;
resources.memory_percent = 62.8f;
```

**Available**: `Heartbeat`, `Status`, `StatusLevel`, `EmergencyStop`, `ResourceUsage`, `SafetyStatus`

## Node Framework

Build modular robotics applications:

```cpp
#include <horus.hpp>

class MyRobotNode : public horus::Node {
public:
    MyRobotNode() : Node("my_robot") {}

    bool init(horus::NodeContext& ctx) override {
        // Create publishers and subscribers
        cmd_pub_ = ctx.create_publisher<horus::Twist>("cmd_vel");
        odom_sub_ = ctx.create_subscriber<horus::Odometry>("odom");
        ctx.log_info("Robot node initialized");
        return true;
    }

    void tick(horus::NodeContext& ctx) override {
        // Read sensor data
        horus::Odometry odom;
        if (odom_sub_.recv(odom)) {
            ctx.log_info("Position: (" +
                std::to_string(odom.pose.x) + ", " +
                std::to_string(odom.pose.y) + ")");
        }

        // Send velocity command
        horus::Twist cmd = horus::Twist::new_2d(1.0, 0.0);
        cmd_pub_.send(cmd);
    }

    void shutdown(horus::NodeContext& ctx) override {
        ctx.log_info("Robot node shutting down");
    }

private:
    horus::Publisher<horus::Twist> cmd_pub_;
    horus::Subscriber<horus::Odometry> odom_sub_;
};

int main() {
    horus::System sys("robot_system");
    horus::Scheduler sched("main_scheduler");

    MyRobotNode robot_node;
    sched.add(robot_node, 2, true);  // priority=Normal, logging=true

    sched.run();  // Runs at 60 FPS
    return 0;
}
```

### Priority Levels

- **0 (Critical)** - Time-critical tasks (control loops)
- **1 (High)** - Important tasks (sensor processing)
- **2 (Normal)** - Standard tasks (navigation) **[default]**
- **3 (Low)** - Background tasks (mapping)
- **4 (Background)** - Lowest priority (logging)

### Node Context

```cpp
// Create publishers/subscribers with logging
auto pub = ctx.create_publisher<horus::Twist>("topic");
auto sub = ctx.create_subscriber<horus::Twist>("topic");

// Shorter aliases
auto pub = ctx.pub<horus::Twist>("topic");
auto sub = ctx.sub<horus::Twist>("topic");

// Logging (appears in HORUS dashboard)
ctx.log_info("Information message");
ctx.log_warn("Warning message");
ctx.log_error("Error message");
```

## Logging

```cpp
// Global logging
horus::Log::info("Application started");
horus::Log::warn("Configuration file not found");
horus::Log::error("Failed to connect to hardware");
horus::Log::debug("Debug information");

// Node logging (with context)
ctx.log_info("Node started");
ctx.log_warn("Sensor timeout");
ctx.log_error("Motor fault detected");
```

Logs stored in `/dev/shm/horus_logs` ring buffer, viewable in HORUS dashboard.

## Utilities

```cpp
// Time
uint64_t now = horus::time_now_ms();  // Milliseconds since epoch
horus::sleep_ms(100);  // Sleep for 100ms

// Helpers
auto vec = horus::make_vector3(1.0, 2.0, 3.0);
auto quat = horus::make_quaternion(0, 0, 0, 1);
auto twist = horus::make_twist(1.0, 0, 0, 0, 0, 0.5);
auto pose = horus::make_pose2d(5.0, 3.0, 1.57);
```

## Examples

See `examples/` directory:

- `pubsub_simple_new.cpp` - Simple publisher/subscriber
- `framework_demo.cpp` - Node framework demonstration
- `message_showcase.cpp` - All 40+ message types showcase
- `lidar_driver_new.cpp` - Lidar sensor driver example
- `robot_system_new.cpp` - Complete robot system example

## Binary Compatibility

All C++ message types are binary-compatible with Rust, enabling:
- Zero-copy shared memory IPC
- Cross-language communication
- Direct memory mapping

**Important**: Fixed-size arrays used for shared memory safety. Respect maximum sizes.

## Error Handling

```cpp
try {
    horus::System sys("my_robot");
    // ... your code
} catch (const horus::HorusException& e) {
    std::cerr << "HORUS error: " << e.what() << std::endl;
    return 1;
}
```

## Thread Safety

- Publishers/subscribers are **not** thread-safe - use one per thread
- Shared memory synchronized internally
- Node framework handles concurrency automatically

## Performance

- **Zero-copy IPC** - Messages shared via memory-mapped files
- **60 FPS node execution** - Consistent real-time performance
- **Lock-free ring buffers** - For logging and telemetry
- **Priority scheduling** - Critical tasks execute first

## Troubleshooting

### Version mismatch
```
Version mismatch detected!
  CLI version:       0.1.5
  Installed libraries: 0.1.3
```
**Solution**: Run `./install.sh`

### Missing libraries
```
/usr/bin/ld: cannot find -lhorus_cpp
```
**Solution**: Check include/lib paths point to correct cache version

### Segmentation fault
- Ensure publishers/subscribers created after `horus::System` init
- Check message sizes don't exceed fixed array limits
- Verify binary compatibility between Rust/C++ message definitions

## License

Apache-2.0
