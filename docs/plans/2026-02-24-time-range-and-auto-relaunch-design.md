# Time Range Filtering & Auto-Relaunch Design

## Feature 1: Time Range Filtering

### Problem

The time range dropdown (1 week, 2 weeks, 30 days, etc.) exists but doesn't filter the heatmap. The grid always shows a full calendar year.

### Design

Two view modes, mutually exclusive, both controls always visible:

- **Year mode** (default): Full calendar year grid. Year arrows active. Time range dropdown appears dimmed.
- **Rolling mode**: Rolling window ending today. Time range dropdown active. Year arrows appear dimmed.

Clicking year arrows switches to Year mode. Selecting a time range switches to Rolling mode. Last selection of each persists when switching back.

### Implementation

**`ViewMode` enum** in `config.rs`:
- `Year` — use `selected_year` and `grid_dates(year)`
- `Rolling` — use `time_range` and new `grid_dates_range(start, end)`

**`TimeRange::days()` method**: Week1 → 7, Weeks2 → 14, Days30 → 30, Months3 → 90, Months6 → 180, Months12 → 365.

**`grid_dates_range(start, end)` in `heatmap.rs`**: New function that generates the week grid for an arbitrary date range, same format as `grid_dates`. Existing `grid_dates` stays unchanged.

**`draw_heatmap`**: Branch on `view_mode` to pick which grid function to call.

**`draw_stats`**: Filter stats to the visible date range (year bounds or rolling window bounds) instead of always using `selected_year`.

**`draw_header`**: Both controls always rendered. Non-active control dimmed via reduced alpha text color. Clicking either switches the mode.

### Files

- `src/config.rs` — add `ViewMode` enum, `TimeRange::days()`, `view_mode` field on `Config`
- `src/heatmap.rs` — add `grid_dates_range(start, end)`
- `src/ui/popover.rs` — update `draw_header`, `draw_heatmap`, `draw_stats`

---

## Feature 2: Auto-Relaunch on Update

### Problem

When the `.app` bundle is replaced with a new build, the old process keeps running. User must manually quit and relaunch.

### Design

The app watches its own binary for changes. When the binary is replaced on disk, it saves state, spawns the new binary, and exits.

### Implementation

**At startup**: Resolve `std::env::current_exe()` and store the path. Set up a `notify` file watcher on the binary path.

**On binary change**: Wait 500ms for the copy to finish. Save config and history. Spawn the new binary via `std::process::Command`. Close the current process via `ViewportCommand::Close`.

**Bundle script**: Add `cp -R` to install the `.app` to `/Applications/GitMap.app` after building.

### Files

- `src/ui/popover.rs` — add binary path field, binary watcher, relaunch logic in `update`
- `scripts/bundle.sh` — add install step
