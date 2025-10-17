#!/usr/bin/env python3
import sys
import os
import json

def main():
    print(f"Python {sys.version_info.major}.{sys.version_info.minor}")
    print(f"OS: {os.name}")
    data = json.dumps({"test": "success"})
    print(data)
    return 0

if __name__ == "__main__":
    exit(main())
