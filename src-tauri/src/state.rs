use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use riotswitcher_core::config::{AppConfig, ConfigStore};
use riotswitcher_core::paths::{AppDirs, LivePaths, validate_riot_client_location};
use riotswitcher_core::processes::{self, READOPT_PROCESS_NAMES, RIOT_PROCESS_NAMES, SysinfoProbe};
use riotswitcher_core::profiles::ProfileDb;
use riotswitcher_core::{CoreError, Result as CoreResult};

use crate::dto::{ConfigDto, Phase, ProfileDto, RuntimeStatus};
use crate::error::{AppError, AppResult};

/// Perfil cuya sesión está ahora en las rutas vivas del Riot Client.
#[derive(Debug, Clone, Default)]
pub struct RuntimeState {
    pub running_profile: Option<String>,
}

/// Estado compartido de la app. Los `std` locks se sueltan antes de cualquier
/// `.await`; `busy` es de tokio porque se mantiene durante todo Play, Stop o Quit.
pub struct AppState {
    pub dirs: AppDirs,
    pub config_store: ConfigStore,
    pub config: RwLock<AppConfig>,
    pub profiles: RwLock<ProfileDb>,
    pub runtime: Mutex<RuntimeState>,
    pub busy: Arc<tokio::sync::Mutex<()>>,
    /// QuitFlow en marcha: las siguientes peticiones de cierre no hacen nada.
    pub quitting: AtomicBool,
    /// QuitFlow terminado: el cierre de la ventana ya no se intercepta.
    pub exiting: AtomicBool,
}

pub fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

pub fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl AppState {
    /// Prepara las carpetas, carga configuración y perfiles y readopta el perfil que
    /// seguía en ejecución al cerrar la app.
    pub fn init(root: PathBuf) -> CoreResult<Self> {
        let dirs = AppDirs::new(root);
        dirs.ensure()?;
        let config_store = ConfigStore::new(dirs.config_file());
        let mut config = config_store.load()?;
        let profiles = ProfileDb::load(&dirs.profiles_file())?;
        let runtime = readopt(&mut config, &config_store, &profiles);
        log::info!(
            "state loaded: {} profile(s), running profile: {}",
            profiles.profiles.len(),
            runtime.running_profile.is_some()
        );
        Ok(AppState {
            dirs,
            config_store,
            config: RwLock::new(config),
            profiles: RwLock::new(profiles),
            runtime: Mutex::new(runtime),
            busy: Arc::new(tokio::sync::Mutex::new(())),
            quitting: AtomicBool::new(false),
            exiting: AtomicBool::new(false),
        })
    }

    /// Toma la guardia de ocupado o devuelve `busy` de inmediato.
    pub fn try_busy(&self) -> AppResult<tokio::sync::OwnedMutexGuard<()>> {
        self.busy
            .clone()
            .try_lock_owned()
            .map_err(|_| AppError::busy())
    }

    pub fn running_profile(&self) -> Option<String> {
        lock(&self.runtime).running_profile.clone()
    }

    /// Rutas vivas con la ubicación del Riot Client configurada y validada.
    pub fn live_paths(&self) -> CoreResult<LivePaths> {
        let location = PathBuf::from(&read(&self.config).riot_client_location);
        validate_riot_client_location(&location)?;
        let local = LivePaths::default_riot_local_data().ok_or_else(|| {
            CoreError::invalid("the LOCALAPPDATA environment variable is not set")
        })?;
        Ok(LivePaths::new(local, location))
    }

    pub fn set_running_profile(&self, running_profile: Option<String>) {
        lock(&self.runtime).running_profile = running_profile;
    }

    /// Persiste `last_running_profile`; un fallo sólo se registra.
    pub fn persist_last_running(&self, name: &str) {
        let mut cfg = write(&self.config);
        if cfg.last_running_profile == name {
            return;
        }
        if let Err(e) = self
            .config_store
            .update(&mut cfg, |c| c.last_running_profile = name.to_string())
        {
            log::error!("could not persist the running profile: {e}");
        }
    }

    pub fn runtime_status(&self) -> RuntimeStatus {
        let running_profile = self.running_profile();
        let busy = self.busy.try_lock().is_err();
        let phase = if busy {
            Phase::Busy
        } else if running_profile.is_some() {
            Phase::Running
        } else {
            Phase::Idle
        };
        let riot_alive = processes::is_any_alive(&mut SysinfoProbe::new(), &RIOT_PROCESS_NAMES);
        RuntimeStatus {
            phase,
            running_profile,
            riot_alive,
        }
    }

    pub fn config_dto(&self) -> ConfigDto {
        ConfigDto::from(&*read(&self.config))
    }

    pub fn profile_dtos(&self) -> Vec<ProfileDto> {
        read(&self.profiles)
            .profiles
            .iter()
            .map(|p| ProfileDto::new(p, &self.dirs))
            .collect()
    }
}

fn readopt(config: &mut AppConfig, store: &ConfigStore, profiles: &ProfileDb) -> RuntimeState {
    let last = config.last_running_profile.clone();
    if last.is_empty() {
        return RuntimeState::default();
    }
    if profiles.get(&last).is_some()
        && processes::is_any_alive(&mut SysinfoProbe::new(), &READOPT_PROCESS_NAMES)
    {
        log::info!("readopting the profile that was running before the app closed");
        return RuntimeState {
            running_profile: Some(last),
        };
    }
    log::info!("the previously running profile is no longer running");
    if let Err(e) = store.update(config, |c| c.last_running_profile.clear()) {
        log::error!("could not clear the running profile: {e}");
    }
    RuntimeState::default()
}
