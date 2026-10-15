use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use common::pec::SDI_PEC_ADDRESS;
use crate::error::AppError;
use crate::services::pec_service::{PecConfig, PecService};

#[derive(Debug, Deserialize)]
pub struct PecCredentialsPayload {
    pub email: String,
    pub password: String,
    pub imap_host: Option<String>,
    pub imap_port: Option<u16>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct PecSendRequest {
    pub pec: Option<PecCredentialsPayload>,
    pub recipient: Option<String>,
    pub xml_content: String,
    pub filename: String,
}

#[derive(Debug, Deserialize)]
pub struct PecPollRequest {
    pub pec: Option<PecCredentialsPayload>,
    pub filter_filenames: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub tipo: String,
    pub identificativo_sdi: Option<String>,
    pub nome_file: Option<String>,
    pub esito: Option<String>,
    pub errors: Vec<NotificationErrorItem>,
    pub raw_xml: String,
}

#[derive(Debug, Serialize)]
pub struct NotificationErrorItem {
    pub codice: String,
    pub descrizione: String,
}

fn resolve_pec_config(creds: Option<PecCredentialsPayload>) -> Result<PecConfig, AppError> {
    if let Some(c) = creds {
        Ok(PecConfig {
            email: c.email,
            password: c.password,
            imap_host: c.imap_host.unwrap_or_else(|| "imaps.pec.aruba.it".to_string()),
            imap_port: c.imap_port.unwrap_or(993),
            smtp_host: c.smtp_host.unwrap_or_else(|| "smtps.pec.aruba.it".to_string()),
            smtp_port: c.smtp_port.unwrap_or(465),
        })
    } else {
        PecConfig::from_env()
    }
}

/// Stateless proxy handler to send an invoice to SDI via PEC SMTP.
pub async fn send_pec(Json(req): Json<PecSendRequest>) -> Result<Json<Value>, AppError> {
    let config = resolve_pec_config(req.pec)?;
    let pec = PecService::new(config);

    let recipient = req.recipient.unwrap_or_else(|| SDI_PEC_ADDRESS.to_string());
    tracing::info!("Stateless PEC send: file '{}' to '{}'", req.filename, recipient);

    let smtp_response = pec.send_invoice(&req.xml_content, &req.filename).await?;

    Ok(Json(json!({
        "sent": true,
        "filename": req.filename,
        "recipient": recipient,
        "smtp_response": smtp_response
    })))
}

/// Stateless proxy handler to poll the PEC IMAP inbox and extract SDI notifications.
pub async fn poll_pec(Json(req): Json<PecPollRequest>) -> Result<Json<Value>, AppError> {
    let config = resolve_pec_config(req.pec)?;
    let pec = PecService::new(config);

    tracing::info!("Stateless PEC poll starting for inbox {}", pec.config.email);
    let messages = pec.check_inbox().await?;

    let filter_set: Option<std::collections::HashSet<String>> = req.filter_filenames.map(|v| {
        v.into_iter().map(|f| f.to_lowercase()).collect()
    });

    let mut notifications = Vec::new();
    for message in &messages {
        for att in &message.attachments {
            if !att.filename.to_lowercase().ends_with(".xml") {
                continue;
            }
            let xml = String::from_utf8_lossy(&att.data).to_string();
            let Ok(parsed) = PecService::parse_notification(&xml) else {
                continue;
            };

            if let Some(ref set) = filter_set {
                if let Some(ref fname) = parsed.nome_file {
                    if !set.contains(&fname.to_lowercase()) {
                        continue;
                    }
                }
            }

            notifications.push(NotificationResponse {
                tipo: parsed.message_type.code().to_string(),
                identificativo_sdi: parsed.identificativo_sdi,
                nome_file: parsed.nome_file,
                esito: parsed.esito,
                errors: parsed
                    .errors
                    .into_iter()
                    .map(|e| NotificationErrorItem {
                        codice: e.codice,
                        descrizione: e.descrizione,
                    })
                    .collect(),
                raw_xml: xml,
            });
        }
    }

    Ok(Json(json!({
        "total_found": notifications.len(),
        "notifications": notifications
    })))
}
