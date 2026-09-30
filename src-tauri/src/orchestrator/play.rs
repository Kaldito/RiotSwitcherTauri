use std::path::Path;
use std::thread::sleep;

use riotswitcher_core::paths::LivePaths;
use riotswitcher_core::processes::{self, RIOT_PROCESS_NAMES, RealClock, SysinfoProbe};
use riotswitcher_core::timing::{AFTER_KILL_SETTLE, AFTER_RESTORE_SETTLE, AFTER_SAVE_SETTLE};
use riotswitcher_core::{CoreError, launcher, session};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use super::{Progress, join_error};
use crate::dto::{RuntimeStatus, SwapProgress, SwapStep};
use crate::error::AppResult;
use crate::events;
use crate::state::{AppState, read};

/// Play: cerrar procesos, guardar la sesión del perfil anterior, restaurar la del nuevo
/// y lanzar el cliente.
pub async fn play(
    app: AppHandle,
    name: String,
    channel: Channel<SwapProgress>,
) -> AppResult<RuntimeStatus> {
    let guard = app.state::<AppState>().try_busy()?;
    events::emit_runtime(&app);
    let task_app = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || run(&task_app, &name, &channel))
        .await
        .map_err(join_error)
        .and_then(|r| r);
    drop(guard);
    events::emit_profiles(&app);
    events::emit_runtime(&app);
    result.map(|()| app.state::<AppState>().runtime_status())
}

fn run(app: &AppHandle, name: &str, channel: &Channel<SwapProgress>) -> AppResult<()> {
    let state = app.state::<AppState>();
    let live = state.live_paths()?;
    let (target_dir, previous_dir) = {
        let db = read(&state.profiles);
        let target = db
            .get(name)
            .ok_or_else(|| CoreError::ProfileNotFound(name.to_string()))?;
        let target_dir = state.dirs.profile_dir_checked(&target.directory_name)?;
        let previous_dir = state
            .running_profile()
            .and_then(|prev| db.get(&prev).map(|p| p.directory_name.clone()))
            .and_then(|dir| state.dirs.profile_dir_checked(&dir).ok());
        (target_dir, previous_dir)
    };

    let total = if previous_dir.is_some() { 5 } else { 4 };
    let mut progress = Progress::new(channel, total);
    match swap_and_launch(&live, &target_dir, previous_dir.as_deref(), &mut progress) {
        Ok(pid) => {
            log::debug!("Riot Client launched with pid {pid}");
            state.set_running_profile(Some(name.to_string()));
            state.persist_last_running(name);
            progress.done();
            Ok(())
        }
        Err(e) => {
            log::error!("play failed: {e}");
            state.set_running_profile(None);
            state.persist_last_running("");
            Err(e.into())
        }
    }
}

fn swap_and_launch(
    live: &LivePaths,
    target_dir: &Path,
    previous_dir: Option<&Path>,
    progress: &mut Progress,
) -> Result<u32, CoreError> {
    let mut probe = SysinfoProbe::new();
    progress.step(SwapStep::KillingProcesses);
    processes::kill_all(&mut probe, &RIOT_PROCESS_NAMES);

    progress.step(SwapStep::WaitingProcesses);
    let outcome =
        processes::wait_until_all_dead(&mut probe, &RIOT_PROCESS_NAMES, &mut RealClock::new());
    if !outcome.all_dead {
        progress.warn("processes_still_running", None);
    }
    sleep(AFTER_KILL_SETTLE);

    // Se guarda también si el perfil anterior es el mismo: así un "volver a jugar" no
    // pierde los tokens que el cliente haya renovado desde el último guardado.
    if let Some(prev) = previous_dir {
        progress.step(SwapStep::SavingPreviousSession);
        let report = session::save_session(live, prev)?;
        for locked in &report.locked {
            progress.warn("file_locked", Some(&locked.path));
        }
        sleep(AFTER_SAVE_SETTLE);
    }

    progress.step(SwapStep::RestoringSession);
    session::restore_session(live, target_dir)?;
    sleep(AFTER_RESTORE_SETTLE);

    progress.step(SwapStep::Launching);
    launcher::launch(&live.riot_client_location)
}
