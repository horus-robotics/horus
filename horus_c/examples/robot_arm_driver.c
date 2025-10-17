// Example: Industrial robot arm driver (e.g., Universal Robots, ABB)
#include "../include/horus.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

// Simulate robot arm SDK (replace with vendor SDK)
typedef struct {
    char* ip_address;
    int socket;
    float joint_positions[6];
    float joint_velocities[6];
    float joint_efforts[6];
    bool connected;
} RobotArm;

RobotArm* robot_connect(const char* ip) {
    printf("[Robot] Connecting to %s\n", ip);

    RobotArm* robot = calloc(1, sizeof(RobotArm));
    robot->ip_address = strdup(ip);
    robot->socket = 1;  // Simulated socket
    robot->connected = true;

    // Initialize to home position
    for (int i = 0; i < 6; i++) {
        robot->joint_positions[i] = 0.0;
    }

    return robot;
}

// Simulate reading joint states (replace with real SDK)
int robot_get_joint_state(RobotArm* robot, float* positions, float* velocities) {
    // In real driver: modbus_read_registers(robot->socket, ...);

    // Simulate joint movement
    static float time = 0.0;
    time += 0.01;

    for (int i = 0; i < 6; i++) {
        positions[i] = sin(time + i) * 0.5;
        velocities[i] = cos(time + i) * 0.1;
    }

    return 0;
}

// Simulate sending joint commands (replace with real SDK)
int robot_move_joints(RobotArm* robot, const float* positions) {
    // In real driver: ur_script_send(robot->socket, positions);

    for (int i = 0; i < 6; i++) {
        robot->joint_positions[i] = positions[i];
    }

    return 0;
}

void robot_disconnect(RobotArm* robot) {
    if (robot) {
        free(robot->ip_address);
        free(robot);
    }
}

int main(int argc, char** argv) {
    printf("=== Robot Arm Driver Bridge for HORUS ===\n");

    // Initialize HORUS
    if (!init("robot_arm_driver")) {
        printf("Failed to initialize HORUS\n");
        return 1;
    }

    // Create publishers and subscribers
    Pub joint_state_pub = publisher("joint_states", MSG_JOINT_STATE);
    Sub joint_cmd_sub = subscriber("joint_commands", MSG_JOINT_STATE);

    // Connect to robot
    const char* robot_ip = argc > 1 ? argv[1] : "192.168.1.100";
    RobotArm* robot = robot_connect(robot_ip);
    if (!robot) {
        printf("Failed to connect to robot\n");
        shutdown();
        return 1;
    }

    printf("Robot arm connected\n");
    printf("Publishing joint states to: joint_states\n");
    printf("Subscribing to commands on: joint_commands\n\n");

    // Joint names
    char* joint_names[6] = {
        "shoulder_pan", "shoulder_lift", "elbow",
        "wrist_1", "wrist_2", "wrist_3"
    };

    // Allocate message buffers
    float positions[6], velocities[6], efforts[6];
    JointState state = {
        .positions = positions,
        .velocities = velocities,
        .efforts = efforts,
        .names = joint_names,
        .count = 6
    };

    JointState cmd;
    float cmd_positions[6];
    cmd.positions = cmd_positions;
    cmd.count = 6;

    uint32_t update_count = 0;

    // Main control loop - 125Hz (8ms)
    while (ok()) {
        // Read current joint state from hardware
        if (robot_get_joint_state(robot, positions, velocities) == 0) {
            // Publish state
            if (send(joint_state_pub, &state)) {
                update_count++;

                // Log every 125 updates (1 second)
                if (update_count % 125 == 0) {
                    char msg[256];
                    snprintf(msg, sizeof(msg),
                            "Robot state: J1=%.2f J2=%.2f J3=%.2f (rad)",
                            positions[0], positions[1], positions[2]);
                    log_debug(msg);
                }
            }
        }

        // Check for joint commands
        if (try_recv(joint_cmd_sub, &cmd)) {
            log_info("Received joint command");

            // Send command to robot
            if (robot_move_joints(robot, cmd.positions) != 0) {
                log_error("Failed to send command to robot");
            }
        }

        // Run at 125Hz for smooth motion
        sleep_ms(8);
    }

    // Cleanup
    printf("\nDisconnecting from robot\n");
    robot_disconnect(robot);
    shutdown();

    return 0;
}