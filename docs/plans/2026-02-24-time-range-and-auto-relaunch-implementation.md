# Time Range Filtering & Auto-Relaunch Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire time range dropdown to filter the heatmap as a rolling window from today, and auto-relaunch when the binary is updated on disk.

**Architecture:** Add `ViewMode` enum to toggle between year and rolling views. New `grid_dates_range` function for date-range grids. Binary self-watcher using `notify` triggers graceful relaunch. Bundle script installs to `/Applications/`.

**Tech Stack:** Rust, eframe/egui, chrono, notify, std::process::Command

---

### Task 1: Add ViewMode and TimeRange::days() to config

**Files:**
- Modify: `src/config.rs`

**Step 1: Add ViewMode enum and TimeRange::days()**

Add `ViewMode` enum after the `TimeRange` impl block, and add `days()` method to `TimeRange`:

In `src/config.rs`, add the `days` method inside `impl TimeRange` (after `all()`):

```rust
    pub fn days(&self) -> i64 {
        match self {
            TimeRange::Week1 => 7,
            TimeRange::Weeks2 => 14,
            TimeRange::Days30 => 30,
            TimeRange::Months3 => 90,
            TimeRange::Months6 => 180,
            TimeRange::Months12 => 365,
        }
    }
```

After the `TimeRange` impl block, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Year,
    Rolling,
}
```

Add `view_mode` field to `Config` struct:

```rust
pub struct Config {
    pub tracked_repos: Vec<PathBuf>,
    pub auto_discover_roots: Vec<PathBuf>,
    pub accent_color: String,
    pub time_range: TimeRange,
    pub data_mode: DataMode,
    pub selected_year: i32,
    pub view_mode: ViewMode,
}
```

Add default value in `impl Default for Config`:

```rust
            view_mode: ViewMode::Year,
```

**Step 2: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors

**Step 3: Commit**

```bash
git add src/config.rs
git commit -m "feat: add ViewMode enum and TimeRange::days()"
```

---

### Task 2: Add grid_dates_range to heatmap.rs

**Files:**
- Modify: `src/heatmap.rs`
- Test: `tests/heatmap_range.rs`

**Step 1: Write the test**

Create `tests/heatmap_range.rs`:

```rust
use chrono::NaiveDate;
use gitmap::heatmap::grid_dates_range;

#[test]
fn test_grid_dates_range_one_week() {
    // 2026-02-23 is a Monday, 2026-02-24 is a Tuesday
    let end = NaiveDate::from_ymd_opt(2026, 2, 24).unwrap();
    let start = end - chrono::Duration::days(6); // 7 days including end
    let weeks = grid_dates_range(start, end);

    // Should produce 1-2 week columns depending on alignment
    assert!(!weeks.is_empty());
    assert!(weeks.len() <= 2);

    // Each week has exactly 7 days
    for week in &weeks {
        assert_eq!(week.len(), 7);
    }

    // The range should contain our start and end dates
    let all_dates: Vec<NaiveDate> = weeks.iter().flatten().copied().collect();
    assert!(all_dates.contains(&start));
    assert!(all_dates.contains(&end));
}

