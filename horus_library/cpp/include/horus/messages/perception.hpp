// HORUS Message Library - Perception Types
// Binary-compatible with Rust definitions in horus_library/messages/perception.rs

#ifndef HORUS_MESSAGES_PERCEPTION_HPP
#define HORUS_MESSAGES_PERCEPTION_HPP

#include "geometry.hpp"
#include <cstdint>
#include <cstring>
#include <chrono>
#include <algorithm>
#include <cmath>

namespace horus {
namespace messages {

// ============================================================================
// PointFieldType - Type enumeration for point cloud fields
// Binary-compatible with Rust enum (repr(u8))
// ============================================================================
enum class PointFieldType : uint8_t {
    Int8 = 1,      // 8-bit integer
    UInt8 = 2,     // 8-bit unsigned integer
    Int16 = 3,     // 16-bit integer
    UInt16 = 4,    // 16-bit unsigned integer
    Int32 = 5,     // 32-bit integer
    UInt32 = 6,    // 32-bit unsigned integer
    Float32 = 7,   // 32-bit float
    Float64 = 8,   // 64-bit float
};

// Helper function to get field type size
inline uint32_t field_type_size(PointFieldType type) {
    switch (type) {
        case PointFieldType::Int8:
        case PointFieldType::UInt8:
            return 1;
        case PointFieldType::Int16:
        case PointFieldType::UInt16:
            return 2;
        case PointFieldType::Int32:
        case PointFieldType::UInt32:
        case PointFieldType::Float32:
            return 4;
        case PointFieldType::Float64:
            return 8;
        default:
            return 0;
    }
}

// ============================================================================
// PointField - Field descriptor for point cloud data
// ============================================================================
struct PointField {
    char name[16];          // Field name ("x", "y", "z", "rgb", "intensity", etc.)
    uint32_t offset;        // Byte offset in point data structure
    PointFieldType datatype;// Data type of this field
    uint8_t _padding[3];    // Padding for alignment
    uint32_t count;         // Number of elements (1 for scalar, >1 for vector/array)

    PointField() {
        std::memset(name, 0, sizeof(name));
        offset = 0;
        datatype = PointFieldType::Float32;
        _padding[0] = _padding[1] = _padding[2] = 0;
        count = 1;
    }

    PointField(const char* field_name, uint32_t off, PointFieldType dtype, uint32_t cnt = 1)
        : offset(off), datatype(dtype), count(cnt) {
        std::memset(name, 0, sizeof(name));
        std::strncpy(name, field_name, sizeof(name) - 1);
        _padding[0] = _padding[1] = _padding[2] = 0;
    }

    // Get field size in bytes
    uint32_t field_size() const {
        return field_type_size(datatype) * count;
    }
};

// ============================================================================
// PointCloud - 3D point cloud data
// Max size: 2MB for point data
// ============================================================================
struct PointCloud {
    static constexpr size_t MAX_DATA_SIZE = 2 * 1024 * 1024; // 2MB

    uint32_t width;              // Point cloud width (for organized clouds)
    uint32_t height;             // Point cloud height (1 for unorganized)
    PointField fields[16];       // Field descriptions
    uint8_t field_count;         // Number of valid fields
    bool is_dense;               // Is data organized or unorganized
    uint8_t _padding[2];         // Padding
    uint32_t point_step;         // Size of each point in bytes
    uint32_t row_step;           // Size of each row in bytes
    uint32_t data_length;        // Actual data length
    uint8_t data[MAX_DATA_SIZE]; // Point data (binary blob)
    char frame_id[32];           // Coordinate frame reference
    uint64_t timestamp;          // nanoseconds since epoch

