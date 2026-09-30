use std::path::{Path, PathBuf};

use crate::error::{CoreError, IoResultExt, Result};

pub const CONFIG_FILE: &str = "config.json";
pub const PROFILES_FILE: &str = "profiles_data.json";
pub const RIOT_CLIENT_EXE: &str = "RiotClientServices.exe";
pub const DEFAULT_RIOT_CLIENT_LOCATION: &str = r"C:\Riot Games\Riot Client";

/// Carpetas de datos de la app bajo `%LOCALAPPDATA%\<identifier>\`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDirs {
    pub root: PathBuf,
    pub data: PathBuf,
    pub profiles: PathBuf,
    pub backgrounds: PathBuf,
}

impl AppDirs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        AppDirs {
            data: root.join("data"),
            profiles: root.join("profiles"),
            backgrounds: root.join("backgrounds"),
            root,
        }
    }

    /// Crea todas las carpetas si no existen.
    pub fn ensure(&self) -> Result<()> {
        for dir in [&self.root, &self.data, &self.profiles, &self.backgrounds] {
            std::fs::create_dir_all(dir).at(dir)?;
        }
        Ok(())
    }

    pub fn config_file(&self) -> PathBuf {
        self.data.join(CONFIG_FILE)
    }

    pub fn profiles_file(&self) -> PathBuf {
        self.data.join(PROFILES_FILE)
    }

    pub fn profile_dir(&self, directory_name: &str) -> PathBuf {
        self.profiles.join(directory_name)
    }

    /// Como `profile_dir`, pero rechaza nombres que escaparían de `profiles/` o lo
    /// señalarían entero (vacío, `.`, `..`, separadores, unidad).
    pub fn profile_dir_checked(&self, directory_name: &str) -> Result<PathBuf> {
        if Self::is_valid_dir_name(directory_name) {
            Ok(self.profiles.join(directory_name))
        } else {
            Err(CoreError::invalid(format!(
                "invalid profile folder name {directory_name:?}"
            )))
        }
    }

    pub fn is_valid_dir_name(name: &str) -> bool {
        !name.trim().is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':'])
    }
}

/// Rutas "vivas" del Riot Client que se intercambian en cada cambio de perfil.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivePaths {
    /// `%LOCALAPPDATA%\Riot Games\Riot Client`
    pub riot_local_data: PathBuf,
    /// Carpeta de instalación que contiene `RiotClientServices.exe`.
    pub riot_client_location: PathBuf,
}

impl LivePaths {
    pub fn new(
        riot_local_data: impl Into<PathBuf>,
        riot_client_location: impl Into<PathBuf>,
    ) -> Self {
        LivePaths {
            riot_local_data: riot_local_data.into(),
            riot_client_location: riot_client_location.into(),
        }
    }

    /// `%LOCALAPPDATA%\Riot Games\Riot Client`, si la variable existe.
    pub fn default_riot_local_data() -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("Riot Games").join("Riot Client"))
    }
}

/// Normaliza una ruta de carpeta al formato de Windows (`\`) y sin separador final.
pub fn normalize_dir_string(raw: &str) -> String {
    let replaced = raw.trim().replace('/', "\\");
    let trimmed = replaced.trim_end_matches('\\');
    // "C:" solo no es una carpeta útil; se conserva la barra de la raíz.
    if trimmed.len() == 2 && trimmed.ends_with(':') {
        format!("{trimmed}\\")
    } else {
        trimmed.to_string()
    }
}

/// Comprueba que `dir` contiene `RiotClientServices.exe`.
pub fn validate_riot_client_location(dir: &Path) -> Result<()> {
    if dir.as_os_str().is_empty() {
        return Err(CoreError::RiotClientNotConfigured);
    }
    if dir.join(RIOT_CLIENT_EXE).is_file() {
        Ok(())
    } else {
        Err(CoreError::RiotClientNotFound(dir.to_path_buf()))
    }
}

/// Devuelve la ubicación por defecto del Riot Client si es válida.
pub fn autodetect_riot_client_location() -> Option<PathBuf> {
    let candidate = PathBuf::from(DEFAULT_RIOT_CLIENT_LOCATION);
    validate_riot_client_location(&candidate)
        .ok()
        .map(|_| candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_dir_strings() {
        assert_eq!(
            normalize_dir_string("C:/Riot Games/Riot Client/"),
            r"C:\Riot Games\Riot Client"
        );
        assert_eq!(normalize_dir_string(r"D:\Games\Riot\\"), r"D:\Games\Riot");
        assert_eq!(normalize_dir_string("E:/"), r"E:\");
        assert_eq!(normalize_dir_string("  "), "");
    }

    #[test]
    fn validates_riot_client_location() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            validate_riot_client_location(Path::new("")),
            Err(CoreError::RiotClientNotConfigured)
        ));
        assert!(matches!(
            validate_riot_client_location(tmp.path()),
            Err(CoreError::RiotClientNotFound(_))
        ));
        std::fs::write(tmp.path().join(RIOT_CLIENT_EXE), b"").unwrap();
        assert!(validate_riot_client_location(tmp.path()).is_ok());
    }

    #[test]
    fn app_dirs_layout() {
        let dirs = AppDirs::new(r"C:\x");
        assert_eq!(dirs.config_file(), PathBuf::from(r"C:\x\data\config.json"));
        assert_eq!(
            dirs.profiles_file(),
            PathBuf::from(r"C:\x\data\profiles_data.json")
        );
        assert_eq!(
            dirs.profile_dir("Main"),
            PathBuf::from(r"C:\x\profiles\Main")
        );
    }

    #[test]
    fn rejects_escaping_profile_dirs() {
        let dirs = AppDirs::new(r"C:\x");
        for bad in ["", " ", ".", "..", r"..\y", "a/b", "C:"] {
            assert!(dirs.profile_dir_checked(bad).is_err(), "{bad:?}");
        }
        assert!(dirs.profile_dir_checked("Main_2").is_ok());
    }
}