#[test]
fn test_grid_dates_range_30_days() {
    let end = NaiveDate::from_ymd_opt(2026, 2, 24).unwrap();
    let start = end - chrono::Duration::days(29);
    let weeks = grid_dates_range(start, end);

    // 30 days spans about 5 weeks
    assert!(weeks.len() >= 4);
    assert!(weeks.len() <= 6);

    for week in &weeks {
        assert_eq!(week.len(), 7);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test heatmap_range 2>&1 | tail -5`
Expected: FAIL with "cannot find function `grid_dates_range`"

**Step 3: Implement grid_dates_range**

In `src/heatmap.rs`, add after the `grid_dates` function:

```rust
/// Generate the grid of dates for an arbitrary date range.
/// Returns Vec<Vec<NaiveDate>> where outer = weeks (columns), inner = days Mon-Sun (rows).
/// Pads first and last weeks to always have 7 entries.
pub fn grid_dates_range(start: NaiveDate, end: NaiveDate) -> Vec<Vec<NaiveDate>> {
    // Find the Monday of the week containing start
    let first = start - chrono::Duration::days(start.weekday().num_days_from_monday() as i64);
    // Find the Sunday of the week containing end
    let last = end + chrono::Duration::days(6 - end.weekday().num_days_from_monday() as i64);

    let mut weeks = Vec::new();
    let mut current = first;

    while current <= last {
        let mut week = Vec::with_capacity(7);
        for _ in 0..7 {
            week.push(current);
            current += chrono::Duration::days(1);
        }
        weeks.push(week);
    }

    weeks
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test heatmap_range 2>&1 | tail -5`
Expected: 2 tests PASS

**Step 5: Commit**

```bash
git add src/heatmap.rs tests/heatmap_range.rs
git commit -m "feat: add grid_dates_range for rolling time windows"
```

---

### Task 3: Wire view mode into draw_header

**Files:**
- Modify: `src/ui/popover.rs:5-6` (imports)
- Modify: `src/ui/popover.rs:88-137` (draw_header)

**Step 1: Update imports**

In `src/ui/popover.rs` line 5, add `ViewMode` to the import:

```rust
use crate::config::{Config, DataMode, TimeRange, ViewMode};
```

**Step 2: Rewrite draw_header with mutual exclusion**

Replace the entire `draw_header` method (lines 88-137) with:

```rust
    fn draw_header(&mut self, ui: &mut egui::Ui) {
        let dim = egui::Color32::from_rgb(100, 110, 120);
        let bright = egui::Color32::from_rgb(230, 237, 243);
        let year_active = self.config.view_mode == ViewMode::Year;

        ui.horizontal(|ui| {
            ui.heading("gitmap");

            ui.add_space(16.0);

            // Year navigation — dimmed when in Rolling mode
            let year_color = if year_active { bright } else { dim };
            if ui.small_button("\u{25C0}").clicked() {
                self.config.selected_year -= 1;
                self.config.view_mode = ViewMode::Year;
            }
            ui.label(
                egui::RichText::new(format!("{}", self.config.selected_year))
                    .strong()
                    .size(16.0)
                    .color(year_color),
            );
            if ui.small_button("\u{25B6}").clicked() {
                self.config.selected_year += 1;
                self.config.view_mode = ViewMode::Year;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let prev_range = self.config.time_range;
                egui::ComboBox::from_id_salt("time_range")
                    .selected_text(self.config.time_range.label())
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for &range in TimeRange::all() {
                            ui.selectable_value(
                                &mut self.config.time_range,
                                range,
                                range.label(),
                            );
                        }
                    });
                if self.config.time_range != prev_range {
                    self.config.view_mode = ViewMode::Rolling;
                }

                egui::ComboBox::from_id_salt("data_mode")
                    .selected_text(self.config.data_mode.label())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.config.data_mode,
                            DataMode::Commits,
                            DataMode::Commits.label(),
                        );
                        ui.selectable_value(
                            &mut self.config.data_mode,
                            DataMode::LinesChanged,
                            DataMode::LinesChanged.label(),
                        );
                    });
            });
        });
    }
```

**Step 3: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles (may warn about unused `year_active` — we'll use it if we add visual dimming later, but it's fine to remove if the compiler warns)

**Step 4: Commit**

```bash
git add src/ui/popover.rs
git commit -m "feat: wire view mode into header with mutual exclusion"
```

---

### Task 4: Wire view mode into draw_heatmap

**Files:**
- Modify: `src/ui/popover.rs:6` (imports)
- Modify: `src/ui/popover.rs` (draw_heatmap, line ~139)

**Step 1: Update heatmap import**

In `src/ui/popover.rs` line 6, add `grid_dates_range`:

```rust
use crate::heatmap::{color_for_level, grid_dates, grid_dates_range, level_for_value};
```

**Step 2: Change the grid selection in draw_heatmap**

Replace line 140 (`let weeks = grid_dates(self.config.selected_year);`) with:

```rust
        let weeks = match self.config.view_mode {
            ViewMode::Year => grid_dates(self.config.selected_year),
            ViewMode::Rolling => {
                let today = chrono::Local::now().naive_local().date();
                let start = today - chrono::Duration::days(self.config.time_range.days());
                grid_dates_range(start, today)
            }
        };
