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
