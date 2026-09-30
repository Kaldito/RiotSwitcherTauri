//! Eventos emitidos al frontend. Los comandos devuelven además el estado nuevo,
//! así que la UI nunca depende sólo de ellos.

use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

pub const RUNTIME_STATUS_CHANGED: &str = "runtime-status-changed";
pub const PROFILES_CHANGED: &str = "profiles-changed";
pub const CONFIG_CHANGED: &str = "config-changed";

pub fn emit_runtime(app: &AppHandle) {
    let status = app.state::<AppState>().runtime_status();
    if let Err(e) = app.emit(RUNTIME_STATUS_CHANGED, status) {
        log::warn!("could not emit {RUNTIME_STATUS_CHANGED}: {e}");
    }
}

pub fn emit_profiles(app: &AppHandle) {
    let profiles = app.state::<AppState>().profile_dtos();
    if let Err(e) = app.emit(PROFILES_CHANGED, profiles) {
        log::warn!("could not emit {PROFILES_CHANGED}: {e}");
    }
}

pub fn emit_config(app: &AppHandle) {
    let config = app.state::<AppState>().config_dto();
    if let Err(e) = app.emit(CONFIG_CHANGED, config) {
        log::warn!("could not emit {CONFIG_CHANGED}: {e}");
    }
}
