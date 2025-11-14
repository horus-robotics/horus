#!/usr/bin/env python3
"""
HORUS Python Bindings - Custom Data Example

Demonstrates how to send arbitrary Python data structures using Generic Hubs.
Any serializable Python data (dicts, lists, numbers, strings, etc.) can be sent.

Features:
- No need to define message types
- Supports nested structures
- MessagePack serialization (fast and compact)
- Works across languages (Python ↔ Rust ↔ C++)
"""

from horus import Hub, Node, run
import time
import math


def example_1_simple_dict():
    """Send and receive simple dictionaries"""
    print("\n" + "=" * 70)
    print("EXAMPLE 1: Simple Dictionary Messages")
    print("=" * 70)

    hub = Hub("sensor_data")

    # Send various sensor readings
    sensors = [
        {"temp": 25.5, "humidity": 60, "pressure": 1013.25},
        {"temp": 26.0, "humidity": 58, "pressure": 1013.50},
        {"temp": 25.8, "humidity": 59, "pressure": 1013.30},
    ]

    for reading in sensors:
        hub.send(reading)
        print(f"Sent: {reading}")

    # Receive messages
    print("\nReceiving messages:")
    for _ in range(3):
        data = hub.recv()
        if data:
            print(f"Received: temp={data['temp']}°C, "
                  f"humidity={data['humidity']}%, "
                  f"pressure={data['pressure']} hPa")


def example_2_nested_structures():
    """Send complex nested data structures"""
    print("\n" + "=" * 70)
    print("EXAMPLE 2: Nested Data Structures")
    print("=" * 70)

    hub = Hub("robot_status")

    # Complex robot status message
    status = {
        "robot_id": "R001",
        "timestamp": time.time(),
        "position": {
            "x": 1.5,
            "y": 2.3,
            "theta": 0.785
        },
        "sensors": {
            "lidar": {"range": 10.5, "status": "ok"},
            "camera": {"fps": 30, "status": "ok"},
            "imu": {"accel": [0.1, 0.2, 9.8], "gyro": [0.0, 0.0, 0.1]}
        },
        "battery": {
            "voltage": 12.4,
            "current": 2.1,
            "percentage": 85
        },
        "active_tasks": ["navigation", "mapping", "obstacle_avoidance"]
    }

    hub.send(status)
    print("Sent complex robot status")

    # Receive and access nested data
    received = hub.recv()
    if received:
        print(f"\nRobot ID: {received['robot_id']}")
        print(f"Position: ({received['position']['x']}, "
              f"{received['position']['y']}, "
              f"θ={received['position']['theta']:.3f})")
        print(f"Battery: {received['battery']['percentage']}% "
              f"({received['battery']['voltage']}V)")
        print(f"Active tasks: {', '.join(received['active_tasks'])}")
        print(f"IMU acceleration: {received['sensors']['imu']['accel']}")


def example_3_lists_and_arrays():
    """Send lists and array-like data"""
    print("\n" + "=" * 70)
    print("EXAMPLE 3: Lists and Arrays")
    print("=" * 70)

    hub = Hub("measurements")

    # Send time series data
    time_series = {
        "timestamps": [0.0, 0.1, 0.2, 0.3, 0.4],
        "values": [1.0, 1.5, 2.0, 1.8, 1.2],
        "unit": "meters"
    }

    hub.send(time_series)
    print(f"Sent time series with {len(time_series['values'])} points")

    received = hub.recv()
    if received:
        avg = sum(received['values']) / len(received['values'])
        print(f"Average value: {avg:.2f} {received['unit']}")
        print(f"Data points: {list(zip(received['timestamps'], received['values']))}")


