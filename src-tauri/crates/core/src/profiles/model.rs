use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Fondo de un perfil. Mismo formato en disco y por IPC:
/// `{"kind":"none"}` o `{"kind":"custom","file":"x.png"}`. Cualquier otro valor se lee
/// como `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackgroundRef {
    /// Nombre de archivo relativo a `backgrounds/`.
    Custom { file: String },
    #[default]
    #[serde(other)]
    None,
}

impl BackgroundRef {
    pub fn is_valid(&self) -> bool {
        match self {
            BackgroundRef::None => true,
            BackgroundRef::Custom { file } => {
                !file.is_empty() && !file.contains(['/', '\\']) && file != "." && file != ".."
            }
        }
    }
}

/// Un perfil tal como se guarda en `profiles_data.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub profile_name: String,
    pub directory_name: String,
    pub description: String,
    pub background: BackgroundRef,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Datos para crear un perfil. Sin nombre se genera uno automático ("Profile N").
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NewProfile {
    pub name: Option<String>,
    pub description: String,
    pub background: BackgroundRef,
}

/// Cambios parciales sobre un perfil. `None` deja el campo como está.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfilePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub background: Option<BackgroundRef>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn background_json_shape() {
        assert_eq!(
            serde_json::to_value(BackgroundRef::None).unwrap(),
            json!({"kind": "none"})
        );
        let custom: BackgroundRef =
            serde_json::from_value(json!({"kind": "custom", "file": "a_1.png"})).unwrap();
        assert_eq!(
            custom,
            BackgroundRef::Custom {
                file: "a_1.png".into()
            }
        );
    }

    #[test]
    fn unknown_background_kinds_read_as_none() {
        let bg: BackgroundRef =
            serde_json::from_value(json!({"kind": "something_else", "index": 3})).unwrap();
        assert_eq!(bg, BackgroundRef::None);
    }

    #[test]
    fn background_validation() {
        assert!(BackgroundRef::None.is_valid());
        assert!(
            !BackgroundRef::Custom {
                file: "../x.png".into()
            }
            .is_valid()
        );
        assert!(
            BackgroundRef::Custom {
                file: "x.png".into()
            }
            .is_valid()
        );
    }

    #[test]
    fn profile_keeps_unknown_keys() {
        let p: Profile = serde_json::from_value(json!({
            "profile_name": "Main",
            "mystery": true
        }))
        .unwrap();
        assert_eq!(p.profile_name, "Main");
        assert_eq!(p.background, BackgroundRef::None);
        assert_eq!(p.extra.get("mystery"), Some(&json!(true)));
        let back = serde_json::to_value(&p).unwrap();
        assert_eq!(back["mystery"], json!(true));
    }
}
