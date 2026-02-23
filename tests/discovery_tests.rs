use gitmap::discovery::discover_repos;
use std::process::Command;

#[test]
fn test_discover_finds_git_repos() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    let repo1 = root.join("project-a");
    let repo2 = root.join("subdir").join("project-b");
    std::fs::create_dir_all(&repo1).unwrap();
    std::fs::create_dir_all(&repo2).unwrap();

    Command::new("git").args(["init"]).current_dir(&repo1).output().unwrap();
    Command::new("git").args(["init"]).current_dir(&repo2).output().unwrap();

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
