"""
HORUS Python - Simple, Intuitive Robotics Framework

A user-friendly Python API for the HORUS robotics framework.
"""

from typing import Optional, Any, Dict, List, Callable, Union
import pickle
import json
from collections import defaultdict
import time

# Maximum size for logged data representation (to prevent buffer overflows)
MAX_LOG_DATA_SIZE = 200

# Import the Rust extension module
try:
    from horus._horus import (
        PyNode as _PyNode,
        PyNodeInfo as _NodeInfo,
        PyHub as _PyHub,
        PyScheduler as _PyScheduler,
        PyNodeState as NodeState,
        get_version,
    )
except ImportError:
    # Fallback for testing without Rust bindings
    print("Warning: Rust bindings not available. Running in mock mode.")
    _PyNode = None
    _NodeInfo = None
    _PyHub = None
    _PyScheduler = None
    # Mock NodeState for testing
    class NodeState:
        UNINITIALIZED = "uninitialized"
        INITIALIZING = "initializing"
        RUNNING = "running"
        PAUSED = "paused"
        STOPPING = "stopping"
        STOPPED = "stopped"
        ERROR = "error"
        CRASHED = "crashed"
    def get_version(): return "0.1.0-mock"

__version__ = "0.1.0"
__all__ = [
    "Node",
    "Scheduler",
    "NodeState",
    "run",
]


def _truncate_for_logging(data: Any, max_size: int = MAX_LOG_DATA_SIZE) -> str:
    """
    Safely convert data to string for logging with size limit.

    Args:
        data: Data to convert to string
        max_size: Maximum string length

    Returns:
        Truncated string representation
    """
    if isinstance(data, (dict, list)):
        data_str = str(data)
    else:
        data_str = repr(data)

    if len(data_str) > max_size:
        # Truncate and add indicator
        return data_str[:max_size-3] + "..."

    return data_str


