//! Icono de bandeja. Se crea sólo en código: declararlo también en
//! `tauri.conf.json` produce dos iconos.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::orchestrator::quit::request_quit;

const QUIT_ID: &str = "quit";

fn quit_label(language: &str) -> &'static str {
    match language {
        "es_ES" => "Salir",
        _ => "Exit",
    }
}

pub fn create(app: &AppHandle, language: &str) -> tauri::Result<()> {
    let quit = MenuItem::with_id(app, QUIT_ID, quit_label(language), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("RiotSwitcher")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == QUIT_ID {
                request_quit(app.clone());
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Muestra, restaura y enfoca la ventana principal.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
