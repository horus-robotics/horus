"""
HORUS Python - Simple, Intuitive Robotics Framework

A user-friendly Python API for the HORUS robotics framework.
"""

from typing import Optional, Any, Dict, List, Callable, Union
import pickle
import json
from collections import defaultdict
import time

# Import the Rust extension module
try:
    from horus._horus import (
        PyNode as _PyNode,
        PyNodeInfo as _NodeInfo,
        PyHub as _PyHub,
        PyScheduler as _PyScheduler,
        get_version,
    )
except ImportError:
    # Fallback for testing without Rust bindings
    print("Warning: Rust bindings not available. Running in mock mode.")
    _PyNode = None
    _NodeInfo = None
    _PyHub = None
    _PyScheduler = None
    def get_version(): return "0.1.0-mock"

__version__ = "0.1.0"
__all__ = [
    "Node",
    "Scheduler",
    "run",
    "quick",
    "get_version",
]


class Node:
    """
    Simple node for HORUS - no inheritance required!

    Example:
        def process(node):
            if node.has_msg("input"):
                data = node.get("input")
                node.send("output", data * 2)

        node = Node(
            name="processor",
            subs=["input"],
            pubs=["output"],
            tick=process,
            rate=30
        )

        run(node)
    """

    def __init__(self,
                 name: str = None,
                 pubs: Union[List[str], str] = None,
                 subs: Union[List[str], str] = None,
                 tick: Callable = None,
                 rate: float = 30,
                 init: Callable = None,
                 shutdown: Callable = None):
        """
        Create a simple HORUS node.

        Args:
            name: Node name (auto-generated if None)
            pubs: Topics to publish to (str or list)
            subs: Topics to subscribe to (str or list)
            tick: Function to call on each tick - signature: tick(node)
            rate: Tick rate in Hz (default 30)
            init: Optional init function - signature: init(node)
            shutdown: Optional shutdown function - signature: shutdown(node)
        """
        # Auto-generate name if not provided
        if name is None:
            import uuid
            name = f"node_{uuid.uuid4().hex[:8]}"

        self.name = name
        self.tick_fn = tick
        self.init_fn = init
        self.shutdown_fn = shutdown
        self.rate = rate

        # Normalize pub/sub to lists
        if isinstance(pubs, str):
            pubs = [pubs]
        if isinstance(subs, str):
            subs = [subs]

        self.pub_topics = pubs or []
        self.sub_topics = subs or []

        # Message queues for subscriptions
        self._msg_queues = defaultdict(list)

        # Create underlying HORUS components if available
        if _PyNode:
            self._node = _PyNode(name)
            self._setup_hubs()
        else:
            # Mock mode for testing
            self._node = None
            self._hubs = {}

    def _setup_hubs(self):
        """Setup publish/subscribe hubs."""
        self._hubs = {}

        # Create publisher hubs
        for topic in self.pub_topics:
            self._hubs[topic] = _PyHub(topic, 1024)

        # Create subscriber hubs
        for topic in self.sub_topics:
            self._hubs[topic] = _PyHub(topic, 1024)

    def has_msg(self, topic: str) -> bool:
        """
        Check if messages are available on a topic.

        Args:
            topic: Topic to check

        Returns:
            True if messages available
        """
        # First try to receive new messages
        self._receive_messages(topic)
        return len(self._msg_queues[topic]) > 0

    def get(self, topic: str) -> Optional[Any]:
        """
        Get next message from topic.

        Args:
            topic: Topic to read from

        Returns:
            Message data or None if no messages
        """
        self._receive_messages(topic)

        if self._msg_queues[topic]:
            return self._msg_queues[topic].pop(0)
        return None

    def get_all(self, topic: str) -> List[Any]:
        """
        Get all available messages from topic.

        Args:
            topic: Topic to read from

        Returns:
            List of messages (empty if none)
        """
        self._receive_messages(topic)

        msgs = self._msg_queues[topic][:]
        self._msg_queues[topic].clear()
        return msgs

    def send(self, topic: str, data: Any) -> bool:
        """
        Send data to a topic.

        Args:
            topic: Topic to send to
            data: Data to send

        Returns:
            True if sent successfully
        """
        if topic not in self.pub_topics:
            return False

        if self._node and topic in self._hubs:
            hub = self._hubs[topic]

            # Serialize based on type
            if isinstance(data, bytes):
                return hub.send_bytes(data)
            elif isinstance(data, str):
                return hub.send_bytes(data.encode('utf-8'))
            elif isinstance(data, (dict, list, tuple, int, float, bool, type(None))):
                json_bytes = json.dumps(data).encode('utf-8')
                return hub.send_with_metadata(json_bytes, "json")
            else:
                pickled = pickle.dumps(data)
                return hub.send_with_metadata(pickled, "pickle")

        # Mock mode
        return True

    def _receive_messages(self, topic: str):
        """Pull messages from hub into queue."""
        if self._node and topic in self._hubs:
            hub = self._hubs[topic]

            # Receive all available messages
            while True:
                result = hub.recv_with_metadata()
                if result is None:
                    break

                data_bytes, metadata = result

                # Deserialize
                if metadata == "json":
                    msg = json.loads(data_bytes.decode('utf-8'))
                elif metadata == "pickle":
                    msg = pickle.loads(data_bytes)
                else:
                    try:
                        msg = data_bytes.decode('utf-8')
                    except:
                        msg = data_bytes

                self._msg_queues[topic].append(msg)

    def _internal_tick(self, info=None):
        """Internal tick called by scheduler."""
        if self.tick_fn:
            self.tick_fn(self)

    def _internal_init(self, info=None):
        """Internal init called by scheduler."""
        if self.init_fn:
            self.init_fn(self)

    def _internal_shutdown(self, info=None):
        """Internal shutdown called by scheduler."""
        if self.shutdown_fn:
            self.shutdown_fn(self)


