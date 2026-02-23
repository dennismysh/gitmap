use eframe::egui;
use std::sync::mpsc;

use crate::config::{Config, DataMode, TimeRange};
use crate::heatmap::{color_for_level, grid_dates, level_for_value};
use crate::scanner::{self, GitIdentity};
use crate::store::CommitStore;
use crate::ui::settings::{self, SettingsState};
use crate::watcher::RepoWatcher;

#[derive(Debug)]
pub enum TrayMessage {
    ToggleWindow { icon_rect: tray_icon::Rect },
    Quit,
}

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
}

impl GitMapApp {
    pub fn new(tray_rx: mpsc::Receiver<TrayMessage>, config: Config, store: CommitStore) -> Self {
        let identity = config
            .tracked_repos
            .first()
            .and_then(|p| scanner::detect_identity(p).ok());

        let mut watcher = RepoWatcher::new().ok();
        if let Some(ref mut w) = watcher {
            for repo in &config.tracked_repos {
                let _ = w.watch_repo(repo);
            }
        }

        let settings_state = SettingsState::new(&config);
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
        }
    }

    pub fn initial_scan(&mut self) {
        let identity = match &self.identity {
            Some(id) => id.clone(),
            None => return,
        };
        let since = self.store.most_recent_date();
        for repo in &self.config.tracked_repos {
            if let Ok(stats) = scanner::scan_repo(repo, &identity, since) {
                self.store.merge(stats);
            }
        }
        let history_path = crate::config::data_dir().join("history.json");
        let _ = self.store.save_to(&history_path);
    }

    fn draw_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("gitmap");

            ui.add_space(16.0);

            if ui.small_button("\u{25C0}").clicked() {
                self.config.selected_year -= 1;
            }
            ui.label(
                egui::RichText::new(format!("{}", self.config.selected_year))
                    .strong()
                    .size(16.0),
            );
            if ui.small_button("\u{25B6}").clicked() {
                self.config.selected_year += 1;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("time_range")
                    .selected_text(self.config.time_range.label())
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for &range in TimeRange::all() {
                            ui.selectable_value(
                                &mut self.config.time_range,
                                range,
                                range.label(),
                            );
                        }
                    });

                egui::ComboBox::from_id_salt("data_mode")
                    .selected_text(self.config.data_mode.label())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.config.data_mode,
                            DataMode::Commits,
                            DataMode::Commits.label(),
                        );
                        ui.selectable_value(
                            &mut self.config.data_mode,
                            DataMode::LinesChanged,
                            DataMode::LinesChanged.label(),
                        );
                    });
            });
        });
    }

    fn draw_heatmap(&mut self, ui: &mut egui::Ui) {
        let weeks = grid_dates(self.config.selected_year);
        let cell_size = 14.0_f32;
        let cell_spacing = 3.0_f32;
        let label_width = 30.0_f32;

        let max_value = self
            .store
            .stats()
            .values()
            .map(|s| match self.config.data_mode {
                DataMode::Commits => s.commits,
                DataMode::LinesChanged => s.insertions + s.deletions,
            })
            .max()
            .unwrap_or(1)
            .max(1);

        let day_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

        let total_width = label_width + weeks.len() as f32 * (cell_size + cell_spacing);
        let total_height = 7.0 * (cell_size + cell_spacing);

        let (response, painter) =
            ui.allocate_painter(egui::vec2(total_width, total_height), egui::Sense::hover());

        let origin = response.rect.min;
        let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
        self.hovered_info = None;

        // Draw day labels
        for (row, label) in day_labels.iter().enumerate() {
            let y = origin.y + row as f32 * (cell_size + cell_spacing) + cell_size / 2.0;
            painter.text(
                egui::pos2(origin.x, y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(139, 148, 158),
            );
        }

        // Draw cells
        for (col, week) in weeks.iter().enumerate() {
            for (row, &date) in week.iter().enumerate() {
                let x = origin.x + label_width + col as f32 * (cell_size + cell_spacing);
                let y = origin.y + row as f32 * (cell_size + cell_spacing);

                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(cell_size, cell_size),
                );

                let stats = self.store.get(date);
                let value = stats
                    .map(|s| match self.config.data_mode {
                        DataMode::Commits => s.commits,
                        DataMode::LinesChanged => s.insertions + s.deletions,
                    })
                    .unwrap_or(0);

                let level = level_for_value(value, max_value);
                let [r, g, b, a] = color_for_level(level, &self.config.accent_color);

                painter.rect_filled(
                    cell_rect,
                    egui::CornerRadius::same(3),
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a),
                );

                if let Some(pos) = pointer_pos {
                    if cell_rect.contains(pos) {
                        painter.rect_stroke(
                            cell_rect,
                            egui::CornerRadius::same(3),
                            egui::Stroke::new(1.5, egui::Color32::WHITE),
                            egui::StrokeKind::Outside,
                        );
                        let info = if let Some(s) = stats {
                            format!(
                                "{}: {} commits, +{} -{} lines",
                                date.format("%b %d, %Y"),
                                s.commits,
                                s.insertions,
                                s.deletions
                            )
                        } else {
                            format!("{}: No commits", date.format("%b %d, %Y"))
                        };
                        self.hovered_info = Some(info);
                    }
                }
            }
        }
    }

    fn draw_legend(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Less")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(139, 148, 158)),
            );
            for level in 0..=4 {
                let [r, g, b, a] = color_for_level(level, &self.config.accent_color);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(2),
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a),
                );
            }
            ui.label(
                egui::RichText::new("More")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(139, 148, 158)),
            );
        });
    }
}