class Node:
    """
    Simple node for HORUS - no inheritance required!

    Example:
        def process(node: Node) -> None:
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
                 name: Optional[str] = None,
                 pubs: Optional[Union[List[str], str]] = None,
                 subs: Optional[Union[List[str], str]] = None,
                 tick: Optional[Callable[['Node'], None]] = None,
                 rate: float = 30,
                 init: Optional[Callable[['Node'], None]] = None,
                 shutdown: Optional[Callable[['Node'], None]] = None,
                 on_error: Optional[Callable[['Node', Exception], None]] = None):
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
            on_error: Optional error handler - signature: on_error(node, exception)
        """
        # Auto-generate name if not provided
        if name is None:
            import uuid
            name = f"node_{uuid.uuid4().hex[:8]}"

        self.name = name
        self.tick_fn = tick
        self.init_fn = init
        self.shutdown_fn = shutdown
        self.on_error_fn = on_error
        self.rate = rate
        self.error_count = 0

        # Normalize pub/sub to lists
        if isinstance(pubs, str):
            pubs = [pubs]
        if isinstance(subs, str):
            subs = [subs]

        self.pub_topics = pubs or []
        self.sub_topics = subs or []

        # Message queues for subscriptions
        self._msg_queues = defaultdict(list)

        # Phase 2: Message timestamps (topic -> [(msg, timestamp), ...])
        self._msg_timestamps = defaultdict(list)

        # NodeInfo context (set by scheduler)
        self.info = None

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
            # Phase 2: Pop timestamp along with message
            if self._msg_timestamps[topic]:
                self._msg_timestamps[topic].pop(0)
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
        # Phase 2: Clear timestamps too
        self._msg_timestamps[topic].clear()
        return msgs

    def get_timestamp(self, topic: str) -> Optional[float]:
        """
        Get timestamp of the next message without consuming it (Phase 2).

        Args:
            topic: Topic to check

        Returns:
            Unix timestamp in seconds (with microsecond precision) or None
        """
        self._receive_messages(topic)

        if self._msg_timestamps[topic]:
            return self._msg_timestamps[topic][0]
        return None

    def get_message_age(self, topic: str) -> Optional[float]:
        """
        Get age of the next message in seconds (Phase 2).

        Args:
            topic: Topic to check

        Returns:
            Message age in seconds or None if no messages
        """
        timestamp = self.get_timestamp(topic)
        if timestamp is not None and timestamp > 0:
            import time
            return time.time() - timestamp
        return None

    def is_stale(self, topic: str, max_age: float) -> bool:
        """
        Check if the next message is stale (Phase 2).

        Args:
            topic: Topic to check
            max_age: Maximum acceptable age in seconds

        Returns:
            True if message is older than max_age, False otherwise
        """
        age = self.get_message_age(topic)
        if age is None:
            return False  # No message = not stale
        return age > max_age

    def get_with_timestamp(self, topic: str) -> Optional[tuple]:
        """
        Get next message with its timestamp (Phase 2).

        Args:
            topic: Topic to read from

        Returns:
            Tuple of (message, timestamp) or None if no messages
        """
        self._receive_messages(topic)

        if self._msg_queues[topic]:
            msg = self._msg_queues[topic].pop(0)
            timestamp = self._msg_timestamps[topic].pop(0) if self._msg_timestamps[topic] else 0.0
            return (msg, timestamp)
        return None

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

            # Measure IPC timing
            import time
            start_ns = time.perf_counter_ns()

            # Serialize based on type
            if isinstance(data, bytes):
                result = hub.send_bytes(data)
            elif isinstance(data, str):
                result = hub.send_bytes(data.encode('utf-8'))
            elif isinstance(data, (dict, list, tuple, int, float, bool, type(None))):
                json_bytes = json.dumps(data).encode('utf-8')
                result = hub.send_with_metadata(json_bytes, "json")
            else:
                pickled = pickle.dumps(data)
                result = hub.send_with_metadata(pickled, "pickle")

            end_ns = time.perf_counter_ns()
            ipc_ns = end_ns - start_ns

            # Log the publish operation if NodeInfo available
            if self.info:
                data_repr = _truncate_for_logging(data)
                self.info.log_pub(topic, data_repr, ipc_ns)

            return result

        # Mock mode
        return True

    def _receive_messages(self, topic: str):
        """Pull messages from hub into queue (Phase 2: with timestamps)."""
        if self._node and topic in self._hubs:
            hub = self._hubs[topic]
            import time

            # Receive all available messages
            while True:
                # Measure IPC timing
                start_ns = time.perf_counter_ns()
                result = hub.recv_with_metadata()
                end_ns = time.perf_counter_ns()

                if result is None:
                    break

                ipc_ns = end_ns - start_ns
                data_bytes, msg_type, timestamp = result  # Phase 2: Now includes timestamp

                # Deserialize
                if msg_type == "json":
                    msg = json.loads(data_bytes.decode('utf-8'))
                elif msg_type == "pickle":
                    msg = pickle.loads(data_bytes)
                else:
                    try:
                        msg = data_bytes.decode('utf-8')
                    except:
                        msg = data_bytes

                # Log the subscribe operation if NodeInfo available
                if self.info:
                    data_repr = _truncate_for_logging(msg)
                    self.info.log_sub(topic, data_repr, ipc_ns)

                # Phase 2: Store message with timestamp
                self._msg_queues[topic].append(msg)
                self._msg_timestamps[topic].append(timestamp)

    def _internal_tick(self, info: Optional[Any] = None) -> None:
        """Internal tick called by scheduler."""
        # DON'T store info - use a context manager approach
        old_info = self.info
        self.info = info
        try:
            if self.tick_fn:
                self.tick_fn(self)
        except Exception as e:
            # Increment error count
            self.error_count += 1

            # Log error if info available
            if self.info:
                self.info.log_error(f"Tick failed: {e}")
                # Transition to error state if too many errors
                if self.error_count > 10:
                    self.info.transition_to_error(f"Too many errors ({self.error_count})")

            # Call user's error handler if provided
            if self.on_error_fn:
                try:
                    self.on_error_fn(self, e)
                except Exception as handler_error:
                    # Error handler itself failed - just log it
                    if self.info:
                        self.info.log_error(f"Error handler failed: {handler_error}")
            else:
                # No error handler - re-raise
                raise
        finally:
            self.info = old_info

    def _internal_init(self, info: Optional[Any] = None) -> None:
        """Internal init called by scheduler."""
        self.info = info  # Store info for access in init function
        if self.init_fn:
            self.init_fn(self)

    def _internal_shutdown(self, info: Optional[Any] = None) -> None:
        """Internal shutdown called by scheduler."""
        self.info = info  # Store info for access in shutdown function
        if self.shutdown_fn:
            self.shutdown_fn(self)

    # Public methods for Rust bindings to call
    def init(self, info: Optional[Any] = None) -> None:
        """Called by Rust scheduler during initialization."""
        self._internal_init(info)

    def tick(self, info: Optional[Any] = None) -> None:
        """Called by Rust scheduler on each tick."""
        self._internal_tick(info)

    def shutdown(self, info: Optional[Any] = None) -> None:
        """Called by Rust scheduler during shutdown."""
        self._internal_shutdown(info)

    # NodeInfo convenience methods (delegate to info if available)
    def log_info(self, message: str) -> None:
        """Log an info message (if logging enabled)."""
        if self.info:
            self.info.log_info(message)

    def log_warning(self, message: str) -> None:
        """Log a warning message (if logging enabled)."""
        if self.info:
            self.info.log_warning(message)

    def log_error(self, message: str) -> None:
        """Log an error message (if logging enabled)."""
        if self.info:
            self.info.log_error(message)

    def log_debug(self, message: str) -> None:
        """Log a debug message (if logging enabled)."""
        if self.info:
            self.info.log_debug(message)

    # Topic introspection methods
    def get_publishers(self) -> List[str]:
        """
        Get list of topics this node publishes to.

        Returns:
            List of publisher topic names

        Example:
            topics = node.get_publishers()
            print(f"Publishing to: {topics}")
        """
        return self.pub_topics.copy()

    def get_subscribers(self) -> List[str]:
        """
        Get list of topics this node subscribes to.

        Returns:
            List of subscriber topic names

        Example:
            topics = node.get_subscribers()
            print(f"Subscribed to: {topics}")
        """
        return self.sub_topics.copy()


