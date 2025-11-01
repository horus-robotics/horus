// HORUS Framework Demo - Shows proper Node/Scheduler integration
#include "../include/horus.hpp"
#include <iostream>
#include <cmath>
#include <optional>

// ============================================================================
// Example Node 1: Sensor Node (publishes data)
// ============================================================================
class SensorNode : public horus::Node {
private:
    std::optional<horus::Publisher<Twist>> velocity_pub_;
    uint32_t tick_count_;

public:
    SensorNode() : Node("sensor_node"), tick_count_(0) {}

    bool init(horus::NodeContext& ctx) override {
        ctx.log_info("Sensor node initializing...");

        // Create publisher
        velocity_pub_.emplace(ctx.create_publisher<Twist>("robot/velocity"));

        ctx.log_info("Sensor node initialized successfully");
        return true;
    }

    void tick(horus::NodeContext& ctx) override {
        // Called automatically by scheduler at 60 FPS
        tick_count_++;

        // Generate simulated sensor data
        float time = tick_count_ * 0.01f;  // Time in seconds

        Twist velocity = horus::make_twist(
            {std::cos(time), 0.0f, 0.0f},       // Linear velocity
            {0.0f, 0.0f, std::sin(time) * 0.5f} // Angular velocity
        );

        // Publish to HORUS
        velocity_pub_.value() << velocity;

        // Log every 60 ticks (1 second at 60 FPS)
        if (tick_count_ % 60 == 0) {
            ctx.log_info("Sensor published " + std::to_string(tick_count_) + " velocity readings");
        }
    }

    void shutdown(horus::NodeContext& ctx) override {
        ctx.log_info("Sensor node shutting down. Total ticks: " + std::to_string(tick_count_));
    }
};

// ============================================================================
// Example Node 2: Controller Node (subscribes and processes)
// ============================================================================
class ControllerNode : public horus::Node {
private:
    std::optional<horus::Subscriber<Twist>> velocity_sub_;
    std::optional<horus::Publisher<Twist>> command_pub_;
    uint32_t messages_received_;

public:
    ControllerNode() : Node("controller_node"), messages_received_(0) {}

    bool init(horus::NodeContext& ctx) override {
        ctx.log_info("Controller node initializing...");

        // Create subscriber and publisher
        velocity_sub_.emplace(ctx.create_subscriber<Twist>("robot/velocity"));
        command_pub_.emplace(ctx.create_publisher<Twist>("robot/cmd_vel"));

        ctx.log_info("Controller node initialized successfully");
        return true;
    }

    void tick(horus::NodeContext& ctx) override {
        // Called automatically at 60 FPS

        Twist current_velocity;

        // Non-blocking receive
        if (velocity_sub_.value().recv(current_velocity)) {
            messages_received_++;

            // Simple control logic: limit velocity
            Twist command = current_velocity;

            // Limit linear velocity to 1.0 m/s
            float linear_mag = std::sqrt(
                command.linear.x * command.linear.x +
                command.linear.y * command.linear.y +
                command.linear.z * command.linear.z
            );

            if (linear_mag > 1.0f) {
                float scale = 1.0f / linear_mag;
                command.linear.x *= scale;
                command.linear.y *= scale;
                command.linear.z *= scale;
            }

            // Limit angular velocity to 0.5 rad/s
            if (std::abs(command.angular.z) > 0.5f) {
                command.angular.z = (command.angular.z > 0) ? 0.5f : -0.5f;
            }

            // Publish command
            command_pub_.value() << command;

            // Log every 60 messages
            if (messages_received_ % 60 == 0) {
                ctx.log_info("Controller processed " + std::to_string(messages_received_) + " messages");
            }
        }
    }

    void shutdown(horus::NodeContext& ctx) override {
        ctx.log_info("Controller shutting down. Messages processed: " +
                     std::to_string(messages_received_));
    }
};

// ============================================================================
// Example Node 3: Monitor Node (high-priority safety check)
// ============================================================================
class MonitorNode : public horus::Node {
private:
    std::optional<horus::Subscriber<Twist>> command_sub_;
    uint32_t safety_violations_;

public:
    MonitorNode() : Node("monitor_node"), safety_violations_(0) {}

    bool init(horus::NodeContext& ctx) override {
        ctx.log_info("Monitor node initializing (CRITICAL priority)...");

        command_sub_.emplace(ctx.create_subscriber<Twist>("robot/cmd_vel"));

        ctx.log_info("Monitor node initialized");
        return true;
    }

    void tick(horus::NodeContext& ctx) override {
        // Critical safety checks run at highest priority

        Twist command;
        if (command_sub_.value().recv(command)) {
            // Check for safety violations
            float linear_mag = std::sqrt(
                command.linear.x * command.linear.x +
                command.linear.y * command.linear.y +
                command.linear.z * command.linear.z
            );

            if (linear_mag > 2.0f || std::abs(command.angular.z) > 1.0f) {
                safety_violations_++;
                ctx.log_warn("Safety violation detected! Linear: " +
                            std::to_string(linear_mag) + " m/s, Angular: " +
                            std::to_string(command.angular.z) + " rad/s");
            }
        }
    }

    void shutdown(horus::NodeContext& ctx) override {
        ctx.log_info("Monitor shutting down. Safety violations: " +
                     std::to_string(safety_violations_));
    }
};

// ============================================================================
// Main: Create scheduler and register nodes
// ============================================================================
int main() {
    std::cout << "=== HORUS Framework Demo ===" << std::endl;
    std::cout << "Demonstrating Node/Scheduler integration\n" << std::endl;

    try {
        // Create scheduler
        horus::Scheduler scheduler("demo_scheduler");

        // Create nodes
        SensorNode sensor;
        ControllerNode controller;
        MonitorNode monitor;

        // Register nodes with different priorities
        std::cout << "Registering nodes with scheduler..." << std::endl;

        // Monitor runs at CRITICAL priority (first)
        scheduler.register_node(monitor, horus::Priority::Critical);

        // Controller at HIGH priority
        scheduler.register_node(controller, horus::Priority::High);

        // Sensor at NORMAL priority
        scheduler.register_node(sensor, horus::Priority::Normal);

        std::cout << "All nodes registered. Starting scheduler at 60 FPS..." << std::endl;
        std::cout << "Priority order: Monitor (Critical) -> Controller (High) -> Sensor (Normal)" << std::endl;
        std::cout << "Press Ctrl+C to stop\n" << std::endl;

        // Run scheduler (blocks until Ctrl+C)
        scheduler.run();

        std::cout << "\nScheduler stopped gracefully" << std::endl;

    } catch (const horus::HorusException& e) {
        std::cerr << "HORUS Error: " << e.what() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
