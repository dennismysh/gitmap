# gitmap Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a macOS menu bar app that displays a GitHub-style git commit heatmap sourced from local repositories.

**Architecture:** Three layers — UI (eframe/egui + tray-icon for menu bar popover), Data (git2 for commit scanning, notify for FSEvents watching, in-memory store with JSON cache), Config (serde JSON persistence in ~/Library/Application Support/gitmap/).

**Tech Stack:** Rust, eframe 0.32, egui 0.32, tray-icon 0.21, git2 0.20, notify 8.2, chrono, serde, rfd, walkdir

---

## API Reference Notes

These are verified against current docs and must be followed exactly:

- **egui 0.31+**: `Rounding` was renamed to `CornerRadius`. Fields are `u8` not `f32`. Use `CornerRadius::same(3)`.
- **egui `rect_stroke`**: Requires a `StrokeKind` parameter (`StrokeKind::Outside`, `Inside`, `Middle`).
- **eframe `app_creator`**: Returns `Result<Box<dyn App>>` — wrap with `Ok(...)`.
- **`ViewportCommand::Visible(false)`**: Has a [known bug](https://github.com/emilk/egui/issues/5229) where setting `Visible(false)` stops the event loop. Use `Minimized(true/false)` + `OuterPosition` as workaround.
- **tray-icon**: `set_event_handler` and `receiver()` are mutually exclusive. On macOS, tray icon must be created on the main thread (inside `app_creator`).
- **tray-icon `with_menu_on_left_click(false)`**: Left click fires our handler, right click opens menu.
- **notify**: `mpsc::Sender<notify::Result<Event>>` implements `EventHandler` — can be passed directly to `recommended_watcher()`.
- **macOS template icons**: Use black + alpha only PNG. Set `with_icon_as_template(true)` for auto dark/light mode adaptation.

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `assets/icon.rgba` (placeholder)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "gitmap"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.32"
egui = "0.32"
tray-icon = "0.21"
git2 = "0.20"
notify = "8.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
rfd = "0.15"
walkdir = "2"
dirs = "6"
```

**Step 2: Create minimal main.rs that compiles**

```rust
fn main() {
    println!("gitmap");
}
```

**Step 3: Build to verify dependencies resolve**

Run: `cargo build`
Expected: Compiles successfully (may take a while for first build)

**Step 4: Create placeholder icon**

Generate a 22x22 RGBA icon file. For now, create a simple black square:

```rust
// Add this as a build step or run once manually:
// 22 * 22 * 4 = 1936 bytes, all black with full alpha
fn main() {
    let mut rgba = Vec::with_capacity(22 * 22 * 4);
    for _ in 0..(22 * 22) {
        rgba.extend_from_slice(&[0, 0, 0, 255]); // black, fully opaque
    }
    std::fs::write("assets/icon.rgba", &rgba).unwrap();
}
```

Create `assets/` directory and generate the icon file.

**Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs assets/
git commit -m "feat: scaffold project with dependencies"
```

---

### Task 2: Config Module

**Files:**
- Create: `src/config.rs`
- Create: `tests/config_tests.rs`
- Modify: `src/main.rs` (add `mod config;`)

**Step 1: Write failing tests for config**

Create `tests/config_tests.rs`:

```rust
use std::path::PathBuf;

// We'll test the config module's serialization/deserialization
// and default creation logic

#[test]
fn test_default_config_has_green_accent() {
    let config = gitmap::config::Config::default();
    assert_eq!(config.accent_color, "#39d353");
}

#[test]
fn test_default_config_has_12m_range() {
    let config = gitmap::config::Config::default();
    assert_eq!(config.time_range, gitmap::config::TimeRange::Months12);
}

#[test]
fn test_config_roundtrip_json() {
    let mut config = gitmap::config::Config::default();
    config.tracked_repos.push(PathBuf::from("/Users/test/repo1"));
    config.auto_discover_roots.push(PathBuf::from("/Users/test/projects"));
    config.accent_color = "#7c3aed".to_string();
    config.time_range = gitmap::config::TimeRange::Months3;

    let json = serde_json::to_string(&config).unwrap();
    let loaded: gitmap::config::Config = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.tracked_repos, config.tracked_repos);
    assert_eq!(loaded.auto_discover_roots, config.auto_discover_roots);
    assert_eq!(loaded.accent_color, "#7c3aed");
    assert_eq!(loaded.time_range, gitmap::config::TimeRange::Months3);
}

#[test]
fn test_config_data_dir() {
    let dir = gitmap::config::data_dir();
    assert!(dir.ends_with("gitmap"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test config_tests`
Expected: FAIL — module doesn't exist yet

**Step 3: Create `src/config.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeRange {
    Days30,
    Months3,
    Months6,
    Months12,
}

impl TimeRange {
    pub fn label(&self) -> &'static str {
        match self {
            TimeRange::Days30 => "30 days",
            TimeRange::Months3 => "3 months",
            TimeRange::Months6 => "6 months",
            TimeRange::Months12 => "12 months",
        }
    }

    pub fn all() -> &'static [TimeRange] {
        &[
            TimeRange::Days30,
            TimeRange::Months3,
            TimeRange::Months6,
            TimeRange::Months12,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataMode {
    Commits,
    LinesChanged,
}

impl DataMode {
    pub fn label(&self) -> &'static str {
        match self {
            DataMode::Commits => "Commits",
            DataMode::LinesChanged => "Lines changed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub tracked_repos: Vec<PathBuf>,
    pub auto_discover_roots: Vec<PathBuf>,
    pub accent_color: String,
    pub time_range: TimeRange,
    pub data_mode: DataMode,
    pub selected_year: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tracked_repos: Vec::new(),
            auto_discover_roots: Vec::new(),
            accent_color: "#39d353".to_string(), // GitHub green
            time_range: TimeRange::Months12,
            data_mode: DataMode::Commits,
            selected_year: chrono::Local::now().year(),
        }
    }
}

use chrono::Datelike;

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitmap")
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&contents).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)
    }
}
```

**Step 4: Update `src/main.rs` to expose config as a library**

```rust
pub mod config;

fn main() {
    println!("gitmap");
}
```

Also add to `Cargo.toml`:

```toml
[lib]
name = "gitmap"
path = "src/lib.rs"

[[bin]]
name = "gitmap"
path = "src/main.rs"
```

Create `src/lib.rs`:

```rust
pub mod config;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test config_tests`
Expected: All 4 tests PASS

**Step 6: Commit**

```bash
git add src/config.rs src/lib.rs src/main.rs tests/config_tests.rs Cargo.toml
git commit -m "feat: add config module with serialization and persistence"
```

---

### Task 3: Git Scanner Module

**Files:**
- Create: `src/scanner.rs`
- Create: `tests/scanner_tests.rs`
- Modify: `src/lib.rs` (add `pub mod scanner;`)

**Step 1: Define data types and write failing tests**

Create `tests/scanner_tests.rs`:

```rust
use gitmap::scanner::{DayStats, GitIdentity, scan_repo};
use std::process::Command;
use tempfile::TempDir;

fn create_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test User"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(path).output().unwrap();

    // Create a file and commit
    std::fs::write(path.join("file.txt"), "hello world\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git").args(["commit", "-m", "initial commit"]).current_dir(path).output().unwrap();

    // Add more content and commit again
    std::fs::write(path.join("file.txt"), "hello world\nline 2\nline 3\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git").args(["commit", "-m", "add lines"]).current_dir(path).output().unwrap();

    dir
}

#[test]
fn test_scan_repo_finds_commits() {
    let repo_dir = create_test_repo();
    let identity = GitIdentity {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };

    let stats = scan_repo(repo_dir.path(), &identity, None).unwrap();
    assert!(!stats.is_empty(), "Should find at least one day with commits");

    let total_commits: u32 = stats.values().map(|s| s.commits).sum();
    assert_eq!(total_commits, 2, "Should find 2 commits");
}

#[test]
fn test_scan_repo_filters_by_author() {
    let repo_dir = create_test_repo();
    let wrong_identity = GitIdentity {
        name: "Wrong User".to_string(),
        email: "wrong@example.com".to_string(),
    };

    let stats = scan_repo(repo_dir.path(), &wrong_identity, None).unwrap();
    assert!(stats.is_empty(), "Should find no commits for wrong author");
}

#[test]
fn test_scan_repo_counts_line_changes() {
    let repo_dir = create_test_repo();
    let identity = GitIdentity {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };

    let stats = scan_repo(repo_dir.path(), &identity, None).unwrap();
    let total_insertions: u32 = stats.values().map(|s| s.insertions).sum();
    assert!(total_insertions > 0, "Should have some insertions");
}

#[test]
fn test_day_stats_merge() {
    let a = DayStats { commits: 3, insertions: 10, deletions: 5 };
    let b = DayStats { commits: 2, insertions: 7, deletions: 3 };
    let merged = a.merge(&b);
    assert_eq!(merged.commits, 5);
    assert_eq!(merged.insertions, 17);
    assert_eq!(merged.deletions, 8);
}

#[test]
fn test_detect_git_identity() {
    let repo_dir = create_test_repo();
    let identity = gitmap::scanner::detect_identity(repo_dir.path()).unwrap();
    assert_eq!(identity.name, "Test User");
    assert_eq!(identity.email, "test@example.com");
}
```

Add `tempfile` as a dev dependency in `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test scanner_tests`
Expected: FAIL — module doesn't exist

**Step 3: Create `src/scanner.rs`**

```rust
use chrono::NaiveDate;
use git2::{Repository, Sort};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayStats {
    pub commits: u32,
    pub insertions: u32,
    pub deletions: u32,
}

impl DayStats {
    pub fn merge(&self, other: &DayStats) -> DayStats {
        DayStats {
            commits: self.commits + other.commits,
            insertions: self.insertions + other.insertions,
            deletions: self.deletions + other.deletions,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitIdentity {
    pub name: String,
    pub email: String,
}

/// Detect the git user.name and user.email configured for a repo.
pub fn detect_identity(repo_path: &Path) -> Result<GitIdentity, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let config = repo.config()?;
    let name = config.get_string("user.name").unwrap_or_default();
    let email = config.get_string("user.email").unwrap_or_default();
    Ok(GitIdentity { name, email })
}

/// Scan a git repository and return daily commit stats.
///
/// If `since` is provided, only commits after that date are included.
/// Commits are filtered to only those matching the given identity (by name or email).
pub fn scan_repo(
    repo_path: &Path,
    identity: &GitIdentity,
    since: Option<NaiveDate>,
) -> Result<HashMap<NaiveDate, DayStats>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let mut stats: HashMap<NaiveDate, DayStats> = HashMap::new();

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        // Extract date
        let timestamp = commit.time().seconds();
        let date = chrono::DateTime::from_timestamp(timestamp, 0)
            .map(|dt| dt.naive_local().date())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

        // If we have a since filter and this commit is before it, skip
        // (commits are sorted by time descending, so we can break early)
        if let Some(since_date) = since {
            if date < since_date {
                break;
            }
        }

        // Filter by author
        let author = commit.author();
        let author_name = author.name().unwrap_or("");
        let author_email = author.email().unwrap_or("");

        if author_name != identity.name && author_email != identity.email {
            continue;
        }

        // Compute diff stats
        let (insertions, deletions) = if commit.parent_count() > 0 {
            let parent = commit.parent(0)?;
            let parent_tree = parent.tree()?;
            let commit_tree = commit.tree()?;
            let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;
            let diff_stats = diff.stats()?;
            (diff_stats.insertions() as u32, diff_stats.deletions() as u32)
        } else {
            let commit_tree = commit.tree()?;
            let diff = repo.diff_tree_to_tree(None, Some(&commit_tree), None)?;
            let diff_stats = diff.stats()?;
            (diff_stats.insertions() as u32, diff_stats.deletions() as u32)
        };

        let entry = stats.entry(date).or_insert(DayStats {
            commits: 0,
            insertions: 0,
            deletions: 0,
        });
        entry.commits += 1;
        entry.insertions += insertions;
        entry.deletions += deletions;
    }

    Ok(stats)
}
```

**Step 4: Add module to `src/lib.rs`**

```rust
pub mod config;
pub mod scanner;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test scanner_tests`
Expected: All 5 tests PASS

**Step 6: Commit**

```bash
git add src/scanner.rs src/lib.rs tests/scanner_tests.rs Cargo.toml
git commit -m "feat: add git scanner with author filtering and diff stats"
```

---

### Task 4: Commit Store Module

**Files:**
- Create: `src/store.rs`
- Create: `tests/store_tests.rs`
- Modify: `src/lib.rs` (add `pub mod store;`)

**Step 1: Write failing tests**

Create `tests/store_tests.rs`:

```rust
use chrono::NaiveDate;
use gitmap::scanner::DayStats;
use gitmap::store::CommitStore;
use std::collections::HashMap;

#[test]
fn test_empty_store() {
    let store = CommitStore::new();
    assert!(store.stats().is_empty());
}

#[test]
fn test_merge_stats() {
    let mut store = CommitStore::new();

    let mut repo1_stats = HashMap::new();
    repo1_stats.insert(
        NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
        DayStats { commits: 3, insertions: 50, deletions: 10 },
    );
    repo1_stats.insert(
        NaiveDate::from_ymd_opt(2026, 2, 21).unwrap(),
        DayStats { commits: 1, insertions: 20, deletions: 5 },
    );

    store.merge(repo1_stats);

    let mut repo2_stats = HashMap::new();
    repo2_stats.insert(
        NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
        DayStats { commits: 2, insertions: 30, deletions: 8 },
    );

    store.merge(repo2_stats);

    let day = store.get(NaiveDate::from_ymd_opt(2026, 2, 22).unwrap());
    assert!(day.is_some());
    let day = day.unwrap();
    assert_eq!(day.commits, 5);
    assert_eq!(day.insertions, 80);
    assert_eq!(day.deletions, 18);
}

#[test]
fn test_store_save_and_load_roundtrip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("history.json");

    let mut store = CommitStore::new();
    let mut stats = HashMap::new();
    stats.insert(
        NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
        DayStats { commits: 4, insertions: 100, deletions: 25 },
    );
    store.merge(stats);
    store.save_to(&path).unwrap();

    let loaded = CommitStore::load_from(&path).unwrap();
    let day = loaded.get(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
    assert!(day.is_some());
    assert_eq!(day.unwrap().commits, 4);
}

#[test]
fn test_most_recent_date() {
    let mut store = CommitStore::new();
    assert!(store.most_recent_date().is_none());

    let mut stats = HashMap::new();
    stats.insert(
        NaiveDate::from_ymd_opt(2026, 2, 20).unwrap(),
        DayStats { commits: 1, insertions: 5, deletions: 0 },
    );
    stats.insert(
        NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
        DayStats { commits: 2, insertions: 10, deletions: 3 },
    );
    store.merge(stats);

    assert_eq!(
        store.most_recent_date(),
        Some(NaiveDate::from_ymd_opt(2026, 2, 22).unwrap())
    );
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test store_tests`
Expected: FAIL — module doesn't exist

**Step 3: Create `src/store.rs`**

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::scanner::DayStats;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStore {
    stats: HashMap<NaiveDate, DayStats>,
}

impl CommitStore {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    pub fn stats(&self) -> &HashMap<NaiveDate, DayStats> {
        &self.stats
    }

    pub fn get(&self, date: NaiveDate) -> Option<&DayStats> {
        self.stats.get(&date)
    }

    pub fn merge(&mut self, new_stats: HashMap<NaiveDate, DayStats>) {
        for (date, new) in new_stats {
            let entry = self.stats.entry(date).or_insert(DayStats {
                commits: 0,
                insertions: 0,
                deletions: 0,
            });
            *entry = entry.merge(&new);
        }
    }

    pub fn most_recent_date(&self) -> Option<NaiveDate> {
        self.stats.keys().max().copied()
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    pub fn load_from(path: &Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
```

**Step 4: Add module to `src/lib.rs`**

```rust
pub mod config;
pub mod scanner;
pub mod store;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test store_tests`
Expected: All 4 tests PASS

**Step 6: Commit**

```bash
git add src/store.rs src/lib.rs tests/store_tests.rs
git commit -m "feat: add commit store with JSON persistence and merge"
```

---

### Task 5: File Watcher Module

**Files:**
- Create: `src/watcher.rs`
- Modify: `src/lib.rs` (add `pub mod watcher;`)

**Step 1: Create `src/watcher.rs`**

Note: FSEvents-based file watching is difficult to unit test (requires actual filesystem events with timing). We test this through integration testing later.

```rust
use notify::{Event, RecursiveMode, Result as NotifyResult, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

pub struct RepoWatcher {
    watcher: notify::RecommendedWatcher,
    watched_paths: Vec<PathBuf>,
    pub rx: mpsc::Receiver<NotifyResult<Event>>,
}

impl RepoWatcher {
    pub fn new() -> NotifyResult<Self> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(tx)?;
        Ok(Self {
            watcher,
            watched_paths: Vec::new(),
            rx,
        })
    }

    /// Start watching a repo's .git directory for changes.
    pub fn watch_repo(&mut self, repo_path: &Path) -> NotifyResult<()> {
        let git_dir = repo_path.join(".git");
        if git_dir.is_dir() {
            self.watcher.watch(&git_dir, RecursiveMode::Recursive)?;
            self.watched_paths.push(git_dir);
        }
        Ok(())
    }

    /// Stop watching a repo.
    pub fn unwatch_repo(&mut self, repo_path: &Path) -> NotifyResult<()> {
        let git_dir = repo_path.join(".git");
        self.watcher.unwatch(&git_dir)?;
        self.watched_paths.retain(|p| p != &git_dir);
        Ok(())
    }

    /// Drain all pending events, returning paths of repos that changed.
    pub fn poll_changed_repos(&self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        while let Ok(Ok(event)) = self.rx.try_recv() {
            for path in &event.paths {
                // Walk up from the changed file to find the repo root
                // (parent of .git directory)
                if let Some(repo_root) = find_repo_root(path) {
                    if !changed.contains(&repo_root) {
                        changed.push(repo_root);
                    }
                }
            }
        }
        changed
    }
}

/// Given a path inside a .git directory, find the repo root (parent of .git).
fn find_repo_root(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    loop {
        if current.file_name().map(|n| n == ".git").unwrap_or(false) {
            return current.parent().map(|p| p.to_path_buf());
        }
        current = current.parent()?;
    }
}
```

**Step 2: Add module to `src/lib.rs`**

```rust
pub mod config;
pub mod scanner;
pub mod store;
pub mod watcher;
```

**Step 3: Build to verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/watcher.rs src/lib.rs
git commit -m "feat: add file watcher for repo .git directory monitoring"
```

---

### Task 6: Repo Discovery

**Files:**
- Create: `src/discovery.rs`
- Create: `tests/discovery_tests.rs`
- Modify: `src/lib.rs` (add `pub mod discovery;`)

**Step 1: Write failing tests**

Create `tests/discovery_tests.rs`:

```rust
use gitmap::discovery::discover_repos;
use std::process::Command;

#[test]
fn test_discover_finds_git_repos() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    // Create two nested git repos
    let repo1 = root.join("project-a");
    let repo2 = root.join("subdir").join("project-b");
    std::fs::create_dir_all(&repo1).unwrap();
    std::fs::create_dir_all(&repo2).unwrap();

    Command::new("git").args(["init"]).current_dir(&repo1).output().unwrap();
    Command::new("git").args(["init"]).current_dir(&repo2).output().unwrap();

    // Create a non-repo directory
    std::fs::create_dir_all(root.join("not-a-repo")).unwrap();

    let repos = discover_repos(root);
    assert_eq!(repos.len(), 2);
    assert!(repos.contains(&repo1));
    assert!(repos.contains(&repo2));
}

#[test]
fn test_discover_skips_hidden_dirs() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    let visible_repo = root.join("visible");
    let hidden_repo = root.join(".hidden").join("repo");
    std::fs::create_dir_all(&visible_repo).unwrap();
    std::fs::create_dir_all(&hidden_repo).unwrap();

    Command::new("git").args(["init"]).current_dir(&visible_repo).output().unwrap();
    Command::new("git").args(["init"]).current_dir(&hidden_repo).output().unwrap();

    let repos = discover_repos(root);
    assert_eq!(repos.len(), 1);
    assert!(repos.contains(&visible_repo));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test discovery_tests`
Expected: FAIL

**Step 3: Create `src/discovery.rs`**

```rust
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Recursively discover all git repositories under `root`.
/// Skips hidden directories (except checking for .git).
pub fn discover_repos(root: &Path) -> Vec<PathBuf> {
    let mut repos = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden directories at depth > 0
            let name = e.file_name().to_string_lossy();
            if e.depth() > 0 && name.starts_with('.') {
                return false;
            }
            true
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }
        if entry.path().join(".git").is_dir() {
            repos.push(entry.path().to_path_buf());
        }
    }

    repos
}
```

**Step 4: Add module to `src/lib.rs`**

```rust
pub mod config;
pub mod discovery;
pub mod scanner;
pub mod store;
pub mod watcher;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test discovery_tests`
Expected: All 2 tests PASS

**Step 6: Commit**

```bash
git add src/discovery.rs src/lib.rs tests/discovery_tests.rs
git commit -m "feat: add recursive git repo discovery with hidden dir filtering"
```

---

### Task 7: Heatmap Data Helpers

**Files:**
- Create: `src/heatmap.rs`
- Create: `tests/heatmap_tests.rs`
- Modify: `src/lib.rs` (add `pub mod heatmap;`)

**Step 1: Write failing tests**

Create `tests/heatmap_tests.rs`:

```rust
use chrono::NaiveDate;
use gitmap::heatmap::{color_for_level, grid_dates, level_for_value};

#[test]
fn test_grid_dates_for_year() {
    let dates = grid_dates(2026);
    // A full year grid: 53 columns x 7 rows = 371 cells max
    // Each inner vec has 7 entries (Mon-Sun)
    assert!(!dates.is_empty());
    assert!(dates.len() >= 52);
    for week in &dates {
        assert_eq!(week.len(), 7);
    }
    // First date should be a Monday
    assert_eq!(dates[0][0].weekday(), chrono::Weekday::Mon);
}

#[test]
fn test_level_for_value_zero_is_level_0() {
    assert_eq!(level_for_value(0, 10), 0);
}

#[test]
fn test_level_for_value_max_is_level_4() {
    assert_eq!(level_for_value(10, 10), 4);
}

#[test]
fn test_level_for_value_distributes_evenly() {
    let level = level_for_value(5, 20);
    assert!(level >= 1 && level <= 3);
}

#[test]
fn test_color_for_level_returns_5_levels() {
    let base = "#39d353";
    for level in 0..=4 {
        let color = color_for_level(level, base);
        // Should return a valid [u8; 4] RGBA
        assert_eq!(color.len(), 4);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test heatmap_tests`
Expected: FAIL

**Step 3: Create `src/heatmap.rs`**

```rust
use chrono::{Datelike, NaiveDate, Weekday};

/// Generate the grid of dates for a given year.
/// Returns Vec<Vec<NaiveDate>> where outer = weeks (columns), inner = days Mon-Sun (rows).
/// Pads the first and last weeks to always have 7 entries.
pub fn grid_dates(year: i32) -> Vec<Vec<NaiveDate>> {
    let jan1 = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let dec31 = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    // Find the Monday of the week containing Jan 1
    let start = jan1 - chrono::Duration::days(jan1.weekday().num_days_from_monday() as i64);
    // Find the Sunday of the week containing Dec 31
    let end = dec31 + chrono::Duration::days(6 - dec31.weekday().num_days_from_monday() as i64);

    let mut weeks = Vec::new();
    let mut current = start;

    while current <= end {
        let mut week = Vec::with_capacity(7);
        for _ in 0..7 {
            week.push(current);
            current += chrono::Duration::days(1);
        }
        weeks.push(week);
    }

    weeks
}

/// Map a value to a level 0-4 based on the maximum value in the dataset.
pub fn level_for_value(value: u32, max_value: u32) -> u8 {
    if value == 0 {
        return 0;
    }
    if max_value == 0 {
        return 0;
    }
    let ratio = value as f32 / max_value as f32;
    match ratio {
        r if r <= 0.25 => 1,
        r if r <= 0.50 => 2,
        r if r <= 0.75 => 3,
        _ => 4,
    }
}

/// Parse a hex color string like "#39d353" into [r, g, b].
fn parse_hex(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    [r, g, b]
}

/// Return an RGBA color [r, g, b, a] for a given level (0-4) and base accent color.
/// Level 0 = dark background, levels 1-4 = increasing intensity of the accent color.
pub fn color_for_level(level: u8, accent_hex: &str) -> [u8; 4] {
    let [r, g, b] = parse_hex(accent_hex);

    match level {
        0 => [22, 27, 34, 255],          // dark empty cell
        1 => [r / 4, g / 4, b / 4, 255], // 25% intensity
        2 => [r / 2, g / 2, b / 2, 255], // 50% intensity
        3 => [r * 3 / 4, g * 3 / 4, b * 3 / 4, 255], // 75% intensity
        _ => [r, g, b, 255],             // full intensity
    }
}
```

**Step 4: Add module to `src/lib.rs`**

```rust
pub mod config;
pub mod discovery;
pub mod heatmap;
pub mod scanner;
pub mod store;
pub mod watcher;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test heatmap_tests`
Expected: All 5 tests PASS

**Step 6: Commit**

```bash
git add src/heatmap.rs src/lib.rs tests/heatmap_tests.rs
git commit -m "feat: add heatmap data helpers for grid layout and color levels"
```

---

### Task 8: Tray Icon + egui Popover Shell

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/popover.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs` (add `pub mod ui;`)

This task wires together the tray icon and egui window. No heatmap rendering yet — just the shell.

**Step 1: Create `src/ui/mod.rs`**

```rust
pub mod popover;
```

**Step 2: Create `src/ui/popover.rs` with the app struct**

```rust
use eframe::egui;
use std::sync::mpsc;

#[derive(Debug)]
pub enum TrayMessage {
    ToggleWindow { icon_rect: tray_icon::Rect },
    Quit,
}

pub struct GitMapApp {
    tray_rx: mpsc::Receiver<TrayMessage>,
    visible: bool,
}

impl GitMapApp {
    pub fn new(tray_rx: mpsc::Receiver<TrayMessage>) -> Self {
        Self {
            tray_rx,
            visible: false,
        }
    }
}

impl eframe::App for GitMapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll tray messages
        while let Ok(msg) = self.tray_rx.try_recv() {
            match msg {
                TrayMessage::ToggleWindow { icon_rect } => {
                    self.visible = !self.visible;
                    if self.visible {
                        let icon_center_x = icon_rect.position.x
                            + (icon_rect.size.width as f64 / 2.0);
                        let popover_width = 420.0_f64;
                        let x = icon_center_x - (popover_width / 2.0);
                        let y = icon_rect.position.y + icon_rect.size.height as f64;

                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                            egui::pos2(x as f32, y as f32),
                        ));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                }
                TrayMessage::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        // Keep polling tray events
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        if !self.visible {
            return;
        }

        // Dark background
        let frame = egui::Frame::new()
            .fill(egui::Color32::from_rgb(13, 17, 23))
            .inner_margin(16.0)
            .corner_radius(12.0);

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(230, 237, 243));
            ui.heading("gitmap");
            ui.add_space(8.0);
            ui.label("Heatmap will render here.");
        });
    }
}
```

**Step 3: Update `src/main.rs` with tray icon + egui integration**

```rust
mod ui;

