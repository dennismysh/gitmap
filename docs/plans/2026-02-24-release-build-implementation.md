# Release Build Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Package GitMap as a macOS .app bundle with a local build script and automated GitHub Actions release workflow.

**Architecture:** A `resources/` directory holds static bundle assets (Info.plist, placeholder icon). A shell script (`scripts/bundle.sh`) assembles the .app bundle locally. A GitHub Actions workflow cross-compiles a universal binary and publishes it as a GitHub Release on tag push.

**Tech Stack:** Bash, cargo, lipo, sips (icon generation), GitHub Actions

---

### Task 1: Create Info.plist

**Files:**
- Create: `resources/Info.plist`

**Step 1: Create the Info.plist file**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>GitMap</string>
    <key>CFBundleDisplayName</key>
    <string>GitMap</string>
    <key>CFBundleIdentifier</key>
    <string>com.beanieandpen.gitmap</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>gitmap</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSUIElement</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
```

Key fields:
- `LSUIElement = true` — hides from Dock, menu-bar-only app
- `CFBundleIconFile = AppIcon` — macOS looks for `AppIcon.icns` in Resources/
- `NSHighResolutionCapable = true` — Retina support
- `LSMinimumSystemVersion = 12.0` — macOS Monterey minimum (reasonable baseline)

**Step 2: Commit**

```bash
git add resources/Info.plist
git commit -m "build: add Info.plist for macOS .app bundle"
```

---

### Task 2: Create placeholder app icon

**Files:**
- Create: `resources/AppIcon.icns`

**Step 1: Generate a placeholder icon**

Use `sips` to create a minimal placeholder .icns from a generated PNG. This creates a solid dark-gray square icon:

```bash
# Create a 1024x1024 PNG placeholder (dark gray square)
python3 -c "
import struct, zlib
def create_png(width, height, r, g, b):
    def chunk(chunk_type, data):
        c = chunk_type + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    header = b'\x89PNG\r\n\x1a\n'
    ihdr = chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
    raw = b''
    for y in range(height):
        raw += b'\x00' + bytes([r, g, b]) * width
    idat = chunk(b'IDAT', zlib.compress(raw))
    iend = chunk(b'IEND', b'')
    return header + ihdr + idat + iend
with open('/tmp/gitmap_icon.png', 'wb') as f:
    f.write(create_png(1024, 1024, 45, 45, 48))
"

# Create iconset directory with required sizes
mkdir -p /tmp/GitMap.iconset
for size in 16 32 128 256 512; do
    sips -z $size $size /tmp/gitmap_icon.png --out "/tmp/GitMap.iconset/icon_${size}x${size}.png" >/dev/null 2>&1
    double=$((size * 2))
    sips -z $double $double /tmp/gitmap_icon.png --out "/tmp/GitMap.iconset/icon_${size}x${size}@2x.png" >/dev/null 2>&1
done

# Convert iconset to icns
iconutil -c icns /tmp/GitMap.iconset -o resources/AppIcon.icns

# Cleanup
rm -rf /tmp/GitMap.iconset /tmp/gitmap_icon.png
```

**Step 2: Verify the .icns file was created**

```bash
file resources/AppIcon.icns
```

Expected: `resources/AppIcon.icns: Mac OS X icon, ...`

**Step 3: Commit**

```bash
git add resources/AppIcon.icns
git commit -m "build: add placeholder app icon"
```

---

### Task 3: Create local build script

**Files:**
- Create: `scripts/bundle.sh`

**Step 1: Write the bundle script**

```bash
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
cargo build --release --target "$TARGET" --manifest-path "$PROJECT_DIR/Cargo.toml"

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
```

**Step 2: Make it executable**

```bash
chmod +x scripts/bundle.sh
```

**Step 3: Run the script and verify the bundle**

```bash
./scripts/bundle.sh
```

Expected output: `Building for x86_64-apple-darwin...` then `Built: .../target/release/bundle/GitMap.app`

Verify the bundle structure:

```bash
ls -R target/release/bundle/GitMap.app/Contents/
```

Expected:
```
Info.plist  MacOS/      Resources/

target/release/bundle/GitMap.app/Contents/MacOS:
gitmap

target/release/bundle/GitMap.app/Contents/Resources:
AppIcon.icns
```

**Step 4: Test launching the app**

```bash
open target/release/bundle/GitMap.app
```

Verify: GitMap appears in the menu bar (not in the Dock) and the heatmap popover works normally.

**Step 5: Commit**

```bash
git add scripts/bundle.sh
git commit -m "build: add local .app bundle script"
```

---

### Task 4: Create GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

**Step 1: Write the workflow**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin

      - name: Build aarch64
        run: cargo build --release --target aarch64-apple-darwin

      - name: Build x86_64
        run: cargo build --release --target x86_64-apple-darwin

      - name: Create universal binary
        run: |
          lipo -create \
            target/aarch64-apple-darwin/release/gitmap \
            target/x86_64-apple-darwin/release/gitmap \
            -output gitmap-universal

      - name: Assemble .app bundle
        run: |
          mkdir -p GitMap.app/Contents/MacOS
          mkdir -p GitMap.app/Contents/Resources
          cp gitmap-universal GitMap.app/Contents/MacOS/gitmap
          cp resources/Info.plist GitMap.app/Contents/Info.plist
          cp resources/AppIcon.icns GitMap.app/Contents/Resources/AppIcon.icns

      - name: Zip .app bundle
        run: |
          TAG="${GITHUB_REF#refs/tags/}"
          zip -r "GitMap-${TAG}-macos-universal.zip" GitMap.app

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: GitMap-*.zip
          generate_release_notes: true
```

**Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add GitHub Actions release workflow for macOS .app bundle"
```

---

### Task 5: Update .gitignore

**Files:**
- Modify: `.gitignore` (create if missing)

**Step 1: Add bundle output to .gitignore**

Ensure the local bundle output isn't committed:

```
# Build artifacts
/target/
```

Check if `.gitignore` already covers `/target/`. If it does, no changes needed.

**Step 2: Commit (if changed)**

```bash
git add .gitignore
git commit -m "build: update gitignore for bundle output"
```

---

### Task 6: Verify end-to-end locally

**Step 1: Clean build and bundle**

```bash
cargo clean
./scripts/bundle.sh
```

**Step 2: Launch and verify**

```bash
open target/release/bundle/GitMap.app
```

Verify:
- App appears in menu bar (tray icon visible)
- App does NOT appear in Dock (LSUIElement working)
- Heatmap popover opens on click
- Settings panel works
- No crashes in Console.app

**Step 3: Verify the .app can be copied to /Applications**

```bash
cp -R target/release/bundle/GitMap.app /Applications/
open /Applications/GitMap.app
```

Verify it works from /Applications the same as from the build directory.

Clean up after test:

```bash
rm -rf /Applications/GitMap.app
```
