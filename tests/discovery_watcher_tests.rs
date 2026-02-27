use gitmap::discovery_watcher::DiscoveryWatcher;
use std::process::Command;

#[test]
fn test_discovery_watcher_detects_new_repo() {
    let dir = tempfile::TempDir::new().unwrap();
    // Canonicalize to match FSEvents paths (e.g., /var -> /private/var on macOS)
    let root = dir.path().canonicalize().unwrap();

    let mut watcher = DiscoveryWatcher::new().unwrap();
    watcher.watch_root(&root).unwrap();

    // Give FSEvents time to register
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Create a new repo in the watched directory
    let repo_path = root.join("new-project");
    std::fs::create_dir_all(&repo_path).unwrap();
    Command::new("git").args(["init"]).current_dir(&repo_path).output().unwrap();

    // First poll: drains FSEvents into pending map with timestamp
    std::thread::sleep(std::time::Duration::from_millis(500));
    let early = watcher.poll_new_repos();
    assert!(early.is_empty(), "Should not be ready before debounce");

    // Wait for debounce (3 seconds) + buffer
    std::thread::sleep(std::time::Duration::from_secs(4));

    // Second poll: debounce elapsed, should return the repo
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
    let root = dir.path().canonicalize().unwrap();

    let mut watcher = DiscoveryWatcher::new().unwrap();
    watcher.watch_root(&root).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));

    // Create a directory that is NOT a git repo
    let not_repo = root.join("just-a-folder");
    std::fs::create_dir_all(&not_repo).unwrap();

    // First poll to drain events
    std::thread::sleep(std::time::Duration::from_millis(500));
    watcher.poll_new_repos();

    // Wait for debounce
    std::thread::sleep(std::time::Duration::from_secs(4));

    let discovered = watcher.poll_new_repos();
    assert!(
        discovered.is_empty(),
        "Expected no repos but got: {:?}",
        discovered
    );
}

#[test]
fn test_discovery_watcher_unwatch_root() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();

    let mut watcher = DiscoveryWatcher::new().unwrap();
    watcher.watch_root(&root).unwrap();
    // Should not panic on unwatch
    watcher.unwatch_root(&root).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));

    // Create a repo after unwatching — should NOT be detected
    let repo_path = root.join("post-unwatch");
    std::fs::create_dir_all(&repo_path).unwrap();
    Command::new("git").args(["init"]).current_dir(&repo_path).output().unwrap();

    // Drain + debounce
    std::thread::sleep(std::time::Duration::from_millis(500));
    watcher.poll_new_repos();
    std::thread::sleep(std::time::Duration::from_secs(4));

    let discovered = watcher.poll_new_repos();
    assert!(discovered.is_empty());
}
