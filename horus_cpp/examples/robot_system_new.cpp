// HORUS C++ Example: Complete Robot System (New API)
// Demonstrates multi-node application with sensors, control, and safety

#include "../include/horus_new.hpp"
#include <iostream>
#include <cmath>
#include <random>

using namespace horus;

// ============================================================================
// Sensor Node - Publishes IMU data
// ============================================================================

struct ImuDriver : Node {
    Publisher<Imu> imu_pub;
    uint32_t reading_count;

    ImuDriver()
        : Node("imu_driver"),
          imu_pub("imu"),
          reading_count(0) {}

    bool init(NodeContext& ctx) override {
        ctx.log_info("Initializing IMU driver...");
        // In real driver: open I2C device, calibrate
        ctx.log_info("IMU ready @ 60Hz");
        return true;
    }

    void tick(NodeContext& ctx) override {
        // Simulate IMU reading
        Imu data{};
        data.accel_x = std::sin(reading_count * 0.01) * 9.81;
        data.accel_y = 0.0;
        data.accel_z = 9.81;
        data.gyro_x = 0.0;
        data.gyro_y = 0.0;
        data.gyro_z = std::cos(reading_count * 0.01) * 0.5;

        imu_pub.send(data);
        reading_count++;
    }

    bool shutdown(NodeContext& ctx) override {
        ctx.log_info("IMU driver shutdown");
        ctx.log_info("Total readings: " + std::to_string(reading_count));
        return true;
    }
};

// ============================================================================
// Sensor Node - Publishes LiDAR data
// ============================================================================

struct LidarDriver : Node {
    Publisher<LaserScan> scan_pub;
    uint32_t scan_count;

    LidarDriver()
        : Node("lidar_driver"),
          scan_pub("scan"),
          scan_count(0) {}

    bool init(NodeContext& ctx) override {
        ctx.log_info("Initializing LiDAR...");
        ctx.log_info("LiDAR ready @ 10Hz");
        return true;
    }

    void tick(NodeContext& ctx) override {
        // Only publish at 10 Hz (every 6 ticks at 60 FPS)
        if (scan_count % 6 != 0) {
            scan_count++;
            return;
        }

        // Simulate LiDAR scan
        static std::vector<float> ranges(360);
        static std::random_device rd;
        static std::mt19937 gen(rd());
        static std::uniform_real_distribution<> noise(0.0, 0.1);

        for (size_t i = 0; i < 360; i++) {
            float angle = (i * M_PI * 2.0f) / 360.0f;
            ranges[i] = 2.0f + std::sin(angle) * 0.5f + noise(gen);
        }

        LaserScan scan{};
        scan.ranges = ranges.data();
        scan.count = 360;
        scan.angle_min = 0.0f;
        scan.angle_max = 2.0f * M_PI;
        scan.angle_increment = (2.0f * M_PI) / 360.0f;
        scan.range_min = 0.1f;
        scan.range_max = 10.0f;

        scan_pub.send(scan);
        scan_count++;
    }

    bool shutdown(NodeContext& ctx) override {
        ctx.log_info("LiDAR shutdown");
        return true;
    }
};

// ============================================================================
// Control Node - Consumes sensor data, publishes commands
// ============================================================================

struct Controller : Node {
    Subscriber<Imu> imu_sub;
    Subscriber<LaserScan> scan_sub;
    Publisher<Twist> cmd_pub;
    uint32_t commands_sent;

    Controller()
        : Node("controller"),
          imu_sub("imu"),
          scan_sub("scan"),
          cmd_pub("cmd_vel"),
          commands_sent(0) {}

    bool init(NodeContext& ctx) override {
        ctx.log_info("Initializing controller...");
        ctx.log_info("Controller ready");
        return true;
    }

    void tick(NodeContext& ctx) override {
        Imu imu;
        LaserScan scan;

        // Non-blocking receive
        bool has_imu = imu_sub.recv(imu);
        bool has_scan = scan_sub.recv(scan);

        if (has_imu && has_scan) {
            // Simple control logic
            float min_distance = scan.min_range();
            float angular_velocity = imu.gyro_z;

            Twist cmd = Twist::new_2d(
                (min_distance > 1.0f) ? 1.0 : 0.0,  // Stop if obstacle close
                angular_velocity * 0.5               // Smooth turning
            );

            cmd_pub.send(cmd);
            commands_sent++;

            if (commands_sent % 60 == 0) {
                ctx.log_debug("Sent " + std::to_string(commands_sent) + " commands");
            }
        }
    }

