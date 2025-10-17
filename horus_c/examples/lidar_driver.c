// Example: LiDAR hardware driver integration with HORUS
#include "../include/horus.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

// Simulate a hardware LiDAR driver (replace with real driver like rplidar)
typedef struct {
    int fd;  // File descriptor or device handle
    float* range_buffer;
    uint32_t points_per_scan;
} LidarDevice;

// Simulate hardware initialization (replace with real driver)
LidarDevice* lidar_open(const char* port) {
    printf("[Driver] Opening LiDAR on %s\n", port);

    LidarDevice* device = malloc(sizeof(LidarDevice));
    device->fd = 1;  // Simulated file descriptor
    device->points_per_scan = 360;
    device->range_buffer = malloc(sizeof(float) * device->points_per_scan);

    return device;
}

// Simulate hardware read (replace with real driver call)
int lidar_get_scan(LidarDevice* device, float* ranges, uint32_t* count) {
    // In real driver: ioctl(device->fd, LIDAR_GET_SCAN, ranges);

    // Simulate scanning - generate fake data
    for (uint32_t i = 0; i < device->points_per_scan; i++) {
        // Simulate walls at different distances
        float angle = (i * M_PI * 2.0) / device->points_per_scan;
        ranges[i] = 2.0 + sin(angle) * 0.5 + (rand() % 100) / 1000.0;
    }

    *count = device->points_per_scan;
    return 0;  // Success
}

void lidar_close(LidarDevice* device) {
    if (device) {
        free(device->range_buffer);
        free(device);
    }
}

int main(int argc, char** argv) {
    printf("=== LiDAR Driver Bridge for HORUS ===\n");

    // Initialize HORUS node
    if (!init("lidar_driver")) {
        printf("Failed to initialize HORUS node\n");
        return 1;
    }

    // Create publisher for laser scans
    Pub scan_pub = publisher("laser_scan", MSG_LASER_SCAN);
    if (!scan_pub) {
        printf("Failed to create publisher\n");
        shutdown();
        return 1;
    }

    // Open hardware device
    const char* port = argc > 1 ? argv[1] : "/dev/ttyUSB0";
    LidarDevice* lidar = lidar_open(port);
    if (!lidar) {
        printf("Failed to open LiDAR device\n");
        shutdown();
        return 1;
    }

    printf("LiDAR driver running at 10Hz...\n");
    printf("Publishing to topic: laser_scan\n\n");

    // Allocate scan message
    float ranges[360];
    LaserScan scan = {
        .ranges = ranges,
        .intensities = NULL,  // No intensity data
        .count = 0,
        .angle_min = 0.0,
        .angle_max = 2.0 * M_PI,
        .angle_increment = (2.0 * M_PI) / 360.0,
        .range_min = 0.1,
        .range_max = 10.0,
        .scan_time = 0.1  // 100ms per scan
    };

    uint32_t scan_count = 0;

    // Main loop - read from hardware and publish
    while (ok()) {
        // Read from hardware
        uint32_t point_count;
        if (lidar_get_scan(lidar, ranges, &point_count) == 0) {
            scan.count = point_count;

            // Publish to HORUS
            if (send(scan_pub, &scan)) {
                scan_count++;

                // Log every 10th scan
                if (scan_count % 10 == 0) {
                    char msg[256];
                    snprintf(msg, sizeof(msg),
                            "Published scan #%u (%u points, min: %.2fm)",
                            scan_count, point_count, ranges[0]);
                    log_debug(msg);
                }
            } else {
                log_error("Failed to publish scan");
            }
        } else {
            log_error("Failed to read from LiDAR");
        }

        // Run at 10Hz
        sleep_ms(100);
    }

    // Cleanup
    printf("\nShutting down LiDAR driver\n");
    lidar_close(lidar);
    shutdown();

    return 0;
}