use chrono::Datelike;
use eframe::egui;
use notify::Watcher;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use crate::config::{Config, DataMode, TimeRange, ViewMode};
use crate::heatmap::{color_for_level, grid_dates, grid_dates_range, level_for_value};
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
    last_icon_rect: Option<tray_icon::Rect>,
    focus_lost_at: Option<std::time::Instant>,
    file_picker_active: Arc<AtomicBool>,
    binary_path: Option<std::path::PathBuf>,
    binary_watcher_rx: Option<mpsc::Receiver<notify::Result<notify::Event>>>,
    _binary_watcher: Option<notify::RecommendedWatcher>,
    binary_changed_at: Option<std::time::Instant>,
    update_rx: Option<mpsc::Receiver<crate::updater::UpdateInfo>>,
    available_update: Option<crate::updater::UpdateInfo>,
    update_in_progress: bool,
    last_update_check: std::time::Instant,
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
        let file_picker_active = Arc::clone(&settings_state.file_picker_active);

        // Watch own binary for updates
        let (binary_path, binary_watcher_rx, binary_watcher) = {
            if let Ok(exe) = std::env::current_exe() {
                let exe = exe.canonicalize().unwrap_or(exe);
                let (tx, rx) = mpsc::channel();
                let mut watcher = notify::recommended_watcher(tx).ok();
                if let Some(ref mut w) = watcher {
                    if let Some(parent) = exe.parent() {
                        let _ = w.watch(parent, notify::RecursiveMode::NonRecursive);
                    }
                }
                (Some(exe), Some(rx), watcher)
            } else {
                (None, None, None)
            }
        };

        // Check for updates in background
        let (update_tx, update_rx) = mpsc::channel();
        std::thread::spawn(move || {
            if let Some(info) = crate::updater::check_for_update() {
                let _ = update_tx.send(info);
            }
        });

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
            binary_path,
            binary_watcher_rx,
            _binary_watcher: binary_watcher,
            binary_changed_at: None,
            update_rx: Some(update_rx),
            available_update: None,
            update_in_progress: false,
            last_update_check: std::time::Instant::now(),
        }
    }

    pub fn initial_scan(&mut self) {
        // Full rescan: start fresh to avoid double-counting
        self.store = CommitStore::new();
        for repo in &self.config.tracked_repos {
            // Detect identity per-repo to handle different user configs
            let identity = match scanner::detect_identity(repo) {
                Ok(id) => id,
                Err(_) => continue,
            };
            if let Ok(stats) = scanner::scan_repo(repo, &identity, None) {
                self.store.merge(stats);
            }
        }
        // Update the display identity from first repo (for settings UI)
        self.identity = self
            .config
            .tracked_repos
            .first()
            .and_then(|p| scanner::detect_identity(p).ok());
        let history_path = crate::config::data_dir().join("history.json");
        let _ = self.store.save_to(&history_path);
    }

    fn draw_header(&mut self, ui: &mut egui::Ui) {
        let dim = egui::Color32::from_rgb(100, 110, 120);
        let bright = egui::Color32::from_rgb(230, 237, 243);
        let year_active = self.config.view_mode == ViewMode::Year;
        let year_color = if year_active { bright } else { dim };

        ui.horizontal(|ui| {
            ui.heading("gitmap");

            ui.add_space(16.0);

            if ui.small_button("\u{25C0}").clicked() {
                self.config.selected_year -= 1;
                self.config.view_mode = ViewMode::Year;
            }
            let year_label = ui.label(
                egui::RichText::new(format!("{}", self.config.selected_year))
                    .strong()
                    .size(16.0)
                    .color(year_color),
            );
            if year_label.interact(egui::Sense::click()).clicked() {
                self.config.selected_year = chrono::Local::now().naive_local().date().year();
                self.config.view_mode = ViewMode::Year;
            }
            if ui.small_button("\u{25B6}").clicked() {
                self.config.selected_year += 1;
                self.config.view_mode = ViewMode::Year;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("time_range")
                    .selected_text(self.config.time_range.label())
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for &range in TimeRange::all() {
                            if ui.selectable_value(
                                &mut self.config.time_range,
                                range,
                                range.label(),
                            ).clicked() {
                                self.config.view_mode = ViewMode::Rolling;
                            }
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
        let weeks = match self.config.view_mode {
            ViewMode::Year => grid_dates(self.config.selected_year),
            ViewMode::Rolling => {
                let today = chrono::Local::now().naive_local().date();
                let start = today - chrono::Duration::days(self.config.time_range.days());
                grid_dates_range(start, today)
            }
        };
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

    fn draw_stats(&self, ui: &mut egui::Ui) {
        let stats = self.store.stats();
        let today = chrono::Local::now().naive_local().date();

        let (range_start, range_end) = match self.config.view_mode {
            ViewMode::Year => {
                let year = self.config.selected_year;
                (
                    chrono::NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
                    chrono::NaiveDate::from_ymd_opt(year, 12, 31).unwrap(),
                )
            }
            ViewMode::Rolling => {
                let start = today - chrono::Duration::days(self.config.time_range.days());
                (start, today)
            }
        };

        let mut total_commits: u32 = 0;
        let mut total_insertions: u32 = 0;
        let mut total_deletions: u32 = 0;
        let mut active_days: u32 = 0;
        let mut current_streak: u32 = 0;
        let mut longest_streak: u32 = 0;

        for (date, day) in stats {
            if *date >= range_start && *date <= range_end {
                total_commits += day.commits;
                total_insertions += day.insertions;
                total_deletions += day.deletions;
                if day.commits > 0 {
                    active_days += 1;
                }
            }
        }

        // Calculate current streak (consecutive days ending today or yesterday)
        let mut check_date = today;
        loop {
            if let Some(day) = stats.get(&check_date) {
                if day.commits > 0 {
                    current_streak += 1;
                    check_date -= chrono::Duration::days(1);
                    continue;
                }
            }
            if check_date == today && current_streak == 0 {
                check_date -= chrono::Duration::days(1);
                continue;
            }
            break;
        }

        // Calculate longest streak in the visible range
        let mut streak: u32 = 0;
        let mut d = range_start;
        while d <= range_end {
            if stats.get(&d).map(|s| s.commits > 0).unwrap_or(false) {
                streak += 1;
                longest_streak = longest_streak.max(streak);
            } else {
                streak = 0;
            }
            d += chrono::Duration::days(1);
        }

        let dim = egui::Color32::from_rgb(139, 148, 158);

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}", total_commits)).strong().size(13.0));
            ui.label(egui::RichText::new("commits").size(11.0).color(dim));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("+{}", total_insertions)).size(11.0).color(egui::Color32::from_rgb(63, 185, 80)));
            ui.label(egui::RichText::new(format!("-{}", total_deletions)).size(11.0).color(egui::Color32::from_rgb(248, 81, 73)));
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}", active_days)).strong().size(13.0));
            ui.label(egui::RichText::new("active days").size(11.0).color(dim));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("{}", current_streak)).strong().size(13.0));
            ui.label(egui::RichText::new("day streak").size(11.0).color(dim));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("{}", longest_streak)).strong().size(13.0));
            ui.label(egui::RichText::new("longest").size(11.0).color(dim));
        });
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
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Match our dark background: rgb(13, 17, 23)
        [13.0 / 255.0, 17.0 / 255.0, 23.0 / 255.0, 1.0]
    }

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
            {
                let since = self.store.most_recent_date();
                for repo_path in &changed_repos {
                    // Detect identity per-repo
                    let identity = match scanner::detect_identity(repo_path) {
                        Ok(id) => id,
                        Err(_) => continue,
                    };
                    if let Ok(stats) = scanner::scan_repo(repo_path, &identity, since) {
                        self.store.merge(stats);
                    }
                }
                let history_path = crate::config::data_dir().join("history.json");
                let _ = self.store.save_to(&history_path);
            }
        }

        // Check for binary update (auto-relaunch)
        if let Some(ref rx) = self.binary_watcher_rx {
            while let Ok(Ok(event)) = rx.try_recv() {
                if let Some(ref exe) = self.binary_path {
                    if event.paths.iter().any(|p| p == exe) {
                        if self.binary_changed_at.is_none() {
                            self.binary_changed_at = Some(std::time::Instant::now());
                        }
                    }
                }
            }
        }

        // Debounced relaunch after binary change (500ms for copy to finish)
        if let Some(changed_at) = self.binary_changed_at {
            if changed_at.elapsed() >= std::time::Duration::from_millis(500) {
                let _ = self.config.save();
                let history_path = crate::config::data_dir().join("history.json");
                let _ = self.store.save_to(&history_path);

                if let Some(ref exe) = self.binary_path {
                    let _ = std::process::Command::new(exe).spawn();
                }

                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        // Poll for update check result
        if let Some(ref rx) = self.update_rx {
            if let Ok(info) = rx.try_recv() {
                if self.config.auto_update {
                    self.update_in_progress = true;
                    let url = info.download_url.clone();
                    let ctx_clone = ctx.clone();
                    std::thread::spawn(move || {
                        if crate::updater::download_and_install(&url).is_ok() {
                            let _ = std::process::Command::new(
                                "/Applications/GitMap.app/Contents/MacOS/gitmap",
                            )
                            .spawn();
                            ctx_clone.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                } else {
                    self.available_update = Some(info);
                }
            }
        }

        // Periodic update check (every 6 hours)
        if self.available_update.is_none()
            && !self.update_in_progress
            && self.last_update_check.elapsed() >= std::time::Duration::from_secs(6 * 60 * 60)
        {
            self.last_update_check = std::time::Instant::now();
            let (tx, rx) = mpsc::channel();
            self.update_rx = Some(rx);
            std::thread::spawn(move || {
                if let Some(info) = crate::updater::check_for_update() {
                    let _ = tx.send(info);
                }
            });
        }

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

        while let Ok(msg) = self.tray_rx.try_recv() {
            match msg {
                TrayMessage::ToggleWindow { icon_rect } => {
                    self.last_icon_rect = Some(icon_rect);
                    self.focus_lost_at = None;
                    self.visible = !self.visible;
                    if self.visible {
                        // Position under the tray icon
                        // tray-icon gives physical pixels, egui expects logical points
                        if let Some(ref rect) = self.last_icon_rect {
                            let scale = ctx.pixels_per_point() as f64;
                            let icon_center_x =
                                (rect.position.x + rect.size.width as f64 / 2.0) / scale;
                            let popover_width = 420.0_f64;
                            let x = icon_center_x - (popover_width / 2.0);
                            let y = (rect.position.y + rect.size.height as f64) / scale;

                            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                                egui::pos2(x as f32, y as f32),
                            ));
                        }
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    } else {
                        // Hide by moving off-screen
                        self.show_settings = false;
                        let _ = self.config.save();
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                            egui::pos2(-10000.0, -10000.0),
                        ));
                    }
                }
                TrayMessage::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        // Debounced click-outside-to-hide
        if let Some(lost_at) = self.focus_lost_at {
            if lost_at.elapsed() >= std::time::Duration::from_millis(150)
                && !self.file_picker_active.load(Ordering::Relaxed)
            {
                self.visible = false;
                self.show_settings = false;
                self.focus_lost_at = None;
                let _ = self.config.save();
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                    egui::pos2(-10000.0, -10000.0),
                ));
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // Always render content (window lives off-screen when hidden)
        let frame = egui::Frame::new()
            .fill(egui::Color32::from_rgb(13, 17, 23))
            .inner_margin(16.0)
            .corner_radius(12.0);

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.visuals_mut().override_text_color =
                Some(egui::Color32::from_rgb(230, 237, 243));

            if self.show_settings {
                if ui.button("\u{25C0} Back").clicked() {
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

                ui.horizontal(|ui| {
                    if ui.button("\u{2699} Settings").clicked() {
                        self.show_settings = true;
                    }
                    ui.label(
                        egui::RichText::new(format!("{} repos", self.config.tracked_repos.len()))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(100, 110, 120)),
                    );
                });

                self.draw_stats(ui);

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
            }
        });
    }
}
