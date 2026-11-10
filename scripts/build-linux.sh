#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building Vimium Desktop (Linux) ==="

CONFIG="${1:-release}"

cd "$PROJECT_ROOT"

if [ "$CONFIG" = "debug" ]; then
    echo "Building debug binary..."
    cargo build
    BINARY_PATH="$PROJECT_ROOT/target/debug/vimium-desktop"
else
    echo "Building release binary..."
    cargo build --release
    BINARY_PATH="$PROJECT_ROOT/target/release/vimium-desktop"
fi

echo "Binary built: $BINARY_PATH"

# Build deb package if dpkg-deb is available
if command -v dpkg-deb >/dev/null 2>&1; then
    echo "Packaging Debian package (.deb)..."
    bash "$SCRIPT_DIR/build-deb.sh" "" "$BINARY_PATH"
else
    echo "dpkg-deb not found; skipping .deb creation."
fi

echo "Build finished successfully!"
