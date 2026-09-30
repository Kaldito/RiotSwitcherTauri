use std::thread::sleep;

use riotswitcher_core::processes::{self, RIOT_PROCESS_NAMES, RealClock, SysinfoProbe};
use riotswitcher_core::session;
use riotswitcher_core::timing::{AFTER_KILL_SETTLE, AFTER_SAVE_SETTLE};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use super::{Progress, join_error};
use crate::dto::{RuntimeStatus, SwapProgress, SwapStep};
use crate::error::AppResult;
use crate::events;
use crate::state::{AppState, read};

/// Stop: cerrar procesos, esperar y guardar la sesión del perfil en ejecución.
pub async fn stop(app: AppHandle, channel: Channel<SwapProgress>) -> AppResult<RuntimeStatus> {
    let guard = app.state::<AppState>().try_busy()?;
    events::emit_runtime(&app);
    let task_app = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || run(&task_app, &channel))
        .await
        .map_err(join_error)
        .and_then(|r| r);
    drop(guard);
    events::emit_runtime(&app);
    result.map(|()| app.state::<AppState>().runtime_status())
}

fn run(app: &AppHandle, channel: &Channel<SwapProgress>) -> AppResult<()> {
    let state = app.state::<AppState>();
    let mut progress = Progress::new(channel, 3);
    let Some(name) = state.running_profile() else {
        progress.done();
        return Ok(());
    };
    let profile_dir = read(&state.profiles)
        .get(&name)
        .map(|p| p.directory_name.clone())
        .and_then(|dir| state.dirs.profile_dir_checked(&dir).ok());

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

    progress.step(SwapStep::SavingSession);
    let saved = match (state.live_paths(), profile_dir) {
        (Ok(live), Some(dir)) => session::save_session(&live, &dir).map(|report| {
            for locked in &report.locked {
                progress.warn("file_locked", Some(&locked.path));
            }
        }),
        (Err(e), _) => Err(e),
        (Ok(_), None) => Ok(()),
    };
    sleep(AFTER_SAVE_SETTLE);

    // El cliente ya está cerrado: pase lo que pase con el guardado, no queda nada en marcha.
    state.set_running_profile(None);
    state.persist_last_running("");
    match saved {
        Ok(()) => {
            progress.done();
            Ok(())
        }
        Err(e) => {
            log::error!("stop could not save the session: {e}");
            Err(e.into())
        }
    }
}
