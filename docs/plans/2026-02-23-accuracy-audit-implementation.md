# Accuracy Audit & Regression Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Audit gitmap's scanner accuracy against git CLI ground truth, fix confirmed bugs, and add regression tests.

**Architecture:** An integration test shells out to `git log` for ground truth, compares against `scan_repo()` output per-day. Bug fixes target `scanner.rs` (timezone, per-repo identity) and `popover.rs` (incremental double-counting). Regression tests use temp repos with controlled commit histories.

**Tech Stack:** Rust, git2, chrono, tempfile, std::process::Command (for git CLI ground truth)

---

### Task 1: Audit Script — Git CLI Ground Truth Parser

**Files:**
- Create: `tests/accuracy_audit.rs`

**Step 1: Write the audit test file with ground truth parsing**

```rust
use chrono::NaiveDate;
use gitmap::config::Config;
use gitmap::scanner::{self, DayStats, GitIdentity};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Parse `git log` output into per-day stats matching scan_repo's structure.
/// Uses: git log --author="name" --author="email" --format="COMMIT %H %ad" --date=short --numstat
fn git_cli_stats(repo_path: &Path, identity: &GitIdentity) -> HashMap<NaiveDate, DayStats> {
    let output = Command::new("git")
        .args([
            "log",
            "--all",
            &format!("--author={}", identity.name),
            &format!("--author={}", identity.email),
            "--format=COMMIT %H %ad",
            "--date=short",
            "--numstat",
        ])
        .current_dir(repo_path)
        .output()
        .expect("git log failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut stats: HashMap<NaiveDate, DayStats> = HashMap::new();
    let mut current_date: Option<NaiveDate> = None;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("COMMIT ") {
            // Format: "COMMIT <hash> <YYYY-MM-DD>"
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(date) = NaiveDate::parse_from_str(parts[1], "%Y-%m-%d") {
                    current_date = Some(date);
                    let entry = stats.entry(date).or_insert(DayStats {
                        commits: 0,
                        insertions: 0,
                        deletions: 0,
                    });
                    entry.commits += 1;
                }
            }
        } else if let Some(date) = current_date {
            // numstat line: "<insertions>\t<deletions>\t<filename>"
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let ins: u32 = parts[0].parse().unwrap_or(0); // "-" for binary
                let del: u32 = parts[1].parse().unwrap_or(0);
                if let Some(entry) = stats.get_mut(&date) {
                    entry.insertions += ins;
                    entry.deletions += del;
                }
            }
        }
    }

    stats
}
```

**Step 2: Run to verify it compiles**

