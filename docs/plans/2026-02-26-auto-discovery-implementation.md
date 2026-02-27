# Auto-Discovery Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Automatically detect and track new git repos appearing in user-configured "discover root" directories.

**Architecture:** A new `DiscoveryWatcher` module watches discover roots with FSEvents (via `notify` crate, already a dependency). On detecting new repos, it sends paths to the app via mpsc channel. The app auto-adds them to tracking. Settings UI manages discover roots and lets users move repos between tracked/untracked lists. Untracked repos are persisted in config to prevent re-addition.

**Tech Stack:** Rust, notify (FSEvents), egui/eframe, serde, rfd

---

### Task 1: Add `untracked_repos` to Config

**Files:**
- Modify: `src/config.rs:73-84` (Config struct)
- Modify: `src/config.rs:86-100` (Default impl)
- Test: `tests/config_tests.rs`

**Step 1: Write the failing test**

Add to `tests/config_tests.rs`:

```rust
#[test]
fn test_config_untracked_repos_default_empty() {
    let config = gitmap::config::Config::default();
    assert!(config.untracked_repos.is_empty());
}

#[test]
fn test_config_migration_without_untracked_repos() {
    // Simulates loading an old config that doesn't have untracked_repos
    let json = r#"{"tracked_repos":[],"auto_discover_roots":[],"accent_color":"#39d353","time_range":"Months12","data_mode":"Commits","selected_year":2026,"view_mode":"Year","auto_update":false}"#;
    let config: gitmap::config::Config = serde_json::from_str(json).unwrap();
    assert!(config.untracked_repos.is_empty());
}

#[test]
fn test_config_roundtrip_with_untracked_repos() {
    let mut config = gitmap::config::Config::default();
    config.untracked_repos.push(std::path::PathBuf::from("/Users/test/ignored"));
    let json = serde_json::to_string(&config).unwrap();
    let loaded: gitmap::config::Config = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.untracked_repos.len(), 1);
    assert_eq!(loaded.untracked_repos[0], std::path::PathBuf::from("/Users/test/ignored"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test config_tests`
Expected: FAIL — `untracked_repos` field doesn't exist

**Step 3: Add `untracked_repos` field to Config**

In `src/config.rs`, add to the `Config` struct after `last_updated_version`:

```rust
    #[serde(default)]
    pub untracked_repos: Vec<PathBuf>,
```

In `Default for Config`, add to the struct literal:

```rust
            untracked_repos: Vec::new(),
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test config_tests`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add src/config.rs tests/config_tests.rs
git commit -m "feat: add untracked_repos to Config with serde migration"
```

---

### Task 2: Create `DiscoveryWatcher` module

**Files:**
- Create: `src/discovery_watcher.rs`
- Modify: `src/lib.rs:1-8` (add module declaration)
- Test: `tests/discovery_watcher_tests.rs`

**Step 1: Write the failing test**

Create `tests/discovery_watcher_tests.rs`:

```rust
use gitmap::discovery_watcher::DiscoveryWatcher;
use std::process::Command;

#[test]
fn test_discovery_watcher_detects_new_repo() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().to_path_buf();

    let mut watcher = DiscoveryWatcher::new().unwrap();
    watcher.watch_root(&root).unwrap();

    // Give FSEvents time to register
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Create a new repo in the watched directory
    let repo_path = root.join("new-project");
    std::fs::create_dir_all(&repo_path).unwrap();
    Command::new("git").args(["init"]).current_dir(&repo_path).output().unwrap();

    // Wait for FSEvents + debounce (3 seconds) + buffer
    std::thread::sleep(std::time::Duration::from_secs(5));

    let discovered = watcher.poll_new_repos();
    assert!(
        discovered.contains(&repo_path),
        "Expected {:?} in discovered repos: {:?}",
        repo_path,
        discovered
    );
}

