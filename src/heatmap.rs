use chrono::{Datelike, NaiveDate};

/// Generate the grid of dates for a given year.
/// Returns Vec<Vec<NaiveDate>> where outer = weeks (columns), inner = days Mon-Sun (rows).
/// Pads the first and last weeks to always have 7 entries.
pub fn grid_dates(year: i32) -> Vec<Vec<NaiveDate>> {
    let jan1 = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let dec31 = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    // Find the Monday of the week containing Jan 1
    let start = jan1 - chrono::Duration::days(jan1.weekday().num_days_from_monday() as i64);
    // Find the Sunday of the week containing Dec 31
    let end = dec31 + chrono::Duration::days(6 - dec31.weekday().num_days_from_monday() as i64);

    let mut weeks = Vec::new();
    let mut current = start;

    while current <= end {
        let mut week = Vec::with_capacity(7);
        for _ in 0..7 {
            week.push(current);
            current += chrono::Duration::days(1);
        }
        weeks.push(week);
    }

    weeks
}

/// Map a value to a level 0-4 based on the maximum value in the dataset.
pub fn level_for_value(value: u32, max_value: u32) -> u8 {
    if value == 0 {
        return 0;
    }
    if max_value == 0 {
        return 0;
    }
    let ratio = value as f32 / max_value as f32;
    match ratio {
        r if r <= 0.25 => 1,
        r if r <= 0.50 => 2,
        r if r <= 0.75 => 3,
        _ => 4,
    }
}

/// Parse a hex color string like "#39d353" into [r, g, b].
fn parse_hex(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    [r, g, b]
}

/// Return an RGBA color [r, g, b, a] for a given level (0-4) and base accent color.
/// Level 0 = dark background, levels 1-4 = increasing intensity of the accent color.
pub fn color_for_level(level: u8, accent_hex: &str) -> [u8; 4] {
    let [r, g, b] = parse_hex(accent_hex);

    match level {
        0 => [22, 27, 34, 255],
        1 => [r / 4, g / 4, b / 4, 255],
        2 => [r / 2, g / 2, b / 2, 255],
        3 => [(r as u16 * 3 / 4) as u8, (g as u16 * 3 / 4) as u8, (b as u16 * 3 / 4) as u8, 255],
        _ => [r, g, b, 255],
    }
}
