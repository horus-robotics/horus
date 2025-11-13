"""
Standardized message types for cross-language communication.

These message classes mirror the Rust message types in horus_library,
providing a consistent API across Python and Rust.

All messages are automatically serialized to MessagePack when sent via PyHub.
"""

import time
from typing import Optional, List
from dataclasses import dataclass, asdict


@dataclass
class Pose2D:
    """2D pose representation (position and orientation)

    Mirrors: horus_library::messages::geometry::Pose2D

    Args:
        x: X position in meters
        y: Y position in meters
        theta: Orientation angle in radians
        timestamp: Timestamp in nanoseconds (auto-generated if None)

    Example:
        >>> pose = Pose2D(x=1.0, y=2.0, theta=0.5)
        >>> hub = Hub(Pose2D)  # Type determines topic
        >>> hub.send(pose, node)
    """
    __topic_name__ = "robot_pose"  # Default topic for cross-language communication

    x: float
    y: float
    theta: float
    timestamp: int = None

    def __post_init__(self):
        if self.timestamp is None:
            self.timestamp = int(time.time() * 1e9)

    def to_dict(self):
        """Convert to dictionary for serialization"""
        return asdict(self)


@dataclass
class CmdVel:
    """Command velocity message for robot control

    Mirrors: horus_library::messages::cmd_vel::CmdVel

    Args:
        linear: Linear velocity in m/s
        angular: Angular velocity in rad/s
        stamp_nanos: Timestamp in nanoseconds (auto-generated if None)

    Example:
        >>> cmd = CmdVel(linear=1.5, angular=0.5)
        >>> hub = Hub(CmdVel)  # Type determines topic
        >>> hub.send(cmd, node)
    """
    __topic_name__ = "cmd_vel"  # Default topic for cross-language communication

    linear: float
    angular: float
    stamp_nanos: int = None

    def __post_init__(self):
        if self.stamp_nanos is None:
            self.stamp_nanos = int(time.time() * 1e9)

    def to_dict(self):
        """Convert to dictionary for serialization"""
        return asdict(self)


@dataclass
class Twist:
    """3D velocity command with linear and angular components

    Mirrors: horus_library::messages::geometry::Twist

    Args:
        linear: Linear velocity [x, y, z] in m/s
        angular: Angular velocity [roll, pitch, yaw] in rad/s
        timestamp: Timestamp in nanoseconds (auto-generated if None)

    Example:
        >>> twist = Twist(linear=[1.0, 0.0, 0.0], angular=[0.0, 0.0, 0.5])
        >>> hub.send(twist, node)
    """
    linear: List[float]
    angular: List[float]
    timestamp: int = None

    def __post_init__(self):
        if self.timestamp is None:
            self.timestamp = int(time.time() * 1e9)
        if len(self.linear) != 3:
            raise ValueError("linear must have 3 elements [x, y, z]")
        if len(self.angular) != 3:
            raise ValueError("angular must have 3 elements [roll, pitch, yaw]")

    def to_dict(self):
        """Convert to dictionary for serialization"""
        return asdict(self)


@dataclass
class Point3:
    """3D point representation

    Mirrors: horus_library::messages::geometry::Point3

    Args:
        x: X coordinate
        y: Y coordinate
        z: Z coordinate
    """
    x: float
    y: float
    z: float

    def to_dict(self):
        """Convert to dictionary for serialization"""
        return asdict(self)


# Add more message types as needed...
# This provides a starting point with the most common messages
