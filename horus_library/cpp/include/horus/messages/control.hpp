// HORUS Message Library - Control Types
// Binary-compatible with Rust definitions in horus_library/messages/control.rs

#ifndef HORUS_MESSAGES_CONTROL_HPP
#define HORUS_MESSAGES_CONTROL_HPP

#include <cstdint>
#include <cstring>
#include <chrono>
#include <algorithm>
#include <cmath>

namespace horus {
namespace messages {

// ============================================================================
// MotorCommand - Direct motor control command
// ============================================================================
struct MotorCommand {
    // Control mode constants
    static constexpr uint8_t MODE_VELOCITY = 0;
    static constexpr uint8_t MODE_POSITION = 1;
    static constexpr uint8_t MODE_TORQUE = 2;
    static constexpr uint8_t MODE_VOLTAGE = 3;

    uint8_t motor_id;           // Motor ID (for multi-motor systems)
    uint8_t mode;               // Control mode
    uint8_t _padding[6];        // Padding
    double target;              // Target value (units depend on mode)
    double max_velocity;        // Maximum velocity (for position mode)
    double max_acceleration;    // Maximum acceleration
    double feed_forward;        // Feed-forward term
    bool enable;                // Enable motor
    uint8_t _padding2[7];       // Padding
    uint64_t timestamp;         // nanoseconds since epoch

    MotorCommand() : motor_id(0), mode(MODE_VELOCITY), target(0.0),
                     max_velocity(INFINITY), max_acceleration(INFINITY),
                     feed_forward(0.0), enable(true) {
        std::memset(_padding, 0, sizeof(_padding));
        std::memset(_padding2, 0, sizeof(_padding2));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Create a velocity command
    static MotorCommand velocity(uint8_t id, double vel) {
        MotorCommand cmd;
        cmd.motor_id = id;
        cmd.mode = MODE_VELOCITY;
        cmd.target = vel;
        cmd.enable = true;
        cmd.update_timestamp();
        return cmd;
    }

    // Create a position command
    static MotorCommand position(uint8_t id, double pos, double max_vel) {
        MotorCommand cmd;
        cmd.motor_id = id;
        cmd.mode = MODE_POSITION;
        cmd.target = pos;
        cmd.max_velocity = max_vel;
        cmd.enable = true;
        cmd.update_timestamp();
        return cmd;
    }

    // Create a stop command
    static MotorCommand stop(uint8_t id) {
        MotorCommand cmd;
        cmd.motor_id = id;
        cmd.mode = MODE_VELOCITY;
        cmd.target = 0.0;
        cmd.enable = false;
        cmd.update_timestamp();
        return cmd;
    }

    // Check if values are valid
    bool is_valid() const {
        return std::isfinite(target) && std::isfinite(max_velocity) &&
               std::isfinite(max_acceleration) && std::isfinite(feed_forward);
    }
};

// ============================================================================
// DifferentialDriveCommand - Two-wheeled differential drive control
// ============================================================================
struct DifferentialDriveCommand {
    double left_velocity;       // Left wheel velocity in rad/s
    double right_velocity;      // Right wheel velocity in rad/s
    double max_acceleration;    // Maximum acceleration in rad/s²
    bool enable;                // Enable motors
    uint8_t _padding[7];        // Padding
    uint64_t timestamp;         // nanoseconds since epoch

    DifferentialDriveCommand() : left_velocity(0.0), right_velocity(0.0),
                                 max_acceleration(INFINITY), enable(true) {
        std::memset(_padding, 0, sizeof(_padding));
        update_timestamp();
    }

