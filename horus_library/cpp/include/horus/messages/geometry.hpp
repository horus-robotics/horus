// HORUS Message Library - Geometry Types
// Binary-compatible with Rust definitions in horus_library/messages/geometry.rs
//
// IMPORTANT: These structures MUST maintain binary layout compatibility with Rust!
// - Use double (f64) not float (f32)
// - Use uint64_t for timestamps
// - Match field order exactly
// - Use standard layout (no virtual functions, no inheritance)

#ifndef HORUS_MESSAGES_GEOMETRY_HPP
#define HORUS_MESSAGES_GEOMETRY_HPP

#include <cstdint>
#include <cmath>
#include <chrono>

namespace horus {
namespace messages {

// ============================================================================
// Vector3 - 3D vector representation
// ============================================================================
struct Vector3 {
    double x;
    double y;
    double z;

    Vector3() : x(0.0), y(0.0), z(0.0) {}
    Vector3(double x_, double y_, double z_) : x(x_), y(y_), z(z_) {}

    static Vector3 zero() { return Vector3(0.0, 0.0, 0.0); }

    double magnitude() const {
        return std::sqrt(x * x + y * y + z * z);
    }

    void normalize() {
        double mag = magnitude();
        if (mag > 0.0) {
            x /= mag;
            y /= mag;
            z /= mag;
        }
    }

    double dot(const Vector3& other) const {
        return x * other.x + y * other.y + z * other.z;
    }

    Vector3 cross(const Vector3& other) const {
        return Vector3(
            y * other.z - z * other.y,
            z * other.x - x * other.z,
            x * other.y - y * other.x
        );
    }
};

// ============================================================================
// Point3 - 3D point representation
// ============================================================================
struct Point3 {
    double x;
    double y;
    double z;

    Point3() : x(0.0), y(0.0), z(0.0) {}
    Point3(double x_, double y_, double z_) : x(x_), y(y_), z(z_) {}

    static Point3 origin() { return Point3(0.0, 0.0, 0.0); }

    double distance_to(const Point3& other) const {
        double dx = x - other.x;
        double dy = y - other.y;
        double dz = z - other.z;
        return std::sqrt(dx * dx + dy * dy + dz * dz);
    }
};

// ============================================================================
// Quaternion - 3D rotation representation
// ============================================================================
struct Quaternion {
    double x;
    double y;
    double z;
    double w;

    Quaternion() : x(0.0), y(0.0), z(0.0), w(1.0) {}
    Quaternion(double x_, double y_, double z_, double w_)
        : x(x_), y(y_), z(z_), w(w_) {}

    static Quaternion identity() { return Quaternion(0.0, 0.0, 0.0, 1.0); }

    static Quaternion from_euler(double roll, double pitch, double yaw) {
        double cr = std::cos(roll / 2.0);
        double sr = std::sin(roll / 2.0);
        double cp = std::cos(pitch / 2.0);
        double sp = std::sin(pitch / 2.0);
        double cy = std::cos(yaw / 2.0);
        double sy = std::sin(yaw / 2.0);

        return Quaternion(
            sr * cp * cy - cr * sp * sy,
            cr * sp * cy + sr * cp * sy,
            cr * cp * sy - sr * sp * cy,
            cr * cp * cy + sr * sp * sy
        );
    }

    void normalize() {
        double norm = std::sqrt(x * x + y * y + z * z + w * w);
        if (norm > 0.0) {
            x /= norm;
            y /= norm;
            z /= norm;
            w /= norm;
        }
    }

    bool is_valid() const {
        return std::isfinite(x) && std::isfinite(y) &&
               std::isfinite(z) && std::isfinite(w);
    }
};

// ============================================================================
// Twist - 3D velocity command (linear + angular)
// Rust equivalent: pub struct Twist { linear: [f64; 3], angular: [f64; 3], timestamp: u64 }
// ============================================================================
struct Twist {
    double linear[3];   // [x, y, z] in m/s
    double angular[3];  // [roll, pitch, yaw] in rad/s
    uint64_t timestamp; // nanoseconds since epoch