use std::sync::mpsc;
use ui::popover::{GitMapApp, TrayMessage};

fn main() -> eframe::Result<()> {
    let (tray_tx, tray_rx) = mpsc::channel::<TrayMessage>();

    // Generate a simple 22x22 black icon (template icon for macOS)
    let mut icon_rgba = Vec::with_capacity(22 * 22 * 4);
    for _ in 0..(22 * 22) {
        icon_rgba.extend_from_slice(&[0, 0, 0, 255]);
    }
    let icon = tray_icon::Icon::from_rgba(icon_rgba, 22, 22)
        .expect("Failed to create tray icon");

    let menu = tray_icon::menu::Menu::new();
    let quit_item = tray_icon::menu::MenuItem::new("Quit", true, None);
    menu.append(&quit_item).unwrap();
    let quit_id = quit_item.id().clone();

    // Set up tray event handlers (must be done before creating the icon)
    let tx_click = tray_tx.clone();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event: tray_icon::TrayIconEvent| {
        if let tray_icon::TrayIconEvent::Click { rect, button_state, .. } = event {
            if matches!(button_state, tray_icon::MouseButtonState::Up) {
                let _ = tx_click.send(TrayMessage::ToggleWindow { icon_rect: rect });
            }
        }
    }));

    let tx_menu = tray_tx;
    tray_icon::menu::MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
        if event.id() == &quit_id {
            let _ = tx_menu.send(TrayMessage::Quit);
        }
    }));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_inner_size([420.0, 380.0])
            .with_always_on_top()
            .with_resizable(false)
            .with_has_shadow(true)
            .with_title_shown(false)
            .with_titlebar_shown(false),
        ..Default::default()
    };

    use std::cell::RefCell;
    use std::rc::Rc;
    let tray_holder: Rc<RefCell<Option<tray_icon::TrayIcon>>> = Rc::new(RefCell::new(None));
    let tray_holder_clone = tray_holder.clone();

    eframe::run_native(
        "GitMap",
        options,
        Box::new(move |_cc| {
            // Create tray icon on the event loop thread (macOS requirement)
            let tray = tray_icon::TrayIconBuilder::new()
                .with_icon(icon)
                .with_icon_as_template(true)
                .with_tooltip("GitMap")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .unwrap();
            tray_holder_clone.borrow_mut().replace(tray);

            Ok(Box::new(GitMapApp::new(tray_rx)))
        }),
    )
}
```

**Step 4: Build and run to verify the tray icon appears**

Run: `cargo run`
Expected: A black icon appears in the macOS menu bar. Clicking it shows a dark popover with "gitmap" heading. Clicking again hides it. Right-click shows a "Quit" menu item.

**Step 5: Commit**

```bash
git add src/ui/ src/main.rs src/lib.rs
git commit -m "feat: add tray icon and egui popover shell"
```

---

### Task 9: Heatmap Rendering in the Popover

**Files:**
- Modify: `src/ui/popover.rs`

This task adds the actual heatmap grid rendering, header controls, hover display, and color legend.

**Step 1: Update `src/ui/popover.rs` with full heatmap rendering**

Update the `GitMapApp` struct to hold app state:

```rust
use crate::config::{Config, DataMode, TimeRange};
use crate::heatmap::{color_for_level, grid_dates, level_for_value};
use crate::scanner::DayStats;
use crate::store::CommitStore;
use eframe::egui;
use std::sync::mpsc;

