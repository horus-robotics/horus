#!/usr/bin/env python3
"""
HORUS Network Link Example - Point-to-Point SPSC Communication

This example demonstrates how to use HORUS Link for direct point-to-point
communication between two nodes. Link is optimized for Single Producer,
Single Consumer (SPSC) patterns with lower latency than Hub.

Link vs Hub:
- Link: 1P1C (one producer, one consumer), direct connection, ~30% faster
- Hub: MPMC (many producers, many consumers), pub/sub pattern

Link transport backends:
- Local shared memory: "topic" (fastest, ~250ns)
- Connected UDP: "topic@192.168.1.5:9000" (optimized for 1P1C, ~3-5us)
- Unix socket: "topic@localhost" (Unix only, ~1-2us)

Usage:
    # Terminal 1 - Run producer (sender)
    python network_link_example.py --producer --endpoint "sensor@192.168.1.5:9000"

    # Terminal 2 - Run consumer (receiver)
    python network_link_example.py --consumer --endpoint "sensor@0.0.0.0:9000"

Note: For network Links:
- Producer connects TO the consumer's address
- Consumer binds/listens ON their own address
"""

import argparse
import time
import sys

from horus import Link, CmdVel, Pose2D


def run_producer(endpoint: str, rate_hz: float = 10.0):
    """
    Run a Link producer that sends CmdVel messages.

    Args:
        endpoint: Network endpoint (producer connects to this address)
        rate_hz: Send rate in Hz
    """
    print(f"Creating Link producer with endpoint: {endpoint}")
    print()

    # Create a Link producer
    # For network: producer connects to the consumer's address
    producer = Link.producer(CmdVel, endpoint)

    print(f"Link producer created:")
    print(f"  Topic: {producer.topic}")
    print(f"  Endpoint: {producer.endpoint}")
    print(f"  Transport: {producer.transport_type}")
    print(f"  Is Producer: {producer.is_producer}")
    print(f"  Is Network: {producer.is_network}")
    print()

    interval = 1.0 / rate_hz
    msg_count = 0

    print(f"Sending at {rate_hz} Hz. Press Ctrl+C to stop.")
    print()

    try:
        while True:
            # Create sensor data with varying values
            t = time.time()
            linear = 2.0 + 0.5 * (t % 5) / 5.0  # 2.0 to 2.5
            angular = 0.5 * ((t % 10) - 5) / 5.0  # -0.5 to 0.5

            cmd = CmdVel(linear, angular)

            # Send via Link
            success = producer.send(cmd)
            msg_count += 1

            if msg_count % 10 == 0:
                print(f"[{msg_count}] Sent: linear={linear:.3f}, angular={angular:.3f} | "
                      f"success={success}")

            time.sleep(interval)

    except KeyboardInterrupt:
        print(f"\nProducer stopped. Total messages sent: {msg_count}")


def run_consumer(endpoint: str):
    """
    Run a Link consumer that receives CmdVel messages.

    Args:
        endpoint: Network endpoint (consumer listens on this address)
    """
    print(f"Creating Link consumer with endpoint: {endpoint}")
    print()

    # Create a Link consumer
    # For network: consumer binds/listens on their own address
    consumer = Link.consumer(CmdVel, endpoint)

    print(f"Link consumer created:")
    print(f"  Topic: {consumer.topic}")
    print(f"  Endpoint: {consumer.endpoint}")
    print(f"  Transport: {consumer.transport_type}")
    print(f"  Is Consumer: {consumer.is_consumer}")
    print(f"  Is Network: {consumer.is_network}")
    print()

    msg_count = 0

    print(f"Waiting for messages. Press Ctrl+C to stop.")
    print()

    try:
        while True:
            # Try to receive
            msg = consumer.recv()

            if msg is not None:
                msg_count += 1
                print(f"[{msg_count}] Received: linear={msg.linear:.3f}, angular={msg.angular:.3f}, "
                      f"timestamp={msg.timestamp}")
            else:
                # No message available
                time.sleep(0.001)

    except KeyboardInterrupt:
        print(f"\nConsumer stopped. Total messages received: {msg_count}")


