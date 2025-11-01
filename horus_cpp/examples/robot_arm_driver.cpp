// Example: Industrial robot arm driver with HORUS C++ API (e.g., Universal Robots, ABB)
#include "../include/horus.hpp"
#include <iostream>
#include <vector>
#include <array>
#include <string>
#include <cmath>

// Simulate robot arm SDK using modern C++ (replace with vendor SDK)
class RobotArm {
public:
    explicit RobotArm(const std::string& ip_address)
        : ip_address_(ip_address), socket_(1), connected_(false) {
        std::cout << "[Robot] Connecting to " << ip_address << std::endl;

        // In real driver: open TCP socket, establish connection
        connected_ = true;

        // Initialize to home position
        joint_positions_.fill(0.0f);
        joint_velocities_.fill(0.0f);
        joint_efforts_.fill(0.0f);
    }

    ~RobotArm() {
        std::cout << "[Robot] Disconnecting" << std::endl;
        // In real driver: close connection, cleanup
    }

    // Non-copyable
    RobotArm(const RobotArm&) = delete;
    RobotArm& operator=(const RobotArm&) = delete;

    // Moveable
    RobotArm(RobotArm&&) = default;
    RobotArm& operator=(RobotArm&&) = default;

    // Simulate reading joint states (replace with real SDK)
    bool get_joint_state(std::array<float, 6>& positions, std::array<float, 6>& velocities) {
        // In real driver: modbus_read_registers(socket_, ...);

        // Simulate joint movement
        static float time = 0.0f;
        time += 0.01f;

        for (size_t i = 0; i < 6; i++) {
            positions[i] = std::sin(time + i) * 0.5f;
            velocities[i] = std::cos(time + i) * 0.1f;
        }

        return true;
    }

    // Simulate sending joint commands (replace with real SDK)
    bool move_joints(const std::array<float, 6>& positions) {
        // In real driver: ur_script_send(socket_, positions);

        joint_positions_ = positions;
        return true;
    }

    bool is_connected() const { return connected_; }

private:
    std::string ip_address_;
    int socket_;
    std::array<float, 6> joint_positions_;
    std::array<float, 6> joint_velocities_;
    std::array<float, 6> joint_efforts_;
    bool connected_;
};

int main(int argc, char** argv) {
    std::cout << "=== Robot Arm Driver Bridge for HORUS (C++) ===" << std::endl;

    try {
        // Initialize HORUS system with RAII
        horus::System system("robot_arm_driver");

        // Create publishers and subscribers
        horus::Publisher<JointState> joint_state_pub("joint_states");
        horus::Subscriber<JointState> joint_cmd_sub("joint_commands");

        // Connect to robot
        const std::string robot_ip = (argc > 1) ? argv[1] : "192.168.1.100";
        RobotArm robot(robot_ip);

        if (!robot.is_connected()) {
            throw horus::HorusException("Failed to connect to robot");
        }

        std::cout << "Robot arm connected" << std::endl;
        std::cout << "Publishing joint states to: joint_states" << std::endl;
        std::cout << "Subscribing to commands on: joint_commands\n" << std::endl;

        // Joint names
        std::array<const char*, 6> joint_name_ptrs = {
            "shoulder_pan", "shoulder_lift", "elbow",
            "wrist_1", "wrist_2", "wrist_3"
        };

        // Allocate message buffers
        std::array<float, 6> positions{}, velocities{}, efforts{};
        JointState state{};
        state.positions = positions.data();
        state.velocities = velocities.data();
        state.efforts = efforts.data();
        state.names = const_cast<char**>(joint_name_ptrs.data());
        state.count = 6;

        JointState cmd{};
        std::array<float, 6> cmd_positions{};
        cmd.positions = cmd_positions.data();
        cmd.count = 6;

        uint32_t update_count = 0;

        // Main control loop - 125Hz (8ms)
        while (system.ok()) {
            // Read current joint state from hardware
            if (robot.get_joint_state(positions, velocities)) {
                // Publish state using modern C++ API
                joint_state_pub << state;
                update_count++;

                // Log every 125 updates (1 second)
                if (update_count % 125 == 0) {
                    horus::Log::debug(
                        "Robot state: J1=" + std::to_string(positions[0]) +
                        " J2=" + std::to_string(positions[1]) +
                        " J3=" + std::to_string(positions[2]) + " (rad)"
                    );
                }
            }

            // Check for joint commands (non-blocking)
            if (joint_cmd_sub.recv(cmd)) {
                horus::Log::info("Received joint command");

                // Copy command data
                std::copy(cmd.positions, cmd.positions + 6, cmd_positions.begin());

                // Send command to robot
                if (!robot.move_joints(cmd_positions)) {
                    horus::Log::error("Failed to send command to robot");
                }
            }

            // Run at 125Hz for smooth motion
            horus::sleep_ms(8);
        }

        // Cleanup is automatic with RAII
        std::cout << "\nDisconnecting from robot" << std::endl;

    } catch (const horus::HorusException& e) {
        std::cerr << "HORUS Error: " << e.what() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
