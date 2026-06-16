//! HTTP handlers for PEC/SDI integration.
//!
//! `POST /api/invoices/{id}/send` sends a validated invoice to SDI over PEC.
//! `POST /api/pec/sync` polls the PEC inbox and applies SDI notifications.
//! PEC connection parameters come from environment variables (`PecConfig::from_env`).

use axum::extract::{Path, State};
use axum::Json;
use sqlx::Row;
use uuid::Uuid;

use common::invoice_status::InvoiceStatus;
use common::pec::sdi_filename;

use crate::error::AppError;
use crate::services::invoice_service::InvoiceService;
use crate::services::pec_service::{PecConfig, PecService};
use crate::state::AppState;

/// Send a validated invoice to SDI via PEC and mark it as `sent`.
pub async fn send_invoice(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stato: String = sqlx::query("SELECT stato FROM invoices WHERE id = $1")
        .bind(id.to_string())
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Invoice {id} not found")))?
        .get("stato");

    let current: InvoiceStatus = stato.parse().map_err(AppError::Internal)?;
    current
        .transition_to(InvoiceStatus::Sent)
        .map_err(AppError::Internal)?;

    let svc = InvoiceService { db: state.db.clone() };
    let fattura = svc.to_fattura(id).await?;
    let xml = common::xml::encode::encode(&fattura)?;
    let filename = sdi_filename(&fattura);

    let pec = PecService::new(PecConfig::from_env()?);
    let response = pec.send_invoice(&xml, &filename).await?;

    sqlx::query("UPDATE invoices SET stato = 'sent', xml_content = $2, updated_at = NOW() WHERE id = $1")
        .bind(id.to_string())
        .bind(&xml)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "sent": true, "smtp_response": response })))
}

/// Poll the PEC inbox, persist SDI notifications, and update invoice statuses.
pub async fn sync(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pec = PecService::new(PecConfig::from_env()?);
    let messages = pec.check_inbox().await?;

    // Map SDI file names of currently-sent invoices to their IDs.
    let sent_index = build_sent_filename_index(&state.db).await?;

    let mut processed = Vec::new();
    for message in &messages {
        for att in &message.attachments {
            if !att.filename.to_lowercase().ends_with(".xml") {
                continue;
            }
            let xml = String::from_utf8_lossy(&att.data).to_string();
            let Ok(notification) = PecService::parse_notification(&xml) else {
                continue;
            };

            let invoice_id = notification
                .nome_file
                .as_ref()
                .and_then(|nf| sent_index.get(nf).cloned());

            let mut new_status: Option<String> = None;
            if let (Some(inv_id), Some(target)) =
                (invoice_id.as_ref(), notification.resulting_status())
            {
                let cur: String = sqlx::query("SELECT stato FROM invoices WHERE id = $1")
                    .bind(inv_id)
                    .fetch_one(&state.db)
                    .await?
                    .get("stato");
                if let Ok(cur) = cur.parse::<InvoiceStatus>() {
                    if cur.transition_to(target).is_ok() {
                        sqlx::query("UPDATE invoices SET stato = $2, updated_at = NOW() WHERE id = $1")
                            .bind(inv_id)
                            .bind(target.as_str())
                            .execute(&state.db)
                            .await?;
                        new_status = Some(target.as_str().to_string());
                    }
                }
            }

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
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&invoice_id)
            .bind(notification.message_type.code())
            .bind(&notification.identificativo_sdi)
            .bind(&notification.nome_file)
            .bind(&notification.esito)
            .bind(&errori_json)
            .bind(&xml)
            .execute(&state.db)
            .await?;

            processed.push(serde_json::json!({
                "tipo": notification.message_type.code(),
                "identificativo_sdi": notification.identificativo_sdi,
                "nome_file": notification.nome_file,
                "invoice_id": invoice_id,
                "new_status": new_status,
            }));
        }
    }

    Ok(Json(serde_json::json!({ "processed": processed.len(), "notifications": processed })))
}

/// Build a map of `{ sdi_filename -> invoice_id }` for invoices in the `sent` state.
async fn build_sent_filename_index(
    pool: &sqlx::PgPool,
) -> Result<std::collections::HashMap<String, String>, AppError> {
    let rows = sqlx::query("SELECT id FROM invoices WHERE stato = 'sent'")
        .fetch_all(pool)
        .await?;

    let svc = InvoiceService { db: pool.clone() };
    let mut index = std::collections::HashMap::new();
    for row in &rows {
        let id_str: String = row.get("id");
        if let Ok(uuid) = id_str.parse::<Uuid>() {
            if let Ok(fattura) = svc.to_fattura(uuid).await {
                index.insert(sdi_filename(&fattura), id_str);
            }
        }
    }
    Ok(index)
}
