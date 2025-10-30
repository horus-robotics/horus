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

// Send/Receive
bool send(Pub pub, const void* data);
bool recv(Sub sub, void* data);
bool try_recv(Sub sub, void* data);

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