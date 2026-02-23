use crate::config::Config;
use crate::discovery::discover_repos;
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct SettingsState {
    folder_picker_result: Arc<Mutex<Option<Vec<PathBuf>>>>,
    discover_result: Arc<Mutex<Option<Vec<PathBuf>>>>,
    pub hex_input: String,
}

impl SettingsState {
    pub fn new(config: &Config) -> Self {
        Self {
            folder_picker_result: Arc::new(Mutex::new(None)),
            discover_result: Arc::new(Mutex::new(None)),
            hex_input: config.accent_color.clone(),
        }
    }
}

const PRESET_COLORS: [(&str, &str); 5] = [
    ("Green", "#39d353"),
    ("Blue", "#58a6ff"),
    ("Purple", "#7c3aed"),
    ("Orange", "#f97316"),
    ("Pink", "#ec4899"),
];

pub fn draw_settings(ui: &mut egui::Ui, config: &mut Config, state: &mut SettingsState) {
    ui.heading("Settings");
    ui.add_space(12.0);

    // --- Git Identity ---
    ui.label(egui::RichText::new("Git Identity").strong().size(14.0));
    ui.add_space(4.0);

    let identity_text = config
        .tracked_repos
        .first()
        .and_then(|p| crate::scanner::detect_identity(p).ok())
        .map(|id| format!("{} <{}>", id.name, id.email))
        .unwrap_or_else(|| "No repos tracked yet".to_string());
    ui.label(
        egui::RichText::new(identity_text)
            .size(12.0)
            .color(egui::Color32::from_rgb(139, 148, 158)),
    );

    ui.add_space(12.0);

    // --- Tracked Repos ---
    ui.label(egui::RichText::new("Tracked Repositories").strong().size(14.0));
    ui.add_space(4.0);

    // Check for folder picker results
    if let Ok(mut guard) = state.folder_picker_result.try_lock() {
        if let Some(paths) = guard.take() {
            for path in paths {
                if !config.tracked_repos.contains(&path) {
                    config.tracked_repos.push(path);
                }
            }
        }
    }

    // Check for discover results
    if let Ok(mut guard) = state.discover_result.try_lock() {
        if let Some(paths) = guard.take() {
            for path in paths {
                if !config.tracked_repos.contains(&path) {
                    config.tracked_repos.push(path);
                }
            }
        }
    }

    // List tracked repos with remove buttons
    let mut to_remove = None;
    for (i, repo) in config.tracked_repos.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(repo.display().to_string())
                    .size(11.0)
                    .color(egui::Color32::from_rgb(200, 200, 200)),
            );
            if ui.small_button("\u{2715}").clicked() {
                to_remove = Some(i);
            }
        });
    }
    if let Some(i) = to_remove {
        config.tracked_repos.remove(i);
    }

    if config.tracked_repos.is_empty() {
        ui.label(
            egui::RichText::new("No repositories tracked")
                .size(11.0)
                .color(egui::Color32::from_rgb(100, 100, 100)),
        );
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("Add Repository...").clicked() {
            let result = Arc::clone(&state.folder_picker_result);
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let folder = rfd::FileDialog::new()
                    .set_title("Select Git Repository")
                    .pick_folder();
                if let Ok(mut guard) = result.lock() {
                    *guard = folder.map(|p| vec![p]);
                }
                ctx.request_repaint();
            });
        }

        if ui.button("Scan Directory...").clicked() {
            let result = Arc::clone(&state.discover_result);
            let ctx = ui.ctx().clone();
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
                ctx.request_repaint();
            });
        }
    });

    ui.add_space(12.0);

    // --- Accent Color ---
    ui.label(egui::RichText::new("Accent Color").strong().size(14.0));
    ui.add_space(4.0);

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
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Custom:");
        let response = ui.text_edit_singleline(&mut state.hex_input);
        if response.lost_focus() {
            let trimmed = state.hex_input.trim();
            if trimmed.len() == 7 && trimmed.starts_with('#') {
                config.accent_color = trimmed.to_string();
            }
        }
    });
}

fn parse_hex_rgb(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    [r, g, b]
}
