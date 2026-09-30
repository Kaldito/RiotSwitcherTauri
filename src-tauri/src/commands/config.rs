use riotswitcher_core::config::ConfigPatch;
use tauri::{AppHandle, Manager};

use super::blocking;
use crate::dto::ConfigDto;
use crate::error::AppResult;
use crate::events;
use crate::state::{AppState, write};

#[tauri::command]
pub async fn get_config(app: AppHandle) -> AppResult<ConfigDto> {
    Ok(app.state::<AppState>().config_dto())
}

#[tauri::command]
pub async fn update_config(app: AppHandle, patch: ConfigPatch) -> AppResult<ConfigDto> {
    let dto = blocking(&app, move |_, state| {
        patch.validate()?;
        let mut cfg = write(&state.config);
        state.config_store.update(&mut cfg, |c| patch.apply(c))?;
        Ok(ConfigDto::from(&*cfg))
    })
    .await?;
    events::emit_config(&app);
    Ok(dto)
}
