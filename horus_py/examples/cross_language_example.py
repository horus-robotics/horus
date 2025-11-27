#!/usr/bin/env python3
"""
HORUS Cross-Language Communication Example

This example demonstrates communication between Python and Rust nodes
using HORUS's unified network communication layer.

The same endpoint can be used by both Rust and Python nodes, enabling
seamless cross-language robotics applications.

Architecture:
                    HORUS Network Layer
    Python Node  <---> [UDP/SharedMem] <---> Rust Node
       (Hub)              (bincode)           (Hub)

Message types are serialized using a common format (bincode for typed messages)
that both languages understand.

Usage:
    # Run Python publisher, Rust subscriber
    python cross_language_example.py --mode python-pub --endpoint "cmdvel@192.168.1.5:9000"

    # In another terminal, run Rust subscriber (example)
    horus run your_rust_node --endpoint "cmdvel@192.168.1.5:9000"

    # Or vice versa - Python subscribing to Rust publisher
    python cross_language_example.py --mode python-sub --endpoint "cmdvel@192.168.1.5:9000"

Cross-Language Scenarios:
1. Python sensor driver -> Rust perception node
2. Rust planner -> Python ML inference node
3. Python teleop -> Rust motor controller
"""

import argparse
import time
import sys
import json

from horus import Hub, Link, CmdVel, Pose2D, Node


def python_publisher(endpoint: str, rate_hz: float = 10.0):
    """
    Python node publishing CmdVel that Rust nodes can subscribe to.

    This demonstrates sending typed messages from Python that Rust can receive.
    """
    print("=" * 60)
    print("HORUS Cross-Language: Python Publisher")
    print("=" * 60)
    print()

    # Create Hub with network endpoint
    hub = Hub(CmdVel, endpoint=endpoint)

    print(f"Configuration:")
    print(f"  Endpoint: {endpoint}")
    print(f"  Transport: {hub.transport_type}")
    print(f"  Topic: {hub.topic()}")
    print()
    print("This Python node is publishing CmdVel messages.")
    print("Any Rust node can subscribe to the same endpoint to receive them.")
    print()
    print(f"Publishing at {rate_hz} Hz. Press Ctrl+C to stop.")
    print()

    node = Node("py_cross_lang_pub")
    interval = 1.0 / rate_hz
    msg_count = 0

    try:
        while True:
            # Generate velocity command
            t = time.time()
            linear = 1.0 + 0.5 * (t % 10) / 10.0
            angular = 0.2 * ((t % 20) - 10) / 10.0

            cmd = CmdVel(linear, angular)
            hub.send(cmd, node)
            msg_count += 1

            if msg_count % 10 == 0:
                print(f"[Python] Sent #{msg_count}: linear={linear:.3f}, angular={angular:.3f}")

            time.sleep(interval)

    except KeyboardInterrupt:
        stats = hub.stats()
        print(f"\n[Python] Publisher stopped. Messages: {stats['messages_sent']}")


def python_subscriber(endpoint: str):
    """
    Python node subscribing to CmdVel from Rust publishers.

    This demonstrates receiving typed messages from Rust in Python.
    """
    print("=" * 60)
    print("HORUS Cross-Language: Python Subscriber")
    print("=" * 60)
    print()

    # Create Hub with network endpoint
    hub = Hub(CmdVel, endpoint=endpoint)

    print(f"Configuration:")
    print(f"  Endpoint: {endpoint}")
    print(f"  Transport: {hub.transport_type}")
    print(f"  Topic: {hub.topic()}")
    print()
    print("This Python node is subscribing to CmdVel messages.")
    print("It can receive messages from any Rust node publishing to the same endpoint.")
    print()
    print("Waiting for messages. Press Ctrl+C to stop.")
    print()

    node = Node("py_cross_lang_sub")
    msg_count = 0

    try:
        while True:
            msg = hub.recv(node)
            if msg:
                msg_count += 1
                print(f"[Python] Received #{msg_count}: linear={msg.linear:.3f}, "
                      f"angular={msg.angular:.3f}, timestamp={msg.timestamp}")
            else:
                time.sleep(0.001)

    except KeyboardInterrupt:
        stats = hub.stats()
        print(f"\n[Python] Subscriber stopped. Messages: {stats['messages_received']}")


def python_pose_publisher(endpoint: str, rate_hz: float = 20.0):
    """
    Python node publishing Pose2D (e.g., localization result).

    Common use case: Python SLAM/localization node publishing poses
    that Rust navigation nodes consume.
    """
    print("=" * 60)
    print("HORUS Cross-Language: Python Pose Publisher")
    print("=" * 60)
    print()

    hub = Hub(Pose2D, endpoint=endpoint)

    print(f"Publishing Pose2D at {rate_hz} Hz to: {endpoint}")
    print(f"Transport: {hub.transport_type}")
    print()

    node = Node("py_localization")
    interval = 1.0 / rate_hz
    msg_count = 0

    # Simulate robot moving in a circle
    x, y, theta = 0.0, 0.0, 0.0
    v = 0.5  # m/s
    omega = 0.2  # rad/s

    try:
        while True:
            # Update pose (simple kinematic model)
            dt = interval
            x += v * dt * math.cos(theta)
            y += v * dt * math.sin(theta)
            theta += omega * dt

            # Create and send pose
            timestamp = int(time.time_ns())
            pose = Pose2D(x, y, theta, timestamp)
            hub.send(pose, node)
            msg_count += 1

            if msg_count % 20 == 0:
                print(f"[Python] Pose #{msg_count}: x={x:.2f}, y={y:.2f}, theta={theta:.2f}")

            time.sleep(interval)

    except KeyboardInterrupt:
        print(f"\n[Python] Pose publisher stopped. Total: {msg_count}")