def run_local_test():
    """
    Run a local test with shared memory (single process demonstration).
    """
    print("Running local Link test (shared memory)...")
    print()

    # Create producer and consumer with the same topic (local shared memory)
    producer = Link.producer(CmdVel, "sensor_test")
    consumer = Link.consumer(CmdVel, "sensor_test")

    print(f"Producer: {producer}")
    print(f"Consumer: {consumer}")
    print()

    # Send some messages
    for i in range(5):
        cmd = CmdVel(float(i + 1) * 0.5, float(i) * 0.1)
        success = producer.send(cmd)
        print(f"Sent: linear={cmd.linear:.1f}, angular={cmd.angular:.2f} (success={success})")

    print()

    # Link is single-slot (overwrites), so we'll get the latest value
    msg = consumer.recv()
    if msg:
        print(f"Received latest: linear={msg.linear}, angular={msg.angular}")
    else:
        print("No message received")

    print()
    print("Local Link test complete!")
    print()
    print("Note: Link uses single-slot design (always latest value).")
    print("For buffered communication, use Hub instead.")


def run_bidirectional_example():
    """
    Demonstrate bidirectional communication with two Links.
    """
    print("Running bidirectional Link example...")
    print()

    # Create two separate Links for bidirectional communication
    # This simulates two processes communicating both ways

    # Process A -> Process B (commands)
    cmd_producer = Link.producer(CmdVel, "commands")
    cmd_consumer = Link.consumer(CmdVel, "commands")

    # Process B -> Process A (feedback)
    feedback_producer = Link.producer(Pose2D, "feedback")
    feedback_consumer = Link.consumer(Pose2D, "feedback")

    print("Links created for bidirectional communication:")
    print(f"  Commands:  {cmd_producer.topic} ({cmd_producer.transport_type})")
    print(f"  Feedback:  {feedback_producer.topic} ({feedback_producer.transport_type})")
    print()

    # Simulate A sending command to B
    cmd = CmdVel(1.5, 0.3)
    cmd_producer.send(cmd)
    print(f"A -> B: Sent command linear={cmd.linear}, angular={cmd.angular}")

    # B receives command
    received_cmd = cmd_consumer.recv()
    if received_cmd:
        print(f"B received: linear={received_cmd.linear}, angular={received_cmd.angular}")

    # B sends feedback to A
    pose = Pose2D(1.0, 2.0, 0.5)
    feedback_producer.send(pose)
    print(f"B -> A: Sent feedback x={pose.x}, y={pose.y}, theta={pose.theta}")

    # A receives feedback
    received_pose = feedback_consumer.recv()
    if received_pose:
        print(f"A received: x={received_pose.x}, y={received_pose.y}, theta={received_pose.theta}")

    print()
    print("Bidirectional Link example complete!")


def main():
    parser = argparse.ArgumentParser(
        description="HORUS Network Link Example",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Local shared memory test
  python network_link_example.py --local

  # Bidirectional communication example
  python network_link_example.py --bidirectional

  # Network producer (connects to consumer)
  python network_link_example.py --producer --endpoint "sensor@192.168.1.5:9000"

  # Network consumer (listens for producer)
  python network_link_example.py --consumer --endpoint "sensor@0.0.0.0:9000"

Link vs Hub:
  - Link: Direct 1P1C, single-slot (always latest), ~30% faster
  - Hub:  MPMC pub/sub, buffered, supports multiple subscribers
        """
    )

    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--producer", action="store_true", help="Run as producer")
    group.add_argument("--consumer", action="store_true", help="Run as consumer")
    group.add_argument("--local", action="store_true", help="Run local test")
    group.add_argument("--bidirectional", action="store_true",
                       help="Run bidirectional example")

    parser.add_argument("--endpoint", type=str, default="sensor",
                        help="Network endpoint (default: sensor for local)")
    parser.add_argument("--rate", type=float, default=10.0,
                        help="Send rate in Hz (default: 10)")

    args = parser.parse_args()

    if args.local:
        run_local_test()
    elif args.bidirectional:
        run_bidirectional_example()
    elif args.producer:
        run_producer(args.endpoint, args.rate)
    elif args.consumer:
        run_consumer(args.endpoint)


if __name__ == "__main__":
    main()
