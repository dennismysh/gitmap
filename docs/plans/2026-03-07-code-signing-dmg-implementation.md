# Code Signing, Notarization & DMG Packaging Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ship GitMap as a signed, notarized DMG with branded drag-to-Applications installer, eliminating Gatekeeper warnings.

**Architecture:** Add code signing, notarization, and DMG creation steps to the existing GitHub Actions release workflow. Update the in-app auto-updater to handle DMG downloads instead of zips. Ship one transitional release with both formats for backward compatibility.

**Tech Stack:** `codesign`, `xcrun notarytool`, `xcrun stapler`, `create-dmg`, `hdiutil`

---

### Task 1: Add Entitlements File

**Files:**
- Create: `resources/GitMap.entitlements`

**Step 1: Create the entitlements plist**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.network.client</key>
    <true/>
</dict>
</plist>
```

**Step 2: Commit**

```bash
git add resources/GitMap.entitlements
git commit -m "feat: add hardened runtime entitlements for code signing"
```

---

### Task 2: Create DMG Background Image

**Files:**
- Create: `resources/dmg-background.png`

**Step 1: Create the background image**

Create a 600x400 PNG image for the DMG window background. It should have:
- Dark or neutral background color (e.g., `#1a1a2e` or similar dark tone)
- A subtle arrow pointing from left to right (indicating drag app to Applications)
- No text (the app icon and Applications folder icon provide context)

Use any image tool (e.g., Figma, Photoshop, or even ImageMagick). The image must be exactly 600x400 pixels. For Retina support, create a 1200x800 version at `resources/dmg-background@2x.png` (optional — `create-dmg` handles this if provided).

If generating programmatically:
```bash
# Example using ImageMagick (install: brew install imagemagick)
convert -size 600x400 xc:'#1a1a2e' \
  -fill '#ffffff20' -draw "polygon 270,180 330,160 330,200" \
  resources/dmg-background.png
```

**Step 2: Commit**

```bash
git add resources/dmg-background.png
git commit -m "feat: add branded DMG background image"
```

---

### Task 3: Update Release Workflow — Code Signing & Notarization

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Replace the entire release workflow**