    PointCloud() {
        width = 0;
        height = 0;
        for (int i = 0; i < 16; i++) {
            fields[i] = PointField();
        }
        field_count = 0;
        is_dense = true;
        _padding[0] = _padding[1] = 0;
        point_step = 0;
        row_step = 0;
        data_length = 0;
        std::memset(data, 0, MAX_DATA_SIZE);
        std::memset(frame_id, 0, sizeof(frame_id));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Create basic XYZ point cloud
    static PointCloud create_xyz(const Point3* points, size_t count) {
        PointCloud cloud;
        cloud.width = static_cast<uint32_t>(count);
        cloud.height = 1;
        cloud.is_dense = true;

        // Define XYZ fields
        cloud.fields[0] = PointField("x", 0, PointFieldType::Float32, 1);
        cloud.fields[1] = PointField("y", 4, PointFieldType::Float32, 1);
        cloud.fields[2] = PointField("z", 8, PointFieldType::Float32, 1);
        cloud.field_count = 3;

        cloud.point_step = 12; // 3 * 4 bytes
        cloud.row_step = cloud.point_step * cloud.width;

        // Copy point data
        cloud.data_length = cloud.point_step * cloud.width;
        if (cloud.data_length <= MAX_DATA_SIZE) {
            uint8_t* ptr = cloud.data;
            for (size_t i = 0; i < count; i++) {
                float x = static_cast<float>(points[i].x);
                float y = static_cast<float>(points[i].y);
                float z = static_cast<float>(points[i].z);
                std::memcpy(ptr, &x, 4); ptr += 4;
                std::memcpy(ptr, &y, 4); ptr += 4;
                std::memcpy(ptr, &z, 4); ptr += 4;
            }
        }

        cloud.update_timestamp();
        return cloud;
    }

    // Add a field descriptor
    bool add_field(const PointField& field) {
        if (field_count >= 16) return false;
        fields[field_count++] = field;
        return true;
    }

    // Get total number of points
    uint32_t point_count() const {
        return width * height;
    }

    // Validate point cloud
    bool is_valid() const {
        return width > 0 && height > 0 && field_count > 0 &&
               point_step > 0 && data_length >= (point_step * point_count()) &&
               data_length <= MAX_DATA_SIZE;
    }

    void set_frame_id(const char* frame) {
        std::memset(frame_id, 0, sizeof(frame_id));
        std::strncpy(frame_id, frame, sizeof(frame_id) - 1);
    }

    // Extract XYZ points (if XYZ fields are present)
    bool extract_xyz(Point3* output, size_t max_output, size_t& extracted_count) const {
        // Find X, Y, Z fields
        const PointField* x_field = nullptr;
        const PointField* y_field = nullptr;
        const PointField* z_field = nullptr;

        for (uint8_t i = 0; i < field_count; i++) {
            if (std::strcmp(fields[i].name, "x") == 0) x_field = &fields[i];
            else if (std::strcmp(fields[i].name, "y") == 0) y_field = &fields[i];
            else if (std::strcmp(fields[i].name, "z") == 0) z_field = &fields[i];
        }

        if (!x_field || !y_field || !z_field) return false;
        if (x_field->datatype != PointFieldType::Float32) return false;

        extracted_count = 0;
        uint32_t pcount = std::min(point_count(), static_cast<uint32_t>(max_output));

        for (uint32_t i = 0; i < pcount; i++) {
            size_t point_offset = i * point_step;
            if (point_offset + 12 > data_length) break;

            float x, y, z;
            std::memcpy(&x, &data[point_offset + x_field->offset], 4);
            std::memcpy(&y, &data[point_offset + y_field->offset], 4);
            std::memcpy(&z, &data[point_offset + z_field->offset], 4);

            output[extracted_count++] = Point3(x, y, z);
        }

        return extracted_count > 0;
    }
};

// ============================================================================
// BoundingBox3D - 3D bounding box for object detection
// ============================================================================
struct BoundingBox3D {
    Point3 center;              // Center of the bounding box
    Vector3 size;               // Size [width, height, depth]
    Quaternion orientation;     // Orientation of the box
    char label[32];             // Object class label
    float confidence;           // Detection confidence (0.0 to 1.0)
    uint32_t track_id;          // Tracking ID
    uint64_t timestamp;         // nanoseconds since epoch

