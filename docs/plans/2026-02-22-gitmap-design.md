# gitmap Design

A macOS menu bar app that displays a git commit heatmap sourced from local repositories.

## Stack

| Crate | Version | Purpose |
|-------|---------|---------|
| `eframe` | 0.32 | Window management + egui integration |
| `egui` | 0.32 | Heatmap rendering, UI widgets |
| `tray-icon` | 0.21 | macOS menu bar icon |
| `git2` | 0.20 | Read commit history from repos |
| `notify` | 8.2 | FSEvents file watching |
| `serde` + `serde_json` | latest | Config and history serialization |
| `chrono` | latest | Date handling |
| `rfd` | latest | Native folder picker dialog |

## Architecture

Three layers:

**UI Layer (eframe + egui):** Menu bar icon via `tray-icon`. Click opens a borderless egui window as a popover displaying the heatmap grid, controls, and settings.

**Data Layer:** `GitScanner` (git2) reads commits filtered by the user's git identity. `FileWatcher` (notify) watches `.git/` directories via FSEvents for real-time updates. `CommitStore` holds aggregated daily stats in memory and persists to disk.

**Config Layer:** Tracked folders, accent color, time range preferences. Persisted to `~/Library/Application Support/gitmap/config.json`.

### Data Flow

1. On launch: load cached history from `history.json`, then incrementally scan commits newer than the most recent cached date.
2. Start FSEvents watchers on each tracked repo's `.git/` directory.
3. On watcher trigger: re-scan that single repo's new commits, merge into store, write cache.
4. On tray icon click: show popover with current in-memory data (instant).

## Heatmap Visualization

- **Data modes (toggle):** commit count per day, or lines changed (insertions + deletions) per day.
- **Time range (configurable):** 30 days, 3 months, 6 months, 12 months.
- **Year navigation:** `< 2026 >` arrows to browse historical years from cache.
- **Grid:** GitHub-style — rows = all 7 days of the week (Mon-Sun), columns = weeks. Color intensity shows activity level.
- **Hover:** Shows date, commit count, and lines changed below the grid.
- **Color legend:** `Less [gradient] More` below the grid.

## Popover Layout

```
+------------------------------------------+
|  gitmap    < 2026 >  [Commits v] [12m v] |
|                                          |
|  Mon ░░▓▓░░▓█░░░░▓░░░▓▓░░█▓░░           |
|  Tue ░░░░▓░░▓░░▓░░▓░░░░▓▓░░░░           |
|  Wed ░░░▓░░▓▓░░░░░░░▓▓░░░▓▓░░           |
|  Thu ░▓░░░░▓░░░░▓░░▓░░░░▓░▓░░           |
|  Fri ░▓▓░░░░▓░░░░▓▓░░░░▓▓░▓░░           |
|  Sat ░░░░░░░░░░░░░░░░░░▓░░░░░           |
|  Sun ░░░░░░░░░░░░░░░░░░░░░░░░           |
|                                          |
|  Less ░░▓▓█ More                         |
|  Feb 22: 5 commits, +127 -43 lines       |
|  ---------------------------------------- |
|  Settings                                |
+------------------------------------------+
```

## Visual Style

- Dark background, minimal theme.
- 5 preset accent colors: Green (GitHub classic), Blue, Purple, Orange, Pink.
- Custom hex color input.

## Folder Selection

- **Manual:** Native macOS folder picker dialog (`rfd` crate) to add individual repos.
- **Auto-discover:** Pick a parent directory; app recursively finds all git repos inside.
- **Management:** List tracked repos with remove buttons in settings.

## Commit Filtering

Only the user's commits, matched against `git config user.name` and `user.email`. Displayed as read-only in settings.

## Data Persistence

- **Config:** `~/Library/Application Support/gitmap/config.json` — tracked folders, auto-discover roots, accent color, time range.
- **History:** `~/Library/Application Support/gitmap/history.json` — `HashMap<NaiveDate, DayStats>` where `DayStats = { commits, insertions, deletions }`. Enables instant access to historical years without re-scanning.

## Project Structure

```
gitmap/
├── Cargo.toml
└── src/
    ├── main.rs              # app entry, tray icon setup, event loop
    ├── config.rs            # load/save config.json, folder management
    ├── scanner.rs           # git2 commit scanning, author filtering
    ├── store.rs             # CommitStore: in-memory + history.json cache
    ├── watcher.rs           # notify FSEvents watcher management
    └── ui/
        ├── mod.rs
        ├── popover.rs       # egui heatmap window, header, legend, hover
        └── settings.rs      # egui settings view (folders, colors)
```
