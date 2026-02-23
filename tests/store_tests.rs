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
