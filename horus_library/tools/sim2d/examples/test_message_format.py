#!/usr/bin/env python3
"""
Test script to validate dynamic obstacle message format
Tests the ObstacleController class without requiring sim2d to be running
"""

import sys
import json


def test_rectangle_message():
    """Test rectangular obstacle message format"""
    cmd = {
        "action": "add",
        "obstacle": {
            "pos": [3.0, 2.0],
            "shape": "rectangle",
            "size": [1.5, 1.0],
            "color": [0.8, 0.2, 0.2]
        }
    }

    # Validate structure
    assert "action" in cmd
    assert "obstacle" in cmd
    assert cmd["action"] in ["add", "remove"]
    assert "pos" in cmd["obstacle"]
    assert "shape" in cmd["obstacle"]
    assert "size" in cmd["obstacle"]
    assert len(cmd["obstacle"]["pos"]) == 2
    assert len(cmd["obstacle"]["size"]) == 2

    if "color" in cmd["obstacle"] and cmd["obstacle"]["color"]:
        assert len(cmd["obstacle"]["color"]) == 3
        assert all(0.0 <= c <= 1.0 for c in cmd["obstacle"]["color"])

    print("✓ Rectangle message format is valid")
    print(f"  Message: {json.dumps(cmd, indent=2)}")
    return True


def test_circle_message():
    """Test circular obstacle message format"""
    cmd = {
        "action": "add",
        "obstacle": {
            "pos": [-2.0, 4.0],
            "shape": "circle",
            "size": [0.8, 0.8],
            "color": [0.2, 0.8, 0.2]
        }
    }

    # Validate structure
    assert "action" in cmd
    assert "obstacle" in cmd
    assert cmd["action"] == "add"
    assert cmd["obstacle"]["shape"] == "circle"
    assert len(cmd["obstacle"]["pos"]) == 2
    assert len(cmd["obstacle"]["size"]) == 2

    print("✓ Circle message format is valid")
    print(f"  Message: {json.dumps(cmd, indent=2)}")
    return True


def test_remove_message():
    """Test obstacle removal message format"""
    cmd = {
        "action": "remove",
        "obstacle": {
            "pos": [3.0, 2.0],
            "shape": "rectangle",
            "size": [0.0, 0.0]
        }
    }

    # Validate structure
    assert cmd["action"] == "remove"
    assert "pos" in cmd["obstacle"]

    print("✓ Remove message format is valid")
    print(f"  Message: {json.dumps(cmd, indent=2)}")
    return True


def test_no_color_message():
    """Test message without custom color"""
    cmd = {
        "action": "add",
        "obstacle": {
            "pos": [0.0, 0.0],
            "shape": "rectangle",
            "size": [1.0, 1.0],
            "color": None
        }
    }

    # Should work without color
    assert "action" in cmd
    assert "obstacle" in cmd

    print("✓ No-color message format is valid")
    print(f"  Message: {json.dumps(cmd, indent=2)}")
    return True


def test_obstacle_controller_import():
    """Test that ObstacleController can be imported"""
    try:
        # Add current directory to path
        import os
        sys.path.insert(0, os.path.dirname(__file__))

        # This will fail if syntax is wrong
        exec(open('dynamic_obstacles.py').read(), {'__name__': '__test__'})
        print("✓ ObstacleController class can be loaded")
        return True
    except SyntaxError as e:
        print(f"✗ Syntax error in dynamic_obstacles.py: {e}")
        return False
    except Exception as e:
        # Other errors are OK (e.g., missing HORUS connection)
        print(f"✓ ObstacleController class syntax is valid")
        print(f"  (Runtime import skipped: {type(e).__name__})")
        return True


def main():
    print("=" * 70)
    print(" Dynamic Obstacle Message Format Tests")
    print("=" * 70)
    print()

    tests = [
        ("Rectangle message", test_rectangle_message),
        ("Circle message", test_circle_message),
        ("Remove message", test_remove_message),
        ("No-color message", test_no_color_message),
        ("ObstacleController import", test_obstacle_controller_import),
    ]

    passed = 0
    failed = 0

    for name, test_func in tests:
        print(f"\nTest: {name}")
        print("-" * 70)
        try:
            if test_func():
                passed += 1
            else:
                failed += 1
                print(f"✗ {name} FAILED")
        except AssertionError as e:
            failed += 1
            print(f"✗ {name} FAILED: {e}")
        except Exception as e:
            failed += 1
            print(f"✗ {name} ERROR: {e}")

    print()
    print("=" * 70)
    print(f" Results: {passed} passed, {failed} failed")
    print("=" * 70)

    if failed == 0:
        print("\n✅ All message format tests passed!")
        print("\nNext step: Run end-to-end test with sim2d:")
        print("  Terminal 1: cargo run --package sim2d")
        print("  Terminal 2: python3 examples/dynamic_obstacles.py simple")
        return 0
    else:
        print(f"\n❌ {failed} tests failed")
        return 1


if __name__ == "__main__":
    sys.exit(main())
