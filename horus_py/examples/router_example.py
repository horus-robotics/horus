#!/usr/bin/env python3
"""
HORUS Router Client Example - WAN/NAT Traversal Communication

This example demonstrates how to use HORUS Router for communication
across networks, through NAT, or for large-scale multi-node deployments.

Router is a TCP broker that enables:
- WAN communication (nodes across different networks)
- NAT traversal (nodes behind firewalls)
- Central message routing for large deployments
- Reliable delivery over unreliable networks

Architecture:
                     HORUS Router (TCP Broker)
                            |
            +---------------+---------------+
            |               |               |
        Node A          Node B          Node C
     (Publisher)      (Subscriber)    (Both P&S)
     [Home Network]   [Office Network] [Cloud VM]

The router acts as a central hub that:
1. Nodes connect to the router (outbound TCP - works through NAT)
2. Publishers send messages to router
3. Router forwards messages to all subscribers
4. Enables communication without direct peer-to-peer connectivity

Usage:
    # Method 1: Implicit router endpoint (uses default router discovery)
    python router_example.py --implicit

    # Method 2: Explicit router client (for custom router addresses)
    python router_example.py --explicit --router-host 192.168.1.100

    # Method 3: Helper functions
    python router_example.py --helpers
"""

import argparse
import time
import sys

from horus import (
    Hub,
    Link,
    RouterClient,
    RouterServer,
    default_router_endpoint,
    router_endpoint,
    CmdVel,
    Node,
)


def implicit_router_demo():
    """
    Demonstrate implicit router usage via endpoint syntax.

    This is the simplest way to use the router - just add "@router" to your topic.
    """
    print("=" * 60)
    print("HORUS Router: Implicit Endpoint Demo")
    print("=" * 60)
    print()

    print("Method 1: Using @router suffix (default router discovery)")
    print("-" * 40)

    # Simple: just add "@router" to auto-discover and connect to router
    endpoint_simple = "cmdvel@router"
    print(f"  Endpoint: {endpoint_simple}")
    print("  -> Automatically discovers router on localhost:7777")
    print()

    # Or specify router address directly
    endpoint_explicit = "cmdvel@192.168.1.100:7777"
    print(f"  Endpoint: {endpoint_explicit}")
    print("  -> Connects to router at 192.168.1.100:7777")
    print()

    print("Method 2: Using helper functions")
    print("-" * 40)

    # Helper function for default router
    ep1 = default_router_endpoint("sensor_data")
    print(f"  default_router_endpoint('sensor_data') -> '{ep1}'")

    # Helper function with custom address
    ep2 = router_endpoint("pose", "10.0.0.50", 7777)
    print(f"  router_endpoint('pose', '10.0.0.50', 7777) -> '{ep2}'")
    print()

    print("Creating Hub with router endpoint:")
    print("-" * 40)

    # Create a Hub that uses the router
    # Note: This requires a running router at the specified address
    try:
        hub = Hub(CmdVel, endpoint="cmdvel@router")
        print(f"  Hub created!")
        print(f"  Topic: {hub.topic()}")
        print(f"  Transport: {hub.transport_type}")
        print(f"  Is Network: {hub.is_network_hub}")
        print(f"  Endpoint: {hub.endpoint}")
    except Exception as e:
        print(f"  Note: Could not connect to router ({e})")
        print("  -> Make sure a router is running: `horus router start`")

    print()


def explicit_router_demo(router_host: str = "127.0.0.1", router_port: int = 7777):
    """
    Demonstrate explicit RouterClient usage for advanced scenarios.

    RouterClient provides:
    - Explicit connection management
    - Multiple topic support on same connection
    - Connection status monitoring
    - Topic tracking
    """
    print("=" * 60)
    print("HORUS Router: Explicit RouterClient Demo")
    print("=" * 60)
    print()

    print(f"Creating RouterClient for {router_host}:{router_port}")
    print("-" * 40)

    # Create explicit router client
    router = RouterClient(router_host, router_port)

    print(f"  Router: {router}")
    print(f"  Host: {router.host}")
    print(f"  Port: {router.port}")
    print(f"  Address: {router.address}")
    print(f"  Is Connected: {router.is_connected}")
    print()

    print("Building endpoints through router:")
    print("-" * 40)

    # Use router to build endpoints
    cmd_endpoint = router.endpoint("cmdvel")
    pose_endpoint = router.endpoint("pose")
    sensor_endpoint = router.endpoint("sensors")

    print(f"  cmdvel endpoint: {cmd_endpoint}")
    print(f"  pose endpoint: {pose_endpoint}")
    print(f"  sensors endpoint: {sensor_endpoint}")
    print()

    print(f"  Topics registered: {router.topics}")
    print(f"  Uptime: {router.uptime_seconds:.2f} seconds")
    print()

    print("Router info:")
    print("-" * 40)
    info = router.info()
    for key, value in info.items():
        print(f"  {key}: {value}")
    print()

    print("Using endpoints with Hub/Link:")
    print("-" * 40)
    print("  # Create Hub using the router endpoint")
    print(f"  hub = Hub(CmdVel, endpoint='{cmd_endpoint}')")
    print()
    print("  # Create Link using the router endpoint")
    print(f"  link = Link.producer(CmdVel, '{cmd_endpoint}')")
    print()