class Scheduler:
    """
    Simple scheduler for running nodes.

    Example:
        scheduler = Scheduler()
        scheduler.add(node1)
        scheduler.add(node2)
        scheduler.run()
    """

    def __init__(self):
        """Create a scheduler."""
        if _PyScheduler:
            self._scheduler = _PyScheduler()
        else:
            self._scheduler = None
        self._nodes = []

    def add(self, *nodes):
        """
        Add nodes to scheduler.

        Args:
            *nodes: One or more Node instances
        """
        for node in nodes:
            self._nodes.append(node)

            if self._scheduler and node._node:
                # Set up callbacks
                node._node.set_callback(node)
                # Add to scheduler
                self._scheduler.add_node(node._node)

                # Set tick rate per node if supported
                # Note: This might need adjustment based on actual Rust API
                if hasattr(self._scheduler, 'set_node_rate'):
                    self._scheduler.set_node_rate(node.name, node.rate)

    def run(self, duration: float = None):
        """
        Run the scheduler.

        Args:
            duration: Optional duration in seconds (runs forever if None)
        """
        if self._scheduler:
            # Initialize all nodes
            for node in self._nodes:
                node._internal_init()

            # Set a default tick rate
            self._scheduler.set_tick_rate(30.0)  # Default 30Hz

            # Run
            if duration:
                self._scheduler.run_for(duration)
            else:
                self._scheduler.run()

            # Shutdown all nodes
            for node in self._nodes:
                node._internal_shutdown()
        else:
            # Mock mode - simple loop
            print(f"Running {len(self._nodes)} nodes in mock mode...")

            for node in self._nodes:
                node._internal_init()

            start = time.time()
            while duration is None or (time.time() - start) < duration:
                for node in self._nodes:
                    node._internal_tick()
                time.sleep(0.03)  # ~30Hz

            for node in self._nodes:
                node._internal_shutdown()

    def stop(self):
        """Stop the scheduler."""
        if self._scheduler:
            self._scheduler.stop()