#[derive(Debug)]
pub enum TrayMessage {
    ToggleWindow { icon_rect: tray_icon::Rect },
    Quit,
}

pub struct GitMapApp {
    tray_rx: mpsc::Receiver<TrayMessage>,
    visible: bool,
    config: Config,
    store: CommitStore,
    hovered_info: Option<String>,
    show_settings: bool,
}

impl GitMapApp {
    pub fn new(tray_rx: mpsc::Receiver<TrayMessage>, config: Config, store: CommitStore) -> Self {
        Self {
            tray_rx,
            visible: false,
            config,
            store,
            hovered_info: None,
            show_settings: false,
        }
    }

    fn draw_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("gitmap");

            ui.add_space(16.0);

            // Year navigation
            if ui.small_button("\u{25C0}").clicked() {
                self.config.selected_year -= 1;
            }
            ui.label(
                egui::RichText::new(format!("{}", self.config.selected_year))
                    .strong()
                    .size(16.0),
            );
            if ui.small_button("\u{25B6}").clicked() {
                self.config.selected_year += 1;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Time range dropdown
                egui::ComboBox::from_id_salt("time_range")
                    .selected_text(self.config.time_range.label())
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for &range in TimeRange::all() {
                            ui.selectable_value(
                                &mut self.config.time_range,
                                range,
                                range.label(),
                            );
                        }
                    });

                // Data mode dropdown
                egui::ComboBox::from_id_salt("data_mode")
                    .selected_text(self.config.data_mode.label())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.config.data_mode,
                            DataMode::Commits,
                            DataMode::Commits.label(),
                        );
                        ui.selectable_value(
                            &mut self.config.data_mode,
                            DataMode::LinesChanged,
                            DataMode::LinesChanged.label(),
                        );
                    });
            });
        });
    }

    fn draw_heatmap(&mut self, ui: &mut egui::Ui) {
        let weeks = grid_dates(self.config.selected_year);
        let cell_size = 14.0_f32;
        let cell_spacing = 3.0_f32;
        let label_width = 30.0_f32;

        // Find max value for color scaling
        let max_value = self.store.stats().values().map(|s| match self.config.data_mode {
            DataMode::Commits => s.commits,
            DataMode::LinesChanged => s.insertions + s.deletions,
        }).max().unwrap_or(1).max(1);

        let day_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

        let total_width = label_width + weeks.len() as f32 * (cell_size + cell_spacing);
        let total_height = 7.0 * (cell_size + cell_spacing);

        let (response, painter) = ui.allocate_painter(
            egui::vec2(total_width, total_height),
            egui::Sense::hover(),
        );

        let origin = response.rect.min;
        let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
        self.hovered_info = None;

        // Draw day labels
        for (row, label) in day_labels.iter().enumerate() {
            let y = origin.y + row as f32 * (cell_size + cell_spacing) + cell_size / 2.0;
            painter.text(
                egui::pos2(origin.x, y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(139, 148, 158),
            );
        }

        // Draw cells
        for (col, week) in weeks.iter().enumerate() {
            for (row, &date) in week.iter().enumerate() {
                let x = origin.x + label_width + col as f32 * (cell_size + cell_spacing);
                let y = origin.y + row as f32 * (cell_size + cell_spacing);

                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(cell_size, cell_size),
                );

                let stats = self.store.get(date);
                let value = stats.map(|s| match self.config.data_mode {
                    DataMode::Commits => s.commits,
                    DataMode::LinesChanged => s.insertions + s.deletions,
                }).unwrap_or(0);

                let level = level_for_value(value, max_value);
                let [r, g, b, a] = color_for_level(level, &self.config.accent_color);

                painter.rect_filled(
                    cell_rect,
                    egui::CornerRadius::same(3),
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a),
                );

                // Check hover
                if let Some(pos) = pointer_pos {
                    if cell_rect.contains(pos) {
                        painter.rect_stroke(
                            cell_rect,
                            egui::CornerRadius::same(3),
                            egui::Stroke::new(1.5, egui::Color32::WHITE),
                            egui::StrokeKind::Outside,
                        );
                        let info = if let Some(s) = stats {
                            format!(
                                "{}: {} commits, +{} -{} lines",
                                date.format("%b %d, %Y"),
                                s.commits, s.insertions, s.deletions
                            )
                        } else {
                            format!("{}: No commits", date.format("%b %d, %Y"))
                        };
                        self.hovered_info = Some(info);
                    }
                }
            }
        }
    }

    fn draw_legend(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Less")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(139, 148, 158)),
            );
            for level in 0..=4 {
                let [r, g, b, a] = color_for_level(level, &self.config.accent_color);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(12.0, 12.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(2),
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a),
                );
            }
            ui.label(
                egui::RichText::new("More")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(139, 148, 158)),
            );
        });
    }
}

