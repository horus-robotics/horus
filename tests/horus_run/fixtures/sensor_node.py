#!/usr/bin/env python3
"""Python sensor node using HORUS"""

import time

def main():
    print("Sensor Node Starting...")

    # Simulate sensor readings
    for i in range(5):
        temperature = 20.0 + i * 0.5
        print(f"Reading #{i+1}: Temperature = {temperature:.1f}C")
        time.sleep(0.1)

    print("Sensor node completed 5 readings")
    return 0

if __name__ == "__main__":
    exit(main())
