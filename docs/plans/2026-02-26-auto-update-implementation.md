# Auto-Update Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add in-app auto-update that checks GitHub Releases on launch, prompts the user, and replaces the .app bundle.

**Architecture:** New `updater.rs` module handles GitHub API check and download/install. Background thread sends update info to UI via `mpsc`. Update banner shown in heatmap view; auto-update toggle in settings.

**Tech Stack:** `ureq` (blocking HTTP), GitHub Releases API, `ditto` (macOS built-in zip extraction)

---

### Task 1: Add `ureq` dependency and `auto_update` config field

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/config.rs`

**Step 1: Add ureq to Cargo.toml**

Add to `[dependencies]`:

```toml
ureq = "3"
```

**Step 2: Add `auto_update` field to Config**

In `src/config.rs`, add `auto_update: bool` to the `Config` struct after `view_mode`:

```rust
pub auto_update: bool,
```

In `Default for Config`, add:

```rust
auto_update: false,
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: success (serde derives auto_update; existing configs without the field get `false` via `unwrap_or_default()`)

Note: `serde_json::from_str` uses `unwrap_or_default()` in `Config::load()`, so missing field in old config files falls back to `Default`. However, this means ALL fields reset. To handle missing fields gracefully, add `#[serde(default)]` to the Config struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
```

This makes each missing field use its `Default` value individually.

**Step 4: Commit**

```
git add Cargo.toml src/config.rs
git commit -m "feat: add ureq dependency and auto_update config field"
```

---

### Task 2: Create `updater.rs` with `check_for_update`

**Files:**
- Create: `src/updater.rs`
- Modify: `src/lib.rs` (add `pub mod updater;`)

**Step 1: Create the updater module**

Create `src/updater.rs`:

```rust
use std::io::Read;

const GITHUB_REPO: &str = "dennismysh/gitmap";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
}

/// Check GitHub Releases API for a newer version.
/// Returns None if up to date or on any error.
pub fn check_for_update() -> Option<UpdateInfo> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = ureq::get(&url)
        .header("User-Agent", "gitmap-updater")
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .ok()?;

    let mut body = String::new();
    response.into_body().read_to_string(&mut body).ok()?;

    let json: serde_json::Value = serde_json::from_str(&body).ok()?;

    let tag = json["tag_name"].as_str()?;
    let remote_version = tag.strip_prefix('v').unwrap_or(tag);

    if remote_version <= CURRENT_VERSION {
        return None;
    }

    // Find the .zip asset
    let assets = json["assets"].as_array()?;
    let asset = assets.iter().find(|a| {
        a["name"]
            .as_str()
            .map(|n| n.ends_with("-macos-universal.zip"))
            .unwrap_or(false)
    })?;

    let download_url = asset["browser_download_url"].as_str()?;

    Some(UpdateInfo {
        version: remote_version.to_string(),
        download_url: download_url.to_string(),
    })
}
```

**Step 2: Register the module**

In `src/lib.rs`, add:

```rust
pub mod updater;
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: success

**Step 4: Commit**

```
git add src/updater.rs src/lib.rs
git commit -m "feat: add updater module with GitHub release check"
```

---

### Task 3: Add `download_and_install` to updater

**Files:**
- Modify: `src/updater.rs`

**Step 1: Add the download_and_install function**

Append to `src/updater.rs`:

