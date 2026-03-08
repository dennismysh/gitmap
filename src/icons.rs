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
