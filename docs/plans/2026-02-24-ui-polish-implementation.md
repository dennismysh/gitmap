# UI Polish Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add custom hex color preview swatch, click-outside-to-hide with debounce, and fix back button arrow.

**Architecture:** Three independent UI changes in `popover.rs` and `settings.rs`. The click-outside-to-hide adds focus-tracking state to `GitMapApp` and a shared `AtomicBool` for file picker guard. No new crates needed.

**Tech Stack:** Rust, eframe/egui, tray-icon, std::sync::atomic

---

### Task 1: Back Button Arrow

**Files:**
- Modify: `src/ui/popover.rs:421`

**Step 1: Change the unicode character**

In `src/ui/popover.rs`, line 421, change `\u{2190}` to `\u{25C0}`:

```rust
// Before:
if ui.button("\u{2190} Back").clicked() {

// After:
if ui.button("\u{25C0} Back").clicked() {
```

**Step 2: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors

**Step 3: Commit**

```bash
git add src/ui/popover.rs
git commit -m "fix: use filled triangle for back button arrow"
```

---

### Task 2: Custom Hex Color Preview Swatch

**Files:**
- Modify: `src/ui/settings.rs:208-234`

**Step 1: Add 6th swatch after the preset loop**

In `src/ui/settings.rs`, inside the `ui.horizontal` block at line 208, after the `for` loop over `PRESET_COLORS` (after line 233's closing brace), add the custom color preview box:

```rust
        ui.horizontal(|ui| {
            for (name, hex) in &PRESET_COLORS {
                let [r, g, b] = parse_hex_rgb(hex);
                let selected = config.accent_color == *hex;
                let size = if selected { 24.0 } else { 20.0 };
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(4),
                    egui::Color32::from_rgb(r, g, b),
                );
                if selected {
                    ui.painter().rect_stroke(
                        rect,
                        egui::CornerRadius::same(4),
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                        egui::StrokeKind::Outside,
                    );
                }
                if response.clicked() {
                    config.accent_color = hex.to_string();
                    state.hex_input = hex.to_string();
                }
                response.on_hover_text(*name);
            }

            // Custom hex color preview swatch
            let is_preset = PRESET_COLORS.iter().any(|(_, hex)| config.accent_color == *hex);
            let trimmed = state.hex_input.trim();
            let valid_hex = trimmed.len() == 7 && trimmed.starts_with('#');
            let [cr, cg, cb] = if valid_hex {
                parse_hex_rgb(trimmed)
            } else {
                [80, 80, 80] // gray placeholder for invalid input
            };
            let custom_selected = !is_preset && valid_hex && config.accent_color == trimmed;
            let size = if custom_selected { 24.0 } else { 20.0 };
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(4),
                egui::Color32::from_rgb(cr, cg, cb),
            );
            if custom_selected {
                ui.painter().rect_stroke(
                    rect,
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }
            if response.clicked() && valid_hex {
                config.accent_color = trimmed.to_string();
            }
            response.on_hover_text("Custom");
        });
```

This replaces the entire `ui.horizontal` block at lines 208-234.

**Step 2: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors

**Step 3: Commit**

```bash
git add src/ui/settings.rs
git commit -m "feat: add custom hex color preview swatch in settings"
```

---

### Task 3: Click-Outside-to-Hide with Debounce

**Files:**
- Modify: `src/ui/popover.rs:1-6` (imports)
- Modify: `src/ui/popover.rs:17-28` (GitMapApp struct)
- Modify: `src/ui/popover.rs:31-57` (GitMapApp::new)
- Modify: `src/ui/popover.rs:348-406` (update method, tray message handling)
- Modify: `src/ui/settings.rs:1-5` (imports)
- Modify: `src/ui/settings.rs:7-13` (SettingsState struct)
- Modify: `src/ui/settings.rs:15-24` (SettingsState::new)
- Modify: `src/ui/settings.rs:136-167` (file picker spawns)

**Step 1: Add `file_picker_active` to SettingsState**

In `src/ui/settings.rs`, add the import and field:

```rust
// Add to imports (line 5):
use std::sync::atomic::{AtomicBool, Ordering};

// Add field to SettingsState struct:
pub struct SettingsState {
    folder_picker_result: Arc<Mutex<Option<Vec<PathBuf>>>>,
    discover_result: Arc<Mutex<Option<Vec<PathBuf>>>>,
    pub hex_input: String,
    /// Repos that were removed this session — available to re-add
    pub untracked_repos: Vec<PathBuf>,
    pub file_picker_active: Arc<AtomicBool>,
}

// Initialize in new():
impl SettingsState {
    pub fn new(config: &Config) -> Self {
        Self {
            folder_picker_result: Arc::new(Mutex::new(None)),
            discover_result: Arc::new(Mutex::new(None)),
            hex_input: config.accent_color.clone(),
            untracked_repos: Vec::new(),
            file_picker_active: Arc::new(AtomicBool::new(false)),
        }
    }
}
```

**Step 2: Set the flag in file picker spawns**

In `src/ui/settings.rs`, wrap the two `std::thread::spawn` calls (lines 140-148 and 154-165) to set/clear the flag:

For "Add Repository..." (around line 137):
```rust
            if ui.button("Add Repository...").clicked() {
                let result = Arc::clone(&state.folder_picker_result);
                let ctx = ui.ctx().clone();
                let picker_flag = Arc::clone(&state.file_picker_active);
                picker_flag.store(true, Ordering::Relaxed);
                std::thread::spawn(move || {
                    let folder = rfd::FileDialog::new()
                        .set_title("Select Git Repository")
                        .pick_folder();
                    if let Ok(mut guard) = result.lock() {
                        *guard = folder.map(|p| vec![p]);
                    }
                    picker_flag.store(false, Ordering::Relaxed);
                    ctx.request_repaint();
                });
            }
```

For "Scan Directory..." (around line 151):
```rust
            if ui.button("Scan Directory...").clicked() {
                let result = Arc::clone(&state.discover_result);
                let ctx = ui.ctx().clone();
                let picker_flag = Arc::clone(&state.file_picker_active);
                picker_flag.store(true, Ordering::Relaxed);
                std::thread::spawn(move || {
                    let folder = rfd::FileDialog::new()
                        .set_title("Select Parent Directory to Scan")
                        .pick_folder();
                    if let Some(root) = folder {
                        let repos = discover_repos(&root);
                        if let Ok(mut guard) = result.lock() {
                            *guard = Some(repos);
                        }
                    }
                    picker_flag.store(false, Ordering::Relaxed);
                    ctx.request_repaint();
                });
            }
```

**Step 3: Add focus-tracking fields to GitMapApp**

In `src/ui/popover.rs`, add the import and fields:

```rust
// Add to imports (after line 2):
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// Add fields to GitMapApp struct:
pub struct GitMapApp {
    tray_rx: mpsc::Receiver<TrayMessage>,
    visible: bool,
    pub config: Config,
    pub store: CommitStore,
    hovered_info: Option<String>,
    pub show_settings: bool,
    settings_state: SettingsState,
    identity: Option<GitIdentity>,
    watcher: Option<RepoWatcher>,
    last_icon_rect: Option<tray_icon::Rect>,
    focus_lost_at: Option<std::time::Instant>,
    file_picker_active: Arc<AtomicBool>,
}
```

**Step 4: Initialize new fields in `GitMapApp::new`**

```rust
        let file_picker_active = Arc::clone(&settings_state.file_picker_active);
        Self {
            tray_rx,
            visible: false,
            config,
            store,
            hovered_info: None,
            show_settings: false,
            settings_state,
            identity,
            watcher,
            last_icon_rect: None,
            focus_lost_at: None,
            file_picker_active,
        }
```

**Step 5: Add focus-loss detection in `update` method**

In the `update` method, after the file watcher polling (after line 372) and before tray message processing (line 374), add focus tracking:

```rust
        // Track focus loss for click-outside-to-hide
        let focused = ctx.input(|i| i.viewport().focused);
        match focused {
            Some(true) => {
                self.focus_lost_at = None;
            }
            Some(false) if self.visible && self.focus_lost_at.is_none() => {
                self.focus_lost_at = Some(std::time::Instant::now());
            }
            _ => {}
        }
```

**Step 6: Cancel pending hide on tray click, and add debounced hide after tray processing**

Modify the `ToggleWindow` handler to clear `focus_lost_at`:

```rust
                TrayMessage::ToggleWindow { icon_rect } => {
                    self.last_icon_rect = Some(icon_rect);
                    self.focus_lost_at = None; // Cancel any pending focus-loss hide
                    self.visible = !self.visible;
                    // ... rest unchanged
                }
```

After the `while let Ok(msg)` loop (after line 406), add the debounced hide:

```rust
        // Debounced click-outside-to-hide
        if let Some(lost_at) = self.focus_lost_at {
            if lost_at.elapsed() >= std::time::Duration::from_millis(150)
                && !self.file_picker_active.load(Ordering::Relaxed)
            {
                self.visible = false;
                self.focus_lost_at = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                    egui::pos2(-10000.0, -10000.0),
                ));
            }
        }
```

**Step 7: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles with no errors

**Step 8: Commit**

```bash
git add src/ui/popover.rs src/ui/settings.rs
git commit -m "feat: hide window on click outside with debounce"
```
