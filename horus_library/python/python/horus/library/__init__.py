"""
HORUS Library - Standard robotics messages, nodes, and algorithms

This package provides typed message classes, standard nodes, and common algorithms
for robotics applications using HORUS.

Example:
    >>> from horus.library import Pose2D, Twist, CmdVel, LaserScan
    >>> pose = Pose2D(x=1.0, y=2.0, theta=0.5)
    >>> cmd = Twist.new_2d(linear_x=0.5, angular_z=0.1)
    >>> vel = CmdVel(linear=1.0, angular=0.5)
    >>> scan = LaserScan()
"""

from ._library import *

__version__ = "0.1.3"

__all__ = [
    # Geometry messages
    "Pose2D",
    "Twist",
    "Transform",
    "Point3",
    "Vector3",
    "Quaternion",
    # Control messages
    "CmdVel",
    # Sensor messages
    "LaserScan",
]
