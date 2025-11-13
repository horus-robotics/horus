#!/usr/bin/env python3
"""
Python Logger Node - Multi-Language Example

Subscribes to velocity commands using standardized CmdVel message.
Demonstrates seamless Rust -> Python communication with typed messages.
"""

import horus
from horus import CmdVel, Hub

# Create typed hub - type determines topic name and memory layout
_cmd_hub = Hub(CmdVel)


def tick(node):
    """Called at 5Hz - logs velocity commands from Rust node"""
    # Receive typed CmdVel object from Rust (automatic deserialization!)
    cmd = _cmd_hub.recv(node)

    if cmd:
        # Access fields directly - same API as Rust!
        # Automatic logging already happened in recv
        # print(f"Logger: CmdVel(linear={cmd.linear:.2f}, angular={cmd.angular:.2f})")
        pass


def main():
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