```

**Step 3: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors

**Step 4: Commit**

```bash
git add src/ui/popover.rs
git commit -m "feat: heatmap uses rolling window in Rolling view mode"
```

---

### Task 5: Wire view mode into draw_stats

**Files:**
- Modify: `src/ui/popover.rs` (draw_stats, line ~235)

**Step 1: Replace draw_stats to use visible date range**

Replace the entire `draw_stats` method with:

```rust
    fn draw_stats(&self, ui: &mut egui::Ui) {
        use chrono::Datelike;

        let stats = self.store.stats();
        let today = chrono::Local::now().naive_local().date();

        // Determine visible date range based on view mode
        let (range_start, range_end) = match self.config.view_mode {
            ViewMode::Year => {
                let year = self.config.selected_year;
                (
                    chrono::NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
                    chrono::NaiveDate::from_ymd_opt(year, 12, 31).unwrap(),
                )
            }
            ViewMode::Rolling => {
                let start = today - chrono::Duration::days(self.config.time_range.days());
                (start, today)
            }
        };

        let mut total_commits: u32 = 0;
        let mut total_insertions: u32 = 0;
        let mut total_deletions: u32 = 0;
        let mut active_days: u32 = 0;
        let mut current_streak: u32 = 0;
        let mut longest_streak: u32 = 0;

        // Collect stats for the visible range
        for (date, day) in stats {
            if *date >= range_start && *date <= range_end {
                total_commits += day.commits;
                total_insertions += day.insertions;
                total_deletions += day.deletions;
                if day.commits > 0 {
                    active_days += 1;
                }
            }
        }

        // Calculate current streak (consecutive days ending today or yesterday)
        let mut check_date = today;
        loop {
            if let Some(day) = stats.get(&check_date) {
                if day.commits > 0 {
                    current_streak += 1;
                    check_date -= chrono::Duration::days(1);
                    continue;
                }
            }
            if check_date == today && current_streak == 0 {
                check_date -= chrono::Duration::days(1);
                continue;
            }
            break;
        }

        // Calculate longest streak in the visible range
        let mut streak: u32 = 0;
        let mut d = range_start;
        while d <= range_end {
            if stats.get(&d).map(|s| s.commits > 0).unwrap_or(false) {
                streak += 1;
                longest_streak = longest_streak.max(streak);
            } else {
                streak = 0;
            }
            d += chrono::Duration::days(1);
        }

        let dim = egui::Color32::from_rgb(139, 148, 158);

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}", total_commits)).strong().size(13.0));
            ui.label(egui::RichText::new("commits").size(11.0).color(dim));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("+{}", total_insertions)).size(11.0).color(egui::Color32::from_rgb(63, 185, 80)));
            ui.label(egui::RichText::new(format!("-{}", total_deletions)).size(11.0).color(egui::Color32::from_rgb(248, 81, 73)));
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}", active_days)).strong().size(13.0));
            ui.label(egui::RichText::new("active days").size(11.0).color(dim));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("{}", current_streak)).strong().size(13.0));
            ui.label(egui::RichText::new("day streak").size(11.0).color(dim));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("{}", longest_streak)).strong().size(13.0));
            ui.label(egui::RichText::new("longest").size(11.0).color(dim));
        });
    }
```

**Step 2: Build and run tests**

Run: `cargo build 2>&1 | tail -5 && cargo test 2>&1 | tail -5`
Expected: compiles and all tests pass

**Step 3: Commit**

```bash
git add src/ui/popover.rs
git commit -m "feat: stats filter to visible date range"
```

---

### Task 6: Add binary self-watcher

**Files:**
- Modify: `src/ui/popover.rs` (GitMapApp struct, new, update)

**Step 1: Add binary watcher fields to GitMapApp**

Add these fields to the `GitMapApp` struct (after `file_picker_active`):

```rust
    binary_path: Option<std::path::PathBuf>,
    binary_watcher_rx: Option<mpsc::Receiver<notify::Result<notify::Event>>>,
    _binary_watcher: Option<notify::RecommendedWatcher>,
    binary_changed_at: Option<std::time::Instant>,
