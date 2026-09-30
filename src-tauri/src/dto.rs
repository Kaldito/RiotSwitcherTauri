//! Tipos que cruzan el IPC. Todos en camelCase; `src/lib/ipc/types.ts` los refleja.

use riotswitcher_core::config::AppConfig;
use riotswitcher_core::paths::AppDirs;
use riotswitcher_core::profiles::{BackgroundRef, Profile};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDto {
    pub riot_client_location: String,
    pub selected_language: String,
    pub last_running_profile: String,
    pub show_add_profile_warning: bool,
}

impl From<&AppConfig> for ConfigDto {
    fn from(c: &AppConfig) -> Self {
        ConfigDto {
            riot_client_location: c.riot_client_location.clone(),
            selected_language: c.selected_language.clone(),
            last_running_profile: c.last_running_profile.clone(),
            show_add_profile_warning: c.show_add_profile_warning,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDto {
    pub name: String,
    pub directory_name: String,
    pub description: String,
    pub background: BackgroundRef,
    /// Ruta absoluta del fondo personalizado, para `convertFileSrc`.
    pub background_path: Option<String>,
}

impl ProfileDto {
    pub fn new(p: &Profile, dirs: &AppDirs) -> Self {
        let background_path = match &p.background {
            BackgroundRef::Custom { file } => {
                Some(dirs.backgrounds.join(file).display().to_string())
            }
            BackgroundRef::None => None,
        };
        ProfileDto {
            name: p.profile_name.clone(),
            directory_name: p.directory_name.clone(),
            description: p.description.clone(),
            background: p.background.clone(),
            background_path,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Busy,
    Running,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub phase: Phase,
    pub running_profile: Option<String>,
    pub riot_alive: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootStatus {
    pub riot_client_location: String,
    pub riot_client_valid: bool,
    /// Ubicación detectada en la ruta por defecto, si la configurada no es válida.
    pub suggested_riot_client_location: Option<String>,
    pub data_dir: String,
    pub runtime: RuntimeStatus,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwapStep {
    KillingProcesses,
    WaitingProcesses,
    SavingPreviousSession,
    SavingSession,
    RestoringSession,
    Launching,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SwapProgress {
    Step {
        step: SwapStep,
        index: u8,
        total: u8,
    },
    Warning {
        code: &'static str,
        path: Option<String>,
    },
    Done,
}
