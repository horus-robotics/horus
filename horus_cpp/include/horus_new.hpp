// HORUS C++ API - Modern interface matching Rust/Python design
// Version: 0.4.0 (Redesigned for consistency)
#ifndef HORUS_NEW_HPP
#define HORUS_NEW_HPP

#include "horus.h"
#include <string>
#include <memory>
#include <stdexcept>
#include <cstring>

// Include HORUS message library
#include "horus/messages.hpp"

namespace horus {

// Re-export message types for convenience
using messages::Twist;
using messages::Pose2D;
using messages::Transform;
using messages::Vector3;
using messages::Point3;
using messages::Quaternion;
using messages::LaserScan;
using messages::Imu;
using messages::Odometry;
using messages::Range;
using messages::BatteryState;
using messages::Image;
using messages::CameraInfo;
using messages::Detection;
using messages::DetectionArray;
using messages::PointCloud;
using messages::BoundingBox3D;
using messages::Goal;
using messages::Path;
using messages::OccupancyGrid;
using messages::MotorCommand;
using messages::DifferentialDriveCommand;
using messages::JointCommand;
using messages::Heartbeat;
using messages::Status;
using messages::EmergencyStop;

// ============================================================================
// Exception Types
// ============================================================================

class HorusException : public std::runtime_error {
public:
    explicit HorusException(const std::string& msg) : std::runtime_error(msg) {}
};

// ============================================================================
// NodeContext - Runtime context for node callbacks (matches Rust's NodeInfo)
// ============================================================================

class NodeContext {
private:
    HorusNodeContext* ffi_ctx_;

public:
    explicit NodeContext(HorusNodeContext* ctx) : ffi_ctx_(ctx) {}

    // Logging (writes to global log buffer for dashboard)
    void log_info(const std::string& msg) {
        node_log_info(ffi_ctx_, msg.c_str());
    }

    void log_warn(const std::string& msg) {
        node_log_warn(ffi_ctx_, msg.c_str());
    }

    void log_error(const std::string& msg) {
        node_log_error(ffi_ctx_, msg.c_str());
    }

    void log_debug(const std::string& msg) {
        // Note: Debug logs only appear if LOG_LEVEL=DEBUG
        node_log_info(ffi_ctx_, (std::string("[DEBUG] ") + msg).c_str());
    }

    // Node information
    const char* node_name() const {
        // TODO: Add FFI function to get node name
        return "unknown";
    }

    uint64_t tick_count() const {
        // TODO: Add FFI function to get tick count
        return 0;
    }
};

// ============================================================================
// Publisher<T> - Type-safe message publisher
// ============================================================================

template<typename T>
class Publisher {
private:
    HorusPub handle_;
    std::string topic_;
    bool valid_;

public:
    // Default constructor (invalid publisher)
    Publisher() : handle_(0), valid_(false) {}

    // Create publisher for topic
    explicit Publisher(const std::string& topic)
        : topic_(topic), valid_(false) {
        // Determine message type from T
        MessageType msg_type = get_message_type<T>();
        handle_ = publisher(topic.c_str(), msg_type);
        valid_ = (handle_ != 0);

        if (!valid_) {
            throw HorusException("Failed to create publisher for topic: " + topic);
        }
    }

    ~Publisher() = default;

    // Non-copyable
    Publisher(const Publisher&) = delete;
    Publisher& operator=(const Publisher&) = delete;

    // Moveable
    Publisher(Publisher&& other) noexcept
        : handle_(other.handle_), topic_(std::move(other.topic_)), valid_(other.valid_) {
        other.handle_ = 0;
        other.valid_ = false;
    }

    Publisher& operator=(Publisher&& other) noexcept {
        if (this != &other) {
            handle_ = other.handle_;
            topic_ = std::move(other.topic_);
            valid_ = other.valid_;
            other.handle_ = 0;
            other.valid_ = false;
        }
        return *this;
    }

    // Send message (throws on error, matches Rust send())
    void send(const T& msg) {
        if (!valid_) {
            throw HorusException("Cannot send on invalid publisher");
        }

        if (!::send(handle_, &msg)) {
            throw HorusException("Failed to send message to topic: " + topic_);
        }
    }