def generic_data_demo(endpoint: str):
    """
    Demonstrate generic data communication (JSON-serializable Python dicts).

    This allows sending arbitrary Python data structures to Rust generic handlers.
    """
    print("=" * 60)
    print("HORUS Cross-Language: Generic Data Demo")
    print("=" * 60)
    print()

    # Create a generic Hub (not typed to CmdVel/Pose2D)
    hub = Hub("sensor_data", endpoint=endpoint)

    print(f"Sending generic Python data to: {endpoint}")
    print(f"Transport: {hub.transport_type}")
    print()

    # Send various Python data structures
    data_samples = [
        {"type": "lidar", "ranges": [1.0, 1.5, 2.0, 1.8, 1.2], "timestamp": time.time()},
        {"type": "imu", "accel": [0.1, 0.0, 9.8], "gyro": [0.01, 0.02, 0.0]},
        {"type": "battery", "voltage": 24.5, "current": 2.3, "soc": 85.0},
        {"type": "status", "state": "running", "uptime_sec": 3600},
    ]

    for i, data in enumerate(data_samples):
        hub.send(data)
        print(f"[Python] Sent generic data #{i+1}: {json.dumps(data)[:60]}...")
        time.sleep(0.5)

    print()
    print("Generic data sent! Rust nodes using Hub<GenericMessage> can receive this.")


def print_architecture():
    """Print cross-language architecture diagram."""
    print("""
HORUS Cross-Language Communication Architecture
================================================

Python Application                    Rust Application
+------------------+                  +------------------+
|  from horus import|                  |  use horus::Hub; |
|  Hub, CmdVel     |                  |                  |
|                  |                  |                  |
|  hub = Hub(      |   Network        |  let hub =       |
|    CmdVel,       | <----------->    |    Hub::<CmdVel> |
|    endpoint=...) |   (bincode)      |    ::new_with..  |
|                  |                  |                  |
|  hub.send(cmd)   |                  |  hub.send(cmd)   |
|  msg = hub.recv()|                  |  hub.recv()      |
+------------------+                  +------------------+

Supported Transports:
- Shared Memory (local): ~250ns latency
- Unix Socket (localhost): ~1-2us latency
- UDP Direct (LAN): ~3-8us latency
- Router/TCP (WAN): ~10-50ms latency

Message Types:
- CmdVel: Velocity commands (linear, angular)
- Pose2D: 2D poses (x, y, theta, timestamp)
- Generic: Any JSON-serializable Python dict

Endpoint Syntax:
- "topic"                 -> Local shared memory
- "topic@192.168.1.5:9000" -> Direct UDP
- "topic@localhost"       -> Unix domain socket
- "topic@router"          -> Via HORUS router
- "topic@*"               -> Multicast discovery
""")


def main():
    parser = argparse.ArgumentParser(
        description="HORUS Cross-Language Communication Example",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Show architecture diagram
  python cross_language_example.py --mode info

  # Python publishing, Rust subscribing
  python cross_language_example.py --mode python-pub --endpoint "cmdvel@192.168.1.5:9000"

  # Python subscribing, Rust publishing
  python cross_language_example.py --mode python-sub --endpoint "cmdvel@192.168.1.5:9000"

  # Python pose publisher (localization)
  python cross_language_example.py --mode pose-pub --endpoint "pose@192.168.1.5:9001"

  # Generic data demo
  python cross_language_example.py --mode generic --endpoint "sensor_data@192.168.1.5:9002"
        """
    )

    parser.add_argument("--mode", type=str, required=True,
                        choices=["python-pub", "python-sub", "pose-pub", "generic", "info"],
                        help="Operating mode")
    parser.add_argument("--endpoint", type=str, default="cmdvel",
                        help="Network endpoint")
    parser.add_argument("--rate", type=float, default=10.0,
                        help="Publishing rate in Hz")

    args = parser.parse_args()

    if args.mode == "info":
        print_architecture()
    elif args.mode == "python-pub":
        python_publisher(args.endpoint, args.rate)
    elif args.mode == "python-sub":
        python_subscriber(args.endpoint)
    elif args.mode == "pose-pub":
        import math
        python_pose_publisher(args.endpoint, args.rate)
    elif args.mode == "generic":
        generic_data_demo(args.endpoint)


if __name__ == "__main__":
    main()
