// HORUS C++ Message Library Showcase
// Demonstrates all major message categories

#include <horus.hpp>
#include <iostream>

using namespace horus;

void showcase_geometry_messages() {
    std::cout << "\n=== Geometry Messages ===" << std::endl;

    // Vector3 and Point3
    Vector3 velocity(1.0, 0.5, 0.0);
    Point3 position(10.0, 5.0, 0.0);
    std::cout << "Velocity: (" << velocity.x << ", " << velocity.y << ", " << velocity.z << ")" << std::endl;
    std::cout << "Position: (" << position.x << ", " << position.y << ", " << position.z << ")" << std::endl;

    // Twist - robot velocity command
    Twist cmd = Twist::new_2d(1.0, 0.5);  // 1 m/s forward, 0.5 rad/s rotation
    std::cout << "Twist command: linear=" << cmd.linear[0] << " m/s, angular=" << cmd.angular[2] << " rad/s" << std::endl;

    // Pose2D - robot position
    Pose2D pose(5.0, 3.0, 1.57);  // x=5m, y=3m, theta=90°
    std::cout << "Robot pose: (" << pose.x << ", " << pose.y << ", " << pose.theta << ")" << std::endl;

    // Quaternion - 3D orientation
    Quaternion q = Quaternion::from_euler(0.0, 0.0, 1.57);  // 90° yaw
    std::cout << "Quaternion: (" << q.x << ", " << q.y << ", " << q.z << ", " << q.w << ")" << std::endl;
}

void showcase_sensor_messages() {
    std::cout << "\n=== Sensor Messages ===" << std::endl;

    // LaserScan - lidar data
    LaserScan scan;
    scan.ranges[0] = 5.2f;
    scan.ranges[90] = 3.1f;
    scan.ranges[180] = 10.5f;
    std::cout << "LaserScan: " << scan.valid_count() << " valid points, min="
              << scan.min_range() << "m" << std::endl;

    // IMU - inertial measurement
    Imu imu;
    imu.set_orientation_from_euler(0.0, 0.0, 1.57);  // 90° yaw
    imu.angular_velocity[2] = 0.5;  // 0.5 rad/s rotation
    std::cout << "IMU: orientation valid=" << imu.has_orientation() << std::endl;

    // Odometry - combined pose + velocity
    Odometry odom;
    odom.pose = Pose2D(10.0, 5.0, 0.0);
    odom.twist = Twist::new_2d(1.0, 0.0);
    std::cout << "Odometry: pose=(" << odom.pose.x << ", " << odom.pose.y << ")" << std::endl;

    // BatteryState - power monitoring
    BatteryState battery;
    battery.voltage = 24.5f;
    battery.percentage = 0.85f;
    battery.power_supply_status = 2;  // discharging
    std::cout << "Battery: " << battery.voltage << "V, "
              << (battery.percentage * 100) << "%" << std::endl;
}

void showcase_vision_messages() {
    std::cout << "\n=== Vision Messages ===" << std::endl;

    // CameraInfo - camera calibration
    CameraInfo cam = CameraInfo::create(640, 480, 525.0, 525.0, 320.0, 240.0);
    double fx, fy, cx, cy;
    cam.get_focal_lengths(fx, fy);
    cam.get_principal_point(cx, cy);
    std::cout << "Camera: " << cam.width << "x" << cam.height
              << ", focal=(" << fx << ", " << fy << ")" << std::endl;

    // Detection - object detection result
    RegionOfInterest bbox(100, 150, 80, 120);
    Detection det("person", 0.95f, bbox);
    std::cout << "Detection: class=" << det.class_name
              << ", confidence=" << det.confidence << std::endl;

    // DetectionArray - multiple detections
    DetectionArray detections;
    detections.add_detection(det);
    detections.add_detection(Detection("car", 0.88f, RegionOfInterest(200, 100, 150, 100)));
    std::cout << "DetectionArray: " << detections.get_count() << " objects" << std::endl;
}

void showcase_perception_messages() {
    std::cout << "\n=== Perception Messages ===" << std::endl;

    // PointCloud - 3D point cloud
    Point3 points[3] = {
        Point3(1.0, 2.0, 3.0),
        Point3(4.0, 5.0, 6.0),
        Point3(7.0, 8.0, 9.0)
    };
    PointCloud cloud = PointCloud::create_xyz(points, 3);
    std::cout << "PointCloud: " << cloud.point_count() << " points" << std::endl;

    // BoundingBox3D - 3D object detection
    BoundingBox3D bbox3d(Point3(0.0, 0.0, 0.0), Vector3(2.0, 4.0, 6.0));
    bbox3d.set_label("car");
    bbox3d.confidence = 0.92f;
    std::cout << "BoundingBox3D: " << bbox3d.label
              << ", volume=" << bbox3d.volume() << "m³" << std::endl;

    // DepthImage - depth camera data
    DepthImage depth;
    depth.width = 640;
    depth.height = 480;
    depth.set_depth(320, 240, 1500);  // 1.5m at center
    std::cout << "DepthImage: " << depth.width << "x" << depth.height
              << ", center depth=" << depth.get_depth(320, 240) << "mm" << std::endl;
}

