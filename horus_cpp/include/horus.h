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
typedef uint32_t Pub;
typedef uint32_t Sub;
typedef uint32_t Node;
typedef uint32_t Scheduler;

// Forward declaration of opaque context
typedef struct NodeContext NodeContext;

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
typedef bool (*NodeInitCallback)(NodeContext* ctx, void* user_data);
typedef void (*NodeTickCallback)(NodeContext* ctx, void* user_data);
typedef void (*NodeShutdownCallback)(NodeContext* ctx, void* user_data);

// ============================================================================
// Internal FFI Functions
// ============================================================================

// System lifecycle
bool init(const char* node_name);
void shutdown(void);
bool ok(void);

// Publisher/Subscriber creation (only for standard message types)
Pub publisher(const char* topic, MessageType type);
Sub subscriber(const char* topic, MessageType type);

// Message send/receive
bool send(Pub pub, const void* data);
bool recv(Sub sub, void* data);

// Context-aware messaging (with logging)
bool node_send(NodeContext* ctx, Pub pub, const void* data);
bool node_recv(NodeContext* ctx, Sub sub, void* data);

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
Node node_create(const char* name,
                 NodeInitCallback init_fn,
                 NodeTickCallback tick_fn,
                 NodeShutdownCallback shutdown_fn,
                 void* user_data);
void node_destroy(Node node);

// Scheduler management
Scheduler scheduler_create(const char* name);
bool scheduler_add(Scheduler sched, Node node, uint32_t priority, bool enable_logging);
void scheduler_run(Scheduler sched);
void scheduler_tick(Scheduler sched, const char** node_names, size_t count);
void scheduler_stop(Scheduler sched);
void scheduler_destroy(Scheduler sched);

// Context API
Pub node_create_publisher(NodeContext* ctx, const char* topic, MessageType type);
Sub node_create_subscriber(NodeContext* ctx, const char* topic, MessageType type);
void node_log_info(NodeContext* ctx, const char* msg);
void node_log_warn(NodeContext* ctx, const char* msg);
void node_log_error(NodeContext* ctx, const char* msg);

// Common message structs
typedef struct {
    float x, y, z;
} Vector3;

typedef struct {
    float x, y, z, w;
} Quaternion;

typedef struct {
    Vector3 linear;
    Vector3 angular;
} Twist;

typedef struct {
    Vector3 position;
    Quaternion orientation;
} Pose;

typedef struct {
    Vector3 linear_acceleration;
    Vector3 angular_velocity;
    Quaternion orientation;
    float covariance[9];
} IMU;

typedef struct {
    float* ranges;
    float* intensities;
    uint32_t count;
    float angle_min;
    float angle_max;
    float angle_increment;
    float range_min;
    float range_max;
    float scan_time;
} LaserScan;

typedef struct {
    uint8_t* data;
    uint32_t width;
    uint32_t height;
    uint32_t step;
    uint8_t channels;
} Image;

typedef struct {
    float* positions;
    float* velocities;
    float* efforts;
    char** names;
    uint32_t count;
} JointState;

typedef struct {
    float* points;  // x,y,z packed array
    uint32_t count;
    uint32_t stride;  // bytes between points
} PointCloud;

#ifdef __cplusplus
}
#endif

#endif // HORUS_H