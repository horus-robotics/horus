#!/usr/bin/env python3
"""
Multiprocess Test: Publisher Node

Run with: horus run multiprocess_publisher.py multiprocess_subscriber.py
"""

import horus
import time

count = [0]

def publisher_tick(node):
    """Publish messages to shared topic"""
    count[0] += 1
    msg = {"count": count[0], "data": "Hello from publisher!"}
    node.send("multiprocess_topic", msg)
    print(f"[Publisher] Sent message #{count[0]}")

# Create publisher node
publisher = horus.Node(
    name="publisher",
    pubs="multiprocess_topic",
    tick=publisher_tick
)

# Create scheduler and run
scheduler = horus.Scheduler()
scheduler.add(publisher)  # Use old API for compatibility

print("Publisher starting...")
scheduler.run(duration=10.0)
print("Publisher done!")
