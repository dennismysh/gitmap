# Icon Customization Design

## Goal

Allow users to choose their GitMap icon color from 7 pre-made variants. The choice affects the menu bar tray icon (optionally) and the Finder app icon. Default is green.

## Icon Variants

7 color PNGs in `assets/`: green, blue, purple, orange, pink, bumblebee, lemon. Plus a transparent/white PNG for the monochrome tray template icon.

## Data Model

New `IconColor` enum in `config.rs`:

```rust
enum IconColor { Green, Blue, Purple, Orange, Pink, Bumblebee, Lemon }
```

New `Config` fields (both `#[serde(default)]` for backward compatibility):
- `icon_color: IconColor` — selected icon variant (default: `Green`)
- `colored_tray_icon: bool` — apply color to tray icon (default: `false`, monochrome)

## Icon Loading (Approach: Embed PNGs in Binary)

- All 8 PNGs embedded via `include_bytes!()` in a new `icons.rs` module
- Decoded at runtime with the `image` crate (png feature only)
- Resized to 22x22 for tray icon usage
- `IconColor` provides a method to return the corresponding embedded bytes

## Tray Icon Updates

- On startup: load icon based on `config.icon_color` + `config.colored_tray_icon`
  - If `colored_tray_icon` is false: use transparent PNG, `set_icon_as_template(true)`
  - If `colored_tray_icon` is true: use selected color PNG, `set_icon_as_template(false)`
- On settings change: send `TrayMessage::UpdateIcon` through existing channel
- `TrayIcon` handle passed into `GitMapApp` (or new message variant) to call `set_icon()` and `set_icon_as_template()`

## Finder App Icon (NSWorkspace.setIcon)

- Use `NSWorkspace.setIcon(_:forFile:)` via `objc2` (already a transitive dependency from `tray-icon`)
- Creates an `NSImage` from the embedded PNG bytes and sets it on the `.app` bundle path
- Stores icon as Finder resource fork metadata — does NOT modify signed bundle contents
- Code signature preserved
- On startup: re-apply if `icon_color` is not Green (handles auto-update resetting the resource fork)
- App locates its own bundle via `std::env::current_exe()` walking up to `.app`

## Settings UI

New "Logo Color" section placed ABOVE the existing "Accent Color" section:

1. "Logo Color" heading (14px bold, matching other sections)
2. Row of 7 clickable color swatches — representative color per variant, selected gets white border + larger size (same pattern as accent color swatches), hover shows name
3. Checkbox: "Use colored tray icon"

Both changes trigger `TrayMessage::UpdateIcon` for immediate tray update + `NSWorkspace.setIcon` for immediate Finder update.

## Dependencies

- `image` crate with `png` feature (new) — for PNG decoding and resizing
- `objc2` — already transitive from `tray-icon`, used for NSWorkspace.setIcon

## Bundle

Green `.icns` remains the default in the `.app` bundle. No changes to `bundle.sh` or release workflow.

## Auto-Update Integration

On launch, if `config.icon_color != Green`, re-apply the Finder icon via NSWorkspace. This handles the case where auto-update replaces the `.app` bundle and clears the resource fork custom icon.

## Files Changed

| File | Change |
|---|---|
| `Cargo.toml` | Add `image` with `png` feature |
| `src/config.rs` | Add `IconColor` enum, new config fields |
| `src/icons.rs` (new) | Embed PNGs, decode/resize, NSWorkspace bridge |
| `src/lib.rs` | Export `icons` module |
| `src/main.rs` | Load initial tray icon from config, pass TrayIcon handle |
| `src/ui/popover.rs` | Handle `TrayMessage::UpdateIcon`, re-apply Finder icon on startup |
| `src/ui/settings.rs` | New "Logo Color" section with swatches + checkbox |
