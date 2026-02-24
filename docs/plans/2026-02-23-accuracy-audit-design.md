# Accuracy Audit & Regression Tests

## Goal

Verify that gitmap's commit counts, line-change stats, and author filtering match git CLI ground truth across all tracked repos. Fix confirmed bugs, then add regression tests.

## Phase 1: Audit Script

Integration test `tests/accuracy_audit.rs` that:

1. Loads gitmap config to find tracked repos
2. Detects identity per-repo (not just the first repo)
3. For each repo, runs `scan_repo()` to get gitmap's view
4. For each repo, shells out to `git log --author=... --format="%H %ad" --date=short --numstat` to get ground truth
5. Compares per-day: commit count, insertions, deletions
6. Prints summary table with discrepancies

Ground truth uses `--author` flag twice (name and email) to match gitmap's OR logic. Uses `--date=short` for day-level grouping.

Key comparison: git CLI uses the commit's author timezone for date assignment, while gitmap currently uses `naive_local()` (machine timezone). The audit will surface any date-boundary mismatches from this.

## Phase 2: Bug Fixes

Known issues to investigate and fix based on audit results:

### Timezone mismatch (scanner.rs:62)
`naive_local()` converts to machine timezone, not commit author timezone. A commit at 11pm PST authored in EST could land on the wrong day. Fix: use commit's timezone offset via `git2::Time::offset_minutes()`.

### Watcher incremental double-counting (popover.rs:352-357)
The watcher uses `since = most_recent_date()` but `scan_repo` includes commits ON that date (`date < since_date` → break). Merged via additive `merge()`, doubling the boundary. Fix: use `since + 1 day` as the cutoff.

### Per-repo identity (popover.rs:33-35)
Identity detected from first tracked repo only. Different `user.name`/`user.email` across repos causes missed commits. Fix: detect identity per-repo in scan loop.

### Merge commit line stats (scanner.rs:79)
Diffs against `parent(0)` only. This is standard first-parent behavior but may diverge from user expectations. Document behavior; compare against `git log --first-parent --numstat`.

## Phase 3: Regression Tests

Unit tests in `tests/scanner_accuracy.rs` using temp repos created with `git2`:

- **Correct commit count per day**: N commits on specific dates, verify counts
- **Correct insertions/deletions**: Known file changes, verify line stats
- **Author filtering**: Multiple authors, verify only matching ones counted
- **Merge commit handling**: Create merge, verify line stats match first-parent diff
- **Initial commit (no parent)**: Verify root commit diff is counted
- **Incremental scan boundary**: Scan with `since`, verify no double-counting

Each test creates a temp repo, makes commits with controlled timestamps/authors/content, runs `scan_repo`, and asserts exact values.
