// Example: Camera driver integration (e.g., RealSense, USB camera)
#include "../include/horus.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Simulate camera SDK (replace with real SDK like librealsense2)
typedef struct {
    int device_id;
    uint32_t width;
    uint32_t height;
    uint8_t* frame_buffer;
} Camera;

Camera* camera_open(int id, uint32_t width, uint32_t height) {
    printf("[Camera] Opening device %d (%ux%u)\n", id, width, height);

    Camera* cam = malloc(sizeof(Camera));
    cam->device_id = id;
    cam->width = width;
    cam->height = height;
    cam->frame_buffer = malloc(width * height * 3);  // RGB

    return cam;
}

// Simulate frame capture (replace with real SDK call)
int camera_capture(Camera* cam, uint8_t* buffer) {
    // In real driver: rs2_pipeline_wait_for_frames(pipeline, &frames);

    // Simulate frame - gradient pattern
    for (uint32_t y = 0; y < cam->height; y++) {
        for (uint32_t x = 0; x < cam->width; x++) {
            uint32_t idx = (y * cam->width + x) * 3;
            buffer[idx] = (x * 255) / cam->width;      // R
            buffer[idx + 1] = (y * 255) / cam->height; // G
            buffer[idx + 2] = 128;                     // B
        }
    }

    return 0;  // Success
}

void camera_close(Camera* cam) {
    if (cam) {
        free(cam->frame_buffer);
        free(cam);
    }
}

int main() {
    printf("=== Camera Driver Bridge for HORUS ===\n");

    // Initialize HORUS
    if (!init("camera_driver")) {
        printf("Failed to initialize HORUS\n");
        return 1;
    }

    // Create publishers
    Pub image_pub = publisher("camera/image", MSG_IMAGE);
    Pub compressed_pub = publisher_custom("camera/compressed", sizeof(Image));

    // Open camera
    Camera* cam = camera_open(0, 640, 480);
    if (!cam) {
        printf("Failed to open camera\n");
        shutdown();
        return 1;
    }

    printf("Camera running at 30 FPS\n");
    printf("Publishing to: camera/image\n\n");

    // Allocate image buffer
    uint8_t* buffer = malloc(cam->width * cam->height * 3);
    Image img = {
        .data = buffer,
        .width = cam->width,
        .height = cam->height,
        .step = cam->width * 3,
        .channels = 3
    };

    uint32_t frame_count = 0;
    uint64_t last_log = time_now_ms();

    // Main capture loop
    while (ok()) {
        // Capture frame from hardware
        if (camera_capture(cam, buffer) == 0) {
            // Publish raw image
            if (send(image_pub, &img)) {
                frame_count++;
            }

            // Log FPS every second
            uint64_t now = time_now_ms();
            if (now - last_log >= 1000) {
                char msg[128];
                snprintf(msg, sizeof(msg),
                        "Camera: %u FPS", frame_count);
                log_info(msg);
                frame_count = 0;
                last_log = now;
            }
        }

        // Run at ~30 FPS
        sleep_ms(33);
    }

    // Cleanup
    printf("\nShutting down camera\n");
    free(buffer);
    camera_close(cam);
    shutdown();

    return 0;
}