    bool shutdown(NodeContext& ctx) override {
        ctx.log_info("Controller shutdown");
        ctx.log_info("Total commands sent: " + std::to_string(commands_sent));

        // Send stop command
        cmd_pub.send(Twist::stop());
        return true;
    }
};

// ============================================================================
// Safety Node - Monitors commands and triggers emergency stop
// ============================================================================

struct SafetyMonitor : Node {
    Subscriber<LaserScan> scan_sub;
    Subscriber<Twist> cmd_sub;
    Publisher<EmergencyStop> estop_pub;
    float danger_distance;
    uint32_t violations;

    SafetyMonitor()
        : Node("safety_monitor"),
          scan_sub("scan"),
          cmd_sub("cmd_vel"),
          estop_pub("estop"),
          danger_distance(0.3f),
          violations(0) {}

    bool init(NodeContext& ctx) override {
        ctx.log_info("Safety monitor starting (CRITICAL priority)");
        ctx.log_info("Danger zone: " + std::to_string(danger_distance) + "m");
        return true;
    }

    void tick(NodeContext& ctx) override {
        // Check 1: Obstacle distance
        LaserScan scan;
        if (scan_sub.recv(scan)) {
            float min_dist = scan.min_range();
            if (min_dist < danger_distance) {
                violations++;
                ctx.log_warn("OBSTACLE TOO CLOSE: " + std::to_string(min_dist) + "m");
                estop_pub.send(EmergencyStop::engage("Obstacle detected"));
            }
        }

        // Check 2: Velocity limits
        Twist cmd;
        if (cmd_sub.recv(cmd)) {
            float linear = std::abs(cmd.linear[0]);
            float angular = std::abs(cmd.angular[2]);

            if (linear > 2.0 || angular > 1.0) {
                violations++;
                ctx.log_warn("VELOCITY LIMIT EXCEEDED");
                estop_pub.send(EmergencyStop::engage("Speed violation"));
            }
        }
    }

    bool shutdown(NodeContext& ctx) override {
        ctx.log_info("Safety monitor shutdown");
        ctx.log_info("Total violations: " + std::to_string(violations));

        // Final safety command
        estop_pub.send(EmergencyStop::engage("System shutdown"));
        return true;
    }
};

// ============================================================================
// Main
// ============================================================================

int main() {
    std::cout << "=====================================" << std::endl;
    std::cout << "   HORUS Robot Control System (C++)" << std::endl;
    std::cout << "=====================================" << std::endl;
    std::cout << "\nSystem topology:" << std::endl;
    std::cout << "  IMU Driver     -> [imu]" << std::endl;
    std::cout << "  LiDAR Driver   -> [scan]" << std::endl;
    std::cout << "  Controller     -> [cmd_vel] (subscribes: imu, scan)" << std::endl;
    std::cout << "  Safety Monitor -> [estop] (subscribes: scan, cmd_vel)" << std::endl;
    std::cout << "\nStarting 4 nodes at 60 FPS..." << std::endl;
    std::cout << "Press Ctrl+C to stop\n" << std::endl;

    try {
        Scheduler scheduler;

        // Add nodes with priorities (0=Critical first, 4=Background last)
        scheduler.add(SafetyMonitor(), 0, true);  // Critical - runs first
        scheduler.add(Controller(),    1, true);  // High priority
        scheduler.add(LidarDriver(),   2, true);  // Normal
        scheduler.add(ImuDriver(),     2, true);  // Normal

        // Run at 60 FPS (blocks until Ctrl+C)
        // Execution order: init() all -> tick() loop at 60 FPS -> shutdown() all
        scheduler.run();

        std::cout << "\n=====================================" << std::endl;
        std::cout << "   System stopped gracefully" << std::endl;
        std::cout << "=====================================" << std::endl;

    } catch (const HorusException& e) {
        std::cerr << "HORUS Error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
