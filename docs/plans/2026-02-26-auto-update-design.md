# Auto-Update from GitHub Releases

## Overview

Add in-app auto-update so users always run the latest version. The app checks GitHub Releases on launch, prompts the user (or silently updates if configured), downloads the new `.app.zip` bundle, replaces the installed copy, and relaunches.

## Version Tracking

- Use `env!("CARGO_PKG_VERSION")` for the compiled-in version (from `Cargo.toml`)
- GitHub releases use semver tags: `v0.2.0`
- Compare by stripping the `v` prefix and doing string comparison

## Update Check Flow

1. On startup, spawn a background thread
2. Hit `GET https://api.github.com/repos/dennismysh/gitmap/releases/latest`
3. Parse `tag_name` and find the `GitMap.app.zip` asset URL
4. Compare remote version against local version
5. If newer, send `UpdateInfo { version, download_url }` to main thread via `mpsc` channel
6. On network failure, silently ignore

Use `ureq` (blocking HTTP, no async runtime) to keep dependencies light.

## User Interaction

**Default (prompt mode):**
- Show a banner at the bottom of the heatmap view: "Update available: v0.2.0" with an "Update" button
- Clicking "Update" triggers download in a background thread, shows "Updating..." state, then relaunches

**Auto-update mode:**
- When `auto_update` is enabled in settings, skip the banner
- Silently download, replace, and relaunch

**Settings additions:**
- "Auto-update" toggle (default: off)
- Display current version (e.g. `v0.1.0`)

## Download and Install

1. Download `GitMap.app.zip` to `/tmp/gitmap-update/`
2. Unzip using `ditto -xk` (built into macOS, preserves attributes)
3. Remove old `/Applications/GitMap.app`
4. Move new bundle to `/Applications/GitMap.app`
5. Spawn the new binary and close current process (existing relaunch mechanism)

## Config Changes

Add `auto_update: bool` to `Config` (default: `false`).

## New Module

`src/updater.rs`:
- `check_for_update() -> Option<UpdateInfo>` — GitHub API check
- `download_and_install(url: &str) -> Result<()>` — download, extract, replace

## New Dependency

- `ureq` — lightweight blocking HTTP client

## Release Asset Convention

GitHub Actions uploads `GitMap.app.zip` as a release asset. The updater matches assets by name containing `GitMap.app.zip`.
