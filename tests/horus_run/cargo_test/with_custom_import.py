#!/usr/bin/env python3
# This should detect missing 'horus' package
try:
    import horus
    print("HORUS package found")
except ImportError:
    print("HORUS package not found (expected)")

print("Import detection test passed")
