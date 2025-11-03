// HORUS C++ API - Modern C++ interface for hardware integration
#ifndef HORUS_HPP
#define HORUS_HPP

#include "horus.h"
#include <string>
#include <memory>
#include <functional>
#include <stdexcept>
#include <cstring>
#include <vector>

// Include HORUS message library
#include "horus/messages.hpp"

namespace horus {

// Re-export message types into horus namespace for convenience
// Geometry
using messages::Twist;
using messages::Pose2D;
using messages::Transform;
using messages::Vector3;
using messages::Point3;
using messages::Quaternion;

// Sensor
using messages::LaserScan;
using messages::Imu;
using messages::Odometry;
using messages::Range;
using messages::BatteryState;

// Vision
using messages::Image;
using messages::ImageEncoding;
using messages::CompressedImage;
using messages::CameraInfo;
using messages::RegionOfInterest;
using messages::Detection;
using messages::DetectionArray;
using messages::StereoInfo;

// Perception
using messages::PointCloud;
using messages::PointField;
using messages::PointFieldType;
using messages::BoundingBox3D;
using messages::BoundingBoxArray3D;
using messages::DepthImage;
using messages::PlaneDetection;
using messages::PlaneArray;

// Navigation
using messages::Goal;
using messages::GoalStatus;
using messages::GoalResult;
using messages::Waypoint;
using messages::Path;
using messages::OccupancyGrid;
using messages::CostMap;
using messages::VelocityObstacle;
using messages::VelocityObstacles;
using messages::PathPlan;

// Control
using messages::MotorCommand;
using messages::DifferentialDriveCommand;
using messages::ServoCommand;
using messages::PidConfig;
using messages::TrajectoryPoint;
using messages::JointCommand;

// Diagnostics
using messages::Heartbeat;
using messages::Status;
using messages::StatusLevel;
using messages::EmergencyStop;
using messages::ResourceUsage;
using messages::SafetyStatus;

// Exception for HORUS errors
class HorusException : public std::runtime_error {
public:
    explicit HorusException(const std::string& msg) : std::runtime_error(msg) {}
};

// RAII wrapper for initialization
class System {
public:
    explicit System(const std::string& node_name) {
        if (!init(node_name.c_str())) {
            throw HorusException("Failed to initialize HORUS system");
        }
    }

    ~System() {
        shutdown();
    }

    // Non-copyable
    System(const System&) = delete;
    System& operator=(const System&) = delete;

    // Moveable
    System(System&& other) noexcept : moved_from_(false) {
        other.moved_from_ = true;
    }

    System& operator=(System&& other) noexcept {
        if (this != &other) {
            if (!moved_from_) {
                shutdown();
            }
            moved_from_ = false;
            other.moved_from_ = true;
        }
        return *this;
    }

    bool ok() const { return ::ok(); }
    void spin_once() const { ::spin_once(); }
    void spin() const { ::spin(); }

private:
    bool moved_from_ = false;
};

// Publisher specializations for standard message types
// Note: C++ API only supports built-in message types (Twist, Pose, etc.)
// For custom types, use the Rust API directly

template<typename T> class Publisher;  // Forward declaration (no definition - only specializations exist)

template<>
class Publisher<Twist> {
public:
    // Default constructor - creates invalid publisher
    Publisher() : handle_(0), ctx_(nullptr) {}

    // Constructor for use within NodeContext (with logging)
    explicit Publisher(const std::string& topic, HorusNodeContext* ctx = nullptr)
        : handle_(publisher(topic.c_str(), MSG_TWIST)), ctx_(ctx) {
        if (handle_ == 0) {
            throw HorusException("Failed to create Twist publisher");
        }
    }

    ~Publisher() = default;

    Publisher(const Publisher&) = delete;
    Publisher& operator=(const Publisher&) = delete;
    Publisher(Publisher&& other) noexcept
        : handle_(other.handle_), ctx_(other.ctx_) {
        other.handle_ = 0;
        other.ctx_ = nullptr;
    }
    Publisher& operator=(Publisher&& other) noexcept {
        if (this != &other) {
            handle_ = other.handle_;
            ctx_ = other.ctx_;
            other.handle_ = 0;
            other.ctx_ = nullptr;
        }
        return *this;
    }

    bool send(const Twist& data) {
        if (handle_ == 0) throw HorusException("Invalid publisher");

        // Use context-aware send for logging if context available
        if (ctx_) {
            return ::node_send(ctx_, handle_, &data);
        } else {
            return ::send(handle_, &data);
        }
    }

