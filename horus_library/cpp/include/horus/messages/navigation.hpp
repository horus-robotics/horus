// HORUS Message Library - Navigation Types
// Binary-compatible with Rust definitions in horus_library/messages/navigation.rs

#ifndef HORUS_MESSAGES_NAVIGATION_HPP
#define HORUS_MESSAGES_NAVIGATION_HPP

#include "geometry.hpp"
#include <cstdint>
#include <cstring>
#include <chrono>
#include <algorithm>
#include <cmath>

namespace horus {
namespace messages {

// ============================================================================
// Goal - Navigation goal specification
// ============================================================================
struct Goal {
    Pose2D target_pose;         // Target pose to reach
    double tolerance_position;  // Position tolerance in meters
    double tolerance_angle;     // Orientation tolerance in radians
    double timeout_seconds;     // Maximum time to reach goal (0 = no limit)
    uint8_t priority;           // Goal priority (0 = highest)
    uint8_t _padding[3];        // Padding
    uint32_t goal_id;           // Unique goal identifier
    uint64_t timestamp;         // nanoseconds since epoch

    Goal() : tolerance_position(0.1), tolerance_angle(0.1), timeout_seconds(0.0), priority(1), goal_id(0) {
        target_pose = Pose2D();
        _padding[0] = _padding[1] = _padding[2] = 0;
        update_timestamp();
    }

    Goal(const Pose2D& target, double pos_tol, double angle_tol)
        : target_pose(target), tolerance_position(pos_tol), tolerance_angle(angle_tol),
          timeout_seconds(0.0), priority(1), goal_id(0) {
        _padding[0] = _padding[1] = _padding[2] = 0;
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Check if position is within tolerance
    bool is_position_reached(const Pose2D& current_pose) const {
        return target_pose.distance_to(current_pose) <= tolerance_position;
    }

    // Check if orientation is within tolerance
    bool is_orientation_reached(const Pose2D& current_pose) const {
        double angle_diff = std::abs(target_pose.theta - current_pose.theta);
        double normalized_diff = (angle_diff > M_PI) ? (2.0 * M_PI - angle_diff) : angle_diff;
        return normalized_diff <= tolerance_angle;
    }

    // Check if goal is fully reached
    bool is_reached(const Pose2D& current_pose) const {
        return is_position_reached(current_pose) && is_orientation_reached(current_pose);
    }
};

// ============================================================================
// GoalStatus - Status enumeration for navigation goals
// ============================================================================
enum class GoalStatus : uint8_t {
    Pending = 0,    // Goal is pending execution
    Active = 1,     // Goal is actively being pursued
    Succeeded = 2,  // Goal was successfully reached
    Aborted = 3,    // Goal was aborted due to error
    Cancelled = 4,  // Goal was cancelled by user
    Preempted = 5,  // Goal was preempted by higher priority goal
    TimedOut = 6,   // Goal timed out
};

// ============================================================================
// GoalResult - Goal status feedback
// ============================================================================
struct GoalResult {
    uint32_t goal_id;           // Goal identifier
    GoalStatus status;          // Current status
    uint8_t _padding[3];        // Padding
    double distance_to_goal;    // Distance to goal in meters
    double eta_seconds;         // Estimated time to reach goal
    float progress;             // Progress percentage (0.0 to 1.0)
    uint8_t _padding2[4];       // Padding
    char error_message[64];     // Error message if failed
    uint64_t timestamp;         // nanoseconds since epoch

    GoalResult() : goal_id(0), status(GoalStatus::Pending), distance_to_goal(0.0),
                   eta_seconds(0.0), progress(0.0f) {
        _padding[0] = _padding[1] = _padding[2] = 0;
        _padding2[0] = _padding2[1] = _padding2[2] = _padding2[3] = 0;
        std::memset(error_message, 0, sizeof(error_message));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    void set_error(const char* msg) {
        std::memset(error_message, 0, sizeof(error_message));
        std::strncpy(error_message, msg, sizeof(error_message) - 1);
    }
};

// ============================================================================
// Waypoint - Single waypoint in a path
// ============================================================================
struct Waypoint {
    Pose2D pose;                // Pose at this waypoint
    Twist velocity;             // Desired velocity at this point
    double time_from_start;     // Time to reach this waypoint from start
    float curvature;            // Curvature at this point (1/radius)
    bool stop_required;         // Whether to stop at this waypoint
    uint8_t _padding[3];        // Padding

