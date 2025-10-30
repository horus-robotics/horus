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
    explicit Publisher(const std::string& topic)
        : handle_(publisher(topic.c_str(), MSG_TWIST)) {
        if (handle_ == 0) {
            throw HorusException("Failed to create Twist publisher");
        }
    }

    ~Publisher() = default;

    Publisher(const Publisher&) = delete;
    Publisher& operator=(const Publisher&) = delete;
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

    bool send(const Twist& data) {
        if (handle_ == 0) throw HorusException("Invalid publisher");
        return ::send(handle_, &data);
    }

    Publisher& operator<<(const Twist& data) {
        send(data);
        return *this;
    }

private:
    Pub handle_;
};

template<>
class Publisher<Pose> {
public:
    explicit Publisher(const std::string& topic)
        : handle_(publisher(topic.c_str(), MSG_POSE)) {
        if (handle_ == 0) {
            throw HorusException("Failed to create Pose publisher");
        }
    }

    ~Publisher() = default;

    Publisher(const Publisher&) = delete;
    Publisher& operator=(const Publisher&) = delete;
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

    bool send(const Pose& data) {
        if (handle_ == 0) throw HorusException("Invalid publisher");
        return ::send(handle_, &data);
    }

    Publisher& operator<<(const Pose& data) {
        send(data);
        return *this;
    }

private:
    Pub handle_;
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
    explicit Subscriber(const std::string& topic)
        : handle_(subscriber(topic.c_str(), MSG_TWIST)) {
        if (handle_ == 0) {
            throw HorusException("Failed to create Twist subscriber");
        }
    }

    ~Subscriber() = default;

    Subscriber(const Subscriber&) = delete;
    Subscriber& operator=(const Subscriber&) = delete;
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

    bool recv(Twist& data) {
        if (handle_ == 0) throw HorusException("Invalid subscriber");
        return ::recv(handle_, &data);
    }

    bool try_recv(Twist& data) {
        if (handle_ == 0) throw HorusException("Invalid subscriber");
        return ::try_recv(handle_, &data);
    }

    Subscriber& operator>>(Twist& data) {
        recv(data);
        return *this;
    }

private:
    Sub handle_;
};

template<>
class Subscriber<Pose> {
public:
    explicit Subscriber(const std::string& topic)
        : handle_(subscriber(topic.c_str(), MSG_POSE)) {
        if (handle_ == 0) {
            throw HorusException("Failed to create Pose subscriber");
        }
    }

    ~Subscriber() = default;

    Subscriber(const Subscriber&) = delete;
    Subscriber& operator=(const Subscriber&) = delete;
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

    bool recv(Pose& data) {
        if (handle_ == 0) throw HorusException("Invalid subscriber");
        return ::recv(handle_, &data);
    }

    bool try_recv(Pose& data) {
        if (handle_ == 0) throw HorusException("Invalid subscriber");
        return ::try_recv(handle_, &data);
    }

    Subscriber& operator>>(Pose& data) {
        recv(data);
        return *this;
    }

private:
    Sub handle_;
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

} // namespace horus

#endif // HORUS_HPP
