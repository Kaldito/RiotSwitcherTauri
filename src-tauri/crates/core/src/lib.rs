//! Lógica de RiotSwitcher sin dependencias de Tauri: rutas, utilidades de archivos,
//! configuración, perfiles, intercambio de sesión, procesos y lanzador. Todo es
//! síncrono; la app lo ejecuta dentro de `spawn_blocking`.

pub mod config;
pub mod error;
pub mod fsutil;
pub mod launcher;
pub mod paths;
pub mod processes;
pub mod profiles;
pub mod session;
pub mod timing;

pub use error::{CoreError, IoResultExt, Result};

use std::time::{SystemTime, UNIX_EPOCH};

/// Segundos desde la época Unix según el reloj del sistema.
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