The new workflow keeps existing build steps and adds signing, notarization, DMG creation, and cleanup. Replace `.github/workflows/release.yml` with:

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
        run: cargo build --release --target x86_64-apple-darwin --features vendored-openssl

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

      - name: Import signing certificate
        env:
          DEVELOPER_ID_CERT_BASE64: ${{ secrets.DEVELOPER_ID_CERT_BASE64 }}
          DEVELOPER_ID_CERT_PASSWORD: ${{ secrets.DEVELOPER_ID_CERT_PASSWORD }}
        run: |
          CERT_PATH="$RUNNER_TEMP/certificate.p12"
          KEYCHAIN_PATH="$RUNNER_TEMP/signing.keychain-db"
          KEYCHAIN_PASSWORD="$(openssl rand -base64 32)"

          # Decode certificate
          echo "$DEVELOPER_ID_CERT_BASE64" | base64 --decode > "$CERT_PATH"

          # Create and configure temporary keychain
          security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
          security set-keychain-settings -lut 3600 "$KEYCHAIN_PATH"
          security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"

          # Import certificate
          security import "$CERT_PATH" \
            -P "$DEVELOPER_ID_CERT_PASSWORD" \
            -A \
            -t cert \
            -f pkcs12 \
            -k "$KEYCHAIN_PATH"

          # Allow codesign to access the keychain
          security set-key-partition-list -S apple-tool:,apple:,codesign: \
            -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"

          # Add temporary keychain to search list
          security list-keychains -d user -s "$KEYCHAIN_PATH" $(security list-keychains -d user | tr -d '"')

          # Clean up certificate file
          rm -f "$CERT_PATH"

      - name: Sign .app bundle
        env:
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        run: |
          IDENTITY="Developer ID Application: ($APPLE_TEAM_ID)"

          # Sign the inner binary first
          codesign --force --options runtime \
            --sign "$IDENTITY" \
            --entitlements resources/GitMap.entitlements \
            GitMap.app/Contents/MacOS/gitmap

          # Sign the outer bundle
          codesign --force --options runtime \
            --sign "$IDENTITY" \
            --entitlements resources/GitMap.entitlements \
            GitMap.app

          # Verify
          codesign --verify --deep --strict GitMap.app
          echo "Code signing verified successfully"

      - name: Notarize .app bundle
        env:
          NOTARY_KEY_BASE64: ${{ secrets.NOTARY_KEY_BASE64 }}
          NOTARY_KEY_ID: ${{ secrets.NOTARY_KEY_ID }}
          NOTARY_ISSUER_ID: ${{ secrets.NOTARY_ISSUER_ID }}
        run: |
          # Decode API key
          NOTARY_KEY_PATH="$RUNNER_TEMP/notary-key.p8"
          echo "$NOTARY_KEY_BASE64" | base64 --decode > "$NOTARY_KEY_PATH"

          # Zip for notarization submission
          ditto -c -k --keepParent GitMap.app GitMap-notarize.zip

          # Submit and wait
          xcrun notarytool submit GitMap-notarize.zip \
            --key "$NOTARY_KEY_PATH" \
            --key-id "$NOTARY_KEY_ID" \
            --issuer "$NOTARY_ISSUER_ID" \
            --wait

          # Staple the ticket
          xcrun stapler staple GitMap.app

          # Clean up
          rm -f GitMap-notarize.zip

      - name: Create transitional zip (backward compat for v0.3.0 updater)
        run: |
          TAG="${GITHUB_REF#refs/tags/}"
          zip -r "GitMap-${TAG}-macos-universal.zip" GitMap.app

      - name: Create DMG
        run: |
          brew install create-dmg

          TAG="${GITHUB_REF#refs/tags/}"

          create-dmg \
            --volname "GitMap" \
            --window-pos 200 120 \
            --window-size 600 400 \
            --icon-size 100 \
            --icon "GitMap.app" 150 200 \
            --app-drop-link 450 200 \
            --background "resources/dmg-background.png" \
            --no-internet-enable \
            "GitMap-${TAG}-macos-universal.dmg" \
            GitMap.app

      - name: Sign and notarize DMG
        env:
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
          NOTARY_KEY_BASE64: ${{ secrets.NOTARY_KEY_BASE64 }}
          NOTARY_KEY_ID: ${{ secrets.NOTARY_KEY_ID }}
          NOTARY_ISSUER_ID: ${{ secrets.NOTARY_ISSUER_ID }}
        run: |
          TAG="${GITHUB_REF#refs/tags/}"
          DMG_NAME="GitMap-${TAG}-macos-universal.dmg"
          NOTARY_KEY_PATH="$RUNNER_TEMP/notary-key.p8"

          # Sign DMG
          codesign --force --sign "Developer ID Application: ($APPLE_TEAM_ID)" "$DMG_NAME"

          # Notarize DMG
          xcrun notarytool submit "$DMG_NAME" \
            --key "$NOTARY_KEY_PATH" \
            --key-id "$NOTARY_KEY_ID" \
            --issuer "$NOTARY_ISSUER_ID" \
            --wait

          # Staple DMG
          xcrun stapler staple "$DMG_NAME"

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            GitMap-*.dmg
            GitMap-*.zip
          generate_release_notes: true

      - name: Cleanup signing artifacts
        if: always()
        run: |
          KEYCHAIN_PATH="$RUNNER_TEMP/signing.keychain-db"
          if [ -f "$KEYCHAIN_PATH" ]; then
            security delete-keychain "$KEYCHAIN_PATH"
          fi
          rm -f "$RUNNER_TEMP/certificate.p12"
          rm -f "$RUNNER_TEMP/notary-key.p8"
```

**Step 2: Review the diff carefully**

Run: `git diff .github/workflows/release.yml`

Verify:
- Build steps (checkout, toolchain, build aarch64, build x86_64, lipo, assemble) are unchanged
- New steps added after "Assemble .app bundle": import cert, sign, notarize, zip, dmg, sign dmg, release, cleanup
- Release step publishes both `.dmg` and `.zip`
- Cleanup step has `if: always()`

**Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat: add code signing, notarization, and DMG creation to release workflow"
```

---

### Task 4: Update Updater — Switch from Zip to DMG

**Files:**
- Modify: `src/updater.rs`

**Step 1: Update asset detection to look for DMG**

In `check_for_update()`, change line 40 from:
```rust
            .map(|n| n.ends_with("-macos-universal.zip"))
```
to:
```rust
            .map(|n| n.ends_with("-macos-universal.dmg"))
```

Also update the comment on line 35 from `// Find the .zip asset` to `// Find the .dmg asset`.

**Step 2: Replace `download_and_install` with DMG logic**

Replace the entire `download_and_install` function (lines 52-109) with:

