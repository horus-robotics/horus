// Example: Camera driver integration with HORUS C++ API (e.g., RealSense, USB camera)
#include "../include/horus.hpp"
#include <iostream>
#include <vector>
#include <memory>
#include <string>

// Simulate camera SDK using modern C++ (replace with real SDK like librealsense2)
class Camera {
public:
    Camera(int device_id, uint32_t width, uint32_t height)
        : device_id_(device_id),
          width_(width),
          height_(height),
          frame_buffer_(width * height * 3, 0) {
        std::cout << "[Camera] Opening device " << device_id
                  << " (" << width << "x" << height << ")" << std::endl;
        // In real driver: initialize SDK, open device
    }

    ~Camera() {
        std::cout << "[Camera] Closing device" << std::endl;
        // In real driver: release resources, close device
    }

    // Non-copyable
    Camera(const Camera&) = delete;
    Camera& operator=(const Camera&) = delete;

    // Moveable
    Camera(Camera&&) = default;
    Camera& operator=(Camera&&) = default;

    // Simulate frame capture (replace with real SDK call)
    bool capture(std::vector<uint8_t>& buffer) {
        // In real driver: rs2_pipeline_wait_for_frames(pipeline, &frames);

        buffer.resize(width_ * height_ * 3);

        // Simulate frame - gradient pattern
        for (uint32_t y = 0; y < height_; y++) {
            for (uint32_t x = 0; x < width_; x++) {
                uint32_t idx = (y * width_ + x) * 3;
                buffer[idx]     = (x * 255) / width_;      // R
                buffer[idx + 1] = (y * 255) / height_;     // G
                buffer[idx + 2] = 128;                     // B
            }
        }

        return true; // Success
    }

    uint32_t width() const { return width_; }
    uint32_t height() const { return height_; }
    uint32_t channels() const { return 3; }

private:
    int device_id_;
    uint32_t width_;
    uint32_t height_;
    std::vector<uint8_t> frame_buffer_;
};

int main() {
    std::cout << "=== Camera Driver Bridge for HORUS (C++) ===" << std::endl;

    try {
        // Initialize HORUS system with RAII
        horus::System system("camera_driver");

        // Create publishers
        horus::Publisher<Image> image_pub("camera/image");

        // Open camera
        Camera cam(0, 640, 480);

        std::cout << "Camera running at 30 FPS" << std::endl;
        std::cout << "Publishing to: camera/image\n" << std::endl;

        // Allocate image buffer
        std::vector<uint8_t> buffer;
        Image img{};
        img.width = cam.width();
        img.height = cam.height();
        img.step = cam.width() * cam.channels();
        img.channels = cam.channels();

        uint32_t frame_count = 0;
        uint64_t last_log = horus::time_now_ms();

        // Main capture loop
        while (system.ok()) {
            // Capture frame from hardware
            if (cam.capture(buffer)) {
                img.data = buffer.data();

                // Publish raw image using modern C++ API
                image_pub << img;
                frame_count++;

                // Log FPS every second
                uint64_t now = horus::time_now_ms();
                if (now - last_log >= 1000) {
                    horus::Log::info("Camera: " + std::to_string(frame_count) + " FPS");
                    frame_count = 0;
                    last_log = now;
                }
            }

            // Run at ~30 FPS
            horus::sleep_ms(33);
        }

        // Cleanup is automatic with RAII
        std::cout << "\nShutting down camera" << std::endl;

    } catch (const horus::HorusException& e) {
        std::cerr << "HORUS Error: " << e.what() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
