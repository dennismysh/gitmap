use std::path::PathBuf;

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

#[test]
fn test_config_untracked_repos_default_empty() {
    let config = gitmap::config::Config::default();
    assert!(config.untracked_repos.is_empty());
}

#[test]
fn test_config_migration_without_untracked_repos() {
    // Simulates loading an old config that doesn't have untracked_repos
    let json = r##"{"tracked_repos":[],"auto_discover_roots":[],"accent_color":"#39d353","time_range":"Months12","data_mode":"Commits","selected_year":2026,"view_mode":"Year","auto_update":false}"##;
    let config: gitmap::config::Config = serde_json::from_str(json).unwrap();
    assert!(config.untracked_repos.is_empty());
}

#[test]
fn test_config_roundtrip_with_untracked_repos() {
    let mut config = gitmap::config::Config::default();
    config.untracked_repos.push(PathBuf::from("/Users/test/ignored"));
    let json = serde_json::to_string(&config).unwrap();
    let loaded: gitmap::config::Config = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.untracked_repos.len(), 1);
    assert_eq!(loaded.untracked_repos[0], PathBuf::from("/Users/test/ignored"));
}
