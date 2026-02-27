# Auto-Discovery: Watch Directories for New Repos

## Problem

Gitmap only detects repos through manual actions (Add Repository, Scan Directory). When a user clones a new repo or creates a new project in a directory they work in, Gitmap doesn't know about it until the user manually adds it.

## Solution

Watch configured "discover root" directories with FSEvents and automatically track any new git repos that appear. The existing scanner already filters commits by git identity, so all repos can be tracked — repos with no commits by the user simply show 0 contributions until the user starts committing.

## Design

### Config Changes

Add `untracked_repos: Vec<PathBuf>` to `Config` (with `#[serde(default)]` for migration). This persists repos the user explicitly removed from tracking, preventing the discovery scan from re-adding them.

The existing `auto_discover_roots: Vec<PathBuf>` field is activated at runtime (currently unused).

### DiscoveryWatcher Module (`src/discovery_watcher.rs`)

New struct that owns a `notify::RecommendedWatcher`:

- Watches each path in `auto_discover_roots` with `NonRecursive` mode
- Detects new directories via FSEvents create events
- 3-second debounce after detecting a new directory before checking for `.git` (handles `git clone` which creates the dir before `.git` is fully populated)
- Emits discovered repo paths via `mpsc::Receiver<PathBuf>`
- Skips repos already in `tracked_repos` or `untracked_repos`

### App Integration (`popover.rs`)

- `GitMapApp` gets a `DiscoveryWatcher` field
- Poll it in `update()` alongside the existing `RepoWatcher` poll
- When a new repo path arrives: add to `tracked_repos`, start watching `.git` with `RepoWatcher`, run `scan_repo`, save config
- On startup: run `discover_repos()` on all `auto_discover_roots` to catch repos added while app was closed, apply same logic

### Settings UI Changes

Replace the one-shot "Scan Directory..." button with persistent discover roots:

**Watch Directories section:**
- Lists current `auto_discover_roots` with remove (x) buttons
- "Add Directory..." button opens folder picker, adds to roots, starts watching

**Tracked Repositories section (existing, modified):**
- Each repo has an "x" button that moves it to `untracked_repos`

**Untracked Repositories section:**
- Persisted in config (replaces the session-only version in `SettingsState`)
- Each repo has a "+" button to move back to `tracked_repos`

### Edge Cases

- **`git clone` timing:** 3-second debounce before checking for `.git`
- **Deleted repos:** Silently skip if path disappears
- **Duplicate detection:** Check `tracked_repos` and `untracked_repos` before adding
- **Empty discover roots:** `DiscoveryWatcher` simply doesn't start
- **Config migration:** `untracked_repos` uses `#[serde(default)]`

### What This Does NOT Include

- No new crate dependencies
- No macOS notifications
- No ownership/identity checks for auto-classification
- No pending repo state or user prompts
