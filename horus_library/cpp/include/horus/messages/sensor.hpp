// HORUS Message Library - Sensor Types
// Binary-compatible with Rust definitions in horus_library/messages/sensor.rs

#ifndef HORUS_MESSAGES_SENSOR_HPP
#define HORUS_MESSAGES_SENSOR_HPP

#include "geometry.hpp"
#include <cstdint>
#include <cmath>
#include <cstring>
#include <chrono>
#include <algorithm>

namespace horus {
namespace messages {

// ============================================================================
// LaserScan - 2D lidar sensor data
// Rust equivalent: pub struct LaserScan { ranges: [f32; 360], ... }
// ============================================================================
struct LaserScan {
    float ranges[360];      // Range measurements in meters (0 = invalid)
    float angle_min;        // Start angle in radians
    float angle_max;        // End angle in radians
    float range_min;        // Minimum valid range in meters
    float range_max;        // Maximum valid range in meters
    float angle_increment;  // Angular resolution in radians
    float time_increment;   // Time between measurements in seconds
    float scan_time;        // Time to complete full scan in seconds
    uint64_t timestamp;     // nanoseconds since epoch

    LaserScan() {
        std::memset(ranges, 0, sizeof(ranges));
        angle_min = -3.14159265f;  // -PI
        angle_max = 3.14159265f;   // PI
        range_min = 0.1f;
        range_max = 30.0f;
        angle_increment = 3.14159265f / 180.0f;  // 1 degree
        time_increment = 0.0f;
        scan_time = 0.1f;
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Get angle for a specific range index
    float angle_at(size_t index) const {
        if (index >= 360) return 0.0f;
        return angle_min + (static_cast<float>(index) * angle_increment);
    }

    // Check if a range reading is valid
    bool is_range_valid(size_t index) const {
        if (index >= 360) return false;
        float range = ranges[index];
        return range >= range_min && range <= range_max && std::isfinite(range);
    }

    // Count valid range readings
    size_t valid_count() const {
        size_t count = 0;
        for (size_t i = 0; i < 360; i++) {
            if (is_range_valid(i)) count++;
        }
        return count;
    }

    // Get minimum valid range reading
    float min_range() const {
        float min_val = range_max + 1.0f;
        for (size_t i = 0; i < 360; i++) {
            if (is_range_valid(i) && ranges[i] < min_val) {
                min_val = ranges[i];
            }
        }
        return (min_val <= range_max) ? min_val : 0.0f;
    }
};

// ============================================================================
// IMU - Inertial Measurement Unit sensor data
// Rust equivalent: pub struct Imu { orientation: [f64; 4], ... }
// ============================================================================
struct Imu {
    double orientation[4];                      // Quaternion [x, y, z, w]
    double orientation_covariance[9];           // Row-major, -1 = no data
    double angular_velocity[3];                 // [x, y, z] in rad/s
    double angular_velocity_covariance[9];      // Row-major
    double linear_acceleration[3];              // [x, y, z] in m/s²
    double linear_acceleration_covariance[9];   // Row-major
    uint64_t timestamp;                         // nanoseconds since epoch

