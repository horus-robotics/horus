// HORUS C++ Example: LiDAR Driver (New API)
// Demonstrates hardware integration with clean, unified API

#include "../include/horus_new.hpp"
#include <iostream>
#include <cmath>
#include <random>

// Simulate hardware LiDAR device (replace with real driver like rplidar)
class LidarDevice {
private:
    std::string port_;
    int fd_;
    bool connected_;

public:
    explicit LidarDevice(const std::string& port = "/dev/ttyUSB0")
        : port_(port), fd_(-1), connected_(false) {}

    bool open() {
        std::cout << "[Hardware] Opening LiDAR on " << port_ << std::endl;
        // In real driver: fd_ = ::open(port_.c_str(), O_RDWR);
        fd_ = 1;  // Simulated
        connected_ = (fd_ >= 0);
        return connected_;
    }

    void close() {
        if (connected_) {
            std::cout << "[Hardware] Closing LiDAR" << std::endl;
            // In real driver: ::close(fd_);
            connected_ = false;
        }
    }

    LaserScan read() {
        // Simulate reading 360-degree scan
        static std::random_device rd;
        static std::mt19937 gen(rd());
        static std::uniform_real_distribution<> noise(0.0, 0.1);

        LaserScan scan{};
        std::vector<float> ranges(360);

        for (size_t i = 0; i < 360; i++) {
            float angle = (i * M_PI * 2.0f) / 360.0f;
            ranges[i] = 2.0f + std::sin(angle) * 0.5f + noise(gen);
        }

        scan.ranges = ranges.data();
        scan.count = 360;
        scan.angle_min = 0.0f;
        scan.angle_max = 2.0f * M_PI;
        scan.angle_increment = (2.0f * M_PI) / 360.0f;
        scan.range_min = 0.1f;
        scan.range_max = 10.0f;
        scan.scan_time = 0.1f;  // 10 Hz

        return scan;
    }

    bool is_connected() const { return connected_; }
};

// ============================================================================
// LiDAR Driver Node - Clean, matches Rust/Python pattern
// ============================================================================

struct LidarDriver : horus::Node {
    horus::Publisher<LaserScan> scan_pub;
    LidarDevice device;
    uint32_t scan_count;

    LidarDriver()
        : horus::Node("lidar_driver"),
          scan_pub("scan"),
          scan_count(0) {}

    bool init(horus::NodeContext& ctx) override {
        ctx.log_info("Initializing LiDAR driver...");

        if (!device.open()) {
            ctx.log_error("Failed to open LiDAR device");
            return false;
        }

        ctx.log_info("LiDAR ready @ 10Hz");
        return true;
    }

    void tick(horus::NodeContext& ctx) override {
        if (!device.is_connected()) return;

        // Read from hardware
        LaserScan scan = device.read();

        // Publish to HORUS (logged to dashboard with IPC timing!)
        scan_pub.send(scan);
        scan_count++;

        // Log every 60 ticks (1 second at 60 FPS)
        if (scan_count % 60 == 0) {
            ctx.log_info("Published " + std::to_string(scan_count) + " scans");
        }

        // Safety check
        if (scan.min_range() < 0.5f) {
            ctx.log_warn("Obstacle detected: " + std::to_string(scan.min_range()) + "m");
        }
    }

    bool shutdown(horus::NodeContext& ctx) override {
        ctx.log_info("Shutting down LiDAR driver");
        ctx.log_info("Total scans published: " + std::to_string(scan_count));
        device.close();
        return true;
    }
};

// ============================================================================
// Main
// ============================================================================

int main() {
    using namespace horus;

    std::cout << "=== HORUS LiDAR Driver (C++) ===" << std::endl;
    std::cout << "Publishing to topic: scan\n" << std::endl;

    try {
        Scheduler scheduler;

        // Add node with Normal priority, logging enabled
        scheduler.add(LidarDriver(), 2, true);

        std::cout << "LiDAR driver starting at 60 FPS..." << std::endl;
        std::cout << "Press Ctrl+C to stop\n" << std::endl;

        // Run at 60 FPS (blocks until Ctrl+C)
        scheduler.run();

        std::cout << "\nLiDAR driver stopped" << std::endl;

    } catch (const HorusException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
