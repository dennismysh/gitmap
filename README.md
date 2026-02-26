# GitMap

A macOS menu bar app that displays a GitHub-style commit heatmap for your local git repositories.

Built with Rust using egui/eframe for the UI and tray-icon for menu bar integration.

## Features

- **Commit heatmap** — GitHub-style contribution grid showing daily activity
- **Two view modes** — Year view (full calendar year) or Rolling view (1 week to 12 months)
- **Two data modes** — Track commits or lines changed (insertions + deletions)
- **Multi-repo tracking** — Add individual repos or scan a directory to discover all git repos
- **Real-time updates** — Watches `.git` directories for changes and rescans automatically
- **Customizable colors** — 5 preset accent colors plus custom hex input
- **Auto-update** — Checks GitHub Releases for new versions, with optional silent auto-update
- **Universal binary** — Runs natively on both Apple Silicon and Intel Macs

## Installation

Download the latest `GitMap-vX.X.X-macos-universal.zip` from [Releases](https://github.com/dennismysh/gitmap/releases), unzip, and move `GitMap.app` to `/Applications`.

The app runs as a menu bar icon — click it to open the heatmap popover.

## Building from Source

Requires [Rust](https://rustup.rs/) toolchain.

```bash
# Build and install to /Applications
./scripts/bundle.sh
```

## How It Works

### Architecture

```
Menu Bar (tray-icon)
  └─ Popover Window (eframe/egui)
       ├─ Heatmap View — grid of colored cells, one per day
       ├─ Stats — commits, active days, streaks
       └─ Settings — repos, colors, auto-update toggle

Background:
  ├─ Git Scanner (git2) — reads commit history, filters by user identity
  ├─ Repo Watcher (notify) — FSEvents on .git dirs for real-time updates
  ├─ Updater (ureq) — checks GitHub Releases API for new versions
  └─ Binary Watcher (notify) — detects binary updates for auto-relaunch
```

### Modules

| Module | Purpose |
|--------|---------|
| `scanner` | Scans git repos using libgit2, filters commits by user identity, collects diff stats |
| `store` | In-memory `HashMap<NaiveDate, DayStats>` with JSON persistence |
| `heatmap` | Generates the date grid and maps values to color intensity levels |
| `watcher` | Watches `.git` directories via FSEvents for real-time commit detection |
| `updater` | Checks GitHub Releases API for updates, downloads and replaces the .app bundle |
| `discovery` | Recursively finds git repos under a given directory |
| `config` | Persists settings (tracked repos, colors, view mode, auto-update) as JSON |
| `ui` | Popover window with heatmap rendering, settings panel, and update banner |

### Data Flow

1. On launch, loads cached history from `~/Library/Application Support/gitmap/history.json`
2. Scans all tracked repos for commits matching your git identity
3. Renders a heatmap grid — rows are days of the week, columns are weeks
4. FSEvents watcher detects new commits and triggers incremental rescans
5. Background thread checks GitHub for updates on launch and every 6 hours

## Tech Stack

- **Language:** Rust
- **UI:** [egui](https://github.com/emilk/egui) 0.32 + [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) 0.32
- **Menu bar:** [tray-icon](https://github.com/nicotineo/tray-icon) 0.21
- **Git:** [git2](https://github.com/rust-lang/git2-rs) 0.20 (libgit2 bindings)
- **File watching:** [notify](https://github.com/notify-rs/notify) 8.2
- **HTTP:** [ureq](https://github.com/algesten/ureq) 3 (for update checks)
- **Serialization:** serde + serde_json
- **Date handling:** chrono

## Release Workflow

Pushing a version tag triggers a GitHub Actions workflow that:

1. Cross-compiles for `aarch64-apple-darwin` and `x86_64-apple-darwin`
2. Creates a universal binary with `lipo`
3. Packages as `GitMap.app` bundle
4. Publishes a GitHub Release with the zipped `.app`

```bash
# To release a new version:
# 1. Bump version in Cargo.toml and resources/Info.plist
# 2. Commit and tag
git tag v0.3.0
git push origin main v0.3.0
```

## License

This project is licensed under the [MIT License](LICENSE).
