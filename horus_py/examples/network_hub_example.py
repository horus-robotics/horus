#!/usr/bin/env python3
"""
HORUS Network Hub Example - Distributed Pub/Sub Communication

This example demonstrates how to use HORUS Hub for network communication
between Python nodes running on different machines or processes.

Hub supports multiple transport backends automatically selected based on endpoint:
- Local shared memory: "topic" (fastest, ~250ns)
- Unix socket: "topic@localhost" (Unix only, ~1-2us)
- Direct UDP: "topic@192.168.1.5:9000" (LAN, ~3-8us)
- Multicast: "topic@*" (discovery)
- Router: "topic@router" (TCP broker for WAN)

Usage:
    # Terminal 1 - Run publisher
    python network_hub_example.py --publish --endpoint "cmdvel@192.168.1.5:9000"

    # Terminal 2 - Run subscriber
    python network_hub_example.py --subscribe --endpoint "cmdvel@192.168.1.5:9000"
"""

import argparse
import time
import sys

from horus import Hub, CmdVel, Pose2D, Node


def run_publisher(endpoint: str, rate_hz: float = 10.0):
    """
    Run a publisher that sends CmdVel messages over the network.

    Args:
        endpoint: Network endpoint (e.g., "cmdvel@192.168.1.5:9000")
        rate_hz: Publishing rate in Hz
    """
    print(f"Creating publisher Hub with endpoint: {endpoint}")

    # Create a Hub with network endpoint
    # The endpoint format determines the transport:
    # - "cmdvel" -> local shared memory
    # - "cmdvel@192.168.1.5:9000" -> UDP to specific host
    # - "cmdvel@localhost" -> Unix domain socket
    hub = Hub(CmdVel, endpoint=endpoint)

    print(f"Hub created:")
    print(f"  Topic: {hub.topic()}")
    print(f"  Transport: {hub.transport_type}")
    print(f"  Is Network: {hub.is_network_hub}")
    print(f"  Endpoint: {hub.endpoint}")
    print()

    # Create a node for logging (optional)
    node = Node("py_publisher")

    interval = 1.0 / rate_hz
    msg_count = 0

    print(f"Publishing at {rate_hz} Hz. Press Ctrl+C to stop.")
    print()

    try:
        while True:
            # Create a velocity command with varying values
            t = time.time()
            linear = 1.0 + 0.5 * (t % 10) / 10.0  # 1.0 to 1.5 m/s
            angular = 0.3 * ((t % 20) - 10) / 10.0  # -0.3 to 0.3 rad/s

            cmd = CmdVel(linear, angular)

            # Send the message
            success = hub.send(cmd, node)
            msg_count += 1

            if msg_count % 10 == 0:
                stats = hub.stats()
                print(f"[{msg_count}] Sent: linear={linear:.2f}, angular={angular:.2f} | "
                      f"Total sent: {stats['messages_sent']}, failures: {stats['send_failures']}")

            time.sleep(interval)

    except KeyboardInterrupt:
        print(f"\nPublisher stopped. Total messages sent: {msg_count}")
        print(f"Final stats: {hub.stats()}")


def run_subscriber(endpoint: str, timeout_ms: int = 100):
    """
    Run a subscriber that receives CmdVel messages from the network.

    Args:
        endpoint: Network endpoint (e.g., "cmdvel@192.168.1.5:9000")
        timeout_ms: Receive timeout in milliseconds
    """
    print(f"Creating subscriber Hub with endpoint: {endpoint}")

    # Create a Hub with the same endpoint as the publisher
    hub = Hub(CmdVel, endpoint=endpoint)

    print(f"Hub created:")
    print(f"  Topic: {hub.topic()}")
    print(f"  Transport: {hub.transport_type}")
    print(f"  Is Network: {hub.is_network_hub}")
    print(f"  Endpoint: {hub.endpoint}")
    print()

    # Create a node for logging (optional)
    node = Node("py_subscriber")

    msg_count = 0

    print(f"Waiting for messages. Press Ctrl+C to stop.")
    print()

    try:
        while True:
            # Try to receive a message
            msg = hub.recv(node)

            if msg is not None:
                msg_count += 1
                print(f"[{msg_count}] Received: linear={msg.linear:.3f}, angular={msg.angular:.3f}, "
                      f"timestamp={msg.timestamp}")
            else:
                # No message available, small sleep to avoid busy-waiting
                time.sleep(0.001)

    except KeyboardInterrupt:
        print(f"\nSubscriber stopped. Total messages received: {msg_count}")
        print(f"Final stats: {hub.stats()}")


def run_local_test():
    """
    Run a local test with shared memory (fastest, single process).
    """
    print("Running local shared memory test...")
    print()

    # Create publisher and subscriber hubs (local shared memory)
    pub_hub = Hub(CmdVel)  # No endpoint = local
    sub_hub = Hub(CmdVel)  # Same topic, local

    print(f"Publisher transport: {pub_hub.transport_type}")
    print(f"Subscriber transport: {sub_hub.transport_type}")
    print()

    # Send some messages
    for i in range(5):
        cmd = CmdVel(float(i), float(i) * 0.1)
        pub_hub.send(cmd)
        print(f"Sent: linear={cmd.linear}, angular={cmd.angular}")

    print()

    # Receive messages
    while True:
        msg = sub_hub.recv()
        if msg is None:
            break
        print(f"Received: linear={msg.linear}, angular={msg.angular}")

    print()
    print("Local test complete!")


def main():
    parser = argparse.ArgumentParser(
        description="HORUS Network Hub Example",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Local shared memory test
  python network_hub_example.py --local

  # Publisher on network
  python network_hub_example.py --publish --endpoint "cmdvel@192.168.1.5:9000"

  # Subscriber on network
  python network_hub_example.py --subscribe --endpoint "cmdvel@192.168.1.5:9000"

  # Via router (for WAN/NAT traversal)
  python network_hub_example.py --publish --endpoint "cmdvel@router"
        """
    )

    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--publish", action="store_true", help="Run as publisher")
    group.add_argument("--subscribe", action="store_true", help="Run as subscriber")
    group.add_argument("--local", action="store_true", help="Run local test")

    parser.add_argument("--endpoint", type=str, default="cmdvel",
                        help="Network endpoint (default: cmdvel for local)")
    parser.add_argument("--rate", type=float, default=10.0,
                        help="Publishing rate in Hz (default: 10)")

    args = parser.parse_args()

    if args.local:
        run_local_test()
    elif args.publish:
        run_publisher(args.endpoint, args.rate)
    elif args.subscribe:
        run_subscriber(args.endpoint)


if __name__ == "__main__":
    main()
