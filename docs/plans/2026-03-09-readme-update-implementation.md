# README Update Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Update README.md with the project logo and refresh all content to match v0.5.3.

**Architecture:** Single-file edit to `README.md`. No code changes, no tests.

**Tech Stack:** Markdown, GitHub-flavored HTML for image centering.

---

### Task 1: Add centered logo above heading

**Files:**
- Modify: `README.md:1`

**Step 1: Add logo HTML before the heading**

Replace the current line 1:
```markdown
# GitMap
```

With:
```markdown
<p align="center">
  <img src="assets/gitmap-green.png" width="160" alt="GitMap logo">
</p>

# GitMap
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add logo to README"
```

---

### Task 2: Update features list

**Files:**
- Modify: `README.md:9-16` (features section)

**Step 1: Replace the features list**

Replace:
```markdown
- **Commit heatmap** — GitHub-style contribution grid showing daily activity
- **Two view modes** — Year view (full calendar year) or Rolling view (1 week to 12 months)
- **Two data modes** — Track commits or lines changed (insertions + deletions)
- **Multi-repo tracking** — Add individual repos or scan a directory to discover all git repos
- **Real-time updates** — Watches `.git` directories for changes and rescans automatically
- **Customizable colors** — 5 preset accent colors plus custom hex input
- **Auto-update** — Checks GitHub Releases for new versions, with optional silent auto-update
- **Universal binary** — Runs natively on both Apple Silicon and Intel Macs
```

With:
```markdown
- **Commit heatmap** — GitHub-style contribution grid showing daily activity
- **Two view modes** — Year view (full calendar year) or Rolling view (1 week to 12 months)
- **Two data modes** — Track commits or lines changed (insertions + deletions)
- **Multi-repo tracking** — Add individual repos or scan a directory to discover all git repos
- **Watch directories** — Auto-discover new repos in watched folders
- **Real-time updates** — Watches `.git` directories for changes and rescans automatically
- **Customizable colors** — 5 heatmap accent presets, custom hex, and 7 icon color themes
- **Auto-update** — Checks GitHub Releases for new versions, with optional silent auto-update
- **Universal binary** — Runs natively on both Apple Silicon and Intel Macs
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update features list with watch directories and icon colors"
```

---

### Task 3: Update architecture diagram

**Files:**
- Modify: `README.md:44-48` (background section of architecture diagram)

**Step 1: Add Discovery Watcher to the diagram**

Replace:
```
Background:
  ├─ Git Scanner (git2) — reads commit history, filters by user identity
  ├─ Repo Watcher (notify) — FSEvents on .git dirs for real-time updates
  ├─ Updater (ureq) — checks GitHub Releases API for new versions
  └─ Binary Watcher (notify) — detects binary updates for auto-relaunch
```

With:
```
Background:
  ├─ Git Scanner (git2) — reads commit history, filters by user identity
  ├─ Repo Watcher (notify) — FSEvents on .git dirs for real-time updates
  ├─ Discovery Watcher (notify) — detects new repos in watched directories
  ├─ Updater (ureq) — checks GitHub Releases API for new versions
  └─ Binary Watcher (notify) — detects binary updates for auto-relaunch
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add discovery watcher to architecture diagram"
```

---

### Task 4: Update modules table

**Files:**
- Modify: `README.md:53-62` (modules table)

**Step 1: Add icons and discovery_watcher rows**

Replace the modules table:
```markdown
| Module | Purpose |
|--------|---------|
| `scanner` | Scans git repos using libgit2, filters commits by user identity, collects diff stats |
| `store` | In-memory `HashMap<NaiveDate, DayStats>` with JSON persistence |
| `heatmap` | Generates the date grid and maps values to color intensity levels |
| `watcher` | Watches `.git` directories via FSEvents for real-time commit detection |
| `updater` | Checks GitHub Releases API for updates, downloads and replaces the .app bundle |
| `discovery` | Recursively finds git repos under a given directory |
| `config` | Persists settings (tracked repos, colors, view mode, auto-update) as JSON |
| `ui` | Popover window with heatmap rendering, settings panel, and update banner |
```

With:
```markdown
| Module | Purpose |
|--------|---------|
| `scanner` | Scans git repos using libgit2, filters commits by user identity, collects diff stats |
| `store` | In-memory `HashMap<NaiveDate, DayStats>` with JSON persistence |
| `heatmap` | Generates the date grid and maps values to color intensity levels |
| `watcher` | Watches `.git` directories via FSEvents for real-time commit detection |
| `discovery` | Recursively finds git repos under a given directory |
| `discovery_watcher` | Watches parent directories via FSEvents to auto-discover new repos |
| `icons` | Embeds icon PNGs, handles decode/resize for tray and Finder icons |
| `updater` | Checks GitHub Releases API for updates, downloads and replaces the .app bundle |
| `config` | Persists settings (tracked repos, colors, view mode, auto-update) as JSON |
| `ui` | Popover window with heatmap rendering, settings panel, and update banner |
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add icons and discovery_watcher to modules table"
```

---

### Task 5: Final review and squash into single commit

**Step 1: Verify the README renders correctly**

Open `README.md` and verify:
- Logo is centered above the title
- Features list includes watch directories and updated colors line
- Architecture diagram includes Discovery Watcher
- Modules table has icons and discovery_watcher rows

**Step 2: Squash tasks 1-4 into a single commit**

```bash
git reset --soft HEAD~4
git commit -m "docs: update README with logo and refresh content for v0.5.3"
```
