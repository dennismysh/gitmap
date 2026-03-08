# Icon Customization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Let users pick from 7 icon color variants that update the tray icon and Finder app icon.

**Architecture:** Embed all icon PNGs in the binary, decode with `image` crate, resize to 22x22 for tray. Use `NSWorkspace.setIcon` via objc2 for Finder icon. New "Logo Color" settings section with color swatches and a "colored tray" toggle.

**Tech Stack:** Rust, egui, tray-icon, image (png), objc2/objc2-app-kit (NSWorkspace, NSImage)

---

### Task 1: Add IconColor enum and config fields

**Files:**
- Modify: `src/config.rs`

**Step 1: Add IconColor enum and config fields**

Add above the `Config` struct in `src/config.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconColor {
    Green,
    Blue,
    Purple,
    Orange,
    Pink,
    Bumblebee,
    Lemon,
}

impl Default for IconColor {
    fn default() -> Self {
        IconColor::Green
    }
}

impl IconColor {
    pub fn label(&self) -> &'static str {
        match self {
            IconColor::Green => "Green",
            IconColor::Blue => "Blue",
            IconColor::Purple => "Purple",
            IconColor::Orange => "Orange",
            IconColor::Pink => "Pink",
            IconColor::Bumblebee => "Bumblebee",
            IconColor::Lemon => "Lemon",
        }
    }

    pub fn all() -> &'static [IconColor] {
        &[
            IconColor::Green,
            IconColor::Blue,
            IconColor::Purple,
            IconColor::Orange,
            IconColor::Pink,
            IconColor::Bumblebee,
            IconColor::Lemon,
        ]
    }

    /// Representative RGB color for the settings UI swatch
    pub fn swatch_rgb(&self) -> [u8; 3] {
        match self {
            IconColor::Green => [57, 211, 83],
            IconColor::Blue => [88, 166, 255],
            IconColor::Purple => [124, 58, 237],
            IconColor::Orange => [249, 115, 22],
            IconColor::Pink => [236, 72, 153],
            IconColor::Bumblebee => [245, 158, 11],
            IconColor::Lemon => [234, 212, 30],
        }
    }
}
```

Add two new fields to `Config` struct (with `#[serde(default)]`):

```rust
#[serde(default)]
pub icon_color: IconColor,
#[serde(default)]
pub colored_tray_icon: bool,
```

Update `Default for Config` to include:

```rust
icon_color: IconColor::default(),
colored_tray_icon: false,
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build (no errors)

**Step 3: Commit**

```bash
git add src/config.rs
git commit -m "feat: add IconColor enum and config fields for icon customization"
```

---

### Task 2: Add image crate dependency and update objc2-app-kit features

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add image dependency and update objc2-app-kit features**

Add to `[dependencies]`:

```toml
image = { version = "0.25", default-features = false, features = ["png"] }
```

Update the `objc2-app-kit` line under `[target.'cfg(target_os = "macos")'.dependencies]` to add NSWorkspace and NSImage features:

```toml
objc2-app-kit = { version = "0.3", features = ["NSApplication", "NSRunningApplication", "NSWorkspace", "NSImage"] }
objc2-foundation = { version = "0.3", features = ["NSData", "NSString"] }
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add image crate and objc2 NSWorkspace/NSImage features"
```

---

### Task 3: Create icons module — embed PNGs and decode

**Files:**
- Create: `src/icons.rs`
- Modify: `src/lib.rs`

**Step 1: Create icons.rs with embedded PNGs and decode logic**

```rust
use crate::config::IconColor;

// Embedded icon PNGs
const GREEN_PNG: &[u8] = include_bytes!("../assets/gitmap-green.png");
const BLUE_PNG: &[u8] = include_bytes!("../assets/gitmap-blue.png");
const PURPLE_PNG: &[u8] = include_bytes!("../assets/gitmap-purple.png");
const ORANGE_PNG: &[u8] = include_bytes!("../assets/gitmap-orange.png");
const PINK_PNG: &[u8] = include_bytes!("../assets/gitmap-pink.png");
const BUMBLEBEE_PNG: &[u8] = include_bytes!("../assets/gitmap-bumblebee.png");
const LEMON_PNG: &[u8] = include_bytes!("../assets/gitmap-lemon.png");
const TRANSPARENT_PNG: &[u8] = include_bytes!("../assets/gitmap-transparent.png");

