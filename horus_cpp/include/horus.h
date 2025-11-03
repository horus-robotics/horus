// HORUS Internal FFI - DO NOT USE DIRECTLY
// This header provides C ABI declarations for the C++ wrapper (horus.hpp)
// Users should include horus.hpp instead, which provides a safe C++ interface
#ifndef HORUS_H
#define HORUS_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Internal FFI Types
// ============================================================================

// Opaque handle types
// Prefixed to avoid conflicts with C++ namespace
typedef uint32_t HorusPub;
typedef uint32_t HorusSub;
typedef uint32_t HorusNode;
typedef uint32_t HorusScheduler;

// C-only aliases for convenience (hidden from C++)
#ifndef __cplusplus
typedef HorusPub Pub;
typedef HorusSub Sub;
typedef HorusNode Node;
typedef HorusScheduler Scheduler;
#endif

// Forward declaration of opaque context
typedef struct HorusNodeContext HorusNodeContext;

// C-only alias (hidden from C++)
#ifndef __cplusplus
typedef HorusNodeContext NodeContext;
#endif

// Message type identifiers
typedef enum {
    MSG_CUSTOM = 0,
    MSG_TWIST,
    MSG_POSE,
    MSG_LASER_SCAN,
    MSG_IMAGE,
    MSG_IMU,
    MSG_JOINT_STATE,
    MSG_POINT_CLOUD,
} MessageType;

// Node lifecycle callbacks
typedef bool (*NodeInitCallback)(HorusNodeContext* ctx, void* user_data);
typedef void (*NodeTickCallback)(HorusNodeContext* ctx, void* user_data);
typedef void (*NodeShutdownCallback)(HorusNodeContext* ctx, void* user_data);

// ============================================================================
// Internal FFI Functions
// ============================================================================

// System lifecycle
bool init(const char* node_name);
void shutdown(void);
bool ok(void);

// Publisher/Subscriber creation (only for standard message types)
HorusPub publisher(const char* topic, MessageType type);
HorusSub subscriber(const char* topic, MessageType type);

// Message send/receive
bool send(HorusPub pub, const void* data);
bool recv(HorusSub sub, void* data);

// Context-aware messaging (with logging)
bool node_send(HorusNodeContext* ctx, HorusPub pub, const void* data);
bool node_recv(HorusNodeContext* ctx, HorusSub sub, void* data);

// Timing utilities
void sleep_ms(uint32_t ms);
uint64_t time_now_ms(void);
void spin_once(void);
void spin(void);

// Logging
void log_info(const char* msg);
void log_warn(const char* msg);
void log_error(const char* msg);
void log_debug(const char* msg);

// Node creation
HorusNode node_create(const char* name,
                      NodeInitCallback init_fn,
                      NodeTickCallback tick_fn,
                      NodeShutdownCallback shutdown_fn,
                      void* user_data);
void node_destroy(HorusNode node);

// Scheduler management
HorusScheduler scheduler_create(const char* name);
bool scheduler_add(HorusScheduler sched, HorusNode node, uint32_t priority, bool enable_logging);
void scheduler_run(HorusScheduler sched);
void scheduler_tick(HorusScheduler sched, const char** node_names, size_t count);
void scheduler_stop(HorusScheduler sched);
void scheduler_destroy(HorusScheduler sched);

// Context API
HorusPub node_create_publisher(HorusNodeContext* ctx, const char* topic, MessageType type);
HorusSub node_create_subscriber(HorusNodeContext* ctx, const char* topic, MessageType type);
void node_log_info(HorusNodeContext* ctx, const char* msg);
void node_log_warn(HorusNodeContext* ctx, const char* msg);
void node_log_error(HorusNodeContext* ctx, const char* msg);

// NOTE: Message type definitions have been moved to horus_library/cpp/include/horus/messages/
// C++ code should include horus.hpp which provides the full message library
// This C header only contains FFI function declarations

#ifdef __cplusplus
}
#endif

#endif // HORUS_H