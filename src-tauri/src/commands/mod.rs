//! Comandos IPC. Son delgados: toman el estado, llaman al core dentro de
//! `spawn_blocking` y traducen el error. Todos son `async` para no ocupar el hilo
//! principal de la ventana.

pub mod boot;
pub mod config;
pub mod profiles;
pub mod session;

use tauri::{AppHandle, Manager};

use crate::error::AppResult;
use crate::orchestrator::join_error;
use crate::state::AppState;

/// Ejecuta `f` en el pool bloqueante con acceso al estado.
pub(crate) async fn blocking<T, F>(app: &AppHandle, f: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(&AppHandle, &AppState) -> AppResult<T> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        f(&app, &state)
    })
    .await
    .map_err(join_error)?
}
