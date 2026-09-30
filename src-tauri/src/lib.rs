//! App Tauri de RiotSwitcher: comandos, estado, eventos, bandeja y orquestación.
//! La lógica vive en `riotswitcher-core`.

mod commands;
mod dto;
mod error;
mod events;
mod orchestrator;
mod state;
mod tray;

use std::sync::atomic::Ordering;

use tauri::{Manager, WindowEvent};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

use crate::state::{AppState, read};

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::LogDir {
                file_name: Some("riotswitcher".into()),
            }),
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::Webview),
        ])
        .level(level)
        .level_for("tao", log::LevelFilter::Warn)
        .level_for("wry", log::LevelFilter::Warn)
        .max_file_size(2_000_000)
        .rotation_strategy(RotationStrategy::KeepSome(5))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Debe ser el primer plugin: una segunda instancia sólo enfoca la primera.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(log_plugin())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            log::info!("RiotSwitcher {} starting", app.package_info().version);
            let root = app.path().app_local_data_dir()?;
            let state = AppState::init(root)?;
            let language = read(&state.config).selected_language.clone();
            app.manage(state);
            tray::create(app.handle(), &language)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if !state.exiting.load(Ordering::SeqCst) {
                    api.prevent_close();
                    orchestrator::quit::request_quit(window.app_handle().clone());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::boot::get_boot_status,
            commands::boot::set_riot_client_location,
            commands::config::get_config,
            commands::config::update_config,
            commands::profiles::list_profiles,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::delete_profile,
            commands::profiles::reorder_profiles,
            commands::profiles::import_background_image,
            commands::session::play_profile,
            commands::session::stop_profile,
            commands::session::get_runtime_status,
            commands::session::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the RiotSwitcher application");
}
