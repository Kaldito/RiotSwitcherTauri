use std::sync::atomic::Ordering;

use riotswitcher_core::session;
use tauri::{AppHandle, Manager};

use crate::state::{AppState, read};

/// Lanza QuitFlow en segundo plano. Lo usan la X de la ventana y la bandeja.
pub fn request_quit(app: AppHandle) {
    tauri::async_runtime::spawn(quit(app));
}

/// QuitFlow: espera a que termine cualquier Play o Stop, guarda la sesión del
/// perfil en ejecución **sin matar el cliente** y sale. `last_running_profile` se
/// conserva para readoptar el perfil en el siguiente arranque.
pub async fn quit(app: AppHandle) {
    let state = app.state::<AppState>();
    if state.quitting.swap(true, Ordering::SeqCst) {
        return;
    }
    log::info!("quitting");
    let guard = state.busy.clone().lock_owned().await;
    let task_app = app.clone();
    if let Err(e) = tauri::async_runtime::spawn_blocking(move || save_running(&task_app)).await {
        log::error!("quit task failed: {e}");
    }
    drop(guard);
    state.exiting.store(true, Ordering::SeqCst);
    app.exit(0);
}

fn save_running(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(name) = state.running_profile() else {
        return;
    };
    let dir = read(&state.profiles)
        .get(&name)
        .map(|p| p.directory_name.clone())
        .and_then(|d| state.dirs.profile_dir_checked(&d).ok());
    let (Ok(live), Some(dir)) = (state.live_paths(), dir) else {
        log::warn!("cannot save the running session on quit: missing paths");
        return;
    };
    match session::save_session(&live, &dir) {
        Ok(report) if report.locked.is_empty() => log::info!("session saved on quit"),
        Ok(report) => log::warn!(
            "session saved on quit with {} locked item(s) kept from the previous copy",
            report.locked.len()
        ),
        Err(e) => log::error!("could not save the session on quit: {e}"),
    }
}
