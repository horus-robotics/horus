// HORUS C API - Hardware driver integration interface
#ifndef HORUS_H
#define HORUS_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle types - users never see internals
typedef uint32_t Node;
typedef uint32_t Pub;
typedef uint32_t Sub;
typedef uint32_t Scheduler;

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

// Core API - Simple and safe
bool init(const char* node_name);
void shutdown(void);
bool ok(void);

// Publisher/Subscriber
Pub publisher(const char* topic, MessageType type);
Pub publisher_custom(const char* topic, size_t msg_size);
Sub subscriber(const char* topic, MessageType type);
Sub subscriber_custom(const char* topic, size_t msg_size);

// Forward declarations
typedef struct HorusNodeContext HorusNodeContext;

// Send/Receive
bool send(Pub pub, const void* data);
bool recv(Sub sub, void* data);
bool try_recv(Sub sub, void* data);

// Context-aware pub/sub (with logging support)
bool node_send(HorusNodeContext* ctx, Pub pub, const void* data);
bool node_recv(HorusNodeContext* ctx, Sub sub, void* data);
bool node_try_recv(HorusNodeContext* ctx, Sub sub, void* data);

// Timing
void sleep_ms(uint32_t ms);
uint64_t time_now_ms(void);
void spin_once(void);
void spin(void);

// Logging
void log_info(const char* msg);
void log_warn(const char* msg);
void log_error(const char* msg);
void log_debug(const char* msg);

// ============================================================================
// Framework API - Node/Scheduler integration
// ============================================================================

// Priority levels (matches Rust NodePriority)
typedef enum {
    PRIORITY_CRITICAL = 0,
    PRIORITY_HIGH = 1,
    PRIORITY_NORMAL = 2,
    PRIORITY_LOW = 3,
    PRIORITY_BACKGROUND = 4,
} Priority;

// Node lifecycle callbacks
typedef bool (*NodeInitCallback)(HorusNodeContext* ctx, void* user_data);
typedef void (*NodeTickCallback)(HorusNodeContext* ctx, void* user_data);
typedef void (*NodeShutdownCallback)(HorusNodeContext* ctx, void* user_data);

// Node handle type
typedef uint32_t HorusNode;
typedef uint32_t HorusScheduler;

// Create a node with callbacks
HorusNode node_create(const char* name,
                      NodeInitCallback init_fn,
                      NodeTickCallback tick_fn,
                      NodeShutdownCallback shutdown_fn,
                      void* user_data);

// Destroy a node
void node_destroy(HorusNode node);

// Create a scheduler
HorusScheduler scheduler_create(const char* name);

// Register node with scheduler
bool scheduler_register(HorusScheduler sched, HorusNode node, Priority priority);

// Run scheduler (blocks until shutdown)
void scheduler_run(HorusScheduler sched);

// Stop scheduler
void scheduler_stop(HorusScheduler sched);

// Destroy scheduler
void scheduler_destroy(HorusScheduler sched);

// Context API - for use in callbacks
Pub node_create_publisher(HorusNodeContext* ctx, const char* topic, MessageType type);
Sub node_create_subscriber(HorusNodeContext* ctx, const char* topic, MessageType type);
void node_log_info(HorusNodeContext* ctx, const char* msg);
void node_log_warn(HorusNodeContext* ctx, const char* msg);
void node_log_error(HorusNodeContext* ctx, const char* msg);

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