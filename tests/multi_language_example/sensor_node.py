#!/usr/bin/env python3
"""
Python Sensor Node - Multi-Language Example

Publishes robot pose data at 10Hz to demonstrate Python -> Rust communication.
Uses generic PyHub with MessagePack serialization for cross-language compatibility.
"""

import horus
from horus._horus import PyHub
import math
import time

# Track start time for elapsed time calculation
_start_time = time.time()

# Create generic hub for cross-language communication (once, outside tick)
_pose_hub = PyHub("robot_pose")


def tick(node):
    """Called at 10Hz - simulates a robot moving in a circle"""
    # Simulate robot moving in a circle
    t = time.time() - _start_time  # Use elapsed time, not epoch time
    x = 2.0 * math.cos(t * 0.5)
    y = 2.0 * math.sin(t * 0.5)
    theta = t * 0.5 + math.pi / 2  # Tangent to circle

    # Send pose message as dict (will be serialized with MessagePack)
    pose = {"x": x, "y": y, "theta": theta}
    _pose_hub.send(pose, node)  # Automatic logging

    node.log_info(f"Published pose: x={x:.2f}, y={y:.2f}, theta={theta:.2f} rad")


def main():
    print("=" * 60)
    print("Python Sensor Node - Multi-Language Example")
    print("=" * 60)
    print("Publishing robot poses at 10Hz on topic 'robot_pose'")
    print("Simulating robot moving in a circle (radius 2m)")
    print()

    # Create node with 10Hz tick rate
    node = horus.Node(
        name="sensor_node",
        tick=tick,
        rate=10  # 10Hz
    )

    # Run forever
    horus.run(node)


if __name__ == "__main__":
    main()