#[test]
fn test_discovery_watcher_ignores_non_git_dirs() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().to_path_buf();

    let mut watcher = DiscoveryWatcher::new().unwrap();
    watcher.watch_root(&root).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));

    // Create a directory that is NOT a git repo
    let not_repo = root.join("just-a-folder");
    std::fs::create_dir_all(&not_repo).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5));

    let discovered = watcher.poll_new_repos();
    assert!(
        discovered.is_empty(),
        "Expected no repos but got: {:?}",
        discovered
    );
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test discovery_watcher_tests`
Expected: FAIL — module doesn't exist

**Step 3: Create `src/discovery_watcher.rs`**

```rust
use notify::{Event, EventKind, RecursiveMode, Result as NotifyResult, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

const DEBOUNCE_SECS: u64 = 3;

pub struct DiscoveryWatcher {
    watcher: notify::RecommendedWatcher,
    rx: mpsc::Receiver<NotifyResult<Event>>,
    watched_roots: Vec<PathBuf>,
    /// Paths pending debounce: path -> first seen timestamp
    pending: HashMap<PathBuf, Instant>,
}

impl DiscoveryWatcher {
    pub fn new() -> NotifyResult<Self> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(tx)?;
        Ok(Self {
            watcher,
            rx,
            watched_roots: Vec::new(),
            pending: HashMap::new(),
        })
    }

    /// Start watching a root directory for new subdirectories.
    pub fn watch_root(&mut self, root: &Path) -> NotifyResult<()> {
        if !self.watched_roots.contains(&root.to_path_buf()) {
            self.watcher.watch(root, RecursiveMode::NonRecursive)?;
            self.watched_roots.push(root.to_path_buf());
        }
        Ok(())
    }

    /// Stop watching a root directory.
    pub fn unwatch_root(&mut self, root: &Path) -> NotifyResult<()> {
        self.watcher.unwatch(root)?;
        self.watched_roots.retain(|p| p != root);
        Ok(())
    }

    /// Poll for newly discovered git repos. Call this from the UI event loop.
    /// Returns repo root paths (parent of .git) that are ready (debounce elapsed).
    pub fn poll_new_repos(&mut self) -> Vec<PathBuf> {
        // Drain FSEvents into pending map
        while let Ok(Ok(event)) = self.rx.try_recv() {
            if matches!(event.kind, EventKind::Create(_)) {
                for path in event.paths {
                    if path.is_dir() && !self.pending.contains_key(&path) {
                        self.pending.insert(path, Instant::now());
                    }
                }
            }
        }

        // Check debounced entries for .git
        let now = Instant::now();
        let debounce = std::time::Duration::from_secs(DEBOUNCE_SECS);
        let mut ready = Vec::new();
        let mut done = Vec::new();

        for (path, first_seen) in &self.pending {
            if now.duration_since(*first_seen) >= debounce {
                done.push(path.clone());
                if path.join(".git").is_dir() {
                    ready.push(path.clone());
                }
            }
        }

        for path in done {
            self.pending.remove(&path);
        }

        ready
    }
}
```

**Step 4: Add module declaration to `src/lib.rs`**

Add after `pub mod discovery;`:

```rust
pub mod discovery_watcher;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test discovery_watcher_tests`
Expected: ALL PASS (may be slow due to sleep-based FSEvents tests)

**Step 6: Commit**

```bash
git add src/discovery_watcher.rs src/lib.rs tests/discovery_watcher_tests.rs
git commit -m "feat: add DiscoveryWatcher module for watching discover roots"
```

---

### Task 3: Startup discovery scan

**Files:**
- Modify: `src/ui/popover.rs:44-106` (GitMapApp::new)
- Modify: `src/ui/popover.rs:109-130` (initial_scan)

**Step 1: Import the discovery module**

In `src/ui/popover.rs`, add import at the top (after existing use statements around line 12):

```rust
use crate::discovery::discover_repos;
use crate::discovery_watcher::DiscoveryWatcher;
```

**Step 2: Add `DiscoveryWatcher` field to `GitMapApp`**

In `src/ui/popover.rs`, add to the `GitMapApp` struct (after `last_update_check` around line 40):

```rust
    discovery_watcher: Option<DiscoveryWatcher>,
```

**Step 3: Initialize discovery watcher and run startup scan in `GitMapApp::new`**

After the `RepoWatcher` setup (around line 55), add:

```rust
        // Startup discovery: scan all discover roots for new repos
        for root in &config.auto_discover_roots {
            let discovered = discover_repos(root);
            for repo in discovered {
                if !config.tracked_repos.contains(&repo)
                    && !config.untracked_repos.contains(&repo)
                {
                    config.tracked_repos.push(repo);
                }
            }
        }

        // Start watching discover roots
        let mut discovery_watcher = DiscoveryWatcher::new().ok();
        if let Some(ref mut dw) = discovery_watcher {
            for root in &config.auto_discover_roots {
                let _ = dw.watch_root(root);
            }
        }