```rust
/// Download the update zip and replace /Applications/GitMap.app.
/// Returns Ok(()) on success, Err on any failure.
pub fn download_and_install(download_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = std::path::Path::new("/tmp/gitmap-update");

    // Clean up any previous update attempt
    if tmp_dir.exists() {
        std::fs::remove_dir_all(tmp_dir)?;
    }
    std::fs::create_dir_all(tmp_dir)?;

    let zip_path = tmp_dir.join("GitMap.zip");

    // Download the zip
    let response = ureq::get(download_url)
        .header("User-Agent", "gitmap-updater")
        .call()?;

    let mut bytes = Vec::new();
    response.into_body().read_to_end(&mut bytes)?;
    std::fs::write(&zip_path, &bytes)?;

    // Extract using ditto (macOS built-in, preserves attributes)
    let status = std::process::Command::new("ditto")
        .args(["-xk", &zip_path.to_string_lossy(), &tmp_dir.to_string_lossy()])
        .status()?;

    if !status.success() {
        return Err("ditto extraction failed".into());
    }

    let extracted_app = tmp_dir.join("GitMap.app");
    if !extracted_app.exists() {
        return Err("GitMap.app not found in zip".into());
    }

    // Replace the installed app
    let installed_app = std::path::Path::new("/Applications/GitMap.app");
    if installed_app.exists() {
        std::fs::remove_dir_all(installed_app)?;
    }

    // Use Command to move (handles cross-device moves)
    let status = std::process::Command::new("mv")
        .args([&extracted_app.to_string_lossy().to_string(), "/Applications/GitMap.app".to_string()])
        .status()?;

    if !status.success() {
        return Err("failed to move GitMap.app to /Applications".into());
    }

    // Clean up temp dir
    let _ = std::fs::remove_dir_all(tmp_dir);

    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: success

**Step 3: Commit**

```
git add src/updater.rs
git commit -m "feat: add download_and_install to updater"
```

---

### Task 4: Wire update check into app startup

**Files:**
- Modify: `src/ui/popover.rs`

**Step 1: Add update state fields to GitMapApp**

Add these fields to `GitMapApp` struct:

```rust
update_rx: Option<mpsc::Receiver<crate::updater::UpdateInfo>>,
available_update: Option<crate::updater::UpdateInfo>,
update_in_progress: bool,
```

**Step 2: Initialize in `GitMapApp::new`**

In `new()`, spawn the background check thread and set up the channel:

```rust
let (update_tx, update_rx) = mpsc::channel();
std::thread::spawn(move || {
    if let Some(info) = crate::updater::check_for_update() {
        let _ = update_tx.send(info);
    }
});
```

Initialize the fields in `Self { ... }`:

```rust
update_rx: Some(update_rx),
available_update: None,
update_in_progress: false,
```

**Step 3: Poll for update result in `update()`**

At the top of the `update()` method (after the binary watcher check), add:

```rust
// Poll for update check result
if let Some(ref rx) = self.update_rx {
    if let Ok(info) = rx.try_recv() {
        if self.config.auto_update {
            // Silent auto-update
            self.update_in_progress = true;
            let url = info.download_url.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                if crate::updater::download_and_install(&url).is_ok() {
                    // Relaunch
                    let _ = std::process::Command::new("/Applications/GitMap.app/Contents/MacOS/gitmap").spawn();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        } else {
            self.available_update = Some(info);
        }
    }
}
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: success

**Step 5: Commit**

```
git add src/ui/popover.rs
git commit -m "feat: wire update check into app startup"
```

---

### Task 5: Add update banner to heatmap view

**Files:**
- Modify: `src/ui/popover.rs`

**Step 1: Add update banner in the main view**

In the `update()` method, inside the `else` branch (heatmap view, not settings), after `self.draw_stats(ui);`, add:

```rust
// Update banner
if let Some(ref info) = self.available_update {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Update available: v{}", info.version))
                .size(12.0)
                .color(egui::Color32::from_rgb(88, 166, 255)),
        );
        if self.update_in_progress {
            ui.label(
                egui::RichText::new("Updating...")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(139, 148, 158)),
            );
        } else if ui.small_button("Update").clicked() {
            self.update_in_progress = true;
            let url = info.download_url.clone();
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                if crate::updater::download_and_install(&url).is_ok() {
                    let _ = std::process::Command::new(
                        "/Applications/GitMap.app/Contents/MacOS/gitmap",
                    )
                    .spawn();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        }
    });
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: success

**Step 3: Commit**

```
git add src/ui/popover.rs
git commit -m "feat: add update banner to heatmap view"
```

---

### Task 6: Add auto-update toggle and version display to settings

**Files:**
- Modify: `src/ui/settings.rs`

**Step 1: Add version display and auto-update toggle**

At the end of `draw_settings()`, before the closing `});` of the ScrollArea, add:

```rust
ui.add_space(12.0);

// --- Updates ---
ui.label(egui::RichText::new("Updates").strong().size(14.0));
ui.add_space(4.0);

ui.horizontal(|ui| {
    ui.label(
        egui::RichText::new(format!("Version: v{}", env!("CARGO_PKG_VERSION")))
            .size(12.0)
            .color(egui::Color32::from_rgb(139, 148, 158)),
    );
});

ui.add_space(4.0);
ui.checkbox(&mut config.auto_update, "Auto-update (silent)");
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: success

**Step 3: Commit**

```
git add src/ui/settings.rs
git commit -m "feat: add auto-update toggle and version to settings"
```

---

### Task 7: Update release workflow to match asset naming

**Files:**
- Verify: `.github/workflows/release.yml`

The existing workflow already creates `GitMap-v0.1.0-macos-universal.zip`. The updater looks for assets ending in `-macos-universal.zip`. No changes needed — just verify the naming matches.

**Step 1: Verify naming convention**

The release workflow line: `zip -r "GitMap-${TAG}-macos-universal.zip" GitMap.app`
The updater check: `n.ends_with("-macos-universal.zip")`

These match. No changes needed.

---

### Task 8: Bump version to prepare for first release

**Files:**
- Modify: `Cargo.toml`
- Modify: `resources/Info.plist`

**Step 1: Set version in Cargo.toml**

Ensure `Cargo.toml` has the version you want for first release (e.g. `0.1.0` is already set).

**Step 2: Verify Info.plist version matches**

Check `resources/Info.plist` has matching `CFBundleShortVersionString` and `CFBundleVersion`.

**Step 3: Commit all remaining changes**

```
git add -A
git commit -m "chore: prepare v0.1.0 for first release"
```

---

### Task 9: End-to-end manual test

**Steps:**
1. Run `cargo build --release` and install via `./scripts/bundle.sh`
2. Open the app — verify no update banner (no releases exist yet on GitHub)
3. Verify settings show "Version: v0.1.0" and the auto-update checkbox
4. Push to GitHub and create a test release with tag `v0.1.1`
5. Restart app — verify update banner appears with "Update available: v0.1.1"
6. Click "Update" — verify download, replace, and relaunch works
7. Toggle auto-update on, create `v0.1.2` release, restart — verify silent update
