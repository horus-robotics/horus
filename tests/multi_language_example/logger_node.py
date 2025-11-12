#!/usr/bin/env python3
"""
Python Logger Node - Multi-Language Example

Subscribes to velocity commands from Rust node to demonstrate Rust -> Python communication.
Uses generic PyHub with MessagePack deserialization for cross-language compatibility.
"""

import horus
from horus._horus import PyHub
import pickle

# Create generic hub for cross-language communication (once, outside tick)
_cmd_hub = PyHub("cmd_vel")


def tick(node):
    """Called at 5Hz - logs velocity commands from Rust node"""
    # Try to receive velocity command with automatic logging
    msg_bytes = _cmd_hub.recv(node)

    if msg_bytes:
        # Deserialize message (MessagePack format from Rust)
        import msgpack
        cmd = msgpack.unpackb(msg_bytes, raw=False)

        # Calculate speed magnitude
        linear = cmd.get('linear', 0.0)
        angular = cmd.get('angular', 0.0)
        speed = abs(linear) + abs(angular)
        status = "MOVING" if speed > 0.1 else "STOPPED"

        node.log_info(f"Received cmd from Rust: linear={linear:.2f} m/s, "
                      f"angular={angular:.2f} rad/s [{status}]")
    else:
        node.log_debug("No command received (waiting for controller)")


def main():
    print("=" * 60)
    print("Python Logger Node - Multi-Language Example")
    print("=" * 60)
    print("Subscribing to 'cmd_vel' from Rust controller at 5Hz")
    print()

    # Create node with 5Hz tick rate (slower logging)
    node = horus.Node(
        name="logger_node",
        tick=tick,
        rate=5  # 5Hz
    )

    # Run forever
    horus.run(node)


if __name__ == "__main__":
    main()