    Waypoint() : time_from_start(0.0), curvature(0.0f), stop_required(false) {
        pose = Pose2D();
        velocity = Twist();
        _padding[0] = _padding[1] = _padding[2] = 0;
    }

    explicit Waypoint(const Pose2D& p) : pose(p), time_from_start(0.0), curvature(0.0f), stop_required(false) {
        velocity = Twist();
        _padding[0] = _padding[1] = _padding[2] = 0;
    }
};

// ============================================================================
// Path - Navigation path message
// ============================================================================
struct Path {
    Waypoint waypoints[256];    // Array of waypoints (max 256)
    uint16_t waypoint_count;    // Number of valid waypoints
    uint8_t _padding[6];        // Padding
    double total_length;        // Total path length in meters
    double duration_seconds;    // Estimated time to complete path
    char frame_id[32];          // Path coordinate frame
    char algorithm[32];         // Path generation algorithm used
    uint64_t timestamp;         // nanoseconds since epoch

    Path() {
        for (int i = 0; i < 256; i++) {
            waypoints[i] = Waypoint();
        }
        waypoint_count = 0;
        for (int i = 0; i < 6; i++) _padding[i] = 0;
        total_length = 0.0;
        duration_seconds = 0.0;
        std::memset(frame_id, 0, sizeof(frame_id));
        std::memset(algorithm, 0, sizeof(algorithm));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Add a waypoint to the path
    bool add_waypoint(const Waypoint& wp) {
        if (waypoint_count >= 256) return false;

        waypoints[waypoint_count] = wp;
        waypoint_count++;

        // Update total length
        if (waypoint_count > 1) {
            const Waypoint& prev = waypoints[waypoint_count - 2];
            const Waypoint& current = waypoints[waypoint_count - 1];
            total_length += prev.pose.distance_to(current.pose);
        }

        return true;
    }

    // Get valid waypoints
    const Waypoint* get_waypoints() const {
        return waypoints;
    }

    size_t get_count() const {
        return waypoint_count;
    }

    // Find closest waypoint to current position
    int closest_waypoint_index(const Pose2D& current_pose) const {
        if (waypoint_count == 0) return -1;

        double min_distance = INFINITY;
        int closest_index = 0;

        for (uint16_t i = 0; i < waypoint_count; i++) {
            double distance = current_pose.distance_to(waypoints[i].pose);
            if (distance < min_distance) {
                min_distance = distance;
                closest_index = i;
            }
        }

        return closest_index;
    }

    // Calculate progress along path (0.0 to 1.0)
    float calculate_progress(const Pose2D& current_pose) const {
        int index = closest_waypoint_index(current_pose);
        if (index < 0 || waypoint_count == 0) return 0.0f;
        return static_cast<float>(index) / static_cast<float>(waypoint_count);
    }

    void set_frame_id(const char* frame) {
        std::memset(frame_id, 0, sizeof(frame_id));
        std::strncpy(frame_id, frame, sizeof(frame_id) - 1);
    }

    void set_algorithm(const char* algo) {
        std::memset(algorithm, 0, sizeof(algorithm));
        std::strncpy(algorithm, algo, sizeof(algorithm) - 1);
    }

    void clear() {
        waypoint_count = 0;
        total_length = 0.0;
        duration_seconds = 0.0;
    }
};

// ============================================================================
// OccupancyGrid - 2D occupancy grid map
// Max size: 2000x2000 = 4MB
// ============================================================================
struct OccupancyGrid {
    static constexpr size_t MAX_CELLS = 2000 * 2000; // 4M cells

    float resolution;           // Map resolution (meters per pixel)
    uint32_t width;             // Map width in pixels
    uint32_t height;            // Map height in pixels
    Pose2D origin;              // Map origin pose (bottom-left corner)
    uint32_t data_length;       // Actual data length
    int8_t data[MAX_CELLS];     // Map data (-1=unknown, 0=free, 100=occupied)
    char frame_id[32];          // Frame ID for map coordinates
    char metadata[64];          // Map metadata
    uint64_t timestamp;         // Timestamp when map was created