    DifferentialDriveCommand(double left, double right)
        : left_velocity(left), right_velocity(right),
          max_acceleration(INFINITY), enable(true) {
        std::memset(_padding, 0, sizeof(_padding));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Create a stop command
    static DifferentialDriveCommand stop() {
        DifferentialDriveCommand cmd;
        cmd.left_velocity = 0.0;
        cmd.right_velocity = 0.0;
        cmd.enable = false;
        cmd.update_timestamp();
        return cmd;
    }

    // Create from linear and angular velocities
    static DifferentialDriveCommand from_twist(double linear, double angular,
                                               double wheel_base, double wheel_radius) {
        double left = (linear - angular * wheel_base / 2.0) / wheel_radius;
        double right = (linear + angular * wheel_base / 2.0) / wheel_radius;
        return DifferentialDriveCommand(left, right);
    }

    // Check if values are valid
    bool is_valid() const {
        return std::isfinite(left_velocity) && std::isfinite(right_velocity) &&
               (std::isfinite(max_acceleration) || std::isinf(max_acceleration));
    }
};

// ============================================================================
// ServoCommand - Position-controlled servo command
// ============================================================================
struct ServoCommand {
    uint8_t servo_id;           // Servo ID
    uint8_t _padding[3];        // Padding
    float position;             // Target position in radians
    float speed;                // Movement speed (0-1, 0=max speed)
    bool enable;                // Torque enable
    uint8_t _padding2[3];       // Padding
    uint64_t timestamp;         // nanoseconds since epoch

    ServoCommand() : servo_id(0), position(0.0f), speed(0.5f), enable(true) {
        std::memset(_padding, 0, sizeof(_padding));
        std::memset(_padding2, 0, sizeof(_padding2));
        update_timestamp();
    }

    ServoCommand(uint8_t id, float pos) : servo_id(id), position(pos),
                                           speed(0.5f), enable(true) {
        std::memset(_padding, 0, sizeof(_padding));
        std::memset(_padding2, 0, sizeof(_padding2));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Create with specific speed
    static ServoCommand with_speed(uint8_t id, float pos, float spd) {
        ServoCommand cmd(id, pos);
        cmd.speed = std::clamp(spd, 0.0f, 1.0f);
        return cmd;
    }

    // Disable servo (remove torque)
    static ServoCommand disable(uint8_t id) {
        ServoCommand cmd;
        cmd.servo_id = id;
        cmd.position = 0.0f;
        cmd.speed = 0.0f;
        cmd.enable = false;
        cmd.update_timestamp();
        return cmd;
    }

    // Convert position from degrees to radians
    static ServoCommand from_degrees(uint8_t id, float degrees) {
        return ServoCommand(id, degrees * M_PI / 180.0f);
    }
};

// ============================================================================
// PidConfig - PID controller gains configuration
// ============================================================================
struct PidConfig {
    uint8_t controller_id;      // Controller ID
    bool anti_windup;           // Enable anti-windup
    uint8_t _padding[6];        // Padding
    double kp;                  // Proportional gain
    double ki;                  // Integral gain
    double kd;                  // Derivative gain
    double integral_limit;      // Integral windup limit
    double output_limit;        // Output limit
    uint64_t timestamp;         // nanoseconds since epoch

    PidConfig() : controller_id(0), anti_windup(true), kp(0.0), ki(0.0), kd(0.0),
                  integral_limit(INFINITY), output_limit(INFINITY) {
        std::memset(_padding, 0, sizeof(_padding));
        update_timestamp();
    }

    PidConfig(double kp_val, double ki_val, double kd_val)
        : controller_id(0), anti_windup(true), kp(kp_val), ki(ki_val), kd(kd_val),
          integral_limit(INFINITY), output_limit(INFINITY) {
        std::memset(_padding, 0, sizeof(_padding));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Create a P-only controller
    static PidConfig proportional(double kp) {
        return PidConfig(kp, 0.0, 0.0);
    }

    // Create a PI controller
    static PidConfig pi(double kp, double ki) {
        return PidConfig(kp, ki, 0.0);
    }

    // Create a PD controller
    static PidConfig pd(double kp, double kd) {
        return PidConfig(kp, 0.0, kd);
    }

    // Set limits
    PidConfig& with_limits(double int_limit, double out_limit) {
        integral_limit = int_limit;
        output_limit = out_limit;
        return *this;
    }

    // Check if gains are valid
    bool is_valid() const {
        return std::isfinite(kp) && std::isfinite(ki) && std::isfinite(kd) &&
               std::isfinite(integral_limit) && std::isfinite(output_limit) &&
               kp >= 0.0 && ki >= 0.0 && kd >= 0.0;
    }
};

// ============================================================================
// TrajectoryPoint - Single point in a trajectory
// ============================================================================
struct TrajectoryPoint {
    double position[3];         // Position [x, y, z]
    double velocity[3];         // Velocity [vx, vy, vz]
    double acceleration[3];     // Acceleration [ax, ay, az]
    double orientation[4];      // Orientation as quaternion [x, y, z, w]
    double angular_velocity[3]; // Angular velocity [wx, wy, wz]
    double time_from_start;     // Time from trajectory start in seconds

