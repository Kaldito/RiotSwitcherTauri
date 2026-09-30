use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use crate::dto::{RuntimeStatus, SwapProgress};
use crate::error::AppResult;
use crate::orchestrator::{play, quit, stop};
use crate::state::AppState;

#[tauri::command]
pub async fn play_profile(
    app: AppHandle,
    name: String,
    on_progress: Channel<SwapProgress>,
) -> AppResult<RuntimeStatus> {
    play::play(app, name, on_progress).await
}

#[tauri::command]
pub async fn stop_profile(
    app: AppHandle,
    on_progress: Channel<SwapProgress>,
) -> AppResult<RuntimeStatus> {
    stop::stop(app, on_progress).await
}

#[tauri::command]
pub async fn get_runtime_status(app: AppHandle) -> AppResult<RuntimeStatus> {
    let task_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || task_app.state::<AppState>().runtime_status())
        .await
        .map_err(crate::orchestrator::join_error)
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> AppResult<()> {
    quit::quit(app).await;
    Ok(())
}