class Scheduler:
    """
    Simple scheduler for running nodes.

    Example:
        scheduler = Scheduler()
        scheduler.add(sensor_node, 0, True)   # priority=0, logging=True
        scheduler.add(control_node, 1, False)  # priority=1, logging=False
        scheduler.add(motor_node, 2, True)     # priority=2, logging=True
        scheduler.run()

        # Or chainable:
        scheduler.add(node1, 0, True).add(node2, 1, False).run()
    """

    def __init__(self):
        """Create a scheduler."""
        if _PyScheduler:
            self._scheduler = _PyScheduler()
        else:
            self._scheduler = None
        self._nodes = []

    def add(self, node: 'Node', priority: int, logging: bool = False) -> 'Scheduler':
        """
        Add a node to the scheduler.

        Args:
            node: Node instance to add
            priority: Priority level (lower number = higher priority, 0 = highest)
            logging: Enable logging for this node (default: False)

        Returns:
            self (for method chaining)

        Example:
            scheduler.add(sensor_node, 0, True)   # Highest priority, logging on
            scheduler.add(control_node, 1, False)  # Medium priority, logging off
            scheduler.add(motor_node, 2, True)     # Lowest priority, logging on

            # Chainable:
            scheduler.add(node1, 0, True).add(node2, 1, False).run()
        """
        self._nodes.append(node)

        if self._scheduler:
            # Register the Python Node wrapper directly, not the internal _node
            # The Rust scheduler will call node.tick(info) and node.init(info)
            self._scheduler.register(node, priority, logging, None)

        return self

    def run(self, duration: Optional[float] = None) -> None:
        """
        Run the scheduler.

        Args:
            duration: Optional duration in seconds (runs forever if None)
        """
        if self._scheduler:
            # Initialize all nodes
            for node in self._nodes:
                node._internal_init()

            # Set scheduler tick rate to a high value to support per-node rate control
            # The scheduler needs to tick faster than the fastest node
            self._scheduler.set_tick_rate(1000.0)  # 1000Hz allows fine-grained control

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

    def stop(self) -> None:
        """Stop the scheduler."""
        if self._scheduler:
            self._scheduler.stop()

    def get_node_stats(self, node_name: str) -> Dict[str, Any]:
        """
        Get statistics for a specific node.

        Args:
            node_name: Name of the node

        Returns:
            Dictionary with node stats including:
            - name: Node name
            - priority: Priority level
            - logging_enabled: Whether logging is enabled
            - total_ticks: Total number of ticks executed
            - errors_count: Number of errors encountered

        Example:
            stats = scheduler.get_node_stats("sensor")
            print(f"Node {stats['name']} at priority {stats['priority']}")
            print(f"Total ticks: {stats['total_ticks']}")
        """
        if self._scheduler:
            return self._scheduler.get_node_stats(node_name)
        return {}


# Convenience functions

def run(*nodes: Node, duration: Optional[float] = None) -> None:
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
    # Add nodes with priority based on insertion order (0, 1, 2, ...)
    for i, node in enumerate(nodes):
        scheduler.add(node, priority=i, logging=False)
    scheduler.run(duration)
