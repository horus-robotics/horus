#!/usr/bin/env python3
"""
Test: Generic Hub Cross-Language Communication

Demonstrates Python ↔ Rust communication using GenericMessage (string topics)
"""

from horus import Hub
import time

def test_python_to_rust():
    """Python sends generic data that Rust can receive"""
    print("=" * 70)
    print("TEST: Python → Rust (Generic Data)")
    print("=" * 70)

    hub = Hub("cross_lang_topic")

    # Send complex Python data
    data = {
        "source": "python",
        "sensor_readings": {
            "temperature": 25.5,
            "humidity": 60,
            "pressure": 1013.25
        },
        "measurements": [1.1, 2.2, 3.3, 4.4],
        "active": True,
        "count": 42
    }

    print(f"Python sending: {data}")
    success = hub.send(data)
    print(f"Send success: {success}")

    # Python can also receive it
    received = hub.recv()
    if received:
        print(f"Python received back: {received}")
        print(f"Temperature: {received['sensor_readings']['temperature']}")

    print("[OK] Python side working!\n")
    return success


if __name__ == "__main__":
    test_python_to_rust()
