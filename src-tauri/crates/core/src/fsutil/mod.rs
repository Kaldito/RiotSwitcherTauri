//! Utilidades de archivos: escritura atómica de JSON, copias con reemplazo y reintentos
//! ante bloqueos.

mod atomic;
mod copy;
mod retry;

pub use atomic::{read_json_lenient, tmp_path, write_bytes_atomic, write_json_atomic};
pub use copy::{
    CopyStats, copy_dir_recursive, copy_dir_replace, copy_file_replace, remove_dir_all_retry,
    remove_file_retry,
};
pub use retry::{is_lock_error, with_retry, with_retry_backoff};
