// HORUS Message Library - Vision Types
// Binary-compatible with Rust definitions in horus_library/messages/vision.rs

#ifndef HORUS_MESSAGES_VISION_HPP
#define HORUS_MESSAGES_VISION_HPP

#include "geometry.hpp"
#include <cstdint>
#include <cstring>
#include <chrono>
#include <algorithm>
#include <cmath>

namespace horus {
namespace messages {

// ============================================================================
// ImageEncoding - Pixel format enumeration
// Binary-compatible with Rust enum (repr(u8))
// ============================================================================
enum class ImageEncoding : uint8_t {
    Mono8 = 0,        // 8-bit monochrome
    Mono16 = 1,       // 16-bit monochrome
    Rgb8 = 2,         // 8-bit RGB (3 channels)
    Bgr8 = 3,         // 8-bit BGR (3 channels, OpenCV format)
    Rgba8 = 4,        // 8-bit RGBA (4 channels)
    Bgra8 = 5,        // 8-bit BGRA (4 channels)
    Yuv422 = 6,       // YUV 4:2:2 format
    Mono32F = 7,      // 32-bit float monochrome
    Rgb32F = 8,       // 32-bit float RGB
    BayerRggb8 = 9,   // Bayer pattern (raw sensor data)
    Depth16 = 10,     // 16-bit depth image (millimeters)
};

// Helper functions for ImageEncoding
inline uint32_t bytes_per_pixel(ImageEncoding encoding) {
    switch (encoding) {
        case ImageEncoding::Mono8: return 1;
        case ImageEncoding::Mono16: return 2;
        case ImageEncoding::Rgb8:
        case ImageEncoding::Bgr8: return 3;
        case ImageEncoding::Rgba8:
        case ImageEncoding::Bgra8: return 4;
        case ImageEncoding::Yuv422: return 2;
        case ImageEncoding::Mono32F: return 4;
        case ImageEncoding::Rgb32F: return 12;
        case ImageEncoding::BayerRggb8: return 1;
        case ImageEncoding::Depth16: return 2;
        default: return 0;
    }
}

inline bool is_color(ImageEncoding encoding) {
    return encoding == ImageEncoding::Rgb8 ||
           encoding == ImageEncoding::Bgr8 ||
           encoding == ImageEncoding::Rgba8 ||
           encoding == ImageEncoding::Bgra8 ||
           encoding == ImageEncoding::Yuv422 ||
           encoding == ImageEncoding::Rgb32F ||
           encoding == ImageEncoding::BayerRggb8;
}

// ============================================================================
// Image - Raw image data (fixed-size for shared memory)
// Max size: 2MB for raw image data
// ============================================================================
struct Image {
    static constexpr size_t MAX_DATA_SIZE = 2 * 1024 * 1024; // 2MB

    uint32_t width;              // Image width in pixels
    uint32_t height;             // Image height in pixels
    ImageEncoding encoding;      // Pixel encoding format
    uint8_t _padding[3];         // Padding for alignment
    uint32_t step;               // Bytes per row (may include padding)
    uint32_t data_length;        // Actual data length in bytes
    uint8_t data[MAX_DATA_SIZE]; // Image data (row-major order)
    char frame_id[32];           // Camera identifier
    uint64_t timestamp;          // nanoseconds since epoch

    Image() {
        width = 0;
        height = 0;
        encoding = ImageEncoding::Rgb8;
        _padding[0] = _padding[1] = _padding[2] = 0;
        step = 0;
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

    // Create image from data pointer
    bool set_data(uint32_t w, uint32_t h, ImageEncoding enc, const uint8_t* img_data, size_t len) {
        if (len > MAX_DATA_SIZE) return false;

        width = w;
        height = h;
        encoding = enc;
        step = w * bytes_per_pixel(enc);
        data_length = static_cast<uint32_t>(len);
        std::memcpy(data, img_data, len);
        update_timestamp();
        return true;
    }

    // Set frame ID
    void set_frame_id(const char* frame) {
        std::memset(frame_id, 0, sizeof(frame_id));
        std::strncpy(frame_id, frame, sizeof(frame_id) - 1);
    }

    // Get expected data size
    size_t expected_size() const {
        return static_cast<size_t>(step) * height;
    }

    // Validate image consistency
    bool is_valid() const {
        return width > 0 && height > 0 &&
               step >= width * bytes_per_pixel(encoding) &&
               data_length >= expected_size() &&
               data_length <= MAX_DATA_SIZE;
    }

    // Get pixel at coordinates (returns pointer to pixel data)
    const uint8_t* get_pixel(uint32_t x, uint32_t y) const {
        if (x >= width || y >= height) return nullptr;
        uint32_t bpp = bytes_per_pixel(encoding);
        size_t offset = y * step + x * bpp;
        if (offset + bpp <= data_length) {
            return &data[offset];
        }
        return nullptr;
    }

    // Get angle for a specific index (for depth/range images)
    float angle_at(size_t index, float angle_min, float angle_increment) const {
        return angle_min + (static_cast<float>(index) * angle_increment);
    }
};

// ============================================================================
// CompressedImage - JPEG, PNG, WebP compressed image
// Max size: 512KB for compressed data
// ============================================================================
struct CompressedImage {
    static constexpr size_t MAX_DATA_SIZE = 512 * 1024; // 512KB