    TrajectoryPoint() : time_from_start(0.0) {
        std::memset(position, 0, sizeof(position));
        std::memset(velocity, 0, sizeof(velocity));
        std::memset(acceleration, 0, sizeof(acceleration));
        orientation[0] = orientation[1] = orientation[2] = 0.0;
        orientation[3] = 1.0; // Identity quaternion
        std::memset(angular_velocity, 0, sizeof(angular_velocity));
    }

    // Create a simple 2D trajectory point
    static TrajectoryPoint new_2d(double x, double y, double vx, double vy, double time) {
        TrajectoryPoint pt;
        pt.position[0] = x;
        pt.position[1] = y;
        pt.position[2] = 0.0;
        pt.velocity[0] = vx;
        pt.velocity[1] = vy;
        pt.velocity[2] = 0.0;
        pt.time_from_start = time;
        return pt;
    }

    // Create a stationary point
    static TrajectoryPoint stationary(double x, double y, double z) {
        TrajectoryPoint pt;
        pt.position[0] = x;
        pt.position[1] = y;
        pt.position[2] = z;
        return pt;
    }
};

// ============================================================================
// JointCommand - Multi-DOF joint command
// ============================================================================
struct JointCommand {
    // Control mode constants
    static constexpr uint8_t MODE_POSITION = 0;
    static constexpr uint8_t MODE_VELOCITY = 1;
    static constexpr uint8_t MODE_EFFORT = 2;

    char joint_names[16][32];   // Joint names (max 16 joints)
    uint8_t joint_count;        // Number of active joints
    uint8_t _padding[7];        // Padding
    double positions[16];       // Position commands in radians
    double velocities[16];      // Velocity commands in rad/s
    double efforts[16];         // Effort/torque commands in Nm
    uint8_t modes[16];          // Control mode per joint
    uint64_t timestamp;         // nanoseconds since epoch

    JointCommand() {
        std::memset(joint_names, 0, sizeof(joint_names));
        joint_count = 0;
        std::memset(_padding, 0, sizeof(_padding));
        std::memset(positions, 0, sizeof(positions));
        std::memset(velocities, 0, sizeof(velocities));
        std::memset(efforts, 0, sizeof(efforts));
        std::memset(modes, 0, sizeof(modes));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Add a joint position command
    bool add_position(const char* name, double position) {
        if (joint_count >= 16) return false;

        size_t idx = joint_count;
        std::strncpy(joint_names[idx], name, 31);
        joint_names[idx][31] = '\0';

        positions[idx] = position;
        modes[idx] = MODE_POSITION;
        joint_count++;

        return true;
    }

    // Add a joint velocity command
    bool add_velocity(const char* name, double velocity) {
        if (joint_count >= 16) return false;

        size_t idx = joint_count;
        std::strncpy(joint_names[idx], name, 31);
        joint_names[idx][31] = '\0';

        velocities[idx] = velocity;
        modes[idx] = MODE_VELOCITY;
        joint_count++;

        return true;
    }

    // Add a joint effort command
    bool add_effort(const char* name, double effort) {
        if (joint_count >= 16) return false;

        size_t idx = joint_count;
        std::strncpy(joint_names[idx], name, 31);
        joint_names[idx][31] = '\0';

        efforts[idx] = effort;
        modes[idx] = MODE_EFFORT;
        joint_count++;

        return true;
    }

    // Clear all commands
    void clear() {
        joint_count = 0;
        std::memset(positions, 0, sizeof(positions));
        std::memset(velocities, 0, sizeof(velocities));
        std::memset(efforts, 0, sizeof(efforts));
    }
};

} // namespace messages
} // namespace horus

#endif // HORUS_MESSAGES_CONTROL_HPP