    Publisher& operator<<(const Twist& data) {
        send(data);
        return *this;
    }

private:
    HorusPub handle_;
    HorusNodeContext* ctx_;
};

// Subscriber specializations for standard message types
// Note: C++ API only supports built-in message types (Twist, Pose, etc.)
// For custom types, use the Rust API directly

template<typename T> class Subscriber;  // Forward declaration (no definition - only specializations exist)

// Specializations for known types
template<>
class Subscriber<Twist> {
public:
    // Default constructor - creates invalid subscriber
    Subscriber() : handle_(0), ctx_(nullptr) {}

    // Constructor for use within NodeContext (with logging)
    explicit Subscriber(const std::string& topic, HorusNodeContext* ctx = nullptr)
        : handle_(subscriber(topic.c_str(), MSG_TWIST)), ctx_(ctx) {
        if (handle_ == 0) {
            throw HorusException("Failed to create Twist subscriber");
        }
    }

    ~Subscriber() = default;

    Subscriber(const Subscriber&) = delete;
    Subscriber& operator=(const Subscriber&) = delete;
    Subscriber(Subscriber&& other) noexcept
        : handle_(other.handle_), ctx_(other.ctx_) {
        other.handle_ = 0;
        other.ctx_ = nullptr;
    }
    Subscriber& operator=(Subscriber&& other) noexcept {
        if (this != &other) {
            handle_ = other.handle_;
            ctx_ = other.ctx_;
            other.handle_ = 0;
            other.ctx_ = nullptr;
        }
        return *this;
    }

    bool recv(Twist& data) {
        if (handle_ == 0) throw HorusException("Invalid subscriber");

        // Use context-aware recv for logging if context available
        if (ctx_) {
            return ::node_recv(ctx_, handle_, &data);
        } else {
            return ::recv(handle_, &data);
        }
    }

    Subscriber& operator>>(Twist& data) {
        recv(data);
        return *this;
    }

private:
    HorusSub handle_;
    HorusNodeContext* ctx_;
};

// Utility functions in namespace
inline void sleep_ms(uint32_t ms) { ::sleep_ms(ms); }
inline uint64_t time_now_ms() { return ::time_now_ms(); }

// Logging utilities
class Log {
public:
    static void info(const std::string& msg) { log_info(msg.c_str()); }
    static void warn(const std::string& msg) { log_warn(msg.c_str()); }
    static void error(const std::string& msg) { log_error(msg.c_str()); }
    static void debug(const std::string& msg) { log_debug(msg.c_str()); }
};

// Helper to create message structs with initializers
inline Vector3 make_vector3(double x, double y, double z) {
    return Vector3(x, y, z);
}

inline Quaternion make_quaternion(double x, double y, double z, double w) {
    return Quaternion(x, y, z, w);
}

inline Twist make_twist(double lx, double ly, double lz, double ax, double ay, double az) {
    return Twist(lx, ly, lz, ax, ay, az);
}

inline Pose2D make_pose2d(double x, double y, double theta) {
    return Pose2D(x, y, theta);
}

inline Transform make_transform(double tx, double ty, double tz, double qx, double qy, double qz, double qw) {
    return Transform(tx, ty, tz, qx, qy, qz, qw);
}

// ============================================================================
// Framework API - Node and Scheduler integration with HORUS
// ============================================================================

// Priority levels for node execution (0=Critical, 1=High, 2=Normal, 3=Low, 4=Background)
// Lower numbers = higher priority (executed first)

// NodeContext provides access to HORUS services within node callbacks
class NodeContext {
public:
    explicit NodeContext(HorusNodeContext* ctx) : ctx_(ctx) {}

    // Create publishers within node context (with logging enabled)
    template<typename T>
    Publisher<T> create_publisher(const std::string& topic) {
        // Pass context pointer to enable rich logging
        return Publisher<T>(topic, ctx_);
    }

    // Create subscribers within node context (with logging enabled)
    template<typename T>
    Subscriber<T> create_subscriber(const std::string& topic) {
        // Pass context pointer to enable rich logging
        return Subscriber<T>(topic, ctx_);
    }

    // Shorter aliases
    template<typename T>
    Publisher<T> pub(const std::string& topic) {
        return Publisher<T>(topic, ctx_);
    }

    template<typename T>
    Subscriber<T> sub(const std::string& topic) {
        return Subscriber<T>(topic, ctx_);
    }

