use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::retry::with_retry;
use crate::error::{CoreError, IoResultExt, Result};

/// `ruta.tmp` junto al destino (mismo volumen, para que `rename` sea atómico).
pub fn tmp_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}

fn corrupt_backup_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".corrupt.bak");
    PathBuf::from(s)
}

/// Escribe `bytes` en `ruta.tmp` y lo renombra sobre `ruta`. En Windows `rename`
/// reemplaza el archivo existente, así que el destino nunca queda a medias.
pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).at(parent)?;
    }
    let tmp = tmp_path(path);
    with_retry(&tmp, || {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()
    })?;
    if let Err(e) = with_retry(path, || fs::rename(&tmp, path)) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

/// Serializa con sangría y escribe de forma atómica.
pub fn write_json_atomic<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| CoreError::json(path, e))?;
    write_bytes_atomic(path, &bytes)
}

/// Lee un JSON tolerando los fallos habituales:
/// - si falta `ruta` pero existe `ruta.tmp` (rename interrumpido), lo recupera;
/// - si falta todo, devuelve `None`;
/// - si el contenido no es válido, lo aparta a `ruta.corrupt.bak` y devuelve `None`.
pub fn read_json_lenient<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        let tmp = tmp_path(path);
        if tmp.is_file() {
            log::warn!("recovering {} from its .tmp copy", path.display());
            with_retry(path, || fs::rename(&tmp, path))?;
        } else {
            return Ok(None);
        }
    }

    let bytes = match with_retry(path, || fs::read(path)) {
        Ok(b) => b,
        Err(CoreError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    let content = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);

    match serde_json::from_slice::<T>(content) {
        Ok(value) => Ok(Some(value)),
        Err(err) => {
            let backup = corrupt_backup_path(path);
            log::warn!(
                "{} is not valid ({err}); moving it to {}",
                path.display(),
                backup.display()
            );
            with_retry(&backup, || fs::copy(path, &backup).map(|_| ()))?;
            let _ = fs::remove_file(path);
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn write_then_read_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("data").join("x.json");
        write_json_atomic(&path, &json!({"a": 1})).unwrap();
        let back: Value = read_json_lenient(&path).unwrap().unwrap();
        assert_eq!(back, json!({"a": 1}));
        assert!(!tmp_path(&path).exists());
    }

    #[test]
    fn overwrites_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("x.json");
        write_json_atomic(&path, &json!({"v": 1})).unwrap();
        write_json_atomic(&path, &json!({"v": 2})).unwrap();
        let back: Value = read_json_lenient(&path).unwrap().unwrap();
        assert_eq!(back["v"], 2);
    }

    #[test]
    fn missing_file_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let out: Option<Value> = read_json_lenient(&tmp.path().join("nope.json")).unwrap();
        assert!(out.is_none());
    }

    #[test]
    fn recovers_from_tmp() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("x.json");
        fs::write(tmp_path(&path), br#"{"ok": true}"#).unwrap();
        let back: Value = read_json_lenient(&path).unwrap().unwrap();
        assert_eq!(back["ok"], true);
        assert!(path.exists());
        assert!(!tmp_path(&path).exists());
    }

    #[test]
    fn corrupt_file_is_backed_up() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("x.json");
        fs::write(&path, b"{ not json").unwrap();
        let out: Option<Value> = read_json_lenient(&path).unwrap();
        assert!(out.is_none());
        assert!(!path.exists());
        let backup = tmp.path().join("x.json.corrupt.bak");
        assert_eq!(fs::read(backup).unwrap(), b"{ not json");
    }

    #[test]
    fn accepts_utf8_bom() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("x.json");
        fs::write(&path, b"\xEF\xBB\xBF{\"a\": 2}").unwrap();
        let back: Value = read_json_lenient(&path).unwrap().unwrap();
        assert_eq!(back["a"], 2);
    }
}
