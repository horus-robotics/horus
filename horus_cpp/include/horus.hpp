// HORUS C++ API - Modern C++ interface for hardware integration
#ifndef HORUS_HPP
#define HORUS_HPP

#include "horus.h"
#include <string>
#include <memory>
#include <functional>
#include <stdexcept>
#include <cstring>

namespace horus {

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

// Template publisher with RAII
template<typename T>
class Publisher {
public:
    Publisher(const std::string& topic, MessageType type = MSG_CUSTOM)
        : handle_(0) {
        if constexpr (std::is_same_v<T, void>) {
            throw HorusException("Cannot create publisher with void type");
        } else {
            handle_ = publisher_custom(topic.c_str(), sizeof(T));
            if (handle_ == 0) {
                throw HorusException("Failed to create publisher for topic: " + topic);
            }
        }
    }

    ~Publisher() {
        // Handle cleanup happens in Rust when handle is dropped
    }

    // Non-copyable
    Publisher(const Publisher&) = delete;
    Publisher& operator=(const Publisher&) = delete;

    // Moveable
    Publisher(Publisher&& other) noexcept : handle_(other.handle_) {
        other.handle_ = 0;
    }

    Publisher& operator=(Publisher&& other) noexcept {
        if (this != &other) {
            handle_ = other.handle_;
            other.handle_ = 0;
        }
        return *this;
    }

    bool send(const T& data) {
        if (handle_ == 0) {
            throw HorusException("Cannot send on moved-from publisher");
        }
        return ::send(handle_, &data);
    }

    // Operator overload for convenient publishing
    Publisher& operator<<(const T& data) {
        send(data);
        return *this;
    }

    bool valid() const { return handle_ != 0; }

private:
    Pub handle_;
};

// Specialization for known message types
template<>
class Publisher<Twist> {
public:
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
    Pub handle_;
    HorusNodeContext* ctx_;
};

template<>
class Publisher<Pose> {
public:
    // Constructor for use within NodeContext (with logging)
    explicit Publisher(const std::string& topic, HorusNodeContext* ctx = nullptr)
        : handle_(publisher(topic.c_str(), MSG_POSE)), ctx_(ctx) {
        if (handle_ == 0) {
            throw HorusException("Failed to create Pose publisher");
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

    bool send(const Pose& data) {
        if (handle_ == 0) throw HorusException("Invalid publisher");

        // Use context-aware send for logging if context available
        if (ctx_) {
            return ::node_send(ctx_, handle_, &data);
        } else {
            return ::send(handle_, &data);
        }
    }

    Publisher& operator<<(const Pose& data) {
        send(data);
        return *this;
    }

private:
    Pub handle_;
    HorusNodeContext* ctx_;
};

// Template subscriber with RAII
template<typename T>
class Subscriber {
public:
    Subscriber(const std::string& topic, MessageType type = MSG_CUSTOM)
        : handle_(0) {
        if constexpr (std::is_same_v<T, void>) {
            throw HorusException("Cannot create subscriber with void type");
        } else {
            handle_ = subscriber_custom(topic.c_str(), sizeof(T));
            if (handle_ == 0) {
                throw HorusException("Failed to create subscriber for topic: " + topic);
            }
        }
    }

    ~Subscriber() = default;

    // Non-copyable
    Subscriber(const Subscriber&) = delete;
    Subscriber& operator=(const Subscriber&) = delete;

    // Moveable
    Subscriber(Subscriber&& other) noexcept : handle_(other.handle_) {
        other.handle_ = 0;
    }

    Subscriber& operator=(Subscriber&& other) noexcept {
        if (this != &other) {
            handle_ = other.handle_;
            other.handle_ = 0;
        }
        return *this;
    }

    bool recv(T& data) {
        if (handle_ == 0) {
            throw HorusException("Cannot receive on moved-from subscriber");
        }
        return ::recv(handle_, &data);
    }

    bool try_recv(T& data) {
        if (handle_ == 0) {
            throw HorusException("Cannot receive on moved-from subscriber");
        }
        return ::try_recv(handle_, &data);
    }

    // Operator overload for convenient receiving
    Subscriber& operator>>(T& data) {
        recv(data);
        return *this;
    }

    bool valid() const { return handle_ != 0; }

private:
    Sub handle_;
};

// Specializations for known types
template<>
class Subscriber<Twist> {
public:
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

    bool try_recv(Twist& data) {
        if (handle_ == 0) throw HorusException("Invalid subscriber");

        // Use context-aware try_recv for logging if context available
        if (ctx_) {
            return ::node_try_recv(ctx_, handle_, &data);
        } else {
            return ::try_recv(handle_, &data);
        }
    }

    Subscriber& operator>>(Twist& data) {
        recv(data);
        return *this;
    }

private:
    Sub handle_;
    HorusNodeContext* ctx_;
};

template<>
class Subscriber<Pose> {
public:
    // Constructor for use within NodeContext (with logging)
    explicit Subscriber(const std::string& topic, HorusNodeContext* ctx = nullptr)
        : handle_(subscriber(topic.c_str(), MSG_POSE)), ctx_(ctx) {
        if (handle_ == 0) {
            throw HorusException("Failed to create Pose subscriber");
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

    bool recv(Pose& data) {
        if (handle_ == 0) throw HorusException("Invalid subscriber");

        // Use context-aware recv for logging if context available
        if (ctx_) {
            return ::node_recv(ctx_, handle_, &data);
        } else {
            return ::recv(handle_, &data);
        }
    }

    bool try_recv(Pose& data) {
        if (handle_ == 0) throw HorusException("Invalid subscriber");

        // Use context-aware try_recv for logging if context available
        if (ctx_) {
            return ::node_try_recv(ctx_, handle_, &data);
        } else {
            return ::try_recv(handle_, &data);
        }
    }

    Subscriber& operator>>(Pose& data) {
        recv(data);
        return *this;
    }

private:
    Sub handle_;
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
inline Vector3 make_vector3(float x, float y, float z) {
    return Vector3{x, y, z};
}

inline Quaternion make_quaternion(float x, float y, float z, float w) {
    return Quaternion{x, y, z, w};
}

inline Twist make_twist(Vector3 linear, Vector3 angular) {
    return Twist{linear, angular};
}

inline Pose make_pose(Vector3 position, Quaternion orientation) {
    return Pose{position, orientation};
}

// ============================================================================
// Framework API - Node and Scheduler integration with HORUS
// ============================================================================

// Priority levels for node execution
enum class Priority {
    Critical = PRIORITY_CRITICAL,
    High = PRIORITY_HIGH,
    Normal = PRIORITY_NORMAL,
    Low = PRIORITY_LOW,
    Background = PRIORITY_BACKGROUND
};

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

    // Register a node with the scheduler
    template<typename NodeType>
    bool register_node(NodeType& node, Priority priority = Priority::Normal) {
        static_assert(std::is_base_of<Node, NodeType>::value,
                     "NodeType must inherit from horus::Node");

        HorusNode node_handle = node.create_handle();
        if (node_handle == 0) {
            return false;
        }

        return scheduler_register(handle_, node_handle, static_cast<::Priority>(priority));
    }

    // Run the scheduler (blocks until stopped)
    void run() {
        if (handle_ == 0) {
            throw HorusException("Cannot run invalid scheduler");
        }
        scheduler_run(handle_);
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
