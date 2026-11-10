#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    VERSION=$(grep -E '^version\s*=' "$PROJECT_ROOT/Cargo.toml" | head -n1 | cut -d'"' -f2)
fi
VERSION="${VERSION#v}"

BINARY_PATH="${2:-}"
if [ -z "$BINARY_PATH" ]; then
    if [ -f "$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/release/vimium-desktop" ]; then
        BINARY_PATH="$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/release/vimium-desktop"
    elif [ -f "$PROJECT_ROOT/target/release/vimium-desktop" ]; then
        BINARY_PATH="$PROJECT_ROOT/target/release/vimium-desktop"
    else
        echo "Error: vimium-desktop binary not found. Build with cargo build --release first." >&2
        exit 1
    fi
fi

OUTPUT_DIR="${3:-$PROJECT_ROOT/dist}"
mkdir -p "$OUTPUT_DIR"

STAGE_DIR=$(mktemp -d /tmp/vimium-deb-staging.XXXXXX)
trap 'rm -rf "$STAGE_DIR"' EXIT

echo "=== Building Debian Package (.deb) for Vimium Desktop v${VERSION} ==="
echo "Source binary: $BINARY_PATH"
echo "Staging directory: $STAGE_DIR"

# 1. Directory layout
mkdir -p "$STAGE_DIR/usr/bin"
mkdir -p "$STAGE_DIR/usr/share/applications"
mkdir -p "$STAGE_DIR/usr/share/pixmaps"
mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$STAGE_DIR/DEBIAN"

# 2. Copy binary
cp "$BINARY_PATH" "$STAGE_DIR/usr/bin/vimium-desktop"
chmod 755 "$STAGE_DIR/usr/bin/vimium-desktop"
strip --strip-unneeded "$STAGE_DIR/usr/bin/vimium-desktop" 2>/dev/null || true

# 3. Copy desktop entry
if [ -f "$PROJECT_ROOT/assets/vimium-desktop.desktop" ]; then
    cp "$PROJECT_ROOT/assets/vimium-desktop.desktop" "$STAGE_DIR/usr/share/applications/vimium-desktop.desktop"
    chmod 644 "$STAGE_DIR/usr/share/applications/vimium-desktop.desktop"
fi

# 4. Copy icons
if [ -f "$PROJECT_ROOT/assets/icon.png" ]; then
    cp "$PROJECT_ROOT/assets/icon.png" "$STAGE_DIR/usr/share/pixmaps/vimium-desktop.png"
    cp "$PROJECT_ROOT/assets/icon.png" "$STAGE_DIR/usr/share/icons/hicolor/256x256/apps/vimium-desktop.png"
    chmod 644 "$STAGE_DIR/usr/share/pixmaps/vimium-desktop.png"
    chmod 644 "$STAGE_DIR/usr/share/icons/hicolor/256x256/apps/vimium-desktop.png"
fi

# 5. Calculate installed size (in KB)
INSTALLED_SIZE=$(du -sk "$STAGE_DIR/usr" | cut -f1)

# 6. Generate DEBIAN/control
cat << EOF > "$STAGE_DIR/DEBIAN/control"
Package: vimium-desktop
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: jools333 <https://github.com/jools333>
Depends: libc6 (>= 2.31), libdbus-1-3
Installed-Size: ${INSTALLED_SIZE}
Homepage: https://github.com/jools333/Vimium-desktop
Description: Vimium-like keyboard navigation for desktop applications
 Vimium Desktop brings the powerful keyboard-driven navigation concept of
 the Vimium browser extension to the entire operating system.
 Allows interacting with any desktop window using keyboard hints and AT-SPI.
EOF
chmod 644 "$STAGE_DIR/DEBIAN/control"

# 7. Generate maintainer scripts
cat << 'EOF' > "$STAGE_DIR/DEBIAN/postinst"
#!/bin/sh
set -e

if [ "$1" = "configure" ]; then
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database -q /usr/share/applications || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
    fi
fi

exit 0
EOF
chmod 755 "$STAGE_DIR/DEBIAN/postinst"

cat << 'EOF' > "$STAGE_DIR/DEBIAN/postrm"
#!/bin/sh
set -e

if [ "$1" = "remove" ] || [ "$1" = "purge" ]; then
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database -q /usr/share/applications || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
    fi
fi

exit 0
EOF
chmod 755 "$STAGE_DIR/DEBIAN/postrm"

# 8. Build .deb package
DEB_NAME="vimium-desktop_${VERSION}_amd64.deb"
OUTPUT_DEB="$OUTPUT_DIR/$DEB_NAME"

echo "Building package: $OUTPUT_DEB"
dpkg-deb --build --root-owner-group "$STAGE_DIR" "$OUTPUT_DEB"

# Also create standardized alias vimium-desktop-linux-x86_64.deb
cp -f "$OUTPUT_DEB" "$OUTPUT_DIR/vimium-desktop-linux-x86_64.deb"

echo "Debian package created successfully!"
echo "Artifact: $OUTPUT_DEB"
echo "Alias: $OUTPUT_DIR/vimium-desktop-linux-x86_64.deb"
