# Code Signing, Notarization & DMG Packaging

## Goal

Ship GitMap as a signed, notarized DMG so users can install without Gatekeeper warnings.

## Scope

- CI only (GitHub Actions release workflow)
- Local `bundle.sh` stays unsigned

## Prerequisites (Manual Setup)

### Apple Developer Certificates & Keys

1. **Developer ID Application certificate** (G2 Sub-CA) — created via Apple Developer portal, exported as `.p12`
2. **App Store Connect API key** — `.p8` file for notarytool authentication

### GitHub Repository Secrets

| Secret | Value |
|--------|-------|
| `DEVELOPER_ID_CERT_BASE64` | `.p12` file, base64-encoded |
| `DEVELOPER_ID_CERT_PASSWORD` | Password set when exporting `.p12` |
| `APPLE_TEAM_ID` | 10-character team ID |
| `NOTARY_KEY_BASE64` | `.p8` API key file, base64-encoded |
| `NOTARY_KEY_ID` | Key ID from App Store Connect |
| `NOTARY_ISSUER_ID` | Issuer ID from App Store Connect |

## Design

### 1. Entitlements

Add `resources/GitMap.entitlements` with hardened runtime permissions:
- `com.apple.security.network.client` — required for update checks (ureq)

### 2. Code Signing (CI)

After assembling the .app bundle in the release workflow:

1. **Import certificate** — decode base64 secret to `.p12` file in `$RUNNER_TEMP`, create temporary keychain with random password (`openssl rand -base64 32`), import `.p12`, set as default keychain. Set keychain timeout (`security set-keychain-settings -t 3600 -u`) as safety net.

2. **Sign explicitly** — sign the inner binary first, then the outer bundle:
   ```
   codesign --force --options runtime --sign "Developer ID Application: ..." \
     --entitlements resources/GitMap.entitlements \
     GitMap.app/Contents/MacOS/gitmap
   codesign --force --options runtime --sign "Developer ID Application: ..." \
     --entitlements resources/GitMap.entitlements \
     GitMap.app
   ```

3. **Notarize** — zip the signed .app, submit via `xcrun notarytool submit --wait` using API key auth (`--key`, `--key-id`, `--issuer`). The `.p8` key is decoded from `NOTARY_KEY_BASE64` to `$RUNNER_TEMP`.

4. **Staple** — `xcrun stapler staple GitMap.app` to embed the notarization ticket for offline verification.

5. **Cleanup** — delete temporary keychain, `.p12`, and `.p8` files. Use an `if: always()` step so cleanup runs even on failure.

### 3. DMG Creation

**Tool:** `create-dmg` (installed via `brew install create-dmg` in CI)

**Branded layout:**
- Background image: `resources/dmg-background.png` (~600x400, dark/neutral with arrow from app to Applications)
- Window size: 600x400
- GitMap.app icon positioned on the left
- Applications symlink on the right
- Volume name: "GitMap"

**DMG signing + notarization:**
- Sign the DMG with `codesign`
- Notarize the DMG via `notarytool submit --wait`
- Staple the DMG via `xcrun stapler staple`

**Output:** `GitMap-v{VERSION}-macos-universal.dmg`

### 4. Transitional Release (v0.4.0)

Ship **both** `.zip` and `.dmg` artifacts so existing v0.3.0 users (whose updater looks for `.zip`) can auto-update. The `.zip` contains the signed .app. After this release, the updater switches to `.dmg` and future releases can drop the `.zip`.

### 5. Updater Changes (`src/updater.rs`)

**Asset detection:** Change filter from `-macos-universal.zip` to `-macos-universal.dmg`

**Install logic** (`download_and_install`):
1. Download `.dmg` to `/tmp/gitmap-update/`
2. Mount: `hdiutil attach -nobrowse -noautoopen <dmg>`
3. Parse mount point from `hdiutil` stdout (typically `/Volumes/GitMap`)
4. Copy `GitMap.app` from mounted volume to `/Applications/`
5. Unmount: `hdiutil detach <mount-point>`
6. Clean up temp files

### 6. README Update

Change installation instructions from "download zip" to "download DMG, drag to Applications".

## Files Changed

| File | Change |
|------|--------|
| `.github/workflows/release.yml` | Add signing, notarization, DMG creation steps |
| `resources/GitMap.entitlements` | New — hardened runtime entitlements |
| `resources/dmg-background.png` | New — branded DMG background image |
| `src/updater.rs` | Switch from zip to DMG install logic |
| `README.md` | Update installation instructions |