impl eframe::App for GitMapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll tray messages
        while let Ok(msg) = self.tray_rx.try_recv() {
            match msg {
                TrayMessage::ToggleWindow { icon_rect } => {
                    self.visible = !self.visible;
                    if self.visible {
                        let icon_center_x = icon_rect.position.x
                            + (icon_rect.size.width as f64 / 2.0);
                        let popover_width = 420.0_f64;
                        let x = icon_center_x - (popover_width / 2.0);
                        let y = icon_rect.position.y + icon_rect.size.height as f64;

                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                            egui::pos2(x as f32, y as f32),
                        ));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                }
                TrayMessage::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        if !self.visible {
            return;
        }

        let frame = egui::Frame::new()
            .fill(egui::Color32::from_rgb(13, 17, 23))
            .inner_margin(16.0)
            .corner_radius(12.0);

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(230, 237, 243));

            if self.show_settings {
                // Settings view (Task 10)
                if ui.button("\u{2190} Back").clicked() {
                    self.show_settings = false;
                }
                ui.add_space(8.0);
                ui.label("Settings view (coming next)");
            } else {
                // Main heatmap view
                self.draw_header(ui);
                ui.add_space(12.0);

                egui::ScrollArea::horizontal().show(ui, |ui| {
                    self.draw_heatmap(ui);
                });

                ui.add_space(8.0);
                self.draw_legend(ui);

                // Hover info
                ui.add_space(4.0);
                if let Some(ref info) = self.hovered_info {
                    ui.label(
                        egui::RichText::new(info)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(139, 148, 158)),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(" ")
                            .size(12.0),
                    );
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                if ui.button("\u{2699} Settings").clicked() {
                    self.show_settings = true;
                }
            }
        });
    }
}
```

**Step 2: Update `src/main.rs` to pass config and store to the app**

Update the `GitMapApp::new` call in main.rs to pass `Config::default()` and `CommitStore::new()` for now:

```rust
Ok(Box::new(GitMapApp::new(tray_rx, Config::default(), CommitStore::new())))
```

Add the necessary imports at the top of main.rs:

```rust
use gitmap::config::Config;
use gitmap::store::CommitStore;
```

**Step 3: Build and run to verify heatmap renders**

Run: `cargo run`
Expected: Clicking the tray icon shows the popover with the full heatmap grid (all empty/dark cells since there's no data yet), header with year nav and dropdowns, color legend, and settings button.

**Step 4: Commit**

```bash
git add src/ui/popover.rs src/main.rs
git commit -m "feat: add heatmap grid rendering with hover, legend, and header controls"
```

---

### Task 10: Settings View

**Files:**
- Create: `src/ui/settings.rs`
- Modify: `src/ui/mod.rs` (add `pub mod settings;`)
- Modify: `src/ui/popover.rs` (use settings view)

**Step 1: Create `src/ui/settings.rs`**

```rust
use crate::config::Config;
use crate::discovery::discover_repos;
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct SettingsState {
    folder_picker_result: Arc<Mutex<Option<Vec<PathBuf>>>>,
    discover_result: Arc<Mutex<Option<Vec<PathBuf>>>>,
    hex_input: String,
}

