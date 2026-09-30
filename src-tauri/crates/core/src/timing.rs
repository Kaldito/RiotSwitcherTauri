//! Tiempos de espera del cambio de perfil, ajustados de forma empírica. Todos viven aquí
//! para poder ajustarlos en un solo sitio.

use std::time::Duration;

/// Espera tras confirmar que los procesos han muerto, antes de tocar archivos.
pub const AFTER_KILL_SETTLE: Duration = Duration::from_millis(250);
/// Espera tras guardar la sesión anterior, antes de restaurar la nueva.
pub const AFTER_SAVE_SETTLE: Duration = Duration::from_millis(100);
/// Espera tras restaurar la sesión, antes de lanzar el cliente.
pub const AFTER_RESTORE_SETTLE: Duration = Duration::from_millis(150);

/// Intervalo de sondeo mientras se espera a que mueran los procesos.
pub const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(250);
/// Cada cuánto se repite el kill mientras quedan procesos vivos.
pub const PROCESS_REKILL_INTERVAL: Duration = Duration::from_secs(2);
/// Tiempo máximo de espera a que mueran los procesos.
pub const PROCESS_KILL_TIMEOUT: Duration = Duration::from_secs(6);

/// Esperas entre reintentos sobre archivos bloqueados (total ~2,5 s, tope de 1 s).
pub const RETRY_BACKOFF: [Duration; 6] = [
    Duration::from_millis(50),
    Duration::from_millis(100),
    Duration::from_millis(200),
    Duration::from_millis(400),
    Duration::from_millis(800),
    Duration::from_millis(1000),
];
