#!/usr/bin/env python3
"""
Python Sensor Node - Multi-Language Example

Publishes robot pose data at 10Hz using standardized Pose2D message.
Demonstrates seamless Python -> Rust communication with typed messages.
"""

import horus
from horus import Pose2D, Hub  # Standardized message type and Hub
import math
import time

# Track start time for elapsed time calculation
_start_time = time.time()

# Create typed hub - type determines topic name and memory layout
_pose_hub = Hub(Pose2D)


def tick(node):
    """Called at 10Hz - simulates a robot moving in a circle"""
    # Simulate robot moving in a circle
    t = time.time() - _start_time  # Use elapsed time, not epoch time
    x = 2.0 * math.cos(t * 0.5)
    y = 2.0 * math.sin(t * 0.5)
    theta = t * 0.5 + math.pi / 2  # Tangent to circle

    # Create typed message - same API as Rust!
    pose = Pose2D(x=x, y=y, theta=theta)
    _pose_hub.send(pose, node)  # Automatic serialization and logging

def main():
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
