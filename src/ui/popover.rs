use eframe::egui;
use std::sync::mpsc;

use crate::config::Config;
use crate::store::CommitStore;

#[derive(Debug)]
pub enum TrayMessage {
    ToggleWindow { icon_rect: tray_icon::Rect },
    Quit,
}

pub struct GitMapApp {
    tray_rx: mpsc::Receiver<TrayMessage>,
    visible: bool,
    config: Config,
    store: CommitStore,
}

impl GitMapApp {
    pub fn new(tray_rx: mpsc::Receiver<TrayMessage>, config: Config, store: CommitStore) -> Self {
        Self {
            tray_rx,
            visible: false,
            config,
            store,
        }
    }
}

impl eframe::App for GitMapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll tray messages
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
            ui.heading("gitmap");
            ui.add_space(8.0);
            ui.label("Heatmap will render here.");
        });
    }
}