    Imu() {
        // Identity quaternion
        orientation[0] = 0.0;
        orientation[1] = 0.0;
        orientation[2] = 0.0;
        orientation[3] = 1.0;

        // No orientation data
        for (int i = 0; i < 9; i++) {
            orientation_covariance[i] = -1.0;
            angular_velocity_covariance[i] = 0.0;
            linear_acceleration_covariance[i] = 0.0;
        }

        // Zero velocity and acceleration
        for (int i = 0; i < 3; i++) {
            angular_velocity[i] = 0.0;
            linear_acceleration[i] = 0.0;
        }

        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Set orientation from Euler angles
    void set_orientation_from_euler(double roll, double pitch, double yaw) {
        Quaternion q = Quaternion::from_euler(roll, pitch, yaw);
        orientation[0] = q.x;
        orientation[1] = q.y;
        orientation[2] = q.z;
        orientation[3] = q.w;
    }

    // Check if orientation data is available
    bool has_orientation() const {
        return orientation_covariance[0] >= 0.0;
    }

    // Check if all values are finite
    bool is_valid() const {
        for (int i = 0; i < 4; i++) {
            if (!std::isfinite(orientation[i])) return false;
        }
        for (int i = 0; i < 3; i++) {
            if (!std::isfinite(angular_velocity[i])) return false;
            if (!std::isfinite(linear_acceleration[i])) return false;
        }
        return true;
    }

    // Get angular velocity as Vector3
    Vector3 angular_velocity_vec() const {
        return Vector3(angular_velocity[0], angular_velocity[1], angular_velocity[2]);
    }

    // Get linear acceleration as Vector3
    Vector3 linear_acceleration_vec() const {
        return Vector3(linear_acceleration[0], linear_acceleration[1], linear_acceleration[2]);
    }
};

// ============================================================================
// Odometry - Combined pose and velocity estimate
// Rust equivalent: pub struct Odometry { pose: Pose2D, twist: Twist, ... }
// ============================================================================
struct Odometry {
    Pose2D pose;                // Current pose estimate
    Twist twist;                // Current velocity estimate
    double pose_covariance[36]; // 6x6 row-major
    double twist_covariance[36];// 6x6 row-major
    char frame_id[32];          // e.g., "odom", "map"
    char child_frame_id[32];    // e.g., "base_link"
    uint64_t timestamp;         // nanoseconds since epoch

    Odometry() : pose(), twist() {
        for (int i = 0; i < 36; i++) {
            pose_covariance[i] = 0.0;
            twist_covariance[i] = 0.0;
        }
        std::memset(frame_id, 0, sizeof(frame_id));
        std::memset(child_frame_id, 0, sizeof(child_frame_id));
        std::strncpy(frame_id, "odom", sizeof(frame_id) - 1);
        std::strncpy(child_frame_id, "base_link", sizeof(child_frame_id) - 1);
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    void set_frame_id(const char* frame) {
        std::memset(frame_id, 0, sizeof(frame_id));
        std::strncpy(frame_id, frame, sizeof(frame_id) - 1);
    }

    void set_child_frame_id(const char* frame) {
        std::memset(child_frame_id, 0, sizeof(child_frame_id));
        std::strncpy(child_frame_id, frame, sizeof(child_frame_id) - 1);
    }
};

// ============================================================================
// Range - Single distance sensor reading
// ============================================================================
struct Range {
    float range;          // Distance in meters
    float min_range;      // Minimum valid range
    float max_range;      // Maximum valid range
    float field_of_view;  // Angular field of view in radians
    uint64_t timestamp;   // nanoseconds since epoch

    Range()
        : range(0.0f), min_range(0.01f), max_range(10.0f),
          field_of_view(0.1f), timestamp(0) {
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    bool is_valid() const {
        return range >= min_range && range <= max_range && std::isfinite(range);
    }
};

// ============================================================================
// BatteryState - Battery monitoring data
// ============================================================================
struct BatteryState {
    float voltage;              // Volts
    float current;              // Amps (negative = charging)
    float charge;               // Amp-hours
    float capacity;             // Total capacity in Amp-hours
    float percentage;           // State of charge (0.0 - 1.0)
    float temperature;          // Celsius
    uint8_t power_supply_status; // 0=unknown, 1=charging, 2=discharging, 3=not_charging, 4=full
    uint8_t power_supply_health; // 0=unknown, 1=good, 2=overheat, 3=dead, 4=overvoltage, 5=unspec_failure, 6=cold, 7=watchdog_timer_expire, 8=safety_timer_expire
    uint64_t timestamp;         // nanoseconds since epoch

    BatteryState()
        : voltage(0.0f), current(0.0f), charge(0.0f), capacity(0.0f),
          percentage(0.0f), temperature(0.0f),
          power_supply_status(0), power_supply_health(0), timestamp(0) {
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    bool is_valid() const {
        return std::isfinite(voltage) && std::isfinite(current) &&
               std::isfinite(charge) && std::isfinite(percentage);
    }

    bool is_charging() const {
        return power_supply_status == 1;
    }

    bool is_healthy() const {
        return power_supply_health == 1;
    }
};

} // namespace messages
} // namespace horus

#endif // HORUS_MESSAGES_SENSOR_HPP