```

Add `discovery_watcher` to the `Self { ... }` return struct:

```rust
            discovery_watcher,
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add src/ui/popover.rs
git commit -m "feat: startup discovery scan and DiscoveryWatcher initialization"
```

---

### Task 4: Poll DiscoveryWatcher in update loop

**Files:**
- Modify: `src/ui/popover.rs:426-451` (update method, after existing RepoWatcher poll)

**Step 1: Add discovery watcher polling**

In the `update()` method, after the existing `RepoWatcher` poll block (around line 450, after the `changed_repos` handling), add:

```rust
        // Poll discovery watcher for new repos
        if let Some(ref mut dw) = self.discovery_watcher {
            let new_repos = dw.poll_new_repos();
            for repo in new_repos {
                if !self.config.tracked_repos.contains(&repo)
                    && !self.config.untracked_repos.contains(&repo)
                {
                    self.config.tracked_repos.push(repo.clone());
                    // Start watching the new repo's .git
                    if let Some(ref mut w) = self.watcher {
                        let _ = w.watch_repo(&repo);
                    }
                    // Scan the new repo
                    if let Ok(identity) = scanner::detect_identity(&repo) {
                        if let Ok(stats) = scanner::scan_repo(&repo, &identity, None) {
                            self.store.merge(stats);
                        }
                    }
                    let _ = self.config.save();
                    let history_path = crate::config::data_dir().join("history.json");
                    let _ = self.store.save_to(&history_path);
                }
            }
        }
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src/ui/popover.rs
git commit -m "feat: poll DiscoveryWatcher in update loop for auto-tracking"
```

---

### Task 5: Settings UI — Watch Directories section

**Files:**
- Modify: `src/ui/settings.rs:8-15` (SettingsState struct)
- Modify: `src/ui/settings.rs:17-27` (SettingsState::new)
- Modify: `src/ui/settings.rs:37-176` (draw_settings function)

**Step 1: Add discover root picker to SettingsState**

In `src/ui/settings.rs`, add a new field to `SettingsState` (around line 14):

```rust
    discover_root_picker_result: Arc<Mutex<Option<PathBuf>>>,
```

Initialize it in `SettingsState::new` (around line 22):

```rust
            discover_root_picker_result: Arc::new(Mutex::new(None)),
```

**Step 2: Add Watch Directories section to draw_settings**

Insert a new section after the Git Identity section (around line 59) and before the Tracked Repos section. Also replace "Scan Directory..." button:

After the Git Identity section (after `ui.add_space(12.0);` on line 59), add:

```rust
        // --- Watch Directories ---
        ui.label(egui::RichText::new("Watch Directories").strong().size(14.0));
        ui.add_space(4.0);

        // Check for discover root picker result
        if let Ok(mut guard) = state.discover_root_picker_result.try_lock() {
            if let Some(path) = guard.take() {
                if !config.auto_discover_roots.contains(&path) {
                    config.auto_discover_roots.push(path);
                }
            }
        }

        if config.auto_discover_roots.is_empty() {
            ui.label(
                egui::RichText::new("No directories watched")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(100, 100, 100)),
            );
        } else {
            let mut root_to_remove = None;
            for (i, root) in config.auto_discover_roots.iter().enumerate() {
                ui.horizontal(|ui| {
                    let display = root
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| root.display().to_string());
                    ui.label(
                        egui::RichText::new(&display)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 200)),
                    );
                    if ui.small_button("\u{2715}").clicked() {
                        root_to_remove = Some(i);
                    }
                })
                .response
                .on_hover_text(root.display().to_string());
            }
            if let Some(i) = root_to_remove {
                config.auto_discover_roots.remove(i);
            }
        }

        ui.add_space(4.0);
        if ui.button("Add Watch Directory...").clicked() {
            let result = Arc::clone(&state.discover_root_picker_result);
            let ctx = ui.ctx().clone();
            let picker_flag = Arc::clone(&state.file_picker_active);
            picker_flag.store(true, Ordering::Relaxed);
            std::thread::spawn(move || {
                let folder = rfd::FileDialog::new()
                    .set_title("Select Directory to Watch for Repos")
                    .pick_folder();
                if let Ok(mut guard) = result.lock() {
                    *guard = folder;
                }
                picker_flag.store(false, Ordering::Relaxed);
                ctx.request_repaint();
            });
        }

        ui.add_space(12.0);
