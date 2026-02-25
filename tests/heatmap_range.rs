use chrono::NaiveDate;
use gitmap::heatmap::grid_dates_range;

#[test]
fn test_grid_dates_range_one_week() {
    let end = NaiveDate::from_ymd_opt(2026, 2, 24).unwrap();
    let start = end - chrono::Duration::days(6);
    let weeks = grid_dates_range(start, end);

    assert!(!weeks.is_empty());
    assert!(weeks.len() <= 2);

    for week in &weeks {
        assert_eq!(week.len(), 7);
    }

    let all_dates: Vec<NaiveDate> = weeks.iter().flatten().copied().collect();
    assert!(all_dates.contains(&start));
    assert!(all_dates.contains(&end));
}

#[test]
fn test_grid_dates_range_30_days() {
    let end = NaiveDate::from_ymd_opt(2026, 2, 24).unwrap();
    let start = end - chrono::Duration::days(29);
    let weeks = grid_dates_range(start, end);

    assert!(weeks.len() >= 4);
    assert!(weeks.len() <= 6);

    for week in &weeks {
        assert_eq!(week.len(), 7);
    }
}
