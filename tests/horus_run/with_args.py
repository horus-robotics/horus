#!/usr/bin/env python3
"""Test passing arguments to program"""

import sys

def main():
    print(f"Program: {sys.argv[0]}")
    print(f"Arguments: {sys.argv[1:]}")

    if len(sys.argv) > 1:
        print(f"Received {len(sys.argv)-1} arguments")
        for i, arg in enumerate(sys.argv[1:], 1):
            print(f"  Arg {i}: {arg}")
    else:
        print("No arguments received")

    return 0

if __name__ == "__main__":
    exit(main())
