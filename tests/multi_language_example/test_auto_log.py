#!/usr/bin/env python3
import horus

def tick(node):
    # Using node.send() instead of typed hub - should show automatic logging
    node.send("test_topic", {"x": 1.0, "y": 2.0})

node = horus.Node(name="test_auto_log", tick=tick, rate=1)
horus.run(node)
