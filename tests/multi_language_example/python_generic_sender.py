#!/usr/bin/env python3
"""
Python Generic Sender - sends arbitrary data to Rust

Run alongside rust_generic_receiver for cross-language demo
"""

from horus import Hub, Node, run
import time
import random


def main():
    print("=" * 70)
    print("Python Generic Sender (Cross-Language)")
    print("=" * 70)

    _data_hub = Hub("generic_cross_lang")

    def sensor_tick(node):
        """Send sensor data that Rust can receive"""
        sensor_data = {
            "source": "python_sensor",
            "timestamp": time.time(),
            "readings": {
                "temperature": 20 + random.uniform(-5, 5),
                "humidity": 50 + random.uniform(-10, 10),
                "pressure": 1013 + random.uniform(-5, 5)
            },
            "measurements": [
                random.uniform(0, 100) for _ in range(5)
            ],
            "status": "active",
            "error_count": 0
        }

        _data_hub.send(sensor_data, node)
        node.log_info(f"Sent: T={sensor_data['readings']['temperature']:.1f}°C, "
                      f"H={sensor_data['readings']['humidity']:.1f}%")

    sensor = Node(name="python_sensor", tick=sensor_tick, rate=2)

    print("\nSending data that Rust can receive...")
    print("Run 'rust_generic_receiver' in another terminal to see it!")
    print("(Ctrl+C to stop)\n")

    try:
        run(sensor, duration=60, logging=True)
    except KeyboardInterrupt:
        print("\nStopped")


if __name__ == "__main__":
    main()
