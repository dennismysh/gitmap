use gitmap::scanner::{DayStats, GitIdentity, scan_repo};
use std::process::Command;
use tempfile::TempDir;

fn create_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test User"]).current_dir(path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(path).output().unwrap();

    std::fs::write(path.join("file.txt"), "hello world\n").unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git").args(["commit", "-m", "initial commit"]).current_dir(path).output().unwrap();

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