# Convenience functions

def run(*nodes, duration=None):
    """
    Quick run helper - create scheduler and run nodes.

    Args:
        *nodes: Node instances to run
        duration: Optional duration in seconds

    Example:
        node = Node(subs="in", pubs="out", tick=lambda n: n.send("out", n.get("in")))
        run(node, duration=5)
    """
    scheduler = Scheduler()
    scheduler.add(*nodes)
    scheduler.run(duration)


def quick(name: str = None,
          sub: str = None,
          pub: str = None,
          fn: Callable = None,
          rate: float = 30) -> Node:
    """
    Create a simple transform node quickly.

    Args:
        name: Node name
        sub: Input topic
        pub: Output topic
        fn: Transform function - signature: fn(data) -> result
        rate: Tick rate

    Returns:
        Configured Node

    Example:
        # Double the input
        node = quick(sub="numbers", pub="doubled", fn=lambda x: x * 2)
        run(node)
    """
    def tick(node):
        if node.has_msg(sub):
            data = node.get(sub)
            if fn:
                result = fn(data)
                if result is not None:
                    node.send(pub, result)
            else:
                node.send(pub, data)

    return Node(
        name=name or f"quick_{sub}_to_{pub}",
        subs=[sub] if sub else [],
        pubs=[pub] if pub else [],
        tick=tick,
        rate=rate
    )


def pipe(input_topic: str, output_topic: str, transform: Callable = None):
    """
    Create and run a simple pipe node.

    Args:
        input_topic: Topic to read from
        output_topic: Topic to write to
        transform: Optional transform function

    Example:
        # Echo input to output
        pipe("raw_sensor", "processed_sensor")

        # Transform data
        pipe("celsius", "fahrenheit", lambda c: c * 9/5 + 32)
    """
    node = quick(sub=input_topic, pub=output_topic, fn=transform)
    run(node)


def echo(input_topic: str, output_topic: str):
    """
    Simple echo node - copy input to output.

    Example:
        echo("sensor_raw", "sensor_copy")
    """
    pipe(input_topic, output_topic)


def filter_node(input_topic: str, output_topic: str, predicate: Callable):
    """
    Filter messages based on predicate.

    Args:
        predicate: Function that returns True to pass message

    Example:
        # Only pass positive values
        filter_node("all_values", "positive_values", lambda x: x > 0)
    """
    def filter_fn(data):
        if predicate(data):
            return data
        return None

    node = quick(sub=input_topic, pub=output_topic, fn=filter_fn)
    run(node)


def map_node(input_topic: str, output_topic: str, mapper: Callable):
    """
    Map/transform messages.

    Example:
        # Square all numbers
        map_node("numbers", "squared", lambda x: x ** 2)
    """
    pipe(input_topic, output_topic, mapper)


# Multi-node helpers

def fanout(input_topic: str, output_topics: List[str]):
    """
    Broadcast input to multiple outputs.

    Example:
        fanout("sensor", ["log", "display", "storage"])
    """
    def tick(node):
        if node.has_msg(input_topic):
            data = node.get(input_topic)
            for topic in output_topics:
                node.send(topic, data)

    node = Node(
        name=f"fanout_{input_topic}",
        subs=input_topic,
        pubs=output_topics,
        tick=tick
    )
    run(node)


def merge(input_topics: List[str], output_topic: str):
    """
    Merge multiple inputs into single output.

    Example:
        merge(["sensor1", "sensor2", "sensor3"], "all_sensors")
    """
    def tick(node):
        for topic in input_topics:
            while node.has_msg(topic):
                data = node.get(topic)
                node.send(output_topic, {"source": topic, "data": data})

    node = Node(
        name=f"merge_to_{output_topic}",
        subs=input_topics,
        pubs=output_topic,
        tick=tick
    )
    run(node)