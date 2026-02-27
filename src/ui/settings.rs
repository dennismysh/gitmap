use crate::config::Config;
use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct SettingsState {
    folder_picker_result: Arc<Mutex<Option<Vec<PathBuf>>>>,
    discover_root_picker_result: Arc<Mutex<Option<PathBuf>>>,
    pub hex_input: String,
    pub file_picker_active: Arc<AtomicBool>,
}

impl SettingsState {
    pub fn new(config: &Config) -> Self {
        Self {
            folder_picker_result: Arc::new(Mutex::new(None)),
            discover_root_picker_result: Arc::new(Mutex::new(None)),
            hex_input: config.accent_color.clone(),
            file_picker_active: Arc::new(AtomicBool::new(false)),
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
    // Wrap entire settings in a vertical scroll area
    egui::ScrollArea::vertical().show(ui, |ui| {
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

        // --- Watch Directories ---
        ui.label(egui::RichText::new("Watch Directories").strong().size(14.0));
        ui.add_space(4.0);

        // Check for discover root picker result
        if let Ok(mut guard) = state.discover_root_picker_result.try_lock() {
            if let Some(path) = guard.take() {
                if !config.auto_discover_roots.contains(&path) {
                    config.auto_discover_roots.push(path);
                }
            }
        }

        if config.auto_discover_roots.is_empty() {
            ui.label(
                egui::RichText::new("No directories watched")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(100, 100, 100)),
            );
        } else {
            let mut root_to_remove = None;
            for (i, root) in config.auto_discover_roots.iter().enumerate() {
                ui.horizontal(|ui| {
                    let display = root
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| root.display().to_string());
                    ui.label(
                        egui::RichText::new(&display)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 200)),
                    );
                    if ui.small_button("\u{2715}").clicked() {
                        root_to_remove = Some(i);
                    }
                })
                .response
                .on_hover_text(root.display().to_string());
            }
            if let Some(i) = root_to_remove {
                config.auto_discover_roots.remove(i);
            }
        }

        ui.add_space(4.0);
        if ui.button("Add Watch Directory...").clicked() {
            let result = Arc::clone(&state.discover_root_picker_result);
            let ctx = ui.ctx().clone();
            let picker_flag = Arc::clone(&state.file_picker_active);
            picker_flag.store(true, Ordering::Relaxed);
            std::thread::spawn(move || {
                let folder = rfd::FileDialog::new()
                    .set_title("Select Directory to Watch for Repos")
                    .pick_folder();
                if let Ok(mut guard) = result.lock() {
                    *guard = folder;
                }
                picker_flag.store(false, Ordering::Relaxed);
                ctx.request_repaint();
            });
        }

        ui.add_space(12.0);

        // --- Tracked Repos ---
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Tracked Repositories").strong().size(14.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Reset All").clicked() {
                    // Move all tracked repos to untracked
                    let all = std::mem::take(&mut config.tracked_repos);
                    for repo in all {
                        if !config.untracked_repos.contains(&repo) {
                            config.untracked_repos.push(repo);
                        }
                    }
                }
            });
        });
        ui.add_space(4.0);

        // Check for folder picker results
        if let Ok(mut guard) = state.folder_picker_result.try_lock() {
            if let Some(paths) = guard.take() {
                for path in paths {
                    config.untracked_repos.retain(|p| p != &path);
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
                let display = repo
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| repo.display().to_string());
                ui.label(
                    egui::RichText::new(&display)
                        .size(12.0)
                        .color(egui::Color32::from_rgb(200, 200, 200)),
                );
                if ui.small_button("\u{2715}").clicked() {
                    to_remove = Some(i);
                }
            })
            .response
            .on_hover_text(repo.display().to_string());
        }
        if let Some(i) = to_remove {
            let removed = config.tracked_repos.remove(i);
            if !config.untracked_repos.contains(&removed) {
                config.untracked_repos.push(removed);
            }
        }

        if config.tracked_repos.is_empty() {
            ui.label(
                egui::RichText::new("No repositories tracked")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(100, 100, 100)),
            );
        }

        ui.add_space(4.0);
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

        // --- Untracked Repos (removed repos available to re-add) ---
        if !config.untracked_repos.is_empty() {
            ui.add_space(12.0);
            ui.label(egui::RichText::new("Untracked Repositories").strong().size(14.0));
            ui.add_space(4.0);

            let mut to_readd = None;
            for (i, repo) in config.untracked_repos.iter().enumerate() {
                ui.horizontal(|ui| {
                    let display = repo
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| repo.display().to_string());
                    ui.label(
                        egui::RichText::new(&display)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(100, 110, 120)),
                    );
                    if ui.small_button("+").clicked() {
                        to_readd = Some(i);
                    }
                })
                .response
                .on_hover_text(repo.display().to_string());
            }
            if let Some(i) = to_readd {
                let repo = config.untracked_repos.remove(i);
                if !config.tracked_repos.contains(&repo) {
                    config.tracked_repos.push(repo);
                }
            }
        }

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

            // Custom hex color preview swatch
            let is_preset = PRESET_COLORS.iter().any(|(_, hex)| config.accent_color == *hex);
            let trimmed = state.hex_input.trim();
            let valid_hex = trimmed.len() == 7 && trimmed.starts_with('#');
            let [cr, cg, cb] = if valid_hex {
                parse_hex_rgb(trimmed)
            } else {
                [80, 80, 80]
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

        ui.add_space(12.0);

        // --- Updates ---
        ui.label(egui::RichText::new("Updates").strong().size(14.0));
        ui.add_space(4.0);

        ui.label(
            egui::RichText::new(format!("Version: v{}", env!("CARGO_PKG_VERSION")))
                .size(12.0)
                .color(egui::Color32::from_rgb(139, 148, 158)),
        );

        ui.add_space(4.0);
        ui.checkbox(&mut config.auto_update, "Auto-update (silent)");
    });
}

fn parse_hex_rgb(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    [r, g, b]
}