    char format[8];              // Compression format ("jpeg", "png", "webp")
    uint32_t data_length;        // Actual compressed data length
    uint8_t data[MAX_DATA_SIZE]; // Compressed image data
    uint32_t width;              // Original image width (if known)
    uint32_t height;             // Original image height (if known)
    char frame_id[32];           // Camera identifier
    uint64_t timestamp;          // nanoseconds since epoch

    CompressedImage() {
        std::memset(format, 0, sizeof(format));
        data_length = 0;
        std::memset(data, 0, MAX_DATA_SIZE);
        width = 0;
        height = 0;
        std::memset(frame_id, 0, sizeof(frame_id));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Set format
    void set_format(const char* fmt) {
        std::memset(format, 0, sizeof(format));
        std::strncpy(format, fmt, sizeof(format) - 1);
    }

    // Set compressed data
    bool set_data(const uint8_t* compressed_data, size_t len) {
        if (len > MAX_DATA_SIZE) return false;
        data_length = static_cast<uint32_t>(len);
        std::memcpy(data, compressed_data, len);
        update_timestamp();
        return true;
    }

    // Set frame ID
    void set_frame_id(const char* frame) {
        std::memset(frame_id, 0, sizeof(frame_id));
        std::strncpy(frame_id, frame, sizeof(frame_id) - 1);
    }

    bool is_valid() const {
        return data_length > 0 && data_length <= MAX_DATA_SIZE;
    }
};

// ============================================================================
// CameraInfo - Camera calibration and intrinsic parameters
// ============================================================================
struct CameraInfo {
    uint32_t width;                      // Image width in pixels
    uint32_t height;                     // Image height in pixels
    char distortion_model[16];           // "plumb_bob", "rational_polynomial"
    double distortion_coefficients[8];   // [k1, k2, p1, p2, k3, k4, k5, k6]
    double camera_matrix[9];             // 3x3 intrinsic matrix (row-major)
    double rectification_matrix[9];      // 3x3 rectification matrix
    double projection_matrix[12];        // 3x4 projection matrix
    char frame_id[32];                   // Camera identifier
    uint64_t timestamp;                  // nanoseconds since epoch

    CameraInfo() {
        width = 0;
        height = 0;
        std::memset(distortion_model, 0, sizeof(distortion_model));

        for (int i = 0; i < 8; i++) {
            distortion_coefficients[i] = 0.0;
        }

        for (int i = 0; i < 9; i++) {
            camera_matrix[i] = 0.0;
            rectification_matrix[i] = 0.0;
        }
        // Identity rectification
        rectification_matrix[0] = rectification_matrix[4] = rectification_matrix[8] = 1.0;

        for (int i = 0; i < 12; i++) {
            projection_matrix[i] = 0.0;
        }

        std::memset(frame_id, 0, sizeof(frame_id));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Create with basic parameters (pinhole camera model)
    static CameraInfo create(uint32_t w, uint32_t h, double fx, double fy, double cx, double cy) {
        CameraInfo info;
        info.width = w;
        info.height = h;
        // Camera matrix: [fx, 0, cx; 0, fy, cy; 0, 0, 1]
        info.camera_matrix[0] = fx;
        info.camera_matrix[2] = cx;
        info.camera_matrix[4] = fy;
        info.camera_matrix[5] = cy;
        info.camera_matrix[8] = 1.0;
        // Projection matrix: [fx, 0, cx, 0; 0, fy, cy, 0; 0, 0, 1, 0]
        info.projection_matrix[0] = fx;
        info.projection_matrix[2] = cx;
        info.projection_matrix[5] = fy;
        info.projection_matrix[6] = cy;
        info.projection_matrix[10] = 1.0;
        return info;
    }

    // Get focal lengths
    void get_focal_lengths(double& fx, double& fy) const {
        fx = camera_matrix[0];
        fy = camera_matrix[4];
    }

    // Get principal point
    void get_principal_point(double& cx, double& cy) const {
        cx = camera_matrix[2];
        cy = camera_matrix[5];
    }

    // Set distortion model
    void set_distortion_model(const char* model) {
        std::memset(distortion_model, 0, sizeof(distortion_model));
        std::strncpy(distortion_model, model, sizeof(distortion_model) - 1);
    }

    void set_frame_id(const char* frame) {
        std::memset(frame_id, 0, sizeof(frame_id));
        std::strncpy(frame_id, frame, sizeof(frame_id) - 1);
    }
};

// ============================================================================
// RegionOfInterest - Image ROI/bounding box
// ============================================================================
struct RegionOfInterest {
    uint32_t x_offset;    // X offset
    uint32_t y_offset;    // Y offset
    uint32_t width;       // Width
    uint32_t height;      // Height
    bool do_rectify;      // Whether to apply rectification
    uint8_t _padding[3];  // Padding for alignment