def example_4_pub_sub_custom():
    """Publisher/Subscriber with custom data"""
    print("\n" + "=" * 70)
    print("EXAMPLE 4: Pub/Sub with Custom Data")
    print("=" * 70)

    _sensor_hub = Hub("environmental_data")

    # Simulated environmental sensor
    def sensor_tick(node):
        t = time.time()
        data = {
            "timestamp": t,
            "temperature": 20 + 5 * math.sin(t * 0.5),
            "humidity": 50 + 10 * math.cos(t * 0.3),
            "air_quality": {
                "pm25": 15 + 5 * math.sin(t * 0.2),
                "pm10": 25 + 10 * math.cos(t * 0.15),
                "co2": 400 + 50 * math.sin(t * 0.1)
            },
            "location": "Room 101"
        }
        _sensor_hub.send(data, node)
        node.log_info(f"Temp: {data['temperature']:.1f}°C, "
                      f"Humidity: {data['humidity']:.1f}%, "
                      f"PM2.5: {data['air_quality']['pm25']:.1f}")

    # Data logger
    def logger_tick(node):
        data = _sensor_hub.recv(node)
        if data:
            node.log_info(f"[{data['location']}] "
                          f"T={data['temperature']:.1f}°C, "
                          f"H={data['humidity']:.1f}%, "
                          f"CO2={data['air_quality']['co2']:.0f}ppm")

    sensor = Node(name="env_sensor", tick=sensor_tick, rate=5)
    logger = Node(name="data_logger", tick=logger_tick, rate=5)

    print("Running environmental monitoring for 3 seconds...")
    print("(Ctrl+C to stop early)")

    try:
        run(sensor, logger, duration=3, logging=True)
    except KeyboardInterrupt:
        print("\nStopped by user")

    print("Done!")


def example_5_mixed_types():
    """Send various Python types"""
    print("\n" + "=" * 70)
    print("EXAMPLE 5: Mixed Data Types")
    print("=" * 70)

    hub = Hub("mixed_data")

    # Different data types
    test_data = [
        {"type": "string", "value": "Hello HORUS!"},
        {"type": "integer", "value": 42},
        {"type": "float", "value": 3.14159},
        {"type": "boolean", "value": True},
        {"type": "null", "value": None},
        {"type": "list", "value": [1, 2, 3, 4, 5]},
        {"type": "nested", "value": {"a": 1, "b": [2, 3], "c": {"d": 4}}},
    ]

    print("Sending various data types:")
    for item in test_data:
        hub.send(item)
        print(f"  {item['type']:10s}: {item['value']}")

    print("\nReceiving:")
    for _ in test_data:
        data = hub.recv()
        if data:
            print(f"  {data['type']:10s}: {data['value']} (type: {type(data['value']).__name__})")


def main():
    print("\n" + "=" * 70)
    print(" HORUS Python Bindings - Custom Data Examples")
    print("=" * 70)
    print("\nGeneric Hubs allow you to send ANY Python data:")
    print("  • Dictionaries (nested or flat)")
    print("  • Lists and tuples")
    print("  • Numbers (int, float)")
    print("  • Strings and booleans")
    print("  • Mixed and nested structures")
    print("  • Fast MessagePack serialization")

    example_1_simple_dict()
    example_2_nested_structures()
    example_3_lists_and_arrays()
    example_4_pub_sub_custom()
    example_5_mixed_types()

    print("\n" + "=" * 70)
    print(" Summary: Custom Data Support")
    print("=" * 70)
    print("\nHow to use:")
    print("  1. Create hub with string topic: hub = Hub('my_topic')")
    print("  2. Send any Python data: hub.send({'x': 1, 'y': 2})")
    print("  3. Receive as Python object: data = hub.recv()")
    print("\nAdvantages:")
    print("  [+] No message type definitions needed")
    print("  [+] Works with any serializable Python data")
    print("  [+] MessagePack: faster and smaller than JSON")
    print("  [+] Cross-language compatible")
    print("  [+] Type safety via Python duck typing")
    print("\nWhen to use:")
    print("  • Rapid prototyping")
    print("  • Dynamic/flexible data structures")
    print("  • Python-only projects")
    print("\nWhen to use typed Hubs instead:")
    print("  • Cross-language communication (Rust ↔ Python)")
    print("  • Maximum performance (zero-copy)")
    print("  • Compile-time type safety")
    print("=" * 70 + "\n")


if __name__ == "__main__":
    main()
