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
    // Commit at 2026-02-16 01:30:00 +0530 = 2026-02-15 20:00:00 UTC
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
