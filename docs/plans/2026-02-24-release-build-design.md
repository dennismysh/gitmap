# Release Build Design

## Goal

Package GitMap as a macOS .app bundle and automate releases via GitHub Actions so users can download and install it from GitHub Releases.

## Decisions

- **Distribution**: GitHub Releases with .app bundle (zip). No Homebrew tap or crates.io for now.
- **Bundle ID**: `com.beanieandpen.gitmap`
- **Icon**: Placeholder for now, replaceable later.
- **Architecture**: Universal binary (aarch64 + x86_64) for releases; local builds use native arch only.
- **No code signing/notarization**: Users bypass Gatekeeper manually. Can add later with Apple Developer account.

## .app Bundle Structure

```
GitMap.app/
└── Contents/
    ├── Info.plist
    ├── MacOS/
    │   └── gitmap
    └── Resources/
        └── AppIcon.icns
```

- `LSUIElement = true` in Info.plist hides the app from the Dock (menu-bar-only).
- The binary is a universal fat binary on release, native-only for local builds.

## New Repo Files

```
resources/
├── Info.plist
└── AppIcon.icns          # placeholder

scripts/
└── bundle.sh             # local build script

.github/workflows/
└── release.yml           # GitHub Actions release workflow
```

## Local Build Script (`scripts/bundle.sh`)

- Auto-detects architecture via `uname -m`
- Runs `cargo build --release` for detected target only
- Assembles .app bundle at `target/release/bundle/GitMap.app`
- Single command: `./scripts/bundle.sh`

## GitHub Actions Workflow (`.github/workflows/release.yml`)

- **Trigger**: Push a tag matching `v*` (e.g., `v0.1.0`)
- **Runner**: `macos-latest`
- **Steps**:
  1. Install Rust toolchain with both targets (`aarch64-apple-darwin`, `x86_64-apple-darwin`)
  2. `cargo build --release --target aarch64-apple-darwin`
  3. `cargo build --release --target x86_64-apple-darwin`
  4. `lipo -create -output gitmap` to combine both binaries
  5. Assemble .app bundle
  6. Zip: `GitMap-<tag>-macos-universal.zip`
  7. Create GitHub Release with the zip attached

## Release Flow

```bash
git tag v0.1.0
git push origin v0.1.0
# GitHub Actions builds, bundles, and publishes the release automatically
```