void showcase_navigation_messages() {
    std::cout << "\n=== Navigation Messages ===" << std::endl;

    // Goal - navigation target
    Goal goal(Pose2D(10.0, 5.0, 0.0), 0.1, 0.1);
    goal.timeout_seconds = 30.0;
    goal.priority = 1;
    std::cout << "Goal: target=(" << goal.target_pose.x << ", " << goal.target_pose.y
              << "), timeout=" << goal.timeout_seconds << "s" << std::endl;

    // Path - navigation waypoints
    Path path;
    path.add_waypoint(Waypoint(Pose2D(0.0, 0.0, 0.0)));
    path.add_waypoint(Waypoint(Pose2D(5.0, 0.0, 0.0)));
    path.add_waypoint(Waypoint(Pose2D(10.0, 5.0, 1.57)));
    std::cout << "Path: " << path.get_count() << " waypoints, length="
              << path.total_length << "m" << std::endl;

    // OccupancyGrid - 2D map
    OccupancyGrid grid;
    grid.init(100, 100, 0.05f, Pose2D(0.0, 0.0, 0.0));  // 5m x 5m map
    grid.set_occupancy(50, 50, 100);  // mark center as occupied
    std::cout << "OccupancyGrid: " << grid.width << "x" << grid.height
              << ", resolution=" << grid.resolution << "m" << std::endl;
}

void showcase_control_messages() {
    std::cout << "\n=== Control Messages ===" << std::endl;

    // MotorCommand - direct motor control
    MotorCommand motor = MotorCommand::velocity(1, 10.0);  // motor 1, 10 rad/s
    std::cout << "MotorCommand: motor_id=" << static_cast<int>(motor.motor_id)
              << ", target=" << motor.target << " rad/s" << std::endl;

    // DifferentialDriveCommand - two-wheeled robot
    DifferentialDriveCommand drive = DifferentialDriveCommand::from_twist(
        1.0, 0.5,     // linear, angular velocity
        0.3, 0.05     // wheel_base, wheel_radius
    );
    std::cout << "DifferentialDrive: left=" << drive.left_velocity
              << ", right=" << drive.right_velocity << " rad/s" << std::endl;

    // PidConfig - PID controller gains
    PidConfig pid = PidConfig::pd(2.0, 0.5);  // kp=2.0, kd=0.5
    std::cout << "PidConfig: kp=" << pid.kp << ", ki=" << pid.ki
              << ", kd=" << pid.kd << std::endl;

    // JointCommand - multi-joint control
    JointCommand joints;
    joints.add_position("shoulder", 1.57);
    joints.add_position("elbow", 0.78);
    joints.add_velocity("wrist", 0.5);
    std::cout << "JointCommand: " << static_cast<int>(joints.joint_count)
              << " joints" << std::endl;
}

void showcase_diagnostics_messages() {
    std::cout << "\n=== Diagnostics Messages ===" << std::endl;

    // Heartbeat - node alive signal
    Heartbeat hb = Heartbeat::create("robot_node", 42);
    hb.update(123.45);  // 123.45 seconds uptime
    std::cout << "Heartbeat: " << hb.node_name << ", uptime="
              << hb.uptime << "s, seq=" << hb.sequence << std::endl;

    // Status - system status
    Status status = Status::warn(100, "Low battery warning");
    status.set_component("power_monitor");
    std::cout << "Status: level=" << static_cast<int>(status.level)
              << ", code=" << status.code << ", msg=" << status.message << std::endl;

    // EmergencyStop - safety signal
    EmergencyStop estop = EmergencyStop::engage("Obstacle detected");
    estop.set_source("safety_scanner");
    std::cout << "EmergencyStop: engaged=" << estop.engaged
              << ", reason=" << estop.reason << std::endl;

    // ResourceUsage - system monitoring
    ResourceUsage resources;
    resources.cpu_percent = 45.2f;
    resources.memory_percent = 62.8f;
    resources.temperature = 55.3f;
    std::cout << "ResourceUsage: CPU=" << resources.cpu_percent
              << "%, Memory=" << resources.memory_percent
              << "%, Temp=" << resources.temperature << "°C" << std::endl;
}

int main() {
    std::cout << "==================================================" << std::endl;
    std::cout << "  HORUS C++ Message Library Showcase" << std::endl;
    std::cout << "  40+ Message Types for Robotics Applications" << std::endl;
    std::cout << "==================================================" << std::endl;

    showcase_geometry_messages();
    showcase_sensor_messages();
    showcase_vision_messages();
    showcase_perception_messages();
    showcase_navigation_messages();
    showcase_control_messages();
    showcase_diagnostics_messages();

    std::cout << "\n==================================================" << std::endl;
    std::cout << "  All message types working correctly!" << std::endl;
    std::cout << "==================================================" << std::endl;

    return 0;
}
