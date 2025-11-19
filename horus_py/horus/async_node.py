"""
Simple Pythonic Async API for HORUS

Makes async I/O dead simple with Python's native async/await.
Just use 'async def tick()' and 'await' - that's it.
"""

import asyncio
from typing import Optional, Callable, Any
from . import Node


class AsyncNode(Node):
    """
    Simple async node - just use async/await!

    Example:
        ```python
        import horus

        class MyAsyncNode(horus.AsyncNode):
            async def setup(self):
                # Called once at start
                self.sub = self.create_subscriber("input", str)
                self.pub = self.create_publisher("output", str)

            async def tick(self):
                # Use normal Python async/await
                msg = await self.sub.recv()  # Async receive
                result = await self.process(msg)  # Any async operation
                await self.pub.send(result)  # Async send

            async def process(self, msg):
                # Your async logic here
                await asyncio.sleep(0.1)  # Non-blocking!
                return msg.upper()
        ```
    """

    def __init__(self):
        super().__init__()
        self._loop: Optional[asyncio.AbstractEventLoop] = None
        self._task: Optional[asyncio.Task] = None

    async def setup(self):
        """
        Override this for one-time setup.
        Called once before first tick.
        """
        pass

    async def tick(self):
        """
        Override this with your async logic.
        Use 'await' for any async operations.
        """
        pass

    async def shutdown(self):
        """
        Override this for cleanup.
        Called once when node stops.
        """
        pass

    def _get_loop(self) -> asyncio.AbstractEventLoop:
        """Get or create event loop"""
        if self._loop is None:
            try:
                self._loop = asyncio.get_running_loop()
            except RuntimeError:
                self._loop = asyncio.new_event_loop()
                asyncio.set_event_loop(self._loop)
        return self._loop

    def start(self):
        """Start the async node"""
        loop = self._get_loop()
        loop.run_until_complete(self.setup())

    def run_once(self):
        """Run one tick (called by scheduler)"""
        loop = self._get_loop()
        loop.run_until_complete(self.tick())

    def stop(self):
        """Stop the async node"""
        loop = self._get_loop()
        loop.run_until_complete(self.shutdown())


class AsyncHub:
    """
    Async wrapper for Hub - use with await.

    Example:
        ```python
        # Create
        hub = horus.AsyncHub("topic", str)

        # Send (async)
        await hub.send("hello")

        # Receive (async, waits for message)
        msg = await hub.recv()

        # Try receive (async, returns None if no message)
        msg = await hub.try_recv()
        ```
    """

    def __init__(self, topic: str, msg_type: type):
        from . import Hub
        self._hub = Hub(topic, msg_type)

    async def send(self, msg: Any):
        """Send message asynchronously"""
        # Run in executor to avoid blocking
        loop = asyncio.get_event_loop()
        await loop.run_in_executor(None, self._hub.send, msg)

    async def recv(self) -> Any:
        """
        Receive message asynchronously.
        Waits until a message is available.
        """
        loop = asyncio.get_event_loop()
        while True:
            msg = await loop.run_in_executor(None, self._hub.try_recv)
            if msg is not None:
                return msg
            await asyncio.sleep(0.001)  # Small delay to avoid busy wait

    async def try_recv(self) -> Optional[Any]:
        """
        Try to receive message asynchronously.
        Returns None immediately if no message.
        """
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self._hub.try_recv)

    def subscribe(self, callback: Callable):
        """Subscribe with callback (synchronous)"""
        self._hub.subscribe(callback)

    async def async_subscribe(self, async_callback: Callable):
        """
        Subscribe with async callback.

        Example:
            ```python
            async def handle(msg):
                await process(msg)

            await hub.async_subscribe(handle)
            ```
        """
        def wrapper(msg):
            asyncio.create_task(async_callback(msg))
        self._hub.subscribe(wrapper)


# Simple async utilities
async def sleep(seconds: float):
    """Sleep without blocking - just use await!"""
    await asyncio.sleep(seconds)


async def gather(*tasks):
    """Run multiple async operations concurrently"""
    return await asyncio.gather(*tasks)


async def wait_for(coro, timeout: float):
    """Wait for async operation with timeout"""
    return await asyncio.wait_for(coro, timeout=timeout)


__all__ = [
    'AsyncNode',
    'AsyncHub',
    'sleep',
    'gather',
    'wait_for',
]
