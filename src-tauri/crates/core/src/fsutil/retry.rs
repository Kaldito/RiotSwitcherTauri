use std::io;
use std::path::Path;
use std::time::Duration;

use crate::error::{CoreError, Result};
use crate::timing::RETRY_BACKOFF;

const ERROR_SHARING_VIOLATION: i32 = 32;
const ERROR_LOCK_VIOLATION: i32 = 33;

/// Errores que indican que otro proceso retiene el archivo y merece la pena reintentar.
pub fn is_lock_error(e: &io::Error) -> bool {
    matches!(
        e.raw_os_error(),
        Some(ERROR_SHARING_VIOLATION) | Some(ERROR_LOCK_VIOLATION)
    ) || e.kind() == io::ErrorKind::PermissionDenied
}

/// Ejecuta `op` reintentando con el backoff estándar mientras el error sea de bloqueo.
/// Agotados los reintentos devuelve `CoreError::FileLocked(path)`.
pub fn with_retry<T>(path: &Path, op: impl FnMut() -> io::Result<T>) -> Result<T> {
    with_retry_backoff(path, &RETRY_BACKOFF, op)
}

pub fn with_retry_backoff<T>(
    path: &Path,
    backoff: &[Duration],
    mut op: impl FnMut() -> io::Result<T>,
) -> Result<T> {
    let mut attempt = 0;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(e) if is_lock_error(&e) => match backoff.get(attempt) {
                Some(wait) => {
                    log::debug!("locked, retrying in {wait:?}: {}", path.display());
                    std::thread::sleep(*wait);
                    attempt += 1;
                }
                None => return Err(CoreError::FileLocked(path.to_path_buf())),
            },
            Err(e) => return Err(CoreError::io(path, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn classifies_lock_errors() {
        assert!(is_lock_error(&io::Error::from_raw_os_error(32)));
        assert!(is_lock_error(&io::Error::from_raw_os_error(33)));
        assert!(is_lock_error(&io::Error::from(
            io::ErrorKind::PermissionDenied
        )));
        assert!(!is_lock_error(&io::Error::from(io::ErrorKind::NotFound)));
    }

    #[test]
    fn retries_then_succeeds() {
        let calls = Cell::new(0);
        let backoff = [Duration::from_millis(1); 3];
        let out = with_retry_backoff(Path::new("x"), &backoff, || {
            calls.set(calls.get() + 1);
            if calls.get() < 3 {
                Err(io::Error::from_raw_os_error(33))
            } else {
                Ok(7)
            }
        });
        assert_eq!(out.unwrap(), 7);
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn gives_up_with_file_locked() {
        let calls = Cell::new(0);
        let backoff = [Duration::from_millis(1); 2];
        let out: Result<()> = with_retry_backoff(Path::new("locked.yaml"), &backoff, || {
            calls.set(calls.get() + 1);
            Err(io::Error::from_raw_os_error(32))
        });
        assert!(matches!(out, Err(CoreError::FileLocked(_))));
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn other_errors_are_not_retried() {
        let calls = Cell::new(0);
        let out: Result<()> = with_retry(Path::new("missing"), || {
            calls.set(calls.get() + 1);
            Err(io::Error::from(io::ErrorKind::NotFound))
        });
        assert!(matches!(out, Err(CoreError::Io { .. })));
        assert_eq!(calls.get(), 1);
    }
}
