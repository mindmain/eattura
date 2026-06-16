use serde::Serialize;

/// Error type for Tauri commands.
///
/// Tauri commands must return `Result<T, String>` or a type that implements
/// `Into<tauri::InvokeError>`. We use a custom error that serializes to JSON
/// for structured error handling on the frontend.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl From<sqlx::Error> for CommandError {
    fn from(e: sqlx::Error) -> Self {
        CommandError {
            code: "DB_ERROR".to_string(),
            message: e.to_string(),
        }
    }
}

impl From<common::xml::XmlError> for CommandError {
    fn from(e: common::xml::XmlError) -> Self {
        CommandError {
            code: "XML_ERROR".to_string(),
            message: e.to_string(),
        }
    }
}

/// Convert CommandError to String for Tauri IPC compatibility.
impl From<CommandError> for String {
    fn from(e: CommandError) -> String {
        serde_json::to_string(&e).unwrap_or_else(|_| e.message)
    }
}