Run: `cargo test --test accuracy_audit --no-run`
Expected: Compiles (no test functions yet, that's fine)

**Step 3: Commit**

```bash
git add tests/accuracy_audit.rs
git commit -m "test: add git CLI ground truth parser for accuracy audit"
```

---

### Task 2: Audit Script — Comparison Logic and Report

**Files:**
- Modify: `tests/accuracy_audit.rs`

**Step 1: Add the comparison test that runs against all tracked repos**

Append to `tests/accuracy_audit.rs`:

```rust
#[test]
fn audit_scanner_accuracy() {
    let config = Config::load();

    if config.tracked_repos.is_empty() {
        eprintln!("No tracked repos configured — skipping audit");
        return;
    }

    let mut total_discrepancies = 0;

    for repo_path in &config.tracked_repos {
        let repo_name = repo_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| repo_path.display().to_string());

        let identity = match scanner::detect_identity(repo_path) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("  SKIP {}: failed to detect identity: {}", repo_name, e);
                continue;
            }
        };

        let gitmap_stats = match scanner::scan_repo(repo_path, &identity, None) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  SKIP {}: scan_repo failed: {}", repo_name, e);
                continue;
            }
        };

        let cli_stats = git_cli_stats(repo_path, &identity);

        // Collect all dates from both sources
        let mut all_dates: Vec<NaiveDate> = gitmap_stats
            .keys()
            .chain(cli_stats.keys())
            .copied()
            .collect();
        all_dates.sort();
        all_dates.dedup();

        let mut repo_discrepancies = 0;

        eprintln!("\n=== {} ({}) ===", repo_name, repo_path.display());
        eprintln!("  Identity: {} <{}>", identity.name, identity.email);
        eprintln!(
            "  gitmap total days: {}, git CLI total days: {}",
            gitmap_stats.len(),
            cli_stats.len()
        );

        for date in &all_dates {
            let gm = gitmap_stats.get(date);
            let cli = cli_stats.get(date);

            let (gm_commits, gm_ins, gm_del) = gm
                .map(|s| (s.commits, s.insertions, s.deletions))
                .unwrap_or((0, 0, 0));
            let (cli_commits, cli_ins, cli_del) = cli
                .map(|s| (s.commits, s.insertions, s.deletions))
                .unwrap_or((0, 0, 0));

            if gm_commits != cli_commits || gm_ins != cli_ins || gm_del != cli_del {
                repo_discrepancies += 1;
                eprintln!(
                    "  MISMATCH {}: gitmap({} commits, +{} -{}) vs cli({} commits, +{} -{})",
                    date, gm_commits, gm_ins, gm_del, cli_commits, cli_ins, cli_del
                );
            }
        }

        if repo_discrepancies == 0 {
            eprintln!("  ALL MATCH ({} days compared)", all_dates.len());
        } else {
            eprintln!(
                "  {} discrepancies out of {} days",
                repo_discrepancies,
                all_dates.len()
            );
        }
        total_discrepancies += repo_discrepancies;
    }

    eprintln!("\n=== SUMMARY: {} total discrepancies ===", total_discrepancies);
    // Don't assert — this is a diagnostic tool. Discrepancies are expected
    // before bug fixes and help us understand what to fix.
}
```

**Step 2: Run the audit**

Run: `cargo test --test accuracy_audit -- --nocapture 2>&1`
Expected: Output showing per-repo comparison results. Note any discrepancies for Phase 2.

**Step 3: Commit**

```bash
git add tests/accuracy_audit.rs
git commit -m "test: add accuracy audit comparing scan_repo vs git CLI"
```

---

### Task 3: Fix Timezone — Use Commit Author Timezone

**Files:**
- Modify: `src/scanner.rs:59-62`

**Step 1: Write a failing test**

Add to `tests/scanner_accuracy.rs` (create this file):

```rust
use chrono::NaiveDate;
use gitmap::scanner::{scan_repo, GitIdentity};
use std::process::Command;
use tempfile::TempDir;

/// Create a repo with a commit at a specific UTC timestamp.
/// GIT_AUTHOR_DATE and GIT_COMMITTER_DATE control the commit time.
fn create_repo_with_dated_commit(date_str: &str, tz_offset: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test User"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(path).output().unwrap();

    std::fs::write(path.join("file.txt"), "content\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();

    // Commit with explicit date and timezone
    let date_with_tz = format!("{} {}", date_str, tz_offset);
    Command::new("git")
        .args(["commit", "-m", "dated commit"])
        .env("GIT_AUTHOR_DATE", &date_with_tz)
        .env("GIT_COMMITTER_DATE", &date_with_tz)
        .current_dir(path)
        .output()
        .unwrap();

    dir
}

#[test]
fn test_commit_date_uses_author_timezone() {
    // Commit at 2026-02-15 23:30:00 +0530 (IST) = 2026-02-15 18:00:00 UTC
    // In IST it's Feb 15. In UTC it's Feb 15. In PST it's Feb 15.
    // But a commit at 2026-02-16 01:30:00 +0530 = 2026-02-15 20:00:00 UTC
    // In IST it's Feb 16. In UTC it's still Feb 15.
    // gitmap should use the author's timezone → Feb 16.
    let repo = create_repo_with_dated_commit("2026-02-16 01:30:00", "+0530");
    let identity = GitIdentity {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };

    let stats = scan_repo(repo.path(), &identity, None).unwrap();
    let feb16 = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();

    assert!(
        stats.contains_key(&feb16),
        "Commit at 2026-02-16 01:30 +0530 should appear on Feb 16 (author tz), got dates: {:?}",
        stats.keys().collect::<Vec<_>>()
    );
}
```

**Step 2: Run to verify it fails**

Run: `cargo test --test scanner_accuracy test_commit_date_uses_author_timezone -- --nocapture`
Expected: FAIL — commit likely lands on Feb 15 due to `naive_local()` using machine timezone (likely PST/UTC-8)

**Step 3: Fix scanner.rs to use commit author timezone**

In `src/scanner.rs`, replace lines 59-62:

```rust
        // Old:
        // let timestamp = commit.time().seconds();
        // let date = chrono::DateTime::from_timestamp(timestamp, 0)
        //     .map(|dt| dt.naive_local().date())
        //     .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

        // New: use the commit's own timezone offset
        let git_time = commit.time();
        let timestamp = git_time.seconds();
        let offset_minutes = git_time.offset_minutes();
        let offset = chrono::FixedOffset::east_opt(offset_minutes * 60)
            .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).unwrap());
        let date = chrono::DateTime::from_timestamp(timestamp, 0)
            .map(|dt| dt.with_timezone(&offset).date_naive())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test scanner_accuracy test_commit_date_uses_author_timezone -- --nocapture`
Expected: PASS

**Step 5: Run all existing tests to check for regressions**

Run: `cargo test`
Expected: All tests pass

**Step 6: Commit**

```bash
git add src/scanner.rs tests/scanner_accuracy.rs
git commit -m "fix: use commit author timezone for date assignment"
```

---

### Task 4: Fix Watcher Incremental Double-Counting

**Files:**
- Modify: `src/ui/popover.rs:352-357`

**Step 1: Write a failing test for incremental scan boundary**

Add to `tests/scanner_accuracy.rs`:

```rust
#[test]
fn test_incremental_scan_excludes_since_date() {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test User"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(path).output().unwrap();

    // Commit on Feb 10
    std::fs::write(path.join("a.txt"), "aaa\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "first"])
        .env("GIT_AUTHOR_DATE", "2026-02-10 12:00:00 +0000")
        .env("GIT_COMMITTER_DATE", "2026-02-10 12:00:00 +0000")
        .current_dir(path)
        .output()
        .unwrap();

    // Commit on Feb 15
    std::fs::write(path.join("b.txt"), "bbb\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "second"])
        .env("GIT_AUTHOR_DATE", "2026-02-15 12:00:00 +0000")
        .env("GIT_COMMITTER_DATE", "2026-02-15 12:00:00 +0000")
        .current_dir(path)
        .output()
        .unwrap();

    let identity = GitIdentity {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };

    // Incremental scan since Feb 15 should NOT include the Feb 15 commit
    // (it was already counted in a previous full scan)
    let since = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
    let stats = scan_repo(path, &identity, Some(since)).unwrap();

    let feb15 = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
    assert!(
        !stats.contains_key(&feb15),
        "Incremental scan with since=Feb15 should exclude Feb 15 commits to prevent double-counting"
    );
}
```

**Step 2: Run to verify it fails**

Run: `cargo test --test scanner_accuracy test_incremental_scan_excludes_since_date -- --nocapture`
Expected: FAIL — `scan_repo` currently includes commits ON the since date (`date < since_date` means Feb 15 is included)

**Step 3: Fix scanner.rs — change `<` to `<=` for since filter**

In `src/scanner.rs`, replace lines 64-68:

```rust
        if let Some(since_date) = since {
            if date <= since_date {
                break;
            }
        }
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test scanner_accuracy test_incremental_scan_excludes_since_date -- --nocapture`
Expected: PASS

**Step 5: Run all tests**

Run: `cargo test`
Expected: All pass

**Step 6: Commit**

```bash
git add src/scanner.rs tests/scanner_accuracy.rs
git commit -m "fix: exclude since-date from incremental scan to prevent double-counting"
```

---

### Task 5: Fix Per-Repo Identity Detection

**Files:**
- Modify: `src/ui/popover.rs:59-73` (initial_scan method)
- Modify: `src/ui/popover.rs:349-361` (watcher update loop)

**Step 1: Write a failing test for per-repo identity**

Add to `tests/scanner_accuracy.rs`:

```rust
#[test]
fn test_scan_filters_by_matching_identity_name_or_email() {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Work Name"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "work@company.com"]).current_dir(path).output().unwrap();

    std::fs::write(path.join("file.txt"), "content\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git").args(["commit", "-m", "work commit"]).current_dir(path).output().unwrap();

    // Using personal identity should NOT match (different name AND email)
    let personal = GitIdentity {
        name: "Personal Name".to_string(),
        email: "personal@email.com".to_string(),
    };
    let stats = scan_repo(path, &personal, None).unwrap();
    assert!(stats.is_empty(), "Personal identity should not match work commits");

    // Using identity with matching email should match
    let email_match = GitIdentity {
        name: "Different Name".to_string(),
        email: "work@company.com".to_string(),
    };
    let stats = scan_repo(path, &email_match, None).unwrap();
    assert!(!stats.is_empty(), "Matching email should find commits even with different name");
}
```

**Step 2: Run to verify it passes (scanner logic is correct)**

Run: `cargo test --test scanner_accuracy test_scan_filters_by_matching_identity_name_or_email -- --nocapture`
Expected: PASS — the scanner's OR logic is correct. The bug is in `popover.rs` which uses one identity for all repos.

**Step 3: Fix popover.rs — detect identity per-repo in initial_scan**

In `src/ui/popover.rs`, replace the `initial_scan` method (lines 59-73):

```rust
    pub fn initial_scan(&mut self) {
        // Full rescan: start fresh to avoid double-counting
        self.store = CommitStore::new();
        for repo in &self.config.tracked_repos {
            // Detect identity per-repo to handle different user configs
            let identity = match scanner::detect_identity(repo) {
                Ok(id) => id,
                Err(_) => continue,
            };
            if let Ok(stats) = scanner::scan_repo(repo, &identity, None) {
                self.store.merge(stats);
            }
        }
        // Update the display identity from first repo (for settings UI)
        self.identity = self
            .config
            .tracked_repos
            .first()
            .and_then(|p| scanner::detect_identity(p).ok());
        let history_path = crate::config::data_dir().join("history.json");
        let _ = self.store.save_to(&history_path);
    }
```

**Step 4: Fix popover.rs — detect identity per-repo in watcher loop**

In `src/ui/popover.rs`, replace lines 349-361 (the watcher polling block inside `update`):

```rust
        if !changed_repos.is_empty() {
            let since = self.store.most_recent_date();
            for repo_path in &changed_repos {
                // Detect identity per-repo
                let identity = match scanner::detect_identity(repo_path) {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                if let Ok(stats) = scanner::scan_repo(repo_path, &identity, since) {
                    self.store.merge(stats);
                }
            }
            let history_path = crate::config::data_dir().join("history.json");
            let _ = self.store.save_to(&history_path);
        }
```

**Step 5: Run all tests**

Run: `cargo test`
Expected: All pass

**Step 6: Commit**

```bash
git add src/ui/popover.rs tests/scanner_accuracy.rs
git commit -m "fix: detect git identity per-repo instead of using first repo only"
```

---

### Task 6: Regression Tests — Line Stats and Merge Commits

**Files:**
- Modify: `tests/scanner_accuracy.rs`

**Step 1: Add test for exact insertions/deletions counts**

```rust
#[test]
fn test_exact_insertion_deletion_counts() {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test User"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(path).output().unwrap();

    // Initial commit: 3 lines
    std::fs::write(path.join("file.txt"), "line1\nline2\nline3\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .env("GIT_AUTHOR_DATE", "2026-03-01 12:00:00 +0000")
        .env("GIT_COMMITTER_DATE", "2026-03-01 12:00:00 +0000")
        .current_dir(path)
        .output()
        .unwrap();

    // Second commit: change line2 to line2-modified, add line4
    // Expected: +2 insertions (line2-modified, line4), -1 deletion (line2)
    std::fs::write(path.join("file.txt"), "line1\nline2-modified\nline3\nline4\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "modify"])
        .env("GIT_AUTHOR_DATE", "2026-03-01 14:00:00 +0000")
        .env("GIT_COMMITTER_DATE", "2026-03-01 14:00:00 +0000")
        .current_dir(path)
        .output()
        .unwrap();

    let identity = GitIdentity {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };

    let stats = scan_repo(path, &identity, None).unwrap();
    let mar1 = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let day = stats.get(&mar1).expect("Should have stats for Mar 1");

    assert_eq!(day.commits, 2, "Two commits on Mar 1");
    // Initial commit: +3 insertions (3 new lines), 0 deletions
    // Second commit: +2 insertions, -1 deletion
    assert_eq!(day.insertions, 5, "3 initial + 2 modified = 5 insertions");
    assert_eq!(day.deletions, 1, "1 line replaced = 1 deletion");
}
```

**Step 2: Add test for initial commit (no parent) line counting**

```rust
#[test]
fn test_initial_commit_counts_all_lines_as_insertions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test User"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(path).output().unwrap();

    std::fs::write(path.join("file.txt"), "a\nb\nc\nd\ne\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .env("GIT_AUTHOR_DATE", "2026-04-01 12:00:00 +0000")
        .env("GIT_COMMITTER_DATE", "2026-04-01 12:00:00 +0000")
        .current_dir(path)
        .output()
        .unwrap();

    let identity = GitIdentity {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };

    let stats = scan_repo(path, &identity, None).unwrap();
    let apr1 = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
    let day = stats.get(&apr1).expect("Should have stats for Apr 1");

    assert_eq!(day.commits, 1);
    assert_eq!(day.insertions, 5, "5 lines in initial commit");
    assert_eq!(day.deletions, 0, "No deletions in initial commit");
}
```

**Step 3: Add test for merge commit first-parent diff**

```rust
#[test]
fn test_merge_commit_uses_first_parent_diff() {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test User"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(path).output().unwrap();

    // Initial commit on main
    std::fs::write(path.join("main.txt"), "main\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .env("GIT_AUTHOR_DATE", "2026-05-01 10:00:00 +0000")
        .env("GIT_COMMITTER_DATE", "2026-05-01 10:00:00 +0000")
        .current_dir(path)
        .output()
        .unwrap();

    // Create branch and add file
    Command::new("git").args(["checkout", "-b", "feature"]).current_dir(path).output().unwrap();
    std::fs::write(path.join("feature.txt"), "feature\nline2\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "feature commit"])
        .env("GIT_AUTHOR_DATE", "2026-05-01 11:00:00 +0000")
        .env("GIT_COMMITTER_DATE", "2026-05-01 11:00:00 +0000")
        .current_dir(path)
        .output()
        .unwrap();

    // Back to main, merge
    Command::new("git").args(["checkout", "main"]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["merge", "feature", "--no-ff", "-m", "merge feature"])
        .env("GIT_AUTHOR_DATE", "2026-05-01 12:00:00 +0000")
        .env("GIT_COMMITTER_DATE", "2026-05-01 12:00:00 +0000")
        .current_dir(path)
        .output()
        .unwrap();

    let identity = GitIdentity {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };

    let stats = scan_repo(path, &identity, None).unwrap();
    let may1 = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
    let day = stats.get(&may1).expect("Should have stats for May 1");

    // 3 commits: initial, feature, merge
    assert_eq!(day.commits, 3, "Should count initial + feature + merge commits");
    // Merge commit diff vs first parent (main) = +2 lines (feature.txt)
    // Feature commit = +2 lines (feature.txt)
    // Initial commit = +1 line (main.txt)
    // Total = 5 insertions
    assert_eq!(day.insertions, 5, "initial(1) + feature(2) + merge-vs-first-parent(2)");
}
```

**Step 4: Run all new tests**

Run: `cargo test --test scanner_accuracy -- --nocapture`
Expected: All pass

**Step 5: Commit**

```bash
git add tests/scanner_accuracy.rs
git commit -m "test: add regression tests for line stats, initial commit, and merge commits"
```

---

### Task 7: Re-run Audit and Verify Fixes

**Files:**
- No new files

**Step 1: Re-run the audit against tracked repos**

Run: `cargo test --test accuracy_audit -- --nocapture 2>&1`
Expected: Fewer discrepancies than the initial run (ideally zero for commit counts; line stats may still differ for `--all` branches vs HEAD-only walk)

**Step 2: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit any remaining adjustments**

If the audit reveals additional issues not covered above, fix and test them before this commit.