```

**Step 3: Move untracked repos from SettingsState to Config**

In `src/ui/settings.rs`, remove `pub untracked_repos: Vec<PathBuf>` from `SettingsState` (line 13) and its initialization in `new()`.

Update all references to `state.untracked_repos` in `draw_settings` to use `config.untracked_repos` instead. There are references at:
- Line 69: `state.untracked_repos.contains` → `config.untracked_repos.contains`
- Line 70: `state.untracked_repos.push` → `config.untracked_repos.push`
- Line 82: `state.untracked_repos.retain` → `config.untracked_repos.retain`
- Line 95: `state.untracked_repos.contains` → `config.untracked_repos.contains`
- Line 125: `state.untracked_repos.contains` → `config.untracked_repos.contains`
- Line 126: `state.untracked_repos.push` → `config.untracked_repos.push`
- Line 179: `state.untracked_repos.is_empty` → `config.untracked_repos.is_empty`
- Line 185: `state.untracked_repos.iter` → `config.untracked_repos.iter`
- Line 203: `state.untracked_repos.remove` → `config.untracked_repos.remove`

**Step 4: Remove "Scan Directory..." button**

Delete the "Scan Directory..." button block (lines 157-175) from `draw_settings`. The `discover_result` field in `SettingsState` and its handler (lines 90-101) can also be removed since discover roots now handle this.

Remove `discover_result` field from `SettingsState` struct and its initialization.

Remove the `discover_result` handler block (lines 90-101).

Remove the unused `use crate::discovery::discover_repos;` import (line 2).

**Step 5: Verify it compiles**

Run: `cargo build`
Expected: SUCCESS

**Step 6: Commit**

```bash
git add src/ui/settings.rs
git commit -m "feat: settings UI for watch directories and persisted untracked repos"
```

---

### Task 6: Wire discovery watcher updates from settings changes

**Files:**
- Modify: `src/ui/popover.rs:605-626` (Back button handler in settings)

**Step 1: Update the Back button handler**

In the settings Back button handler (around line 606-626), after the existing watcher update, add discovery watcher sync:

```rust
                    // Update discovery watcher for any changed roots
                    if let Some(ref mut dw) = self.discovery_watcher {
                        // Simple approach: unwatching missing roots would require
                        // tracking previous state. Instead, re-create the watcher
                        // with current roots on settings change.
                    }
                    // Re-discover repos from all roots
                    for root in &self.config.auto_discover_roots {
                        let discovered = crate::discovery::discover_repos(root);
                        for repo in discovered {
                            if !self.config.tracked_repos.contains(&repo)
                                && !self.config.untracked_repos.contains(&repo)
                            {
                                self.config.tracked_repos.push(repo);
                            }
                        }
                    }
```

Also rebuild the discovery watcher with current roots. Replace the simple comment block above with:

```rust
                    // Rebuild discovery watcher with current roots
                    self.discovery_watcher = DiscoveryWatcher::new().ok();
                    if let Some(ref mut dw) = self.discovery_watcher {
                        for root in &self.config.auto_discover_roots {
                            let _ = dw.watch_root(root);
                        }
                    }
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src/ui/popover.rs
git commit -m "feat: sync discovery watcher when settings change"
```

---

### Task 7: Integration test — end to end discovery

**Files:**
- Create: `tests/discovery_watcher_tests.rs` (already created in Task 2, add more tests)

**Step 1: Add test for skipping already-tracked repos**

Add to `tests/discovery_watcher_tests.rs`:

```rust
#[test]
fn test_discovery_watcher_unwatch_root() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().to_path_buf();

    let mut watcher = DiscoveryWatcher::new().unwrap();
    watcher.watch_root(&root).unwrap();
    // Should not panic on unwatch
    watcher.unwatch_root(&root).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));

    // Create a repo after unwatching — should NOT be detected
    let repo_path = root.join("post-unwatch");
    std::fs::create_dir_all(&repo_path).unwrap();
    Command::new("git").args(["init"]).current_dir(&repo_path).output().unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5));

    let discovered = watcher.poll_new_repos();
    assert!(discovered.is_empty());
}
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: ALL PASS

**Step 3: Commit**

```bash
git add tests/discovery_watcher_tests.rs
git commit -m "test: add discovery watcher unwatch test"
```

---

### Task 8: Run full test suite and verify

**Step 1: Run complete test suite**

Run: `cargo test`
Expected: ALL PASS

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Manual smoke test**

Run: `cargo run`

1. Open Settings
2. Add a Watch Directory (e.g., a temp folder)
3. Go back to heatmap view
4. Create a new git repo inside the watched folder: `mkdir /tmp/test-watch/new-project && cd /tmp/test-watch/new-project && git init`
5. Wait ~5 seconds, re-open popover
6. Verify repo count increased and repo appears in Settings

**Step 4: Commit any fixes if needed, then final commit**

```bash
git add -A
git commit -m "chore: auto-discovery feature complete"
```
