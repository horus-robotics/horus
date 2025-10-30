// Example: LiDAR hardware driver integration with HORUS C++ API
#include "../include/horus.hpp"
#include <iostream>
#include <vector>
#include <memory>
#include <cmath>
#include <random>
#include <string>

// Simulate a hardware LiDAR driver using modern C++ (replace with real driver like rplidar)
class LidarDevice {
public:
    explicit LidarDevice(const std::string& port, uint32_t points = 360)
        : port_(port), points_per_scan_(points), range_buffer_(points) {
        std::cout << "[Driver] Opening LiDAR on " << port << std::endl;
        // In real driver: open serial port, initialize device
        fd_ = 1; // Simulated file descriptor
    }

    ~LidarDevice() {
        std::cout << "[Driver] Closing LiDAR device" << std::endl;
        // In real driver: close device, cleanup resources
    }

    // Non-copyable
    LidarDevice(const LidarDevice&) = delete;
    LidarDevice& operator=(const LidarDevice&) = delete;

    // Moveable
    LidarDevice(LidarDevice&&) = default;
    LidarDevice& operator=(LidarDevice&&) = default;

    // Simulate hardware read (replace with real driver call)
    bool get_scan(std::vector<float>& ranges) {
        // In real driver: ioctl(fd_, LIDAR_GET_SCAN, ranges.data());

        // Simulate scanning - generate fake data
        ranges.resize(points_per_scan_);

        static std::random_device rd;
        static std::mt19937 gen(rd());
        static std::uniform_real_distribution<> noise(0.0, 0.1);

        for (uint32_t i = 0; i < points_per_scan_; i++) {
            // Simulate walls at different distances with noise
            float angle = (i * M_PI * 2.0f) / points_per_scan_;
            ranges[i] = 2.0f + std::sin(angle) * 0.5f + noise(gen);
        }

        return true; // Success
    }

    uint32_t points_per_scan() const { return points_per_scan_; }

private:
    std::string port_;
    int fd_;
    uint32_t points_per_scan_;
    std::vector<float> range_buffer_;
};

int main(int argc, char** argv) {
    std::cout << "=== LiDAR Driver Bridge for HORUS (C++) ===" << std::endl;

    try {
        // Initialize HORUS system with RAII
        horus::System system("lidar_driver");

        // Create publisher for laser scans
        horus::Publisher<LaserScan> scan_pub("laser_scan");

        // Open hardware device
        const std::string port = (argc > 1) ? argv[1] : "/dev/ttyUSB0";
        LidarDevice lidar(port);

        std::cout << "LiDAR driver running at 10Hz..." << std::endl;
        std::cout << "Publishing to topic: laser_scan\n" << std::endl;

        // Allocate scan message
        std::vector<float> ranges(360);
        LaserScan scan{};
        scan.ranges = ranges.data();
        scan.intensities = nullptr;  // No intensity data
        scan.count = 0;
        scan.angle_min = 0.0f;
        scan.angle_max = 2.0f * M_PI;
        scan.angle_increment = (2.0f * M_PI) / 360.0f;
        scan.range_min = 0.1f;
        scan.range_max = 10.0f;
        scan.scan_time = 0.1f;  // 100ms per scan

        uint32_t scan_count = 0;

        // Main loop - read from hardware and publish
        while (system.ok()) {
            // Read from hardware
            if (lidar.get_scan(ranges)) {
                scan.count = ranges.size();

                // Publish to HORUS using modern C++ API
                scan_pub << scan;
                scan_count++;

                // Log every 10th scan
                if (scan_count % 10 == 0) {
                    horus::Log::debug(
                        "Published scan #" + std::to_string(scan_count) +
                        " (" + std::to_string(scan.count) + " points, " +
                        "min: " + std::to_string(ranges[0]) + "m)"
                    );
                }
            } else {
                horus::Log::error("Failed to read from LiDAR");
            }

            // Run at 10Hz
            horus::sleep_ms(100);
        }

        // Cleanup is automatic with RAII
        std::cout << "\nShutting down LiDAR driver" << std::endl;

    } catch (const horus::HorusException& e) {
        std::cerr << "HORUS Error: " << e.what() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