    Twist() : linear{0.0, 0.0, 0.0}, angular{0.0, 0.0, 0.0}, timestamp(0) {
        update_timestamp();
    }

    Twist(double linear_x, double linear_y, double linear_z,
          double angular_x, double angular_y, double angular_z)
        : linear{linear_x, linear_y, linear_z},
          angular{angular_x, angular_y, angular_z},
          timestamp(0) {
        update_timestamp();
    }

    // 2D convenience constructor (common for mobile robots)
    static Twist new_2d(double linear_x, double angular_z) {
        return Twist(linear_x, 0.0, 0.0, 0.0, 0.0, angular_z);
    }

    static Twist stop() {
        return Twist(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    bool is_valid() const {
        for (int i = 0; i < 3; i++) {
            if (!std::isfinite(linear[i]) || !std::isfinite(angular[i]))
                return false;
        }
        return true;
    }
};

// ============================================================================
// Pose2D - 2D pose (x, y, theta)
// Rust equivalent: pub struct Pose2D { x: f64, y: f64, theta: f64, timestamp: u64 }
// ============================================================================
struct Pose2D {
    double x;           // meters
    double y;           // meters
    double theta;       // radians
    uint64_t timestamp; // nanoseconds since epoch

    Pose2D() : x(0.0), y(0.0), theta(0.0), timestamp(0) {
        update_timestamp();
    }

    Pose2D(double x_, double y_, double theta_)
        : x(x_), y(y_), theta(theta_), timestamp(0) {
        update_timestamp();
    }

    static Pose2D origin() {
        return Pose2D(0.0, 0.0, 0.0);
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    double distance_to(const Pose2D& other) const {
        double dx = x - other.x;
        double dy = y - other.y;
        return std::sqrt(dx * dx + dy * dy);
    }

    void normalize_angle() {
        const double PI = 3.14159265358979323846;
        while (theta > PI) theta -= 2.0 * PI;
        while (theta < -PI) theta += 2.0 * PI;
    }

    bool is_valid() const {
        return std::isfinite(x) && std::isfinite(y) && std::isfinite(theta);
    }
};

// ============================================================================
// Transform - 3D transformation (translation + rotation)
// Rust equivalent: pub struct Transform { translation: [f64; 3], rotation: [f64; 4], timestamp: u64 }
// ============================================================================
struct Transform {
    double translation[3]; // [x, y, z] in meters
    double rotation[4];    // quaternion [x, y, z, w]
    uint64_t timestamp;    // nanoseconds since epoch

    Transform()
        : translation{0.0, 0.0, 0.0},
          rotation{0.0, 0.0, 0.0, 1.0},
          timestamp(0) {
        update_timestamp();
    }

    Transform(double tx, double ty, double tz,
              double qx, double qy, double qz, double qw)
        : translation{tx, ty, tz},
          rotation{qx, qy, qz, qw},
          timestamp(0) {
        update_timestamp();
    }

    static Transform identity() {
        return Transform(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    }

    static Transform from_pose_2d(const Pose2D& pose) {
        double half_theta = pose.theta / 2.0;
        return Transform(
            pose.x, pose.y, 0.0,
            0.0, 0.0, std::sin(half_theta), std::cos(half_theta)
        );
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    bool is_valid() const {
        // Check finite values
        for (int i = 0; i < 3; i++) {
            if (!std::isfinite(translation[i])) return false;
        }
        for (int i = 0; i < 4; i++) {
            if (!std::isfinite(rotation[i])) return false;
        }

        // Check quaternion normalization
        double norm = 0.0;
        for (int i = 0; i < 4; i++) {
            norm += rotation[i] * rotation[i];
        }
        norm = std::sqrt(norm);
        return std::abs(norm - 1.0) < 0.01;
    }

    void normalize_rotation() {
        double norm = 0.0;
        for (int i = 0; i < 4; i++) {
            norm += rotation[i] * rotation[i];
        }
        norm = std::sqrt(norm);
        if (norm > 0.0) {
            for (int i = 0; i < 4; i++) {
                rotation[i] /= norm;
            }
        }
    }
};

} // namespace messages
} // namespace horus

#endif // HORUS_MESSAGES_GEOMETRY_HPP
