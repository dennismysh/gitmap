use chrono::Datelike;
use gitmap::heatmap::{color_for_level, grid_dates, level_for_value};

#[test]
fn test_grid_dates_for_year() {
    let dates = grid_dates(2026);
    assert!(!dates.is_empty());
    assert!(dates.len() >= 52);
    for week in &dates {
        assert_eq!(week.len(), 7);
    }
    assert_eq!(dates[0][0].weekday(), chrono::Weekday::Mon);
}

#[test]
fn test_level_for_value_zero_is_level_0() {
    assert_eq!(level_for_value(0, 10), 0);
}

#[test]
fn test_level_for_value_max_is_level_4() {
    assert_eq!(level_for_value(10, 10), 4);
}

#[test]
fn test_level_for_value_distributes_evenly() {
    let level = level_for_value(5, 20);
    assert!(level >= 1 && level <= 3);
}

#[test]
fn test_color_for_level_returns_5_levels() {
    let base = "#39d353";
    for level in 0..=4 {
        let color = color_for_level(level, base);
        assert_eq!(color.len(), 4);
    }
}
