# UI Polish: Color Preview, Click-Outside-to-Hide, Back Arrow

## Overview

Three small UI improvements to the popover and settings panels.

## Change 1: Custom Hex Color Preview Swatch

Add a 6th color box at the end of the preset swatch row in settings. It renders using `parse_hex_rgb` on `state.hex_input` and updates live as the user types. White selection border when the current `accent_color` doesn't match any preset. Clicking it applies the hex value. Shows "Custom" tooltip on hover. If the hex input is invalid, shows a gray placeholder.

**Files**: `src/ui/settings.rs`

## Change 2: Click-Outside-to-Hide with Debounce

When the popover window loses focus, hide it back to the tray. Uses a debounce to avoid conflict with tray icon clicks.

**New fields on `GitMapApp`**:
- `focus_lost_at: Option<std::time::Instant>` — set when viewport reports focus lost
- `file_picker_active: Arc<AtomicBool>` — shared with file picker threads to prevent hiding during dialogs

**Logic in `update`**:
1. Check `ctx.input(|i| i.viewport().focused)`. If `Some(false)` and window is visible, set `focus_lost_at`.
2. If `Some(true)`, clear `focus_lost_at`.
3. When processing `ToggleWindow` tray message, clear `focus_lost_at` (cancel pending hide).
4. After tray messages: if `focus_lost_at` is set, elapsed >= 150ms, and `file_picker_active` is false, hide window.

**File picker guard**: Set `file_picker_active = true` before spawning picker threads in settings, reset to `false` when result is stored.

**Files**: `src/ui/popover.rs`, `src/ui/settings.rs`

## Change 3: Back Button Arrow

Change `\u{2190}` (← which renders as a box) to `\u{25C0}` (filled triangle), matching the year navigation arrows.

**Files**: `src/ui/popover.rs`