    // Logging through node context
    void log_info(const std::string& msg) {
        node_log_info(ctx_, msg.c_str());
    }

    void log_warn(const std::string& msg) {
        node_log_warn(ctx_, msg.c_str());
    }

    void log_error(const std::string& msg) {
        node_log_error(ctx_, msg.c_str());
    }

private:
    HorusNodeContext* ctx_;
};

// Abstract Node class - inherit from this to create HORUS nodes
class Node {
public:
    explicit Node(const std::string& name) : name_(name), user_data_(this) {}
    virtual ~Node() = default;

    // Node lifecycle - override these methods
    virtual bool init(NodeContext& ctx) = 0;
    virtual void tick(NodeContext& ctx) = 0;
    virtual void shutdown(NodeContext& ctx) {
        // Default: nothing to clean up
    }

    const std::string& name() const { return name_; }

    // Internal: get handle for registration
    HorusNode create_handle() {
        return node_create(
            name_.c_str(),
            &Node::init_callback,
            &Node::tick_callback,
            &Node::shutdown_callback,
            user_data_
        );
    }

private:
    std::string name_;
    void* user_data_;

    // Static C callbacks that forward to virtual methods
    static bool init_callback(HorusNodeContext* ctx, void* user_data) {
        Node* node = static_cast<Node*>(user_data);
        NodeContext cpp_ctx(ctx);
        return node->init(cpp_ctx);
    }

    static void tick_callback(HorusNodeContext* ctx, void* user_data) {
        Node* node = static_cast<Node*>(user_data);
        NodeContext cpp_ctx(ctx);
        node->tick(cpp_ctx);
    }

    static void shutdown_callback(HorusNodeContext* ctx, void* user_data) {
        Node* node = static_cast<Node*>(user_data);
        NodeContext cpp_ctx(ctx);
        node->shutdown(cpp_ctx);
    }
};

// Scheduler - manages node execution at 60 FPS
class Scheduler {
public:
    explicit Scheduler(const std::string& name = "scheduler")
        : handle_(scheduler_create(name.c_str())) {
        if (handle_ == 0) {
            throw HorusException("Failed to create scheduler");
        }
    }

    ~Scheduler() {
        if (handle_ != 0) {
            scheduler_destroy(handle_);
        }
    }

    // Non-copyable
    Scheduler(const Scheduler&) = delete;
    Scheduler& operator=(const Scheduler&) = delete;

    // Moveable
    Scheduler(Scheduler&& other) noexcept : handle_(other.handle_) {
        other.handle_ = 0;
    }

    Scheduler& operator=(Scheduler&& other) noexcept {
        if (this != &other) {
            if (handle_ != 0) {
                scheduler_destroy(handle_);
            }
            handle_ = other.handle_;
            other.handle_ = 0;
        }
        return *this;
    }

    // Add a node to the scheduler
    // priority: 0=Critical, 1=High, 2=Normal (default), 3=Low, 4=Background
    // enable_logging: true=rich logging with timestamps/IPC timing, false=no logging
    template<typename NodeType>
    bool add(NodeType& node, uint32_t priority = 2, bool enable_logging = true) {
        static_assert(std::is_base_of<Node, NodeType>::value,
                     "NodeType must inherit from horus::Node");

        HorusNode node_handle = node.create_handle();
        if (node_handle == 0) {
            return false;
        }

        return scheduler_add(handle_, node_handle, priority, enable_logging);
    }

    // Run the scheduler with all nodes (blocks until stopped)
    void run() {
        if (handle_ == 0) {
            throw HorusException("Cannot run invalid scheduler");
        }
        scheduler_run(handle_);
    }

    // Run the scheduler with specific nodes only (blocks until stopped)
    void tick(const std::vector<std::string>& node_names) {
        if (handle_ == 0) {
            throw HorusException("Cannot run invalid scheduler");
        }

        // Convert std::vector<std::string> to const char**
        std::vector<const char*> c_names;
        c_names.reserve(node_names.size());
        for (const auto& name : node_names) {
            c_names.push_back(name.c_str());
        }

        scheduler_tick(handle_, c_names.data(), c_names.size());
    }

    // Stop the scheduler
    void stop() {
        if (handle_ != 0) {
            scheduler_stop(handle_);
        }
    }

    bool valid() const { return handle_ != 0; }

private:
    HorusScheduler handle_;
};

} // namespace horus

#endif // HORUS_HPP
