// HORUS Message Library - Diagnostics Types
// Binary-compatible with Rust definitions in horus_library/messages/diagnostics.rs

#ifndef HORUS_MESSAGES_DIAGNOSTICS_HPP
#define HORUS_MESSAGES_DIAGNOSTICS_HPP

#include <cstdint>
#include <cstring>
#include <chrono>

namespace horus {
namespace messages {

// ============================================================================
// Heartbeat - System heartbeat message
// ============================================================================
struct Heartbeat {
    char node_name[32];         // Node name (null-terminated string)
    uint32_t node_id;           // Node ID
    uint64_t sequence;          // Sequence number (increments each heartbeat)
    bool alive;                 // Node is alive and responding
    uint8_t _padding[7];        // Padding
    double uptime;              // Time since startup in seconds
    uint64_t timestamp;         // nanoseconds since epoch

    Heartbeat() : node_id(0), sequence(0), alive(true), uptime(0.0) {
        std::memset(node_name, 0, sizeof(node_name));
        std::memset(_padding, 0, sizeof(_padding));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    static Heartbeat create(const char* name, uint32_t id) {
        Heartbeat hb;
        hb.node_id = id;
        std::strncpy(hb.node_name, name, sizeof(hb.node_name) - 1);
        return hb;
    }

    void update(double up) {
        sequence++;
        uptime = up;
        update_timestamp();
    }
};

// ============================================================================
// StatusLevel - Status severity enumeration
// ============================================================================
enum class StatusLevel : uint8_t {
    Ok = 0,      // Everything is OK
    Warn = 1,    // Warning condition
    Error = 2,   // Error condition (recoverable)
    Fatal = 3,   // Fatal error (system should stop)
};

// ============================================================================
// Status - System status message
// ============================================================================
struct Status {
    StatusLevel level;          // Severity level
    uint8_t _padding[3];        // Padding
    uint32_t code;              // Error/status code (component-specific)
    char message[128];          // Human-readable message
    char component[32];         // Component name reporting the status
    uint64_t timestamp;         // nanoseconds since epoch

    Status() : level(StatusLevel::Ok), code(0) {
        std::memset(_padding, 0, sizeof(_padding));
        std::memset(message, 0, sizeof(message));
        std::memset(component, 0, sizeof(component));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    static Status ok(const char* msg) {
        Status s;
        s.level = StatusLevel::Ok;
        s.code = 0;
        std::strncpy(s.message, msg, sizeof(s.message) - 1);
        s.update_timestamp();
        return s;
    }

    static Status warn(uint32_t code, const char* msg) {
        Status s;
        s.level = StatusLevel::Warn;
        s.code = code;
        std::strncpy(s.message, msg, sizeof(s.message) - 1);
        s.update_timestamp();
        return s;
    }

    static Status error(uint32_t code, const char* msg) {
        Status s;
        s.level = StatusLevel::Error;
        s.code = code;
        std::strncpy(s.message, msg, sizeof(s.message) - 1);
        s.update_timestamp();
        return s;
    }

    static Status fatal(uint32_t code, const char* msg) {
        Status s;
        s.level = StatusLevel::Fatal;
        s.code = code;
        std::strncpy(s.message, msg, sizeof(s.message) - 1);
        s.update_timestamp();
        return s;
    }

    void set_component(const char* comp) {
        std::strncpy(component, comp, sizeof(component) - 1);
    }
};

// ============================================================================
// EmergencyStop - Emergency stop message
// ============================================================================
struct EmergencyStop {
    bool engaged;               // Emergency stop is active
    bool auto_reset;            // Auto-reset allowed after clearing
    uint8_t _padding[6];        // Padding
    char reason[64];            // Reason for emergency stop
    char source[32];            // Source that triggered the stop
    uint64_t timestamp;         // nanoseconds since epoch

    EmergencyStop() : engaged(false), auto_reset(false) {
        std::memset(_padding, 0, sizeof(_padding));
        std::memset(reason, 0, sizeof(reason));
        std::memset(source, 0, sizeof(source));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    static EmergencyStop engage(const char* rsn) {
        EmergencyStop estop;
        estop.engaged = true;
        std::strncpy(estop.reason, rsn, sizeof(estop.reason) - 1);
        estop.update_timestamp();
        return estop;
    }

    static EmergencyStop release() {
        EmergencyStop estop;
        estop.engaged = false;
        estop.update_timestamp();
        return estop;
    }

    void set_source(const char* src) {
        std::strncpy(source, src, sizeof(source) - 1);
    }
};

// ============================================================================
// ResourceUsage - System resource usage
// ============================================================================
struct ResourceUsage {
    float cpu_percent;          // CPU usage percentage (0-100)
    uint64_t memory_bytes;      // Memory usage in bytes
    float memory_percent;       // Memory usage percentage (0-100)
    uint64_t disk_bytes;        // Disk usage in bytes
    float disk_percent;         // Disk usage percentage (0-100)
    uint64_t network_tx_bytes;  // Network bytes sent
    uint64_t network_rx_bytes;  // Network bytes received
    float temperature;          // System temperature in celsius
    uint32_t thread_count;      // Number of active threads
    uint32_t process_count;     // Number of active processes
    uint64_t timestamp;         // nanoseconds since epoch

    ResourceUsage() : cpu_percent(0.0f), memory_bytes(0), memory_percent(0.0f),
                      disk_bytes(0), disk_percent(0.0f), network_tx_bytes(0),
                      network_rx_bytes(0), temperature(0.0f), thread_count(0),
                      process_count(0) {
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }
};

// ============================================================================
// SafetyStatus - Safety system status
// ============================================================================
struct SafetyStatus {
    bool emergency_stop_active; // Emergency stop is active
    bool safety_override;       // Safety override is active
    bool motion_enabled;        // Motion is enabled
    bool all_limits_ok;         // All limit switches OK
    uint8_t safety_zone;        // Current safety zone (0=safe, higher=restricted)
    uint8_t _padding[3];        // Padding
    char last_fault[64];        // Last safety fault description
    uint64_t fault_count;       // Total safety faults since startup
    uint64_t timestamp;         // nanoseconds since epoch

    SafetyStatus() : emergency_stop_active(false), safety_override(false),
                     motion_enabled(false), all_limits_ok(true), safety_zone(0),
                     fault_count(0) {
        std::memset(_padding, 0, sizeof(_padding));
        std::memset(last_fault, 0, sizeof(last_fault));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    void record_fault(const char* fault) {
        std::strncpy(last_fault, fault, sizeof(last_fault) - 1);
        fault_count++;
        update_timestamp();
    }
};

} // namespace messages
} // namespace horus

#endif // HORUS_MESSAGES_DIAGNOSTICS_HPP