impl IconColor {
    pub fn png_bytes(&self) -> &'static [u8] {
        match self {
            IconColor::Green => GREEN_PNG,
            IconColor::Blue => BLUE_PNG,
            IconColor::Purple => PURPLE_PNG,
            IconColor::Orange => ORANGE_PNG,
            IconColor::Pink => PINK_PNG,
            IconColor::Bumblebee => BUMBLEBEE_PNG,
            IconColor::Lemon => LEMON_PNG,
        }
    }
}

/// Decode a PNG byte slice and resize to the given dimensions, returning RGBA bytes.
pub fn decode_and_resize(png_bytes: &[u8], width: u32, height: u32) -> Vec<u8> {
    let img = image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
        .expect("Failed to decode embedded PNG");
    let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    resized.to_rgba8().into_raw()
}

/// Build a tray_icon::Icon from the given PNG bytes at 22x22.
pub fn build_tray_icon(png_bytes: &[u8]) -> tray_icon::Icon {
    let rgba = decode_and_resize(png_bytes, 22, 22);
    tray_icon::Icon::from_rgba(rgba, 22, 22).expect("Failed to create tray icon")
}

/// Get the appropriate tray icon and template flag based on config.
pub fn tray_icon_for_config(
    icon_color: IconColor,
    colored_tray: bool,
) -> (tray_icon::Icon, bool) {
    if colored_tray {
        (build_tray_icon(icon_color.png_bytes()), false)
    } else {
        (build_tray_icon(TRANSPARENT_PNG), true)
    }
}
```

**Step 2: Add icons module to lib.rs**

Add to `src/lib.rs`:

```rust
pub mod icons;
```

**Step 3: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 4: Commit**

```bash
git add src/icons.rs src/lib.rs
git commit -m "feat: add icons module with embedded PNGs and decode/resize"
```

---

### Task 4: Add NSWorkspace Finder icon bridge

**Files:**
- Modify: `src/icons.rs`

**Step 1: Add Finder icon function using NSWorkspace.setIcon**

Append to `src/icons.rs`:

```rust
/// Set the Finder icon for the .app bundle using NSWorkspace.
/// This uses resource fork metadata and does NOT modify the signed bundle.
#[cfg(target_os = "macos")]
pub fn set_finder_icon(icon_color: IconColor) {
    use objc2::rc::Retained;
    use objc2_app_kit::{NSImage, NSWorkspace};
    use objc2_foundation::NSData;

    // Find the .app bundle path from the current executable
    let app_path = match find_app_bundle_path() {
        Some(p) => p,
        None => return, // Not running from a .app bundle (e.g., cargo run)
    };

    let png_bytes = icon_color.png_bytes();

    unsafe {
        let data = NSData::with_bytes(png_bytes);
        let image = match NSImage::initWithData(NSImage::alloc(), &data) {
            Some(img) => img,
            None => return,
        };

        let path_str = objc2_foundation::NSString::from_str(&app_path);
        let workspace = NSWorkspace::sharedWorkspace();
        let _ = workspace.setIcon_forFile_options_(Some(&image), &path_str, 0);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_finder_icon(_icon_color: IconColor) {
    // No-op on non-macOS
}

/// Walk up from the current executable to find the .app bundle path.
fn find_app_bundle_path() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.canonicalize().unwrap_or(exe);
    // Walk up looking for a directory ending in .app
    let mut current = exe.as_path();
    while let Some(parent) = current.parent() {
        if let Some(name) = current.file_name() {
            if name.to_string_lossy().ends_with(".app") {
                return Some(current.to_string_lossy().to_string());
            }
        }
        current = parent;
    }
    None
}
```

**NOTE:** The exact `NSWorkspace` method signature may need adjustment based on the objc2-app-kit 0.3 API. Check docs if `setIcon_forFile_options_` isn't found — it may be named `setIcon_forFile_options` (no trailing underscore) or require different parameter types. The `NSData::with_bytes` may be `NSData::from_bytes` or similar. Verify against the actual objc2-foundation 0.3 API.

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | tail -10`
Expected: successful build (may need to adjust objc2 method names based on API)

**Step 3: Commit**

```bash
git add src/icons.rs
git commit -m "feat: add NSWorkspace Finder icon bridge for .app bundle"
```

---

### Task 5: Update main.rs — load tray icon from config

**Files:**
- Modify: `src/main.rs`

**Step 1: Replace hardcoded black square with config-based icon**

In `src/main.rs`, replace lines 17-22 (the hardcoded black square icon generation):

```rust
let mut icon_rgba = Vec::with_capacity(22 * 22 * 4);
for _ in 0..(22 * 22) {
    icon_rgba.extend_from_slice(&[0, 0, 0, 255]);
}
let icon =
    tray_icon::Icon::from_rgba(icon_rgba, 22, 22).expect("Failed to create tray icon");
```

With:

```rust
let (icon, icon_as_template) =
    gitmap::icons::tray_icon_for_config(config.icon_color, config.colored_tray_icon);
```

Then update the `TrayIconBuilder` to use the variable for template:

Change `.with_icon_as_template(true)` to `.with_icon_as_template(icon_as_template)`.

**Step 2: Verify it compiles and runs**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

Run: `cargo run` and verify the tray icon appears (should be the transparent/monochrome template icon by default)

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: load tray icon from config instead of hardcoded black square"
```

---

### Task 6: Add TrayMessage::UpdateIcon and pass TrayIcon handle

**Files:**
- Modify: `src/ui/popover.rs`
- Modify: `src/main.rs`

**Step 1: Add UpdateIcon message variant**

In `src/ui/popover.rs`, add to the `TrayMessage` enum:

```rust
pub enum TrayMessage {
    ToggleWindow { icon_rect: tray_icon::Rect },
    Quit,
    UpdateIcon,
}
```

**Step 2: Add tray_icon handle to GitMapApp**

Add a field to `GitMapApp` struct:

```rust
tray_icon: Option<tray_icon::TrayIcon>,
```

Add to `GitMapApp::new` parameters: accept the tray icon handle. Or better: add a method to set it after construction since the tray icon is created after the app. Add a setter method:

```rust
pub fn set_tray_icon(&mut self, tray: tray_icon::TrayIcon) {
    self.tray_icon = Some(tray);
}
```

Initialize `tray_icon: None` in the constructor.

**Step 3: Handle UpdateIcon in the message loop**

In the `update` method of `GitMapApp` (the section around line 590 that processes `tray_rx`), add handling for `UpdateIcon`:

```rust
TrayMessage::UpdateIcon => {
    if let Some(ref tray) = self.tray_icon {
        let (icon, as_template) = crate::icons::tray_icon_for_config(
            self.config.icon_color,
            self.config.colored_tray_icon,
        );
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_icon_as_template(as_template);
    }
    // Also update Finder icon
    crate::icons::set_finder_icon(self.config.icon_color);
}
```

**Step 4: Pass tray icon handle in main.rs**

In `main.rs`, after the tray is built and the app is created, pass the tray handle. The current code creates the tray inside the `Box::new(move |_cc| { ... })` closure. After `let mut app = GitMapApp::new(...)` and `app.initial_scan()`, add:

```rust
app.set_tray_icon(tray);
```

Note: this means `tray` can't be moved into `tray_holder_clone` AND given to the app. Since `tray_holder` is only used to keep the tray alive, and the app now holds it, remove the `tray_holder` pattern entirely. The app struct keeps the tray alive.

**Step 5: Apply Finder icon on startup**

At the end of `GitMapApp::new` (or in `initial_scan`), add:

```rust
// Apply Finder icon if not default
if self.config.icon_color != crate::config::IconColor::Green {
    crate::icons::set_finder_icon(self.config.icon_color);
}
```

**Step 6: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 7: Commit**

```bash
git add src/ui/popover.rs src/main.rs
git commit -m "feat: add TrayMessage::UpdateIcon and wire tray icon handle to app"
```

---

### Task 7: Add Logo Color settings UI section

**Files:**
- Modify: `src/ui/settings.rs`

**Step 1: Add icon_changed flag to SettingsState**

Add to `SettingsState`:

```rust
pub icon_changed: bool,
```

Initialize to `false` in `SettingsState::new`.

**Step 2: Add Logo Color section above Accent Color**

In `draw_settings`, insert before the `// --- Accent Color ---` section (around line 247):

```rust
// --- Logo Color ---
ui.label(egui::RichText::new("Logo Color").strong().size(14.0));
ui.add_space(4.0);

ui.horizontal(|ui| {
    for color in crate::config::IconColor::all() {
        let [r, g, b] = color.swatch_rgb();
        let selected = config.icon_color == *color;
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
            config.icon_color = *color;
            state.icon_changed = true;
        }
        response.on_hover_text(color.label());
    }
});

ui.add_space(4.0);
if ui.checkbox(&mut config.colored_tray_icon, "Use colored tray icon").changed() {
    state.icon_changed = true;
}

ui.add_space(12.0);
```

**Step 3: Use icon_changed flag in popover to trigger UpdateIcon**

In `src/ui/popover.rs`, after calling `settings::draw_settings(...)`, check the flag:

```rust
if self.settings_state.icon_changed {
    self.settings_state.icon_changed = false;
    if let Some(ref tray) = self.tray_icon {
        let (icon, as_template) = crate::icons::tray_icon_for_config(
            self.config.icon_color,
            self.config.colored_tray_icon,
        );
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_icon_as_template(as_template);
    }
    crate::icons::set_finder_icon(self.config.icon_color);
}
```

**Step 4: Verify it compiles and test manually**

Run: `cargo run`
- Open settings
- Verify "Logo Color" section appears above "Accent Color"
- Click different colors — tray icon should NOT change (colored tray is off by default)
- Check "Use colored tray icon" then click colors — tray icon should update in real time
- Uncheck "Use colored tray icon" — tray should go back to monochrome

**Step 5: Commit**

```bash
git add src/ui/settings.rs src/ui/popover.rs
git commit -m "feat: add Logo Color settings UI with tray icon and Finder icon updates"
```

---

### Task 8: Clean up old icon.rgba and verify end-to-end

**Files:**
- Delete: `assets/icon.rgba` (the old 22x22 black square)

**Step 1: Remove the old icon.rgba file**

```bash
rm assets/icon.rgba
```

Verify no code references it:

Run: `grep -r "icon.rgba" src/`
Expected: no matches

**Step 2: Full end-to-end test**

Run: `cargo run`

Verify:
1. App launches with monochrome tray icon (transparent template)
2. Open settings → Logo Color section shows 7 colored swatches, Green selected
3. Click Blue → no tray change (colored tray off)
4. Check "Use colored tray icon" → tray becomes blue
5. Click Purple → tray becomes purple immediately
6. Uncheck "Use colored tray icon" → tray returns to monochrome
7. Quit and relaunch → settings persist, tray reflects saved preference

**Step 3: Commit**

```bash
git rm assets/icon.rgba
git add -A assets/
git commit -m "chore: remove old icon.rgba, add icon PNGs to repo"
```

---

## Notes

- The `objc2-app-kit` method names may differ slightly from what's written. Check the actual 0.3 API docs during implementation. Common patterns: `setIcon_forFile_options_` or `setIcon_forFile_options`. `NSData::with_bytes` may be `NSData::from_vec` or need `NSData::initWithBytes_length_`.
- The `image` crate is already a transitive dependency from eframe, so adding it directly doesn't increase the dependency tree.
- The swatch RGB values in `IconColor::swatch_rgb()` are approximations of each icon's mid-tone. Adjust after seeing them in the UI if needed.
