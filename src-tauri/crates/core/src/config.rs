use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::Result;
use crate::fsutil::{read_json_lenient, write_json_atomic};

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// Idiomas con traducción disponible.
pub const SUPPORTED_LANGUAGES: [&str; 2] = ["en_US", "es_ES"];

/// `data/config.json`. Las claves desconocidas se conservan en `extra`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub schema_version: u32,
    pub riot_client_location: String,
    /// Vacío hasta que el usuario elige; el frontend usa entonces el idioma del sistema.
    pub selected_language: String,
    /// `profile_name` del perfil en ejecución; se conserva al salir para readoptarlo.
    pub last_running_profile: String,
    pub show_add_profile_warning: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            riot_client_location: String::new(),
            selected_language: String::new(),
            last_running_profile: String::new(),
            show_add_profile_warning: true,
            extra: Map::new(),
        }
    }
}

/// Cambios parciales enviados por el frontend. `None` deja el campo como está.
/// `riot_client_location` y `last_running_profile` tienen su propio camino y no están aquí.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigPatch {
    pub selected_language: Option<String>,
    pub show_add_profile_warning: Option<bool>,
}

impl ConfigPatch {
    pub fn validate(&self) -> Result<()> {
        if let Some(lang) = &self.selected_language
            && !lang.is_empty()
            && !SUPPORTED_LANGUAGES.contains(&lang.as_str())
        {
            return Err(crate::CoreError::invalid(format!(
                "unsupported language {lang:?}"
            )));
        }
        Ok(())
    }

    pub fn apply(&self, cfg: &mut AppConfig) {
        if let Some(v) = &self.selected_language {
            cfg.selected_language = v.clone();
        }
        if let Some(v) = self.show_add_profile_warning {
            cfg.show_add_profile_warning = v;
        }
    }
}

/// Lectura y escritura de `data/config.json`.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        ConfigStore { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Carga la configuración. Si falta o está corrupta devuelve la de por defecto.
    pub fn load(&self) -> Result<AppConfig> {
        let value: Option<Value> = read_json_lenient(&self.path)?;
        let Some(value) = value else {
            return Ok(AppConfig::default());
        };
        match serde_json::from_value::<AppConfig>(value) {
            Ok(mut cfg) => {
                cfg.schema_version = CONFIG_SCHEMA_VERSION;
                Ok(cfg)
            }
            Err(e) => {
                log::warn!("config has unexpected types ({e}); using defaults");
                Ok(AppConfig::default())
            }
        }
    }

    pub fn save(&self, cfg: &AppConfig) -> Result<()> {
        write_json_atomic(&self.path, cfg)
    }

    /// Aplica `change` sobre una copia, la guarda y sólo entonces actualiza `cfg`.
    pub fn update(&self, cfg: &mut AppConfig, change: impl FnOnce(&mut AppConfig)) -> Result<()> {
        let mut next = cfg.clone();
        change(&mut next);
        self.save(&next)?;
        *cfg = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_file_gives_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(tmp.path().join("config.json"));
        let cfg = store.load().unwrap();
        assert_eq!(cfg, AppConfig::default());
        assert!(cfg.show_add_profile_warning);
    }

    #[test]
    fn roundtrip_preserves_unknown_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&json!({
                "schema_version": 1,
                "riot_client_location": "C:\\Riot Games\\Riot Client",
                "future_key": {"nested": [1, 2]}
            }))
            .unwrap(),
        )
        .unwrap();
        let store = ConfigStore::new(&path);
        let mut cfg = store.load().unwrap();
        assert_eq!(cfg.riot_client_location, "C:\\Riot Games\\Riot Client");
        assert!(cfg.show_add_profile_warning, "missing fields take defaults");
        store
            .update(&mut cfg, |c| c.selected_language = "es_ES".into())
            .unwrap();
        let raw: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(raw["future_key"], json!({"nested": [1, 2]}));
        assert_eq!(raw["selected_language"], "es_ES");
        assert_eq!(store.load().unwrap(), cfg);
    }

    #[test]
    fn corrupt_file_resets_to_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.json");
        std::fs::write(&path, b"{{{").unwrap();
        let cfg = ConfigStore::new(&path).load().unwrap();
        assert_eq!(cfg, AppConfig::default());
        assert!(tmp.path().join("config.json.corrupt.bak").exists());
    }

    #[test]
    fn patch_applies_only_given_fields() {
        let mut cfg = AppConfig::default();
        let patch: ConfigPatch =
            serde_json::from_value(json!({"selectedLanguage": "es_ES"})).unwrap();
        patch.validate().unwrap();
        patch.apply(&mut cfg);
        assert_eq!(cfg.selected_language, "es_ES");
        assert!(cfg.show_add_profile_warning);
    }

    #[test]
    fn patch_rejects_unknown_language() {
        let patch = ConfigPatch {
            selected_language: Some("xx_XX".into()),
            ..Default::default()
        };
        assert!(patch.validate().is_err());
    }
}
