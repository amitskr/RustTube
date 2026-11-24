#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_MODE="${1:-release}"
TARGET_DIR="$ROOT_DIR/target/$BUILD_MODE"

echo "==> Building rusttube ($BUILD_MODE)..."
if [ "$BUILD_MODE" = "release" ]; then
    cargo build --release --locked --manifest-path "$ROOT_DIR/Cargo.toml"
else
    cargo build --manifest-path "$ROOT_DIR/Cargo.toml"
fi

APP_NAME="RustTube.app"
DIST_DIR="$ROOT_DIR/dist"
APP_DIR="$DIST_DIR/$APP_NAME"

echo "==> Assembling $APP_NAME..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"
mkdir -p "$APP_DIR/Contents/Frameworks"

cp "$TARGET_DIR/rusttube" "$APP_DIR/Contents/MacOS/rusttube"
chmod +x "$APP_DIR/Contents/MacOS/rusttube"

# Ensure icon is available
if [ ! -f "$SCRIPT_DIR/rusttube.icns" ]; then
    "$SCRIPT_DIR/make_icns.sh"
fi

cp "$SCRIPT_DIR/rusttube.icns" "$APP_DIR/Contents/Resources/rusttube.icns"

# Update version in Info.plist from Cargo.toml if needed
VERSION=$(grep -m1 '^version' "$ROOT_DIR/Cargo.toml" | cut -d'"' -f2)
sed -E "s|(<string>)[0-9]+\.[0-9]+\.[0-9]+(-[a-z0-9]+)?(</string>)|\1$VERSION\3|g" "$SCRIPT_DIR/Info.plist" > "$APP_DIR/Contents/Info.plist"

# Bundle libmpv if available in Homebrew or Frameworks
if [ -f "/opt/homebrew/lib/libmpv.dylib" ]; then
    echo "==> Detected Homebrew libmpv (Apple Silicon)"
elif [ -f "/usr/local/lib/libmpv.dylib" ]; then
    echo "==> Detected Homebrew libmpv (Intel)"
fi

# Ad-hoc sign bundle
if command -v codesign &>/dev/null; then
    echo "==> Ad-hoc signing $APP_NAME..."
    codesign --force --deep --sign - "$APP_DIR" || true
fi

echo "==> $APP_NAME successfully created at $APP_DIR"

# Build DMG if hdiutil is available and release mode
if command -v hdiutil &>/dev/null && [ "$BUILD_MODE" = "release" ]; then
    DMG_DIR="$DIST_DIR/dmg"
    DMG_OUTPUT="$DIST_DIR/RustTube-${VERSION}-macos.dmg"
    echo "==> Creating $DMG_OUTPUT..."
    rm -rf "$DMG_DIR" "$DMG_OUTPUT"
    mkdir -p "$DMG_DIR"
    cp -R "$APP_DIR" "$DMG_DIR/"
    ln -s /Applications "$DMG_DIR/Applications"
    hdiutil create -volname "RustTube" -srcfolder "$DMG_DIR" -ov -format UDZO "$DMG_OUTPUT"
    rm -rf "$DMG_DIR"
    echo "==> DMG created at $DMG_OUTPUT"
fi