impl SettingsState {
    pub fn new(config: &Config) -> Self {
        Self {
            folder_picker_result: Arc::new(Mutex::new(None)),
            discover_result: Arc::new(Mutex::new(None)),
            hex_input: config.accent_color.clone(),
        }
    }
}

const PRESET_COLORS: [(&str, &str); 5] = [
    ("Green", "#39d353"),
    ("Blue", "#58a6ff"),
    ("Purple", "#7c3aed"),
    ("Orange", "#f97316"),
    ("Pink", "#ec4899"),
];

pub fn draw_settings(ui: &mut egui::Ui, config: &mut Config, state: &mut SettingsState) {
    ui.heading("Settings");
    ui.add_space(12.0);

    // --- Git Identity ---
    ui.label(egui::RichText::new("Git Identity").strong().size(14.0));
    ui.add_space(4.0);

    // Try to detect identity from first tracked repo
    let identity_text = config.tracked_repos.first()
        .and_then(|p| crate::scanner::detect_identity(p).ok())
        .map(|id| format!("{} <{}>", id.name, id.email))
        .unwrap_or_else(|| "No repos tracked yet".to_string());
    ui.label(
        egui::RichText::new(identity_text)
            .size(12.0)
            .color(egui::Color32::from_rgb(139, 148, 158)),
    );

    ui.add_space(12.0);

    // --- Tracked Repos ---
    ui.label(egui::RichText::new("Tracked Repositories").strong().size(14.0));
    ui.add_space(4.0);

    // Check for folder picker results
    if let Ok(mut guard) = state.folder_picker_result.try_lock() {
        if let Some(paths) = guard.take() {
            for path in paths {
                if !config.tracked_repos.contains(&path) {
                    config.tracked_repos.push(path);
                }
            }
        }
    }

    // Check for discover results
    if let Ok(mut guard) = state.discover_result.try_lock() {
        if let Some(paths) = guard.take() {
            for path in paths {
                if !config.tracked_repos.contains(&path) {
                    config.tracked_repos.push(path);
                }
            }
        }
    }

    // List tracked repos with remove buttons
    let mut to_remove = None;
    for (i, repo) in config.tracked_repos.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(repo.display().to_string())
                    .size(11.0)
                    .color(egui::Color32::from_rgb(200, 200, 200)),
            );
            if ui.small_button("\u{2715}").clicked() {
                to_remove = Some(i);
            }
        });
    }
    if let Some(i) = to_remove {
        config.tracked_repos.remove(i);
    }

    if config.tracked_repos.is_empty() {
        ui.label(
            egui::RichText::new("No repositories tracked")
                .size(11.0)
                .color(egui::Color32::from_rgb(100, 100, 100)),
        );
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        // Add repo button
        if ui.button("Add Repository...").clicked() {
            let result = Arc::clone(&state.folder_picker_result);
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let folder = rfd::FileDialog::new()
                    .set_title("Select Git Repository")
                    .pick_folder();
                if let Ok(mut guard) = result.lock() {
                    *guard = folder.map(|p| vec![p]);
                }
                ctx.request_repaint();
            });
        }

        // Scan directory button
        if ui.button("Scan Directory...").clicked() {
            let result = Arc::clone(&state.discover_result);
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let folder = rfd::FileDialog::new()
                    .set_title("Select Parent Directory to Scan")
                    .pick_folder();
                if let Some(root) = folder {
                    let repos = discover_repos(&root);
                    if let Ok(mut guard) = result.lock() {
                        *guard = Some(repos);
                    }
                }
                ctx.request_repaint();
            });
        }
    });

    ui.add_space(12.0);

    // --- Accent Color ---
    ui.label(egui::RichText::new("Accent Color").strong().size(14.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        for (name, hex) in &PRESET_COLORS {
            let [r, g, b] = parse_hex_rgb(hex);
            let selected = config.accent_color == *hex;
            let size = if selected { 24.0 } else { 20.0 };
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(size, size),
                egui::Sense::click(),
            );
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(4),
                egui::Color32::from_rgb(r, g, b),
            );
            if selected {
                ui.painter().rect_stroke(
                    rect,
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }
            if response.clicked() {
                config.accent_color = hex.to_string();
                state.hex_input = hex.to_string();
            }
            response.on_hover_text(*name);
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Custom:");
        let response = ui.text_edit_singleline(&mut state.hex_input);
        if response.lost_focus() {
            // Validate hex input
            let trimmed = state.hex_input.trim();
            if trimmed.len() == 7 && trimmed.starts_with('#') {
                config.accent_color = trimmed.to_string();
            }
        }
    });
}

