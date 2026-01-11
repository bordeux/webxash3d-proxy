#!/bin/bash
# Build script for webxash3d-proxy client
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
CLIENT_DIR="$PROJECT_DIR/client"
DIST_DIR="$PROJECT_DIR/dist"

echo "=== Building webxash3d-proxy client ==="
echo ""

# Check if npm is available
if ! command -v npm &> /dev/null; then
    echo "Error: npm is required but not installed"
    exit 1
fi

echo "1. Installing dependencies..."
cd "$CLIENT_DIR"
npm install

echo ""
echo "2. Building client..."
npm run build

echo ""
echo "3. Build complete!"
echo "   Output: $DIST_DIR"
echo ""

# Check what's in dist
if [ -d "$DIST_DIR" ]; then
    echo "=== Built files ==="
    ls -la "$DIST_DIR"
fi

echo ""
echo "=== Required additional files ==="
echo ""
echo "You need to add these files to $DIST_DIR:"
echo ""
echo "  valve.zip               - Half-Life base assets (maps, models, sounds)"
echo "  mainui.wasm             - From node_modules/xash3d-fwgs/"
echo "  filesystem_stdio.wasm   - From node_modules/xash3d-fwgs/"
echo "  cstrike/                - Game directory"
echo "    cl_dlls/client.wasm   - Client module"
echo "    extras.pk3            - Extra resources"
echo ""
echo "To copy WASM files from node_modules:"
echo "  cp $CLIENT_DIR/node_modules/xash3d-fwgs/*.wasm $DIST_DIR/"
echo ""
echo "=== Run the proxy ==="
echo ""
echo "  cargo run --release -- --server YOUR_SERVER:27015 --static-dir ./dist"
echo ""
