"""
Simple Async Example

Shows how easy async I/O is with HORUS - just use async/await!
"""

import horus


class DatabaseNode(horus.AsyncNode):
    """Fetch data from database without blocking"""

    async def setup(self):
        self.query_sub = horus.AsyncHub("query", str)
        self.result_pub = horus.AsyncHub("result", str)

    async def tick(self):
        # Wait for query (non-blocking!)
        query = await self.query_sub.recv()

        # Fetch from database (non-blocking!)
        result = await self.fetch_from_db(query)

        # Send result
        await self.result_pub.send(result)

    async def fetch_from_db(self, query):
        # Simulated async database call
        await horus.sleep(0.5)  # Non-blocking!
        return f"Result for: {query}"


class SensorNode(horus.AsyncNode):
    """Read sensor data asynchronously"""

    async def setup(self):
        self.sensor_pub = horus.AsyncHub("sensor_data", float)

    async def tick(self):
        # Read sensor (async I/O)
        value = await self.read_sensor()

        # Publish
        await self.sensor_pub.send(value)

        # Wait before next reading
        await horus.sleep(0.1)

    async def read_sensor(self):
        # Simulated async sensor read
        await horus.sleep(0.05)
        return 23.5


# Run it!
if __name__ == "__main__":
    import asyncio

    async def main():
        # Create nodes
        db = DatabaseNode()
        sensor = SensorNode()

        # Setup
        await db.setup()
        await sensor.setup()

        # Run for a bit
        for _ in range(10):
            await asyncio.gather(
                db.tick(),
                sensor.tick()
            )

    asyncio.run(main())
    print("Done!")
