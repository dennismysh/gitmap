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

/// Set the Finder icon for the .app bundle using NSWorkspace.
/// This uses resource fork metadata and does NOT modify the signed bundle.
#[cfg(target_os = "macos")]
pub fn set_finder_icon(icon_color: IconColor) {
    use objc2::AnyThread;
    use objc2_app_kit::{NSImage, NSWorkspace, NSWorkspaceIconCreationOptions};
    use objc2_foundation::{NSData, NSString};

    let app_path = match find_app_bundle_path() {
        Some(p) => p,
        None => return, // Not running from a .app bundle (e.g., cargo run)
    };

    let png_bytes = icon_color.png_bytes();
    let data = NSData::with_bytes(png_bytes);
    let image = match NSImage::initWithData(NSImage::alloc(), &data) {
        Some(img) => img,
        None => return,
    };

    let path_str = NSString::from_str(&app_path);
    let workspace = NSWorkspace::sharedWorkspace();
    let _ = workspace.setIcon_forFile_options(
        Some(&image),
        &path_str,
        NSWorkspaceIconCreationOptions(0),
    );
}

#[cfg(not(target_os = "macos"))]
pub fn set_finder_icon(_icon_color: IconColor) {}

/// Walk up from the current executable to find the .app bundle path.
fn find_app_bundle_path() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.canonicalize().unwrap_or(exe);
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