    // Try send (returns false on error, no exceptions)
    bool try_send(const T& msg) {
        if (!valid_) return false;
        return ::send(handle_, &msg);
    }

    const std::string& topic() const { return topic_; }
    bool is_valid() const { return valid_; }

private:
    // Template specialization for message type mapping
    template<typename MsgType>
    static MessageType get_message_type() {
        // Default: return custom type
        return MSG_CUSTOM;
    }
};

// Template specializations for message type mapping
template<> inline MessageType Publisher<Twist>::get_message_type<Twist>() { return MSG_TWIST; }
template<> inline MessageType Publisher<LaserScan>::get_message_type<LaserScan>() { return MSG_LASER_SCAN; }
template<> inline MessageType Publisher<Image>::get_message_type<Image>() { return MSG_IMAGE; }
template<> inline MessageType Publisher<Imu>::get_message_type<Imu>() { return MSG_IMU; }

// ============================================================================
// Subscriber<T> - Type-safe message subscriber
// ============================================================================

template<typename T>
class Subscriber {
private:
    HorusSub handle_;
    std::string topic_;
    bool valid_;

public:
    // Default constructor (invalid subscriber)
    Subscriber() : handle_(0), valid_(false) {}

    // Create subscriber for topic
    explicit Subscriber(const std::string& topic)
        : topic_(topic), valid_(false) {
        MessageType msg_type = get_message_type<T>();
        handle_ = subscriber(topic.c_str(), msg_type);
        valid_ = (handle_ != 0);

        if (!valid_) {
            throw HorusException("Failed to create subscriber for topic: " + topic);
        }
    }

    ~Subscriber() = default;

    // Non-copyable
    Subscriber(const Subscriber&) = delete;
    Subscriber& operator=(const Subscriber&) = delete;

    // Moveable
    Subscriber(Subscriber&& other) noexcept
        : handle_(other.handle_), topic_(std::move(other.topic_)), valid_(other.valid_) {
        other.handle_ = 0;
        other.valid_ = false;
    }

    Subscriber& operator=(Subscriber&& other) noexcept {
        if (this != &other) {
            handle_ = other.handle_;
            topic_ = std::move(other.topic_);
            valid_ = other.valid_;
            other.handle_ = 0;
            other.valid_ = false;
        }
        return *this;
    }

    // Receive message (returns true if message available, matches Rust recv())
    bool recv(T& msg) {
        if (!valid_) return false;
        return ::recv(handle_, &msg);
    }

    // Check if messages are available
    bool has_messages() const {
        // TODO: Add FFI function for this
        return false;
    }

    const std::string& topic() const { return topic_; }
    bool is_valid() const { return valid_; }

private:
    template<typename MsgType>
    static MessageType get_message_type() {
        return MSG_CUSTOM;
    }
};

// Template specializations
template<> inline MessageType Subscriber<Twist>::get_message_type<Twist>() { return MSG_TWIST; }
template<> inline MessageType Subscriber<LaserScan>::get_message_type<LaserScan>() { return MSG_LASER_SCAN; }
template<> inline MessageType Subscriber<Image>::get_message_type<Image>() { return MSG_IMAGE; }
template<> inline MessageType Subscriber<Imu>::get_message_type<Imu>() { return MSG_IMU; }

// ============================================================================
// Node - Base class for all HORUS nodes (matches Rust's Node trait)
// ============================================================================

class Node {
private:
    std::string name_;

public:
    explicit Node(const std::string& name) : name_(name) {}
    virtual ~Node() = default;

    // Lifecycle callbacks (same as Rust!)
    // Returns true on success, false on failure
    virtual bool init(NodeContext& ctx) { return true; }

    // Called at 60 FPS by scheduler
    virtual void tick(NodeContext& ctx) = 0;  // Pure virtual - must implement

    // Returns true on success, false on failure
    virtual bool shutdown(NodeContext& ctx) { return true; }

    // Node info
    const std::string& name() const { return name_; }