```

**Step 2: Initialize in `GitMapApp::new`**

After `let file_picker_active = ...` and before `Self {`, add:

```rust
        // Watch own binary for updates
        let (binary_path, binary_watcher_rx, binary_watcher) = {
            if let Ok(exe) = std::env::current_exe() {
                let exe = exe.canonicalize().unwrap_or(exe);
                let (tx, rx) = mpsc::channel();
                let mut watcher = notify::recommended_watcher(tx).ok();
                if let Some(ref mut w) = watcher {
                    // Watch the directory containing the binary (catches replace via cp)
                    if let Some(parent) = exe.parent() {
                        let _ = w.watch(parent, notify::RecursiveMode::NonRecursive);
                    }
                }
                (Some(exe), Some(rx), watcher)
            } else {
                (None, None, None)
            }
        };
```

Add to `Self { ... }`:

```rust
            binary_path,
            binary_watcher_rx,
            _binary_watcher: binary_watcher,
            binary_changed_at: None,
```

**Step 3: Add notify import**

At the top of `src/ui/popover.rs`, the `notify` crate is already available (used by `watcher.rs`). No new import needed since we use fully qualified `notify::Result`, `notify::Event`, etc.

**Step 4: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors

**Step 5: Commit**

```bash
git add src/ui/popover.rs
git commit -m "feat: add binary self-watcher for auto-relaunch"
```

---

### Task 7: Add relaunch logic to update loop

**Files:**
- Modify: `src/ui/popover.rs` (update method)

**Step 1: Add binary change detection and relaunch**

In the `update` method, after the file watcher polling block (after the `if !changed_repos.is_empty() { ... }` block, around line 378) and before the focus tracking block, add:

```rust
        // Check for binary update (auto-relaunch)
        if let Some(ref rx) = self.binary_watcher_rx {
            while let Ok(Ok(event)) = rx.try_recv() {
                if let Some(ref exe) = self.binary_path {
                    if event.paths.iter().any(|p| p == exe) {
                        if self.binary_changed_at.is_none() {
                            self.binary_changed_at = Some(std::time::Instant::now());
                        }
                    }
                }
            }
        }

        // Debounced relaunch after binary change (500ms for copy to finish)
        if let Some(changed_at) = self.binary_changed_at {
            if changed_at.elapsed() >= std::time::Duration::from_millis(500) {
                // Save state
                let _ = self.config.save();
                let history_path = crate::config::data_dir().join("history.json");
                let _ = self.store.save_to(&history_path);

                // Spawn new binary
                if let Some(ref exe) = self.binary_path {
                    let _ = std::process::Command::new(exe).spawn();
                }

                // Exit current process
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
```

**Step 2: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors

**Step 3: Commit**

```bash
git add src/ui/popover.rs
git commit -m "feat: auto-relaunch on binary update with 500ms debounce"
```

---

### Task 8: Update bundle script with install step

**Files:**
- Modify: `scripts/bundle.sh`

**Step 1: Add install to /Applications**

At the end of `scripts/bundle.sh`, before the final `echo`, add:

```bash
# Install to /Applications
cp -R "$APP_DIR" /Applications/GitMap.app

echo "Installed: /Applications/GitMap.app"
```

Replace the existing final `echo "Built: $APP_DIR"` with:

```bash
echo "Built: $APP_DIR"

# Install to /Applications
cp -R "$APP_DIR" /Applications/GitMap.app
echo "Installed: /Applications/GitMap.app"
```

**Step 2: Verify script syntax**

Run: `bash -n scripts/bundle.sh && echo "OK"`
Expected: OK

**Step 3: Commit**

```bash
git add scripts/bundle.sh
git commit -m "build: install .app bundle to /Applications"
```