```rust
/// Download the update DMG and replace /Applications/GitMap.app.
pub fn download_and_install(download_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = std::path::Path::new("/tmp/gitmap-update");

    // Clean up any previous update attempt
    if tmp_dir.exists() {
        std::fs::remove_dir_all(tmp_dir)?;
    }
    std::fs::create_dir_all(tmp_dir)?;

    let dmg_path = tmp_dir.join("GitMap.dmg");

    // Download the DMG
    let response = ureq::get(download_url)
        .header("User-Agent", "gitmap-updater")
        .call()?;

    let mut bytes = Vec::new();
    use std::io::Read;
    response.into_body().into_reader().read_to_end(&mut bytes)?;
    std::fs::write(&dmg_path, &bytes)?;

    // Mount the DMG silently
    let output = std::process::Command::new("hdiutil")
        .args([
            "attach",
            &dmg_path.to_string_lossy(),
            "-nobrowse",
            "-noautoopen",
            "-mountpoint",
            "/tmp/gitmap-update/mount",
        ])
        .output()?;

    if !output.status.success() {
        return Err("hdiutil attach failed".into());
    }

    let mount_point = std::path::Path::new("/tmp/gitmap-update/mount");
    let source_app = mount_point.join("GitMap.app");

    if !source_app.exists() {
        let _ = std::process::Command::new("hdiutil")
            .args(["detach", &mount_point.to_string_lossy()])
            .status();
        return Err("GitMap.app not found in DMG".into());
    }

    // Replace the installed app
    let installed_app = std::path::Path::new("/Applications/GitMap.app");
    if installed_app.exists() {
        std::fs::remove_dir_all(installed_app)?;
    }

    let status = std::process::Command::new("cp")
        .args([
            "-R",
            &source_app.to_string_lossy(),
            "/Applications/GitMap.app",
        ])
        .status()?;

    if !status.success() {
        let _ = std::process::Command::new("hdiutil")
            .args(["detach", &mount_point.to_string_lossy()])
            .status();
        return Err("failed to copy GitMap.app to /Applications".into());
    }

    // Unmount and clean up
    let _ = std::process::Command::new("hdiutil")
        .args(["detach", &mount_point.to_string_lossy()])
        .status();
    let _ = std::fs::remove_dir_all(tmp_dir);

    Ok(())
}
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

**Step 4: Commit**

```bash
git add src/updater.rs
git commit -m "feat: switch auto-updater from zip to DMG install"
```

---

### Task 5: Update README

**Files:**
- Modify: `README.md`

**Step 1: Update installation instructions**

Change line 20 from:
```
Download the latest `GitMap-vX.X.X-macos-universal.zip` from [Releases](https://github.com/dennismysh/gitmap/releases), unzip, and move `GitMap.app` to `/Applications`.
```
to:
```
Download the latest `GitMap-vX.X.X-macos-universal.dmg` from [Releases](https://github.com/dennismysh/gitmap/releases), open it, and drag `GitMap.app` to the Applications folder.
```

**Step 2: Update release workflow description**

Change lines 85-91 from:
```
Pushing a version tag triggers a GitHub Actions workflow that:

1. Cross-compiles for `aarch64-apple-darwin` and `x86_64-apple-darwin`
2. Creates a universal binary with `lipo`
3. Packages as `GitMap.app` bundle
4. Publishes a GitHub Release with the zipped `.app`
```
to:
```
Pushing a version tag triggers a GitHub Actions workflow that:

1. Cross-compiles for `aarch64-apple-darwin` and `x86_64-apple-darwin`
2. Creates a universal binary with `lipo`
3. Packages as `GitMap.app` bundle
4. Signs with Developer ID certificate and notarizes with Apple
5. Creates a branded DMG installer
6. Publishes a GitHub Release with the signed `.dmg`
```

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: update installation and release docs for DMG distribution"
```

---

### Task 6: Test the Release Workflow

**Step 1: Verify signing identity format**

The workflow uses `"Developer ID Application: ($APPLE_TEAM_ID)"` as the signing identity. This format works when there's only one Developer ID Application certificate in the keychain. If the CI step fails with "no identity found", check the exact certificate common name. You can find it by running locally:

```bash
security find-identity -v -p codesigning
```

The output shows the full identity string (e.g., `"Developer ID Application: Your Name (TEAMID123)"`). If needed, update the `IDENTITY` variable in the workflow.

**Step 2: Do a test release**

Bump version in `Cargo.toml` and `resources/Info.plist` to `0.4.0`, commit, tag, and push:

```bash
# After bumping versions:
git add Cargo.toml resources/Info.plist
git commit -m "chore: bump version to v0.4.0"
git tag v0.4.0
git push origin main v0.4.0
```

**Step 3: Monitor the release workflow**

```bash
gh run watch
```

Check that:
- Build succeeds
- Signing succeeds (no "identity not found" errors)
- Notarization succeeds (notarytool returns "Accepted")
- Stapling succeeds
- DMG creation succeeds
- Release has both `.dmg` and `.zip` artifacts

**Step 4: Verify the DMG**

Download the DMG from the release. On your Mac:
1. Open the DMG — verify branded background, app icon on left, Applications on right
2. Drag to Applications — verify no Gatekeeper warning
3. Launch the app — verify it works normally
4. Check signing: `codesign -dv --verbose=2 /Applications/GitMap.app`
5. Check notarization: `spctl --assess --verbose /Applications/GitMap.app` (should say "accepted source=Notarized Developer ID")