fn parse_hex_rgb(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    [r, g, b]
}
```

**Step 2: Update `src/ui/mod.rs`**

```rust
pub mod popover;
pub mod settings;
```

**Step 3: Update `src/ui/popover.rs` to integrate settings**

Add `settings_state: SettingsState` field to `GitMapApp`:

```rust
use crate::ui::settings::{self, SettingsState};
```

Add field: `settings_state: SettingsState` to the struct.
Initialize in `new()`: `settings_state: SettingsState::new(&config)`.

Replace the settings placeholder in the `update` method:

```rust
if self.show_settings {
    if ui.button("\u{2190} Back").clicked() {
        self.show_settings = false;
        let _ = self.config.save();
    }
    ui.add_space(8.0);
    settings::draw_settings(ui, &mut self.config, &mut self.settings_state);
}
```

**Step 4: Build and run to verify settings view works**

Run: `cargo run`
Expected: Clicking "Settings" shows the settings panel with tracked repos (empty), add/scan buttons, accent color presets, and custom hex input. Clicking back returns to the heatmap. Selecting different accent colors updates the heatmap.

**Step 5: Commit**

```bash
git add src/ui/settings.rs src/ui/mod.rs src/ui/popover.rs
git commit -m "feat: add settings view with folder picker, color presets, and hex input"
```

---

### Task 11: Full Integration — Wire Everything Together

**Files:**
- Modify: `src/main.rs`
- Modify: `src/ui/popover.rs`

This task connects the scanner, store, watcher, and config so the app actually shows real data.

**Step 1: Update `src/ui/popover.rs` to handle scanning and watching**

Add fields for the watcher and identity:

```rust
use crate::scanner::{self, GitIdentity};
use crate::watcher::RepoWatcher;