    OccupancyGrid() {
        resolution = 0.05f; // 5cm default
        width = 0;
        height = 0;
        origin = Pose2D();
        data_length = 0;
        std::memset(data, -1, sizeof(data)); // Initialize as unknown
        std::memset(frame_id, 0, sizeof(frame_id));
        std::memset(metadata, 0, sizeof(metadata));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Initialize map with dimensions
    bool init(uint32_t w, uint32_t h, float res, const Pose2D& orig) {
        if (w * h > MAX_CELLS) return false;
        width = w;
        height = h;
        resolution = res;
        origin = orig;
        data_length = w * h;
        std::memset(data, -1, data_length); // Initialize as unknown
        update_timestamp();
        return true;
    }

    // Convert world coordinates to grid indices
    bool world_to_grid(double x, double y, uint32_t& grid_x, uint32_t& grid_y) const {
        const double EPSILON = 1e-6;
        int32_t gx = static_cast<int32_t>(std::floor((x - origin.x) / resolution + EPSILON));
        int32_t gy = static_cast<int32_t>(std::floor((y - origin.y) / resolution + EPSILON));

        if (gx >= 0 && gx < static_cast<int32_t>(width) &&
            gy >= 0 && gy < static_cast<int32_t>(height)) {
            grid_x = static_cast<uint32_t>(gx);
            grid_y = static_cast<uint32_t>(gy);
            return true;
        }
        return false;
    }

    // Convert grid indices to world coordinates (center of cell)
    bool grid_to_world(uint32_t grid_x, uint32_t grid_y, double& x, double& y) const {
        if (grid_x < width && grid_y < height) {
            x = origin.x + (grid_x + 0.5) * resolution;
            y = origin.y + (grid_y + 0.5) * resolution;
            return true;
        }
        return false;
    }

    // Get occupancy value at grid coordinates
    int8_t get_occupancy(uint32_t grid_x, uint32_t grid_y) const {
        if (grid_x < width && grid_y < height) {
            size_t index = grid_y * width + grid_x;
            if (index < data_length) {
                return data[index];
            }
        }
        return -1; // Unknown
    }

    // Set occupancy value at grid coordinates
    bool set_occupancy(uint32_t grid_x, uint32_t grid_y, int8_t value) {
        if (grid_x < width && grid_y < height) {
            size_t index = grid_y * width + grid_x;
            if (index < data_length) {
                data[index] = std::clamp(value, static_cast<int8_t>(-1), static_cast<int8_t>(100));
                return true;
            }
        }
        return false;
    }

    // Check if a point is free (< 50% occupancy)
    bool is_free(double x, double y) const {
        uint32_t gx, gy;
        if (world_to_grid(x, y, gx, gy)) {
            int8_t occ = get_occupancy(gx, gy);
            return occ >= 0 && occ < 50;
        }
        return false;
    }

    // Check if a point is occupied (>= 50% occupancy)
    bool is_occupied(double x, double y) const {
        uint32_t gx, gy;
        if (world_to_grid(x, y, gx, gy)) {
            return get_occupancy(gx, gy) >= 50;
        }
        return false;
    }

    void set_frame_id(const char* frame) {
        std::memset(frame_id, 0, sizeof(frame_id));
        std::strncpy(frame_id, frame, sizeof(frame_id) - 1);
    }

    void set_metadata(const char* meta) {
        std::memset(metadata, 0, sizeof(metadata));
        std::strncpy(metadata, meta, sizeof(metadata) - 1);
    }
};

// ============================================================================
// CostMap - Cost map for navigation planning
// ============================================================================
struct CostMap {
    OccupancyGrid occupancy_grid; // Base occupancy grid
    uint32_t costs_length;        // Actual costs data length
    uint8_t costs[OccupancyGrid::MAX_CELLS]; // Cost values (0-255, 255=lethal)
    float inflation_radius;       // Inflation radius in meters
    float cost_scaling_factor;    // Cost scaling factor
    uint8_t lethal_cost;          // Lethal cost threshold
    uint8_t _padding[3];          // Padding

