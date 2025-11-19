"""
Async HTTP API Example

Shows how to call web APIs without blocking.
Uses standard Python aiohttp - no special HORUS magic needed!
"""

import horus


class WeatherNode(horus.AsyncNode):
    """Fetch weather data from API"""

    async def setup(self):
        self.location_sub = horus.AsyncHub("location", str)
        self.weather_pub = horus.AsyncHub("weather", str)

        # Use any async library you want!
        try:
            import aiohttp
            self.session = aiohttp.ClientSession()
        except ImportError:
            print("Install aiohttp: pip install aiohttp")
            self.session = None

    async def tick(self):
        if not self.session:
            return

        # Get location request
        location = await self.location_sub.try_recv()
        if not location:
            await horus.sleep(0.1)
            return

        # Fetch weather (async HTTP call)
        weather = await self.fetch_weather(location)

        # Publish result
        await self.weather_pub.send(weather)

    async def fetch_weather(self, location):
        # Real async HTTP call!
        url = f"https://api.weather.gov/points/{location}"
        async with self.session.get(url) as response:
            data = await response.json()
            return f"Weather for {location}: {data['properties']['gridId']}"

    async def shutdown(self):
        if self.session:
            await self.session.close()


# That's it! Just use normal Python async libraries.