pub struct GitMapApp {
    tray_rx: mpsc::Receiver<TrayMessage>,
    visible: bool,
    config: Config,
    store: CommitStore,
    hovered_info: Option<String>,
    show_settings: bool,
    settings_state: SettingsState,
    identity: Option<GitIdentity>,
    watcher: Option<RepoWatcher>,
}
```

Add a method to perform the initial scan:

```rust
impl GitMapApp {
    pub fn new(tray_rx: mpsc::Receiver<TrayMessage>, config: Config, store: CommitStore) -> Self {
        // Detect identity from first tracked repo or global git config
        let identity = config.tracked_repos.first()
            .and_then(|p| scanner::detect_identity(p).ok());

        // Set up file watcher
        let mut watcher = RepoWatcher::new().ok();
        if let Some(ref mut w) = watcher {
            for repo in &config.tracked_repos {
                let _ = w.watch_repo(repo);
            }
        }

        let settings_state = SettingsState::new(&config);

        Self {
            tray_rx,
            visible: false,
            config,
            store,
            hovered_info: None,
            show_settings: false,
            settings_state,
            identity,
            watcher,
        }
    }

    pub fn initial_scan(&mut self) {
        let identity = match &self.identity {
            Some(id) => id.clone(),
            None => return,
        };
        let since = self.store.most_recent_date();
        for repo in &self.config.tracked_repos {
            if let Ok(stats) = scanner::scan_repo(repo, &identity, since) {
                self.store.merge(stats);
            }
        }
        let history_path = crate::config::data_dir().join("history.json");
        let _ = self.store.save_to(&history_path);
    }
}
```

In the `update` method, add watcher polling before the tray message loop:

```rust
// Poll file watcher for changed repos
if let (Some(ref watcher), Some(ref identity)) = (&self.watcher, &self.identity) {
    let changed = watcher.poll_changed_repos();
    for repo_path in changed {
        if let Ok(stats) = scanner::scan_repo(&repo_path, identity, self.store.most_recent_date()) {
            self.store.merge(stats);
        }
    }
    if !changed.is_empty() {
        let history_path = crate::config::data_dir().join("history.json");
        let _ = self.store.save_to(&history_path);
    }
}
```

Note: The `poll_changed_repos` call above has a borrow issue (`&self.watcher` and `&self.identity` borrow self, but `self.store.merge` needs `&mut self`). Fix by collecting into local variables first:

```rust
// Poll file watcher for changed repos
let changed_repos = self.watcher.as_ref()
    .map(|w| w.poll_changed_repos())
    .unwrap_or_default();

