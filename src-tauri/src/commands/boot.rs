use std::path::PathBuf;

use riotswitcher_core::CoreError;
use riotswitcher_core::paths::{
    autodetect_riot_client_location, normalize_dir_string, validate_riot_client_location,
};
use tauri::AppHandle;

use super::blocking;
use crate::dto::{BootStatus, ConfigDto};
use crate::error::AppResult;
use crate::events;
use crate::state::{read, write};

#[tauri::command]
pub async fn get_boot_status(app: AppHandle) -> AppResult<BootStatus> {
    blocking(&app, |_, state| {
        let location = read(&state.config).riot_client_location.clone();
        let valid = validate_riot_client_location(&PathBuf::from(&location)).is_ok();
        let suggested = if valid {
            None
        } else {
            autodetect_riot_client_location().map(|p| p.display().to_string())
        };
        Ok(BootStatus {
            riot_client_location: location,
            riot_client_valid: valid,
            suggested_riot_client_location: suggested,
            data_dir: state.dirs.root.display().to_string(),
            runtime: state.runtime_status(),
        })
    })
    .await
}

/// Fija la carpeta del Riot Client tras comprobar que contiene `RiotClientServices.exe`.
#[tauri::command]
pub async fn set_riot_client_location(app: AppHandle, path: String) -> AppResult<ConfigDto> {
    let dto = blocking(&app, move |_, state| {
        if let Some(running) = state.running_profile() {
            return Err(CoreError::ProfileRunning(running).into());
        }
        let normalized = normalize_dir_string(&path);
        validate_riot_client_location(&PathBuf::from(&normalized))?;
        let mut cfg = write(&state.config);
        state
            .config_store
            .update(&mut cfg, |c| c.riot_client_location = normalized)?;
        Ok(ConfigDto::from(&*cfg))
    })
    .await?;
    events::emit_config(&app);
    Ok(dto)
}
