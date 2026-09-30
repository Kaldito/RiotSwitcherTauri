use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::atomic::tmp_path;
use super::retry::with_retry;
use crate::error::{IoResultExt, Result};

/// Número de archivos y bytes de un árbol.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CopyStats {
    pub files: u64,
    pub bytes: u64,
}

fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn ignore_not_found(r: io::Result<()>) -> io::Result<()> {
    match r {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Borra un archivo con reintentos. No falla si no existe.
pub fn remove_file_retry(path: &Path) -> Result<()> {
    with_retry(path, || ignore_not_found(fs::remove_file(path)))
}

/// Borra un directorio completo con reintentos. No falla si no existe.
pub fn remove_dir_all_retry(path: &Path) -> Result<()> {
    with_retry(path, || ignore_not_found(fs::remove_dir_all(path)))
}

/// Copia `src` sobre `dest`: copia a `dest.tmp` y renombra. El bloqueo al leer se
/// atribuye a `src`; el bloqueo al reemplazar, a `dest`.
pub fn copy_file_replace(src: &Path, dest: &Path) -> Result<u64> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).at(parent)?;
    }
    let tmp = tmp_path(dest);
    remove_file_retry(&tmp)?;
    let bytes = with_retry(src, || fs::copy(src, &tmp)).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    with_retry(dest, || fs::rename(&tmp, dest)).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    Ok(bytes)
}

/// Copia el árbol `src` dentro de `dest` (que se crea si falta). Con `overwrite = false`
/// los archivos que ya existen en destino se conservan. `on_file` recibe el tamaño de
/// cada archivo procesado, copiado o no.
pub fn copy_dir_recursive(
    src: &Path,
    dest: &Path,
    overwrite: bool,
    on_file: &mut dyn FnMut(u64),
) -> Result<CopyStats> {
    let mut stats = CopyStats::default();
    fs::create_dir_all(dest).at(dest)?;
    for entry in fs::read_dir(src).at(src)? {
        let entry = entry.at(src)?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let file_type = entry.file_type().at(&from)?;
        if file_type.is_dir() {
            let sub = copy_dir_recursive(&from, &to, overwrite, on_file)?;
            stats.files += sub.files;
            stats.bytes += sub.bytes;
        } else if file_type.is_file() {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if overwrite || !to.exists() {
                with_retry(&from, || fs::copy(&from, &to))?;
                stats.files += 1;
                stats.bytes += size;
            }
            on_file(size);
        } else {
            log::warn!("skipping non-regular file {}", from.display());
        }
    }
    Ok(stats)
}

/// Sustituye el directorio `dest` por una copia de `src`: copia a `dest_tmp`, borra
/// `dest` y renombra. `rename` no reemplaza directorios con contenido, por eso el borrado.
pub fn copy_dir_replace(src: &Path, dest: &Path) -> Result<CopyStats> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).at(parent)?;
    }
    let tmp = sibling_with_suffix(dest, "_tmp");
    remove_dir_all_retry(&tmp)?;
    let stats = copy_dir_recursive(src, &tmp, true, &mut |_| {}).inspect_err(|_| {
        let _ = fs::remove_dir_all(&tmp);
    })?;
    remove_dir_all_retry(dest)?;
    with_retry(dest, || fs::rename(&tmp, dest))?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CoreError;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn copy_file_replaces_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.yaml");
        let dest = tmp.path().join("out").join("a.yaml");
        write(&src, "new");
        write(&dest, "old");
        copy_file_replace(&src, &dest).unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), "new");
        assert!(!tmp_path(&dest).exists());
    }

    #[test]
    fn copy_dir_replace_drops_stale_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        write(&src.join("x.txt"), "x");
        write(&src.join("sub").join("y.txt"), "y");
        write(&dest.join("stale.txt"), "old");
        let stats = copy_dir_replace(&src, &dest).unwrap();
        assert_eq!(stats.files, 2);
        assert!(dest.join("sub").join("y.txt").exists());
        assert!(!dest.join("stale.txt").exists());
        assert!(!tmp.path().join("dest_tmp").exists());
    }

    #[test]
    fn copy_dir_recursive_can_keep_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        write(&src.join("a.txt"), "src");
        write(&src.join("b.txt"), "src");
        write(&dest.join("a.txt"), "dest");
        let mut seen = 0;
        let stats = copy_dir_recursive(&src, &dest, false, &mut |_| seen += 1).unwrap();
        assert_eq!(seen, 2);
        assert_eq!(stats.files, 1);
        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "dest");
        assert_eq!(fs::read_to_string(dest.join("b.txt")).unwrap(), "src");
    }

    #[test]
    fn remove_missing_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        remove_dir_all_retry(&tmp.path().join("nope")).unwrap();
        remove_file_retry(&tmp.path().join("nope.txt")).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn locked_source_gives_file_locked() {
        use std::os::windows::fs::OpenOptionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("locked.yaml");
        write(&src, "secret");
        let _guard = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&src)
            .unwrap();
        let err = copy_file_replace(&src, &tmp.path().join("out.yaml")).unwrap_err();
        assert!(
            matches!(err, CoreError::FileLocked(ref p) if p == &src),
            "{err:?}"
        );
    }
}
