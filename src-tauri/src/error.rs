use riotswitcher_core::CoreError;
use serde::Serialize;
use serde_json::{Value, json};

pub type AppResult<T> = Result<T, AppError>;

/// Error que recibe el frontend al rechazarse la promesa de `invoke`. `code` es estable
/// y se traduce como `error.<code>`; `message` es el texto en inglés de respaldo.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl AppError {
    pub fn busy() -> Self {
        AppError {
            code: "busy",
            message: "another operation is in progress".into(),
            details: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        AppError {
            code: "internal",
            message: message.into(),
            details: None,
        }
    }
}

impl From<CoreError> for AppError {
    fn from(e: CoreError) -> Self {
        AppError {
            code: e.code(),
            message: e.to_string(),
            details: e.path().map(|p| json!({ "path": p.display().to_string() })),
        }
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::internal(e.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
