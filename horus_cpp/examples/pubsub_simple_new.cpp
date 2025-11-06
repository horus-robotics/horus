// HORUS C++ Example: Simple Publisher-Subscriber (New API)
// Demonstrates basic communication pattern - minimal boilerplate

#include "../include/horus_new.hpp"
#include <iostream>
#include <cmath>

using namespace horus;

// ============================================================================
// Publisher Node - Generates temperature data
// ============================================================================

struct TemperatureSensor : Node {
    Publisher<float> temp_pub;
    float counter;

    TemperatureSensor()
        : Node("temperature_sensor"),
          temp_pub("temperature"),
          counter(0.0f) {}

    bool init(NodeContext& ctx) override {
        ctx.log_info("Temperature sensor starting...");
        return true;
    }

    void tick(NodeContext& ctx) override {
        // Simulate temperature reading
        float temperature = 20.0f + std::sin(counter) * 5.0f;
        temp_pub.send(temperature);

        counter += 0.1f;

        // Log every 60 ticks (1 second)
        if (static_cast<int>(counter * 10) % 600 == 0) {
            ctx.log_info("Temperature: " + std::to_string(temperature) + "°C");
        }
    }

    bool shutdown(NodeContext& ctx) override {
        ctx.log_info("Temperature sensor stopped");
        return true;
    }
};

// ============================================================================
// Subscriber Node - Consumes temperature data
// ============================================================================

struct TemperatureMonitor : Node {
    Subscriber<float> temp_sub;
    uint32_t readings_received;
    float min_temp;
    float max_temp;

    TemperatureMonitor()
        : Node("temperature_monitor"),
          temp_sub("temperature"),
          readings_received(0),
          min_temp(999.0f),
          max_temp(-999.0f) {}

    bool init(NodeContext& ctx) override {
        ctx.log_info("Temperature monitor starting...");
        return true;
    }

    void tick(NodeContext& ctx) override {
        float temperature;

        // Non-blocking receive
        if (temp_sub.recv(temperature)) {
            readings_received++;

            // Update statistics
            if (temperature < min_temp) min_temp = temperature;
            if (temperature > max_temp) max_temp = temperature;

            // Log every 60 readings
            if (readings_received % 60 == 0) {
                ctx.log_info(
                    "Received " + std::to_string(readings_received) + " readings | " +
                    "Min: " + std::to_string(min_temp) + "°C | " +
                    "Max: " + std::to_string(max_temp) + "°C"
                );
            }

            // Alert on extremes
            if (temperature > 30.0f) {
                ctx.log_warn("HIGH TEMPERATURE: " + std::to_string(temperature) + "°C");
            } else if (temperature < 10.0f) {
                ctx.log_warn("LOW TEMPERATURE: " + std::to_string(temperature) + "°C");
            }
        }
    }

    bool shutdown(NodeContext& ctx) override {
        ctx.log_info("Temperature monitor stopped");
        ctx.log_info(
            "Statistics: " + std::to_string(readings_received) + " readings | " +
            "Range: " + std::to_string(min_temp) + "°C to " + std::to_string(max_temp) + "°C"
        );
        return true;
    }
};

// ============================================================================
// Main
// ============================================================================

int main() {
    std::cout << "=== HORUS Simple Pub-Sub Example ===" << std::endl;
    std::cout << "\nTopology:" << std::endl;
    std::cout << "  TemperatureSensor → [temperature] → TemperatureMonitor" << std::endl;
    std::cout << "\nPress Ctrl+C to stop\n" << std::endl;

    try {
        Scheduler scheduler;

        // Add both nodes with Normal priority, logging enabled
        scheduler.add(TemperatureSensor(), 2, true);
        scheduler.add(TemperatureMonitor(), 2, true);

        // Run at 60 FPS
        scheduler.run();

        std::cout << "\nSystem stopped" << std::endl;

    } catch (const HorusException& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