    BoundingBox3D() : center(), size(), orientation(), confidence(1.0f), track_id(0) {
        std::memset(label, 0, sizeof(label));
        update_timestamp();
    }

    BoundingBox3D(const Point3& ctr, const Vector3& sz)
        : center(ctr), size(sz), orientation(Quaternion::identity()), confidence(1.0f), track_id(0) {
        std::memset(label, 0, sizeof(label));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    void set_label(const char* lbl) {
        std::memset(label, 0, sizeof(label));
        std::strncpy(label, lbl, sizeof(label) - 1);
    }

    // Check if point is inside bounding box (axis-aligned approximation)
    bool contains_point(const Point3& point) const {
        double dx = std::abs(point.x - center.x);
        double dy = std::abs(point.y - center.y);
        double dz = std::abs(point.z - center.z);

        return dx <= size.x / 2.0 && dy <= size.y / 2.0 && dz <= size.z / 2.0;
    }

    // Get volume
    double volume() const {
        return size.x * size.y * size.z;
    }

    // Get 8 corner points (axis-aligned)
    void corners(Point3 output[8]) const {
        double hx = size.x / 2.0;
        double hy = size.y / 2.0;
        double hz = size.z / 2.0;

        output[0] = Point3(center.x - hx, center.y - hy, center.z - hz);
        output[1] = Point3(center.x + hx, center.y - hy, center.z - hz);
        output[2] = Point3(center.x - hx, center.y + hy, center.z - hz);
        output[3] = Point3(center.x + hx, center.y + hy, center.z - hz);
        output[4] = Point3(center.x - hx, center.y - hy, center.z + hz);
        output[5] = Point3(center.x + hx, center.y - hy, center.z + hz);
        output[6] = Point3(center.x - hx, center.y + hy, center.z + hz);
        output[7] = Point3(center.x + hx, center.y + hy, center.z + hz);
    }
};

// ============================================================================
// BoundingBoxArray3D - Multiple 3D bounding boxes
// ============================================================================
struct BoundingBoxArray3D {
    BoundingBox3D boxes[32];    // Array of bounding boxes (max 32)
    uint8_t count;              // Number of valid boxes
    uint8_t _padding[3];        // Padding
    char frame_id[32];          // Source sensor frame
    uint64_t timestamp;         // nanoseconds since epoch

    BoundingBoxArray3D() {
        for (int i = 0; i < 32; i++) {
            boxes[i] = BoundingBox3D();
        }
        count = 0;
        _padding[0] = _padding[1] = _padding[2] = 0;
        std::memset(frame_id, 0, sizeof(frame_id));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Add a bounding box
    bool add_box(const BoundingBox3D& bbox) {
        if (count >= 32) return false;
        boxes[count++] = bbox;
        return true;
    }

    // Get valid bounding boxes
    const BoundingBox3D* get_boxes() const {
        return boxes;
    }

    size_t get_count() const {
        return count;
    }

    // Filter by confidence threshold
    size_t filter_by_confidence(float threshold, BoundingBox3D* output, size_t max_output) const {
        size_t matched = 0;
        for (size_t i = 0; i < count && matched < max_output; i++) {
            if (boxes[i].confidence >= threshold) {
                output[matched++] = boxes[i];
            }
        }
        return matched;
    }

    // Filter by label
    size_t filter_by_label(const char* lbl, BoundingBox3D* output, size_t max_output) const {
        size_t matched = 0;
        for (size_t i = 0; i < count && matched < max_output; i++) {
            if (std::strcmp(boxes[i].label, lbl) == 0) {
                output[matched++] = boxes[i];
            }
        }
        return matched;
    }

    void set_frame_id(const char* frame) {
        std::memset(frame_id, 0, sizeof(frame_id));
        std::strncpy(frame_id, frame, sizeof(frame_id) - 1);
    }

    void clear() {
        count = 0;
    }
};

// ============================================================================
// DepthImage - Depth image from depth camera
// Max size: 1280x960 = 1,228,800 pixels (2.4MB with uint16)
// ============================================================================
struct DepthImage {
    static constexpr size_t MAX_PIXELS = 1280 * 960; // 1.2M pixels