    RegionOfInterest() : x_offset(0), y_offset(0), width(0), height(0), do_rectify(false) {
        _padding[0] = _padding[1] = _padding[2] = 0;
    }

    RegionOfInterest(uint32_t x, uint32_t y, uint32_t w, uint32_t h)
        : x_offset(x), y_offset(y), width(w), height(h), do_rectify(false) {
        _padding[0] = _padding[1] = _padding[2] = 0;
    }

    // Check if point is inside ROI
    bool contains(uint32_t x, uint32_t y) const {
        return x >= x_offset && x < x_offset + width &&
               y >= y_offset && y < y_offset + height;
    }

    // Get area
    uint32_t area() const {
        return width * height;
    }

    bool is_valid() const {
        return width > 0 && height > 0;
    }
};

// ============================================================================
// Detection - Object detection/recognition result
// ============================================================================
struct Detection {
    char class_name[32];        // Object class name
    float confidence;           // Detection confidence (0.0 to 1.0)
    RegionOfInterest bbox;      // Bounding box
    Transform pose;             // 3D pose (if available)
    bool has_pose;              // Whether pose is valid
    uint8_t _padding[3];        // Padding
    uint32_t track_id;          // Object ID for tracking
    uint64_t timestamp;         // nanoseconds since epoch

    Detection() {
        std::memset(class_name, 0, sizeof(class_name));
        confidence = 0.0f;
        bbox = RegionOfInterest();
        pose = Transform();
        has_pose = false;
        _padding[0] = _padding[1] = _padding[2] = 0;
        track_id = 0;
        update_timestamp();
    }

    Detection(const char* name, float conf, const RegionOfInterest& box)
        : confidence(conf), bbox(box), has_pose(false), track_id(0) {
        std::memset(class_name, 0, sizeof(class_name));
        std::strncpy(class_name, name, sizeof(class_name) - 1);
        _padding[0] = _padding[1] = _padding[2] = 0;
        pose = Transform();
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    void set_class_name(const char* name) {
        std::memset(class_name, 0, sizeof(class_name));
        std::strncpy(class_name, name, sizeof(class_name) - 1);
    }

    bool is_valid() const {
        return confidence >= 0.0f && confidence <= 1.0f && bbox.is_valid();
    }
};

// ============================================================================
// DetectionArray - Multiple detections
// ============================================================================
struct DetectionArray {
    Detection detections[32];   // Array of detections (max 32)
    uint8_t count;              // Number of valid detections
    uint8_t _padding[3];        // Padding
    uint32_t image_width;       // Source image width
    uint32_t image_height;      // Source image height
    char frame_id[32];          // Frame identifier
    uint64_t timestamp;         // nanoseconds since epoch

    DetectionArray() {
        for (int i = 0; i < 32; i++) {
            detections[i] = Detection();
        }
        count = 0;
        _padding[0] = _padding[1] = _padding[2] = 0;
        image_width = 0;
        image_height = 0;
        std::memset(frame_id, 0, sizeof(frame_id));
        update_timestamp();
    }

    void update_timestamp() {
        auto now = std::chrono::system_clock::now();
        auto duration = now.time_since_epoch();
        timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
    }

    // Add a detection
    bool add_detection(const Detection& det) {
        if (count >= 32) return false;
        detections[count] = det;
        count++;
        return true;
    }

    // Get valid detections slice
    const Detection* get_detections() const {
        return detections;
    }

    size_t get_count() const {
        return count;
    }

    // Filter by confidence threshold (returns count of matching detections)
    size_t filter_by_confidence(float threshold, Detection* output, size_t max_output) const {
        size_t matched = 0;
        for (size_t i = 0; i < count && matched < max_output; i++) {
            if (detections[i].confidence >= threshold) {
                output[matched++] = detections[i];
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
// StereoInfo - Stereo camera pair calibration
// ============================================================================
struct StereoInfo {
    CameraInfo left_camera;     // Left camera info
    CameraInfo right_camera;    // Right camera info
    double baseline;            // Distance between cameras (meters)
    double depth_scale;         // Disparity-to-depth conversion factor

    StereoInfo() : baseline(0.0), depth_scale(1.0) {
        left_camera = CameraInfo();
        right_camera = CameraInfo();
    }

    // Calculate depth from disparity
    float depth_from_disparity(float disparity) const {
        if (disparity <= 0.0f) return INFINITY;
        double fx;
        double fy_unused;
        left_camera.get_focal_lengths(fx, fy_unused);
        return static_cast<float>((baseline * fx) / disparity);
    }

    // Calculate disparity from depth
    float disparity_from_depth(float depth) const {
        if (depth <= 0.0f) return 0.0f;
        double fx;
        double fy_unused;
        left_camera.get_focal_lengths(fx, fy_unused);
        return static_cast<float>((baseline * fx) / depth);
    }
};

} // namespace messages
} // namespace horus

#endif // HORUS_MESSAGES_VISION_HPP
