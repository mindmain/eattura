//! Tauri commands for PEC/SDI integration on the desktop.
//!
//! These commands send invoices to SDI over PEC, poll the PEC inbox for SDI
//! notifications, and persist them locally. Connection parameters come from the
//! app settings; the PEC password is kept in the OS keyring.

use sqlx::Row;

use common::invoice_status::InvoiceStatus;
use common::pec::{parse_notification, sdi_filename};

use crate::commands::invoices::build_fattura_from_db;
use crate::commands::settings::AppSettings;
use crate::pec::{self, PecConfig};
use crate::state::TauriState;

/// Summary of a received SDI notification returned to the frontend.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSummary {
    pub tipo: String,
    pub identificativo_sdi: Option<String>,
    pub nome_file: Option<String>,
    pub invoice_id: Option<String>,
    pub new_status: Option<String>,
}

/// Store the PEC password securely in the OS keyring.
#[tauri::command]
pub async fn set_pec_password(email: String, password: String) -> Result<(), String> {
    pec::store_password(&email, &password)
}

/// Build a [`PecConfig`] from the saved settings and the keyring password.
async fn build_config(pool: &sqlx::SqlitePool) -> Result<PecConfig, String> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = 'app_settings'")
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    let json: String = row
        .map(|r| r.get("value"))
        .ok_or("PEC settings not configured. Set them up in Settings first.")?;
    let settings: AppSettings = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let pec = settings
        .pec
        .ok_or("PEC settings not configured. Set them up in Settings first.")?;
    let password = pec::load_password(&pec.email)?;
    Ok(PecConfig {
        email: pec.email,
        password,
        imap_host: pec.imap_host,
        imap_port: pec.imap_port,
        smtp_host: pec.smtp_host,
        smtp_port: pec.smtp_port,
    })
}

/// Send a validated invoice to SDI via PEC and mark it as `sent`.
#[tauri::command]
pub async fn send_invoice_pec(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<String, String> {
    // Only validated invoices may be sent.
    let stato: String = sqlx::query("SELECT stato FROM invoices WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .map(|r| r.get("stato"))
        .ok_or_else(|| format!("Invoice not found: {id}"))?;

    let current: InvoiceStatus = stato.parse()?;
    current.transition_to(InvoiceStatus::Sent)?;

    // Build the XML and SDI-compliant filename.
    let fattura = build_fattura_from_db(&state.db, &id).await?;
    let xml = common::xml::encode::encode(&fattura).map_err(|e| e.to_string())?;
    let filename = sdi_filename(&fattura);

    let config = build_config(&state.db).await?;
    let response = pec::send_invoice(&config, &xml, &filename).await?;

    // Persist the sent XML and advance the status.
    sqlx::query(
        "UPDATE invoices SET stato = 'sent', xml_content = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&xml)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(response)
}

/// Poll the PEC inbox, persist SDI notifications, and update invoice statuses.
///
/// Returns a summary of the notifications processed in this run.
#[tauri::command]
pub async fn sync_pec(
    state: tauri::State<'_, TauriState>,
) -> Result<Vec<NotificationSummary>, String> {
    run_sync(&state.db).await
}

/// Core PEC sync routine, usable both from the command and the background cron.
pub(crate) async fn run_sync(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<NotificationSummary>, String> {
    let config = build_config(pool).await?;
    let messages = pec::check_inbox(&config).await?;

    // Map SDI file names of currently-sent invoices to their IDs for linking.
    let sent_index = build_sent_filename_index(pool).await?;

    let mut summaries = Vec::new();
    for message in &messages {
        for att in &message.attachments {
            if !att.filename.to_lowercase().ends_with(".xml") {
                continue;
            }
            let xml = String::from_utf8_lossy(&att.data);
            let Ok(notification) = parse_notification(&xml) else {
                continue; // not an SDI notification we understand
            };

            // Try to link the notification to an invoice by its referenced file name.
            let invoice_id = notification
                .nome_file
                .as_ref()
                .and_then(|nf| sent_index.get(nf).cloned());

            // Apply the resulting status transition where legal.
            let mut new_status = None;
            if let (Some(inv_id), Some(target)) =
                (invoice_id.as_ref(), notification.resulting_status())
                && let Ok(cur) = current_status(pool, inv_id).await
                    && cur.transition_to(target).is_ok() {
                        sqlx::query(
                            "UPDATE invoices SET stato = ?, updated_at = datetime('now') WHERE id = ?",
                        )
                        .bind(target.as_str())
                        .bind(inv_id)
                        .execute(pool)
                        .await
                        .map_err(|e| e.to_string())?;
                        new_status = Some(target.as_str().to_string());
                    }

            // Persist the notification.
            let errori_json = serde_json::to_string(
                &notification
                    .errors
                    .iter()
                    .map(|e| serde_json::json!({"codice": e.codice, "descrizione": e.descrizione}))
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".to_string());

            sqlx::query(
                "INSERT INTO sdi_notifications (id, invoice_id, tipo, identificativo_sdi, nome_file, esito, errori, raw_xml)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&invoice_id)
            .bind(notification.message_type.code())
            .bind(&notification.identificativo_sdi)
            .bind(&notification.nome_file)
            .bind(&notification.esito)
            .bind(&errori_json)
            .bind(xml.as_ref())
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

            summaries.push(NotificationSummary {
                tipo: notification.message_type.code().to_string(),
                identificativo_sdi: notification.identificativo_sdi.clone(),
                nome_file: notification.nome_file.clone(),
                invoice_id,
                new_status,
            });
        }
    }

    Ok(summaries)
}

/// Read the current status of an invoice.
async fn current_status(pool: &sqlx::SqlitePool, id: &str) -> Result<InvoiceStatus, String> {
    let stato: String = sqlx::query("SELECT stato FROM invoices WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .get("stato");
    stato.parse()
}

/// Build a map of `{ sdi_filename -> invoice_id }` for invoices in the `sent` state.
async fn build_sent_filename_index(
    pool: &sqlx::SqlitePool,
) -> Result<std::collections::HashMap<String, String>, String> {
    let rows = sqlx::query("SELECT id FROM invoices WHERE stato IN ('sent','accepted','rejected')")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut index = std::collections::HashMap::new();
    for row in &rows {
        let id: String = row.get("id");
        if let Ok(fattura) = build_fattura_from_db(pool, &id).await {
            index.insert(sdi_filename(&fattura), id);
        }
    }
    Ok(index)
}