if !changed_repos.is_empty() {
    if let Some(ref identity) = self.identity {
        let identity = identity.clone();
        let since = self.store.most_recent_date();
        for repo_path in &changed_repos {
            if let Ok(stats) = scanner::scan_repo(repo_path, &identity, since) {
                self.store.merge(stats);
            }
        }
        let history_path = crate::config::data_dir().join("history.json");
        let _ = self.store.save_to(&history_path);
    }
}
```

**Step 2: Update `src/main.rs` to load stored history and run initial scan**

```rust
use gitmap::config::Config;
use gitmap::store::CommitStore;

fn main() -> eframe::Result<()> {
    let config = Config::load();

    // Load cached history or start fresh
    let history_path = gitmap::config::data_dir().join("history.json");
    let store = CommitStore::load_from(&history_path).unwrap_or_else(|_| CommitStore::new());

    // ... (rest of tray icon setup stays the same) ...

    eframe::run_native(
        "GitMap",
        options,
        Box::new(move |_cc| {
            // Create tray icon on the event loop thread
            let tray = tray_icon::TrayIconBuilder::new()
                .with_icon(icon)
                .with_icon_as_template(true)
                .with_tooltip("GitMap")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .unwrap();
            tray_holder_clone.borrow_mut().replace(tray);

            let mut app = GitMapApp::new(tray_rx, config, store);
            app.initial_scan();
            Ok(Box::new(app))
        }),
    )
}
```

**Step 3: Handle settings changes triggering rescan and watcher updates**

In the settings "Back" button handler, after saving config, re-detect identity, update watchers, and rescan new repos:

```rust
if self.show_settings {
    if ui.button("\u{2190} Back").clicked() {
        self.show_settings = false;
        let _ = self.config.save();

        // Re-detect identity
        self.identity = self.config.tracked_repos.first()
            .and_then(|p| scanner::detect_identity(p).ok());

        // Update watchers for any new repos
        if let Some(ref mut w) = self.watcher {
            for repo in &self.config.tracked_repos {
                let _ = w.watch_repo(repo);
            }
        }

        // Rescan
        self.initial_scan();
    }
    // ...
}
```

**Step 4: Build and test with a real repo**

Run: `cargo run`
Expected: Open the app, go to Settings, click "Add Repository..." and select a local git repo. Click Back. The heatmap should now show your commit activity for the selected year.

**Step 5: Commit**

```bash
git add src/main.rs src/ui/popover.rs
git commit -m "feat: wire scanner, store, and watcher into the app for live data"
```

---

### Task 12: Save Config on Close and Polish

**Files:**
- Modify: `src/ui/popover.rs`
- Modify: `src/main.rs`

**Step 1: Save config when the app closes**

In `src/ui/popover.rs`, implement the `on_exit` method:

```rust
impl eframe::App for GitMapApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.config.save();
        let history_path = crate::config::data_dir().join("history.json");
        let _ = self.store.save_to(&history_path);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ... existing code ...
    }
}
```

**Step 2: Auto-hide popover when it loses focus**

Add focus detection in the `update` method, after tray message polling:

```rust
// Auto-hide when the window loses focus
if self.visible {
    let has_focus = ctx.input(|i| i.focused);
    if !has_focus {
        self.visible = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
}
```

**Step 3: Build and test**

Run: `cargo run`
Expected: The popover automatically hides when you click elsewhere. Config and history are persisted between app restarts.

**Step 4: Commit**

```bash
git add src/ui/popover.rs src/main.rs
git commit -m "feat: persist state on exit and auto-hide popover on focus loss"
```

---

### Task 13: Final Integration Test

**Step 1: Run all unit tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run the app end-to-end**

Run: `cargo run`

Verify:
- [ ] Tray icon appears in menu bar
- [ ] Left-click toggles the popover
- [ ] Right-click shows context menu with Quit
- [ ] Add a repo via Settings > Add Repository
- [ ] Heatmap renders with commit data
- [ ] Year navigation works (< 2026 >)
- [ ] Data mode toggle works (Commits / Lines changed)
- [ ] Time range dropdown works
- [ ] Hover shows date and stats
- [ ] Color legend displays correctly
- [ ] Accent color presets change the heatmap colors
- [ ] Custom hex color input works
- [ ] Scan Directory discovers repos
- [ ] Popover auto-hides on focus loss
- [ ] App remembers settings and history on restart
- [ ] Quit menu item closes the app

**Step 3: Commit any fixes**

```bash
git add -A
git commit -m "fix: integration test fixes"
```