    CostMap() {
        occupancy_grid = OccupancyGrid();
        costs_length = 0;
        std::memset(costs, 0, sizeof(costs));
        inflation_radius = 0.55f;
        cost_scaling_factor = 10.0f;
        lethal_cost = 253;
        _padding[0] = _padding[1] = _padding[2] = 0;
    }

    // Create costmap from occupancy grid
    void from_occupancy_grid(const OccupancyGrid& grid, float inflation_rad) {
        occupancy_grid = grid;
        inflation_radius = inflation_rad;
        compute_costs();
    }

    // Compute cost values from occupancy data
    void compute_costs() {
        costs_length = occupancy_grid.data_length;

        // Convert occupancy to basic costs
        for (size_t i = 0; i < costs_length; i++) {
            int8_t occupancy = occupancy_grid.data[i];
            if (occupancy == -1) {
                costs[i] = 255; // Unknown = lethal
            } else if (occupancy >= 65) {
                costs[i] = lethal_cost; // Occupied = lethal
            } else {
                costs[i] = static_cast<uint8_t>(std::max(0, occupancy * 2));
            }
        }
    }

    // Get cost at world coordinates
    uint8_t get_cost(double x, double y) const {
        uint32_t gx, gy;
        if (occupancy_grid.world_to_grid(x, y, gx, gy)) {
            size_t index = gy * occupancy_grid.width + gx;
            if (index < costs_length) {
                return costs[index];
            }
        }
        return lethal_cost; // Outside map = lethal
    }
};

// ============================================================================
// VelocityObstacle - Dynamic obstacle for collision avoidance
// ============================================================================
struct VelocityObstacle {
    double position[2];         // Obstacle position
    double velocity[2];         // Obstacle velocity
    float radius;               // Obstacle radius
    float time_horizon;         // Time horizon for collision prediction
    uint32_t obstacle_id;       // Obstacle ID for tracking

    VelocityObstacle() {
        position[0] = position[1] = 0.0;
        velocity[0] = velocity[1] = 0.0;
        radius = 0.5f;
        time_horizon = 2.0f;
        obstacle_id = 0;
    }
};

// ============================================================================
// VelocityObstacles - Array of velocity obstacles
// ============================================================================
struct VelocityObstacles {
    VelocityObstacle obstacles[32]; // Array of obstacles (max 32)
    uint8_t count;                  // Number of valid obstacles
    uint8_t _padding[7];            // Padding
    uint64_t timestamp;             // nanoseconds since epoch

    VelocityObstacles() {
        for (int i = 0; i < 32; i++) {
            obstacles[i] = VelocityObstacle();
        }
        count = 0;
        for (int i = 0; i < 7; i++) _padding[i] = 0;
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    bool add_obstacle(const VelocityObstacle& obstacle) {
        if (count >= 32) return false;
        obstacles[count++] = obstacle;
        return true;
    }

    void clear() {
        count = 0;
    }
};

// ============================================================================
// PathPlan - Simplified path plan message
// ============================================================================
struct PathPlan {
    static constexpr size_t MAX_WAYPOINTS = 256;

    float waypoints[MAX_WAYPOINTS][3]; // Array of waypoints as [x, y, theta]
    float goal_pose[3];                // Goal pose [x, y, theta]
    uint32_t path_length;              // Number of waypoints in path
    uint64_t timestamp;                // nanoseconds since epoch

    PathPlan() {
        std::memset(waypoints, 0, sizeof(waypoints));
        goal_pose[0] = goal_pose[1] = goal_pose[2] = 0.0f;
        path_length = 0;
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Add waypoint to path
    bool add_waypoint(float x, float y, float theta) {
        if (path_length >= MAX_WAYPOINTS) return false;
        waypoints[path_length][0] = x;
        waypoints[path_length][1] = y;
        waypoints[path_length][2] = theta;
        path_length++;
        return true;
    }

    // Get waypoint at index
    const float* get_waypoint(size_t index) const {
        if (index < path_length) {
            return waypoints[index];
        }
        return nullptr;
    }

    bool is_empty() const {
        return path_length == 0;
    }

    void clear() {
        path_length = 0;
    }
};

} // namespace messages
} // namespace horus

#endif // HORUS_MESSAGES_NAVIGATION_HPP
