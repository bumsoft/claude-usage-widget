//! Tauri shell for the Claude Usage widget.
//!
//! Thin layer over `claude_usage_core`: registers commands, builds the tray,
//! and keeps the frameless window out of the taskbar (hide-to-tray on close).

mod commands;
mod config;
mod discovery;

use std::time::Duration;

use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

/// Shared application state. The HTTP client is reused across requests.
pub struct AppState {
    pub http: reqwest::Client,
}

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("claude-usage-widget")
        .build()
        .expect("failed to build HTTP client")
}

/// Toggle the main window's visibility (used by tray click and menu).
fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            http: build_client(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::discover_sources,
            commands::fetch_usage,
            commands::read_stats,
            commands::get_config,
            commands::set_config,
        ])
        .on_window_event(|window, event| {
            // Closing hides to the tray so polling keeps running in the background.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            let show = MenuItemBuilder::with_id("show", "Show / Hide").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            // Embed the icon directly so the tray never depends on a configured
            // window icon being present. The app retains the tray for its lifetime.
            let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))?;
            let _tray = TrayIconBuilder::with_id("main")
                .icon(icon)
                .tooltip("Claude Usage")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => toggle_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Claude Usage widget");
}