impl eframe::App for GitMapApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.config.save();
        let history_path = crate::config::data_dir().join("history.json");
        let _ = self.store.save_to(&history_path);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll file watcher for changed repos
        let changed_repos = self
            .watcher
            .as_ref()
            .map(|w| w.poll_changed_repos())
            .unwrap_or_default();

        if !changed_repos.is_empty() {
            if let Some(ref identity) = self.identity {
                let identity = identity.clone();
                let since = self.store.most_recent_date();
                for repo_path in &changed_repos {
                    if let Ok(stats) = scanner::scan_repo(repo_path, &identity, since) {
                        self.store.merge(stats);
                    }
                }
                let history_path = crate::config::data_dir().join("history.json");
                let _ = self.store.save_to(&history_path);
            }
        }

        while let Ok(msg) = self.tray_rx.try_recv() {
            match msg {
                TrayMessage::ToggleWindow { icon_rect } => {
                    self.visible = !self.visible;
                    if self.visible {
                        let icon_center_x =
                            icon_rect.position.x + (icon_rect.size.width as f64 / 2.0);
                        let popover_width = 420.0_f64;
                        let x = icon_center_x - (popover_width / 2.0);
                        let y = icon_rect.position.y + icon_rect.size.height as f64;

                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                            egui::pos2(x as f32, y as f32),
                        ));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                }
                TrayMessage::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // Auto-hide when the window loses focus
        if self.visible {
            let has_focus = ctx.input(|i| i.focused);
            if !has_focus {
                self.visible = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
        }

        if !self.visible {
            return;
        }

        let frame = egui::Frame::new()
            .fill(egui::Color32::from_rgb(13, 17, 23))
            .inner_margin(16.0)
            .corner_radius(12.0);

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.visuals_mut().override_text_color =
                Some(egui::Color32::from_rgb(230, 237, 243));

            if self.show_settings {
                if ui.button("\u{2190} Back").clicked() {
                    self.show_settings = false;
                    let _ = self.config.save();

                    // Re-detect identity
                    self.identity = self
                        .config
                        .tracked_repos
                        .first()
                        .and_then(|p| scanner::detect_identity(p).ok());

                    // Update watchers for any new repos
                    if let Some(ref mut w) = self.watcher {
                        for repo in &self.config.tracked_repos {
                            let _ = w.watch_repo(repo);
                        }
                    }

                    // Rescan
                    self.initial_scan();
                }
                ui.add_space(8.0);
                settings::draw_settings(ui, &mut self.config, &mut self.settings_state);
            } else {
                self.draw_header(ui);
                ui.add_space(12.0);

                egui::ScrollArea::horizontal().show(ui, |ui| {
                    self.draw_heatmap(ui);
                });

                ui.add_space(8.0);
                self.draw_legend(ui);

                ui.add_space(4.0);
                if let Some(ref info) = self.hovered_info {
                    ui.label(
                        egui::RichText::new(info)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(139, 148, 158)),
                    );
                } else {
                    ui.label(egui::RichText::new(" ").size(12.0));
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                if ui.button("\u{2699} Settings").clicked() {
                    self.show_settings = true;
                }
            }
        });
    }
}
