use std::path::Path;

use super::model::BackgroundRef;
use super::sanitize::{resolve_unique, sanitize_directory_name};
use crate::error::{CoreError, Result};
use crate::fsutil::{copy_file_replace, write_bytes_atomic};
use crate::paths::AppDirs;

pub const ALLOWED_IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

/// Copia una imagen elegida por el usuario a `backgrounds/<nombre>_<unix>.<ext>`.
pub fn import_background_image(
    dirs: &AppDirs,
    source: &Path,
    name_hint: &str,
    now_unix: u64,
) -> Result<BackgroundRef> {
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| ALLOWED_IMAGE_EXTENSIONS.contains(&e.as_str()))
        .ok_or_else(|| CoreError::invalid("the background must be a PNG, JPG or WEBP image"))?;
    let meta = std::fs::metadata(source)
        .map_err(|_| CoreError::invalid("the selected image does not exist"))?;
    if !meta.is_file() {
        return Err(CoreError::invalid("the selected image is not a file"));
    }
    if meta.len() > MAX_IMAGE_BYTES {
        return Err(CoreError::invalid("the image is larger than 20 MB"));
    }

    let file = unique_file_name(dirs, name_hint, &ext, now_unix);
    copy_file_replace(source, &dirs.backgrounds.join(&file))?;
    Ok(BackgroundRef::Custom { file })
}

/// Guarda el icono de invocador descargado del cliente de League (JPG) en `backgrounds/`.
pub fn save_league_icon(
    dirs: &AppDirs,
    bytes: &[u8],
    name_hint: &str,
    now_unix: u64,
) -> Result<BackgroundRef> {
    let file = unique_file_name(dirs, name_hint, "jpg", now_unix);
    write_bytes_atomic(&dirs.backgrounds.join(&file), bytes)?;
    Ok(BackgroundRef::Custom { file })
}

/// `<nombre>_<unix>.<ext>` que todavía no existe en `backgrounds/`.
fn unique_file_name(dirs: &AppDirs, name_hint: &str, ext: &str, now_unix: u64) -> String {
    let hint = if name_hint.trim().is_empty() {
        "background"
    } else {
        name_hint
    };
    let stem = format!("{}_{now_unix}", sanitize_directory_name(hint, now_unix));
    let stem = resolve_unique(&stem, |c| {
        dirs.backgrounds.join(format!("{c}.{ext}")).exists()
    });
    format!("{stem}.{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_image_into_backgrounds() {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = AppDirs::new(tmp.path().join("app"));
        dirs.ensure().unwrap();
        let src = tmp.path().join("Pic.PNG");
        std::fs::write(&src, b"png").unwrap();
        let bg = import_background_image(&dirs, &src, "My Main", 100).unwrap();
        assert_eq!(
            bg,
            BackgroundRef::Custom {
                file: "My_Main_100.png".into()
            }
        );
        let again = import_background_image(&dirs, &src, "My Main", 100).unwrap();
        assert_eq!(
            again,
            BackgroundRef::Custom {
                file: "My_Main_100_2.png".into()
            }
        );
        assert!(dirs.backgrounds.join("My_Main_100.png").exists());
    }

    #[test]
    fn rejects_other_extensions() {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = AppDirs::new(tmp.path());
        let src = tmp.path().join("x.gif");
        std::fs::write(&src, b"gif").unwrap();
        let err = import_background_image(&dirs, &src, "x", 1).unwrap_err();
        assert_eq!(err.code(), "invalid_input");
    }
}