    uint32_t width;              // Image width in pixels
    uint32_t height;             // Image height in pixels
    uint16_t depths[MAX_PIXELS]; // Depth values in millimeters (0 = invalid)
    uint16_t min_depth;          // Minimum reliable depth value
    uint16_t max_depth;          // Maximum reliable depth value
    float depth_scale;           // Depth scale (mm per unit)
    char frame_id[32];           // Frame ID for camera reference
    uint64_t timestamp;          // nanoseconds since epoch

    DepthImage() {
        width = 0;
        height = 0;
        std::memset(depths, 0, sizeof(depths));
        min_depth = 200;   // 20cm minimum
        max_depth = 10000; // 10m maximum
        depth_scale = 1.0f; // 1mm per unit
        std::memset(frame_id, 0, sizeof(frame_id));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Create depth image with data
    bool set_data(uint32_t w, uint32_t h, const uint16_t* depth_data) {
        if (w * h > MAX_PIXELS) return false;
        width = w;
        height = h;
        std::memcpy(depths, depth_data, w * h * sizeof(uint16_t));
        update_timestamp();
        return true;
    }

    // Get depth at pixel coordinates
    uint16_t get_depth(uint32_t x, uint32_t y) const {
        if (x < width && y < height) {
            return depths[y * width + x];
        }
        return 0;
    }

    // Set depth at pixel coordinates
    bool set_depth(uint32_t x, uint32_t y, uint16_t depth) {
        if (x < width && y < height) {
            depths[y * width + x] = depth;
            return true;
        }
        return false;
    }

    // Check if depth value is valid
    bool is_valid_depth(uint16_t depth) const {
        return depth > 0 && depth >= min_depth && depth <= max_depth;
    }

    void set_frame_id(const char* frame) {
        std::memset(frame_id, 0, sizeof(frame_id));
        std::strncpy(frame_id, frame, sizeof(frame_id) - 1);
    }

    // Calculate depth statistics (min, max, mean)
    void depth_statistics(float& out_min, float& out_max, float& out_mean) const {
        uint16_t min_val = 65535;
        uint16_t max_val = 0;
        uint64_t sum = 0;
        size_t valid_count = 0;

        size_t pixel_count = width * height;
        for (size_t i = 0; i < pixel_count; i++) {
            if (is_valid_depth(depths[i])) {
                if (depths[i] < min_val) min_val = depths[i];
                if (depths[i] > max_val) max_val = depths[i];
                sum += depths[i];
                valid_count++;
            }
        }

        if (valid_count > 0) {
            out_min = static_cast<float>(min_val);
            out_max = static_cast<float>(max_val);
            out_mean = static_cast<float>(sum) / valid_count;
        } else {
            out_min = out_max = out_mean = 0.0f;
        }
    }

    // Convert to point cloud using camera intrinsics
    PointCloud to_point_cloud(double fx, double fy, double cx, double cy) const {
        // Allocate temporary buffer for points (max 10K points to fit in 2MB point cloud)
        static constexpr size_t MAX_POINTS = 10000;
        Point3 temp_points[MAX_POINTS];
        size_t point_idx = 0;

        for (uint32_t y = 0; y < height && point_idx < MAX_POINTS; y++) {
            for (uint32_t x = 0; x < width && point_idx < MAX_POINTS; x++) {
                uint16_t depth = get_depth(x, y);
                if (is_valid_depth(depth)) {
                    double depth_m = (depth * depth_scale) / 1000.0; // Convert to meters

                    // Back-project to 3D
                    double x_3d = (x - cx) * depth_m / fx;
                    double y_3d = (y - cy) * depth_m / fy;
                    double z_3d = depth_m;

                    temp_points[point_idx++] = Point3(x_3d, y_3d, z_3d);
                }
            }
        }

        return PointCloud::create_xyz(temp_points, point_idx);
    }
};

// ============================================================================
// PlaneDetection - Planar surface detection result
// ============================================================================
struct PlaneDetection {
    double coefficients[4];     // Plane equation [a, b, c, d] where ax + by + cz + d = 0
    Point3 center;              // Center point of the plane
    Vector3 normal;             // Normal vector of the plane
    double size[2];             // Plane size (width, height) if bounded
    uint32_t inlier_count;      // Number of inlier points
    float confidence;           // Confidence in detection (0.0 to 1.0)
    char plane_type[16];        // Plane type label ("floor", "wall", "table", etc.)
    uint64_t timestamp;         // nanoseconds since epoch

