use std::path::PathBuf;

use riotswitcher_core::{lcu, now_unix};
use riotswitcher_core::profiles::{self as core_profiles, BackgroundRef, NewProfile, ProfilePatch};
use tauri::{AppHandle, Manager};

use super::blocking;
use crate::dto::ProfileDto;
use crate::error::AppResult;
use crate::events;
use crate::state::{AppState, write};

#[tauri::command]
pub async fn list_profiles(app: AppHandle) -> AppResult<Vec<ProfileDto>> {
    Ok(app.state::<AppState>().profile_dtos())
}

#[tauri::command]
pub async fn create_profile(app: AppHandle, input: NewProfile) -> AppResult<ProfileDto> {
    let dto = blocking(&app, move |_, state| {
        let profile = write(&state.profiles).create(&state.dirs, input, now_unix())?;
        Ok(ProfileDto::new(&profile, &state.dirs))
    })
    .await?;
    events::emit_profiles(&app);
    Ok(dto)
}

#[tauri::command]
pub async fn update_profile(
    app: AppHandle,
    name: String,
    patch: ProfilePatch,
) -> AppResult<ProfileDto> {
    let _guard = app.state::<AppState>().try_busy()?;
    let dto = blocking(&app, move |_, state| {
        let running = state.running_profile();
        let profile = write(&state.profiles).update(
            &state.dirs,
            &name,
            patch,
            running.as_deref(),
            now_unix(),
        )?;
        Ok(ProfileDto::new(&profile, &state.dirs))
    })
    .await?;
    events::emit_profiles(&app);
    Ok(dto)
}

#[tauri::command]
pub async fn delete_profile(app: AppHandle, name: String) -> AppResult<Vec<ProfileDto>> {
    let _guard = app.state::<AppState>().try_busy()?;
    let list = blocking(&app, move |_, state| {
        let running = state.running_profile();
        write(&state.profiles).delete(&state.dirs, &name, running.as_deref())?;
        Ok(state.profile_dtos())
    })
    .await?;
    events::emit_profiles(&app);
    Ok(list)
}

#[tauri::command]
pub async fn reorder_profiles(app: AppHandle, ordered: Vec<String>) -> AppResult<Vec<ProfileDto>> {
    let list = blocking(&app, move |_, state| {
        write(&state.profiles).reorder(&state.dirs, &ordered)?;
        Ok(state.profile_dtos())
    })
    .await?;
    events::emit_profiles(&app);
    Ok(list)
}

/// Copia una imagen elegida con el diálogo a `backgrounds/` y devuelve la referencia que
/// luego se pasa a `create_profile` o `update_profile`.
#[tauri::command]
pub async fn import_background_image(
    app: AppHandle,
    source_path: String,
    name_hint: Option<String>,
) -> AppResult<BackgroundRef> {
    blocking(&app, move |_, state| {
        let hint = name_hint.unwrap_or_default();
        Ok(core_profiles::import_background_image(
            &state.dirs,
            &PathBuf::from(source_path),
            &hint,
            now_unix(),
        )?)
    })
    .await
}

/// Descarga el icono de invocador de la cuenta abierta en el cliente de League, lo guarda
/// en `backgrounds/` y devuelve la referencia, igual que `import_background_image`.
#[tauri::command]
pub async fn import_league_icon(
    app: AppHandle,
    name_hint: Option<String>,
) -> AppResult<BackgroundRef> {
    blocking(&app, move |_, state| {
        let icon = lcu::fetch_profile_icon()?;
        let hint = name_hint.unwrap_or_default();
        Ok(core_profiles::save_league_icon(
            &state.dirs,
            &icon,
            &hint,
            now_unix(),
        )?)
    })
    .await
}
