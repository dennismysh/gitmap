#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUNDLE_DIR="$PROJECT_DIR/target/release/bundle"
APP_DIR="$BUNDLE_DIR/GitMap.app"

# Auto-detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    arm64)  TARGET="aarch64-apple-darwin" ;;
    x86_64) TARGET="x86_64-apple-darwin" ;;
    *)      echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "Building for $TARGET..."
cargo build --release --target "$TARGET" --features vendored-openssl --manifest-path "$PROJECT_DIR/Cargo.toml"

# Clean previous bundle
rm -rf "$APP_DIR"

# Create .app structure
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary
cp "$PROJECT_DIR/target/$TARGET/release/gitmap" "$APP_DIR/Contents/MacOS/gitmap"

# Copy resources
cp "$PROJECT_DIR/resources/Info.plist" "$APP_DIR/Contents/Info.plist"
cp "$PROJECT_DIR/resources/AppIcon.icns" "$APP_DIR/Contents/Resources/AppIcon.icns"

echo "Built: $APP_DIR"

# Kill running instance before replacing
pkill -x gitmap 2>/dev/null && echo "Stopped running instance" || true

# Install to /Applications
rm -rf /Applications/GitMap.app
cp -R "$APP_DIR" /Applications/GitMap.app
echo "Installed: /Applications/GitMap.app"

# Launch the app
open /Applications/GitMap.app
echo "Launched: GitMap.app"