    // Internal: Create FFI handle for registration
    HorusNode create_ffi_handle(void* user_data) {
        return node_create(
            name_.c_str(),
            &Node::ffi_init_callback,
            &Node::ffi_tick_callback,
            &Node::ffi_shutdown_callback,
            user_data
        );
    }

private:
    // Static C callbacks that forward to virtual methods
    static bool ffi_init_callback(HorusNodeContext* ctx, void* user_data) {
        Node* node = static_cast<Node*>(user_data);
        NodeContext cpp_ctx(ctx);
        return node->init(cpp_ctx);
    }

    static void ffi_tick_callback(HorusNodeContext* ctx, void* user_data) {
        Node* node = static_cast<Node*>(user_data);
        NodeContext cpp_ctx(ctx);
        node->tick(cpp_ctx);
    }

    static void ffi_shutdown_callback(HorusNodeContext* ctx, void* user_data) {
        Node* node = static_cast<Node*>(user_data);
        NodeContext cpp_ctx(ctx);
        node->shutdown(cpp_ctx);
    }
};

// ============================================================================
// Scheduler - Manages node execution at 60 FPS (matches Rust's Scheduler)
// ============================================================================

class Scheduler {
private:
    HorusScheduler ffi_handle_;
    std::vector<std::unique_ptr<Node>> nodes_;

public:
    Scheduler() : ffi_handle_(scheduler_create("cpp_scheduler")) {
        if (ffi_handle_ == 0) {
            throw HorusException("Failed to create scheduler");
        }
    }

    ~Scheduler() {
        if (ffi_handle_ != 0) {
            scheduler_destroy(ffi_handle_);
        }
    }

    // Non-copyable
    Scheduler(const Scheduler&) = delete;
    Scheduler& operator=(const Scheduler&) = delete;

    // Moveable
    Scheduler(Scheduler&& other) noexcept : ffi_handle_(other.ffi_handle_), nodes_(std::move(other.nodes_)) {
        other.ffi_handle_ = 0;
    }

    Scheduler& operator=(Scheduler&& other) noexcept {
        if (this != &other) {
            if (ffi_handle_ != 0) {
                scheduler_destroy(ffi_handle_);
            }
            ffi_handle_ = other.ffi_handle_;
            nodes_ = std::move(other.nodes_);
            other.ffi_handle_ = 0;
        }
        return *this;
    }

    // Add node with priority and logging (SAME API as Rust!)
    // priority: 0=Critical, 1=High, 2=Normal, 3=Low, 4=Background
    // enable_logging: true for rich logging with timestamps/IPC timing
    template<typename NodeType>
    void add(NodeType&& node, uint32_t priority, bool enable_logging) {
        static_assert(std::is_base_of<Node, NodeType>::value,
                     "NodeType must inherit from horus::Node");

        // Store node ownership
        auto node_ptr = std::make_unique<NodeType>(std::forward<NodeType>(node));
        Node* node_raw = node_ptr.get();

        // Create FFI handle
        HorusNode ffi_node = node_raw->create_ffi_handle(node_raw);

        if (ffi_node == 0) {
            throw HorusException("Failed to create node FFI handle");
        }

        // Add to Rust scheduler
        if (!scheduler_add(ffi_handle_, ffi_node, priority, enable_logging)) {
            throw HorusException("Failed to add node to scheduler");
        }

        nodes_.push_back(std::move(node_ptr));
    }

    // Run scheduler at 60 FPS (blocks until Ctrl+C)
    void run() {
        if (ffi_handle_ == 0) {
            throw HorusException("Cannot run invalid scheduler");
        }
        scheduler_run(ffi_handle_);
    }

    // Stop scheduler
    void stop() {
        if (ffi_handle_ != 0) {
            scheduler_stop(ffi_handle_);
        }
    }

    bool is_valid() const { return ffi_handle_ != 0; }
};

// ============================================================================
// Utility Functions
// ============================================================================

inline void sleep_ms(uint32_t ms) { ::sleep_ms(ms); }
inline uint64_t time_now_ms() { return ::time_now_ms(); }

// ============================================================================
// Helper Functions for Message Creation
// ============================================================================

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

} // namespace horus

#endif // HORUS_NEW_HPP