    PlaneDetection() : center(), normal(), inlier_count(0), confidence(0.5f) {
        coefficients[0] = coefficients[1] = coefficients[2] = coefficients[3] = 0.0;
        size[0] = size[1] = 0.0;
        std::memset(plane_type, 0, sizeof(plane_type));
        update_timestamp();
    }

    PlaneDetection(const double coeff[4], const Point3& ctr, const Vector3& norm)
        : center(ctr), normal(norm), inlier_count(0), confidence(0.5f) {
        coefficients[0] = coeff[0];
        coefficients[1] = coeff[1];
        coefficients[2] = coeff[2];
        coefficients[3] = coeff[3];
        size[0] = size[1] = 0.0;
        std::memset(plane_type, 0, sizeof(plane_type));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Calculate distance from point to plane
    double distance_to_point(const Point3& point) const {
        double numerator = std::abs(coefficients[0] * point.x +
                                    coefficients[1] * point.y +
                                    coefficients[2] * point.z +
                                    coefficients[3]);
        double denominator = std::sqrt(coefficients[0] * coefficients[0] +
                                       coefficients[1] * coefficients[1] +
                                       coefficients[2] * coefficients[2]);
        return (denominator > 0.0) ? (numerator / denominator) : 0.0;
    }

    // Check if point is on the plane (within tolerance)
    bool contains_point(const Point3& point, double tolerance) const {
        return distance_to_point(point) <= tolerance;
    }

    void set_plane_type(const char* type) {
        std::memset(plane_type, 0, sizeof(plane_type));
        std::strncpy(plane_type, type, sizeof(plane_type) - 1);
    }
};

// ============================================================================
// PlaneArray - Multiple plane detections
// ============================================================================
struct PlaneArray {
    PlaneDetection planes[16];  // Array of plane detections (max 16)
    uint8_t count;              // Number of valid planes
    uint8_t _padding[3];        // Padding
    char frame_id[32];          // Source sensor frame
    char algorithm[32];         // Detection algorithm used
    uint64_t timestamp;         // nanoseconds since epoch

    PlaneArray() {
        for (int i = 0; i < 16; i++) {
            planes[i] = PlaneDetection();
        }
        count = 0;
        _padding[0] = _padding[1] = _padding[2] = 0;
        std::memset(frame_id, 0, sizeof(frame_id));
        std::memset(algorithm, 0, sizeof(algorithm));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Add a plane
    bool add_plane(const PlaneDetection& plane) {
        if (count >= 16) return false;
        planes[count++] = plane;
        return true;
    }

    // Get valid planes
    const PlaneDetection* get_planes() const {
        return planes;
    }

    size_t get_count() const {
        return count;
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
        count = 0;
    }
};

} // namespace messages
} // namespace horus

#endif // HORUS_MESSAGES_PERCEPTION_HPP
