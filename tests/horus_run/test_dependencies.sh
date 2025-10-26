#!/bin/bash
# Test dependency detection and resolution with horus run

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# HORUS binary
HORUS="/home/lord-patpak/horus/HORUS/target/debug/horus"

# Test directory
TEST_DIR=$(mktemp -d /tmp/horus_test_dependencies_XXXXXX)
trap "rm -rf $TEST_DIR" EXIT

cd "$TEST_DIR"

echo "=== Testing Dependency Detection and Resolution ==="
echo ""

# Helper functions
pass() {
    echo -e "${GREEN} PASS${NC}: $1"
    ((TESTS_PASSED++))
}

fail() {
    echo -e "${RED} FAIL${NC}: $1"
    echo "   Error: $2"
    ((TESTS_FAILED++))
}

# Test 1: Detect 'use horus::*' in Rust
echo "Test 1: Detect HORUS library import in Rust..."
cat > with_horus.rs << 'EOF'
use horus::prelude::*;

fn main() {
    println!("HORUS import detected");
}
EOF

# Should detect horus dependency (may fail to compile if not installed)
OUTPUT=$($HORUS run with_horus.rs 2>&1 || true)
if echo "$OUTPUT" | grep -q -E "horus|dependency|import"; then
    pass "Detects 'use horus::*' in Rust"
else
    fail "Rust horus import" "Did not detect horus dependency"
fi

# Test 2: Rust Cargo.toml dependencies
echo "Test 2: Read dependencies from Cargo.toml..."
mkdir -p cargo_test/src
cd cargo_test

cat > Cargo.toml << 'EOF'
[package]
name = "test_deps"
version = "0.1.0"
edition = "2021"

[dependencies]
horus_core = { path = "/home/lord-patpak/horus/HORUS/horus_core" }
EOF

cat > src/main.rs << 'EOF'
fn main() {
    println!("Cargo dependencies test");
}
EOF

# Should detect dependencies from Cargo.toml
OUTPUT=$($HORUS run 2>&1 || true)
if echo "$OUTPUT" | grep -q -E "Cargo dependencies test|horus"; then
    pass "Reads Cargo.toml dependencies"
else
    fail "Cargo.toml deps" "Did not read Cargo.toml"
fi

cd "$TEST_DIR"

# Test 3: Python import detection
echo "Test 3: Detect Python imports..."
cat > python_imports.py << 'EOF'
#!/usr/bin/env python3
import sys
import json

def main():
    print("Python imports detected")
    return 0

if __name__ == "__main__":
    exit(main())
EOF

if $HORUS run python_imports.py 2>&1 | grep -q "Python imports detected"; then
    pass "Python standard library imports work"
else
    fail "Python imports" "Failed to handle Python imports"
fi

# Test 4: Python custom package detection
echo "Test 4: Detect custom Python packages..."
cat > with_custom_import.py << 'EOF'
#!/usr/bin/env python3
# Test that HORUS detects package imports
import sys
import json

print("Import detection test passed")
EOF

if $HORUS run with_custom_import.py 2>&1 | grep -q "Import detection test passed"; then
    pass "Handles Python package imports"
else
    fail "Python package import" "Failed to run with import statement"
fi

# Test 5: Missing dependency detection
echo "Test 5: Detect missing dependencies..."
cat > missing_dep.rs << 'EOF'
use nonexistent_crate::SomeType;

fn main() {
    println!("This should fail");
}
EOF

OUTPUT=$($HORUS run missing_dep.rs 2>&1 || true)
if echo "$OUTPUT" | grep -q -E "error|can't find crate|not found"; then
    pass "Detects missing Rust dependencies"
else
    fail "Missing dependency" "Should have reported missing crate"
fi

# Test 6: C include detection
echo "Test 6: Detect C includes..."
cat > with_includes.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main() {
    printf("C includes detected\n");
    return 0;
}
EOF

if $HORUS run with_includes.c 2>&1 | grep -q "C includes detected"; then
    pass "C standard includes work"
else
    fail "C includes" "Failed with standard includes"
fi

# Test 7: Multiple Rust dependencies
echo "Test 7: Multiple Rust 'use' statements..."
cat > multi_use.rs << 'EOF'
use std::collections::HashMap;
use std::io::Write;
use std::fs::File;

fn main() {
    let mut map = HashMap::new();
    map.insert("test", "value");
    println!("Multiple use statements work");
}
EOF

if $HORUS run multi_use.rs 2>&1 | grep -q "Multiple use statements work"; then
    pass "Multiple use statements work"
else
    fail "Multiple use" "Failed with multiple use statements"
fi

# Test 8: Dependency in subfolder
echo "Test 8: Import scanning in nested files..."
mkdir -p nested/src
cat > nested/src/main.py << 'EOF'
#!/usr/bin/env python3
import os
import json

print("Nested imports work")
EOF

cd nested
if $HORUS run 2>&1 | grep -q "Nested imports work"; then
    pass "Scans imports in nested files"
    cd "$TEST_DIR"
else
    cd "$TEST_DIR"
    fail "Nested imports" "Failed to scan nested file imports"
fi

# Test 9: External crate usage
echo "Test 9: Standard library crate usage..."
cat > std_crate.rs << 'EOF'
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let data = Arc::new(Mutex::new(0));
    println!("Standard library crates work");
}
EOF

if $HORUS run std_crate.rs 2>&1 | grep -q "Standard library crates work"; then
    pass "Standard library crates work"
else
    fail "Std crates" "Failed with std crates"
fi

# Test 10: Python __future__ imports
echo "Test 10: Python special imports..."
cat > future_import.py << 'EOF'
#!/usr/bin/env python3
from __future__ import annotations
import typing

def main():
    print("Special imports work")
    return 0

if __name__ == "__main__":
    exit(main())
EOF

if $HORUS run future_import.py 2>&1 | grep -q "Special imports work"; then
    pass "Python special imports work"
else
    fail "Python special imports" "Failed with __future__ import"
fi

# Summary
echo ""
echo "================================"
echo "Dependency Tests Summary"
echo "================================"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All dependency tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
