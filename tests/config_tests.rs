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
