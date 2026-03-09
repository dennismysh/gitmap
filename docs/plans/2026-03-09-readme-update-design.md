# README Update Design

## Goal

Update the GitHub README to include the project logo and refresh all content to reflect the current v0.5.3 state.

## Changes

### 1. Add Logo

- Centered green logo (`assets/gitmap-green.png`) at ~160px above the `# GitMap` heading
- Uses `<p align="center"><img>` for GitHub rendering

### 2. Update Features List

- Clarify color system: 5 heatmap accent presets + custom hex, 7 icon color themes
- Add "Watch directories" feature (auto-discover repos in watched folders)
- Remove ambiguous "5 preset accent colors" phrasing

### 3. Update Modules Table

- Add `icons` module — embeds icon PNGs, handles decode/resize for tray and Finder icons
- Add `discovery_watcher` module — watches parent directories for new git repos via FSEvents

### 4. Update Architecture Diagram

- Add "Discovery Watcher (notify)" to the background services section

### 5. No Screenshot

- Logo only, no app screenshot for now
