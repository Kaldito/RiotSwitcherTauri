use std::io;
use std::path::{Path, PathBuf};

pub type Result<T, E = CoreError> = std::result::Result<T, E>;

/// Errores del core. Cada variante tiene un código estable (`code()`) que el frontend
/// traduce como `error.<code>`.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("the Riot Client location is not configured")]
    RiotClientNotConfigured,

    #[error("RiotClientServices.exe not found in {}", .0.display())]
    RiotClientNotFound(PathBuf),

    #[error("profile not found: {0}")]
    ProfileNotFound(String),

    #[error("a profile named {0:?} already exists")]
    ProfileNameTaken(String),

    #[error("profile {0:?} is running")]
    ProfileRunning(String),

    #[error("file is locked: {}", .0.display())]
    FileLocked(PathBuf),

    #[error("could not restore session item {item}: {source}")]
    SessionRestoreFailed {
        item: &'static str,
        #[source]
        source: Box<CoreError>,
    },

    #[error("Riot processes still running: {}", .0.join(", "))]
    ProcessesStillRunning(Vec<String>),

    #[error("could not launch the Riot Client: {0}")]
    LaunchFailed(#[source] io::Error),

    #[error("the League client is not running")]
    LeagueClientNotRunning,

    #[error("the League client request failed: {0}")]
    LeagueClientRequestFailed(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("I/O error on {}: {source}", .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("JSON error on {}: {source}", .path.display())]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl CoreError {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        CoreError::Io {
            path: path.into(),
            source,
        }
    }

    pub fn json(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        CoreError::Json {
            path: path.into(),
            source,
        }
    }

    pub fn invalid(msg: impl Into<String>) -> Self {
        CoreError::InvalidInput(msg.into())
    }

    /// Código estable para el frontend.
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::RiotClientNotConfigured => "riot_client_not_configured",
            CoreError::RiotClientNotFound(_) => "riot_client_not_found",
            CoreError::ProfileNotFound(_) => "profile_not_found",
            CoreError::ProfileNameTaken(_) => "profile_name_taken",
            CoreError::ProfileRunning(_) => "profile_running",
            CoreError::FileLocked(_) => "file_locked",
            CoreError::SessionRestoreFailed { .. } => "session_restore_failed",
            CoreError::ProcessesStillRunning(_) => "processes_still_running",
            CoreError::LaunchFailed(_) => "launch_failed",
            CoreError::LeagueClientNotRunning => "league_client_not_running",
            CoreError::LeagueClientRequestFailed(_) => "league_client_request_failed",
            CoreError::InvalidInput(_) => "invalid_input",
            CoreError::Io { .. } | CoreError::Json { .. } => "io",
        }
    }

    /// Ruta implicada en el error, si la hay.
    pub fn path(&self) -> Option<&Path> {
        match self {
            CoreError::RiotClientNotFound(p) | CoreError::FileLocked(p) => Some(p),
            CoreError::Io { path, .. } | CoreError::Json { path, .. } => Some(path),
            CoreError::SessionRestoreFailed { source, .. } => source.path(),
            _ => None,
        }
    }
}

/// Adjunta la ruta a un `io::Result`.
pub trait IoResultExt<T> {
    fn at(self, path: &Path) -> Result<T>;
}

impl<T> IoResultExt<T> for io::Result<T> {
    fn at(self, path: &Path) -> Result<T> {
        self.map_err(|e| CoreError::io(path, e))
    }
}
