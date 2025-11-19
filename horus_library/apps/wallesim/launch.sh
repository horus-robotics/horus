#!/bin/bash
# WALL-E Simulation Launcher
# Ensures world.yaml is found with absolute path

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🤖 Launching WALL-E Simulation..."
echo "   World: $SCRIPT_DIR/world.yaml"
echo "   Robot: $SCRIPT_DIR/models/walle/walle.urdf"
echo ""

horus sim3d --world "$SCRIPT_DIR/world.yaml"
