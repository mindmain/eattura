use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::state::TauriState;

/// Application settings stored locally.
///
/// Contains the user's company information (used as default cedente in invoices)
/// and PEC credentials for SDI communication.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    /// Company info (default invoice issuer).
    pub company: Option<CompanySettings>,
    /// PEC credentials for SDI.
    pub pec: Option<PecSettings>,
}

/// Company information used as default cedente (invoice issuer).
#[derive(Debug, Serialize, Deserialize)]
pub struct CompanySettings {
    pub denominazione: Option<String>,
    pub nome: Option<String>,
    pub cognome: Option<String>,
    pub id_paese: String,
    pub id_codice: String,
    pub codice_fiscale: Option<String>,
    pub regime_fiscale: String,
    pub indirizzo: String,
    pub numero_civico: Option<String>,
    pub cap: String,
    pub comune: String,
    pub provincia: Option<String>,
    pub nazione: String,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub pec: Option<String>,
}

/// PEC (Certified Email) configuration for SDI communication.
#[derive(Debug, Serialize, Deserialize)]
pub struct PecSettings {
    /// PEC email address.
    pub email: String,
    /// IMAP server hostname.
    pub imap_host: String,
    /// IMAP server port (usually 993 for IMAPS).
    pub imap_port: u16,
    /// SMTP server hostname.
    pub smtp_host: String,
    /// SMTP server port (usually 465 for SMTPS).
    pub smtp_port: u16,
    // Note: password should be stored securely, not in plain settings.
    // Consider using the system keychain via tauri-plugin-stronghold or similar.
}

/// Get current application settings.
///
/// Reads from the `settings` table in the local SQLite database.
/// Returns default empty settings if no entry exists yet.
#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, TauriState>,
) -> Result<AppSettings, String> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = 'app_settings'")
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    match row {
        Some(row) => {
            let json: String = row.get("value");
            serde_json::from_str(&json).map_err(|e| e.to_string())
        }
        None => Ok(AppSettings {
            company: None,
            pec: None,
        }),
    }
}

/// Update application settings.
///
/// Serializes the settings to JSON and persists them using INSERT OR REPLACE.
#[tauri::command]
pub async fn update_settings(
    state: tauri::State<'_, TauriState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let json = serde_json::to_string(&settings).map_err(|e| e.to_string())?;

    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('app_settings', ?)")
        .bind(&json)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(settings)
}