def helper_functions_demo():
    """
    Demonstrate the helper functions for building router endpoints.
    """
    print("=" * 60)
    print("HORUS Router: Helper Functions Demo")
    print("=" * 60)
    print()

    print("default_router_endpoint(topic)")
    print("-" * 40)
    print("  Returns: 'topic@router' (uses default router discovery)")
    print()

    topics = ["cmdvel", "pose2d", "laser_scan", "camera/rgb", "imu/data"]
    for topic in topics:
        ep = default_router_endpoint(topic)
        print(f"  default_router_endpoint('{topic}')")
        print(f"    -> '{ep}'")
    print()

    print("router_endpoint(topic, host, port)")
    print("-" * 40)
    print("  Returns: 'topic@host:port' (explicit router address)")
    print()

    configs = [
        ("cmdvel", "127.0.0.1", 7777),
        ("pose", "192.168.1.100", 7777),
        ("sensor_data", "10.0.0.50", 8888),
        ("telemetry", "router.mycompany.com", 7777),  # Note: DNS names work too
    ]

    for topic, host, port in configs:
        ep = router_endpoint(topic, host, port)
        print(f"  router_endpoint('{topic}', '{host}', {port})")
        print(f"    -> '{ep}'")
    print()


def router_server_info():
    """
    Demonstrate RouterServer class (informational).
    """
    print("=" * 60)
    print("HORUS Router: Server Info")
    print("=" * 60)
    print()

    print("Starting a HORUS Router:")
    print("-" * 40)
    print()
    print("  Option 1: CLI command (recommended for production)")
    print("    $ horus router start --port 7777")
    print()
    print("  Option 2: Python API (for testing/development)")
    print("    server = RouterServer(port=7777)")
    print("    server.start()")
    print()

    # Show RouterServer API
    server = RouterServer(port=7777)
    print(f"  RouterServer: {server}")
    print(f"  Port: {server.port}")
    print(f"  Is Running: {server.is_running}")
    print()

    print("Router Features:")
    print("-" * 40)
    print("  - TCP-based for reliable delivery")
    print("  - Automatic reconnection")
    print("  - NAT traversal (outbound connections only)")
    print("  - Message buffering during disconnects")
    print("  - Topic-based routing")
    print("  - Zero-copy forwarding where possible")
    print()


def main():
    parser = argparse.ArgumentParser(
        description="HORUS Router Client Example",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Show implicit endpoint syntax
  python router_example.py --implicit

  # Show explicit RouterClient usage
  python router_example.py --explicit --router-host 192.168.1.100

  # Show helper functions
  python router_example.py --helpers

  # Show router server info
  python router_example.py --server-info

  # Show all demos
  python router_example.py --all
        """
    )

    parser.add_argument("--implicit", action="store_true",
                        help="Show implicit router endpoint demo")
    parser.add_argument("--explicit", action="store_true",
                        help="Show explicit RouterClient demo")
    parser.add_argument("--helpers", action="store_true",
                        help="Show helper functions demo")
    parser.add_argument("--server-info", action="store_true",
                        help="Show router server info")
    parser.add_argument("--all", action="store_true",
                        help="Show all demos")
    parser.add_argument("--router-host", type=str, default="127.0.0.1",
                        help="Router host address (default: 127.0.0.1)")
    parser.add_argument("--router-port", type=int, default=7777,
                        help="Router port (default: 7777)")

    args = parser.parse_args()

    # If no specific option, show all
    if not any([args.implicit, args.explicit, args.helpers, args.server_info, args.all]):
        args.all = True

    if args.all or args.implicit:
        implicit_router_demo()

    if args.all or args.explicit:
        explicit_router_demo(args.router_host, args.router_port)

    if args.all or args.helpers:
        helper_functions_demo()

    if args.all or args.server_info:
        router_server_info()


if __name__ == "__main__":
    main()
