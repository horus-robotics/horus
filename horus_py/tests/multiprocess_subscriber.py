#!/usr/bin/env python3
"""
Multiprocess Test: Subscriber Node

Run with: horus run multiprocess_publisher.py multiprocess_subscriber.py
"""

import horus
import time

received_count = [0]

def subscriber_tick(node):
    """Receive messages from shared topic"""
    if node.has_msg("multiprocess_topic"):
        msg = node.get("multiprocess_topic")
        received_count[0] += 1
        print(f"[Subscriber] Received message #{received_count[0]}: {msg}")

# Create subscriber node
subscriber = horus.Node(
    name="subscriber",
    subs="multiprocess_topic",
    tick=subscriber_tick
)

# Create scheduler and run
scheduler = horus.Scheduler()
scheduler.add(subscriber)  # Use old API for compatibility

print("Subscriber starting...")
scheduler.run(duration=10.0)
print(f"Subscriber done! Received {received_count[0]} messages")
