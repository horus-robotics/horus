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
    >>> imu = Imu()
    >>> battery = BatteryState(voltage=12.6, percentage=85.0)
    >>> gps = NavSatFix(latitude=37.7749, longitude=-122.4194, altitude=10.0)
"""

from ._library import *

__version__ = "0.1.4"

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
    "MotorCommand",
    "DifferentialDriveCommand",
    "ServoCommand",
    "PwmCommand",
    "StepperCommand",
    "PidConfig",
    # Sensor messages
    "LaserScan",
    "Imu",
    "BatteryState",
    "NavSatFix",
    "Odometry",
    "Range",
    # Diagnostics messages
    "Status",
    "EmergencyStop",
    "Heartbeat",
    "ResourceUsage",
    # Input messages
    "JoystickInput",
    "KeyboardInput",
    # I/O messages
    "DigitalIO",
    "AnalogIO",
]
