use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use gitmap::config::Config;
use gitmap::store::CommitStore;
use gitmap::ui::popover::{GitMapApp, TrayMessage};

fn main() -> eframe::Result<()> {
    let config = Config::load();

    let history_path = gitmap::config::data_dir().join("history.json");
    let store = CommitStore::load_from(&history_path).unwrap_or_else(|_| CommitStore::new());

    let (tray_tx, tray_rx) = mpsc::channel::<TrayMessage>();

    // Generate a simple 22x22 black icon (template icon for macOS)
    let mut icon_rgba = Vec::with_capacity(22 * 22 * 4);
    for _ in 0..(22 * 22) {
        icon_rgba.extend_from_slice(&[0, 0, 0, 255]);
    }
    let icon =
        tray_icon::Icon::from_rgba(icon_rgba, 22, 22).expect("Failed to create tray icon");

    let menu = tray_icon::menu::Menu::new();
    let quit_item = tray_icon::menu::MenuItem::new("Quit", true, None);
    menu.append(&quit_item).unwrap();
    let quit_id = quit_item.id().clone();

    // Set up tray event handlers
    let tx_click = tray_tx.clone();
    tray_icon::TrayIconEvent::set_event_handler(Some(
        move |event: tray_icon::TrayIconEvent| {
            if let tray_icon::TrayIconEvent::Click {
                rect,
                button_state,
                ..
            } = event
            {
                if matches!(button_state, tray_icon::MouseButtonState::Up) {
                    let _ = tx_click.send(TrayMessage::ToggleWindow { icon_rect: rect });
                }
            }
        },
    ));

    let tx_menu = tray_tx;
    tray_icon::menu::MenuEvent::set_event_handler(Some(
        move |event: tray_icon::menu::MenuEvent| {
            if event.id() == &quit_id {
                let _ = tx_menu.send(TrayMessage::Quit);
            }
        },
    ));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([420.0, 500.0])
            .with_position([-10000.0_f32, -10000.0_f32])
            .with_always_on_top()
            .with_resizable(false)
            .with_has_shadow(true)
            .with_title_shown(false)
            .with_titlebar_shown(false),
        ..Default::default()
    };

    let tray_holder: Rc<RefCell<Option<tray_icon::TrayIcon>>> = Rc::new(RefCell::new(None));
    let tray_holder_clone = tray_holder.clone();

    eframe::run_native(
        "GitMap",
        options,
        Box::new(move |_cc| {
            // Set macOS activation policy to Accessory (no Dock icon, no menu bar)
            #[cfg(target_os = "macos")]
            {
                use objc2::MainThreadMarker;
                use objc2_app_kit::NSApplication;
                use objc2_app_kit::NSApplicationActivationPolicy;
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let app = NSApplication::sharedApplication(mtm);
                app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            }

            let tray = tray_icon::TrayIconBuilder::new()
                .with_icon(icon)
                .with_icon_as_template(true)
                .with_tooltip("GitMap")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .unwrap();
            tray_holder_clone.borrow_mut().replace(tray);

            let mut app = GitMapApp::new(tray_rx, config, store);
            app.initial_scan();
            Ok(Box::new(app))
        }),
    )
}
