//! Tauri commands for PEC/SDI integration on the desktop.
//!
//! These commands send invoices to SDI over PEC, poll the PEC inbox for SDI
//! notifications, and persist them locally. Connection parameters come from the
//! app settings; the PEC password is kept in the OS keyring.

use common::invoice_status::InvoiceStatus;
use common::pec::{
    SDI_PEC_ADDRESS, extract_sdi_pec_address, parse_notification, sdi_filename,
    select_preferred_sdi_address,
};
use std::sync::OnceLock;

use crate::app_log::{AppLog, LogLevel};
use crate::commands::invoices::build_fattura_from_store;
use crate::commands::settings::AppSettings;
use crate::pec::{self, PecConfig};
use crate::state::TauriState;
use crate::storage::digest;

const SYNC_INTERVAL_MS: i64 = 300_000;
static SYNC_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PecSyncStatus {
    pub in_progress: bool,
    pub configured: bool,
    pub last_attempt_at: Option<i64>,
    pub last_success_at: Option<i64>,
    pub next_sync_at: i64,
    pub last_error: Option<String>,
    pub sdi_address: String,
    pub sdi_address_detected: Option<String>,
    pub sdi_address_verified: bool,
}

async fn sync_record(
    store: &crate::storage::FileStore,
) -> Result<Option<crate::storage::Record>, String> {
    store
        .read("pec_sync", "id", vec![serde_json::json!("inbox")], "")
        .await
        .fetch_optional()
        .await
}

pub async fn sync_status_store(store: &crate::storage::FileStore) -> Result<PecSyncStatus, String> {
    let row = sync_record(store).await?;
    let pec_settings = store
        .read("settings", "settings", vec![], "")
        .await
        .fetch_optional()
        .await?
        .and_then(|settings| {
            serde_json::from_str::<AppSettings>(&settings.get::<String>("value")).ok()
        })
        .and_then(|settings| settings.pec);

    let mailbox = pec_settings
        .as_ref()
        .map(|pec| pec.email.trim())
        .filter(|email| !email.is_empty())
        .map(str::to_string);

    let same_mailbox = row.as_ref().is_some_and(|r| {
        mailbox
            .as_ref()
            .is_some_and(|email| email.eq_ignore_ascii_case(&r.get::<String>("email")))
    });

    let sdi_address_detected = row
        .as_ref()
        .filter(|_| same_mailbox)
        .and_then(|r| r.get::<Option<String>>("detected_sdi_address"))
        .and_then(|addr| extract_sdi_pec_address(&addr));

    let configured_sdi = pec_settings
        .as_ref()
        .and_then(|pec| pec.sdi_address.as_deref())
        .map(str::trim)
        .filter(|addr| !addr.is_empty())
        .map(str::to_ascii_lowercase);

    let preferred_sdi = select_preferred_sdi_address(
        sdi_address_detected
            .iter()
            .cloned()
            .chain(configured_sdi.iter().cloned()),
    );

    let sdi_address = preferred_sdi
        .or_else(|| configured_sdi.clone())
        .unwrap_or_else(|| SDI_PEC_ADDRESS.to_string());

    let sdi_address_verified = sdi_address_detected.is_some()
        || configured_sdi
            .as_deref()
            .is_some_and(|addr| !addr.eq_ignore_ascii_case(SDI_PEC_ADDRESS));

    let last_attempt_at = row
        .as_ref()
        .filter(|_| same_mailbox)
        .and_then(|r| r.get("last_attempt_at"));

    Ok(PecSyncStatus {
        in_progress: SYNC_LOCK.get().is_some_and(|lock| lock.try_lock().is_err()),
        configured: mailbox.is_some(),
        last_attempt_at,
        last_success_at: row
            .as_ref()
            .filter(|_| same_mailbox)
            .and_then(|r| r.get("last_success_at")),
        next_sync_at: last_attempt_at
            .map(|time| time + SYNC_INTERVAL_MS)
            .unwrap_or(0),
        last_error: row
            .as_ref()
            .filter(|_| same_mailbox)
            .and_then(|r| r.get("last_error")),
        sdi_address,
        sdi_address_detected,
        sdi_address_verified,
    })
}

#[tauri::command]
pub async fn pec_sync_status(state: tauri::State<'_, TauriState>) -> Result<PecSyncStatus, String> {
    sync_status_store(&state.store).await
}

/// Summary of an imported invoice or received SDI notification.
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
async fn build_config(store: &crate::storage::FileStore) -> Result<PecConfig, String> {
    let row = store
        .read("settings", "settings", vec![], "")
        .await
        .fetch_optional()
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

    let detected_sdi = sync_record(store)
        .await?
        .filter(|r| r.get::<String>("email").eq_ignore_ascii_case(&pec.email))
        .and_then(|r| r.get::<Option<String>>("detected_sdi_address"))
        .and_then(|addr| extract_sdi_pec_address(&addr));

    let configured_recipient = pec.sdi_recipient().to_string();
    let sdi_address = select_preferred_sdi_address(
        detected_sdi
            .into_iter()
            .chain(std::iter::once(configured_recipient.clone())),
    )
    .unwrap_or(configured_recipient);

    Ok(PecConfig {
        email: pec.email,
        sdi_address,
        password,
        imap_host: pec.imap_host,
        imap_port: pec.imap_port,
        smtp_host: pec.smtp_host,
        smtp_port: pec.smtp_port,
    })
}

/// Automatically persist the SDI PEC address intercepted from PEC notifications into settings.
pub(crate) async fn persist_detected_sdi_address(
    store: &crate::storage::FileStore,
    detected: &str,
) -> Result<(), String> {
    let Some(detected_clean) = extract_sdi_pec_address(detected) else {
        return Ok(());
    };
    let Some(row) = store
        .read("settings", "settings", vec![], "")
        .await
        .fetch_optional()
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let json: String = row.get("value");
    let Ok(mut settings) = serde_json::from_str::<AppSettings>(&json) else {
        return Ok(());
    };
    let Some(pec) = settings.pec.as_mut() else {
        return Ok(());
    };

    let existing = pec
        .sdi_address
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase);

    let next_sdi = select_preferred_sdi_address(
        std::iter::once(detected_clean.clone()).chain(existing.iter().cloned()),
    )
    .unwrap_or(detected_clean);

    if existing.as_deref() != Some(next_sdi.as_str()) {
        pec.sdi_address = Some(next_sdi);
        let updated_json = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
        let mut tx = store.begin().await.map_err(|e| e.to_string())?;
        tx.insert(
            "settings",
            serde_json::json!({"key": "app_settings", "value": updated_json}),
        )
        .await
        .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Send a validated invoice to SDI via PEC and mark it as `sent`.
#[tauri::command]
pub async fn send_invoice_pec(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<String, String> {
    // Only validated invoices may be sent.
    let stato: String = state
        .store
        .read("invoices", "id", vec![serde_json::json!(&id)], "")
        .await
        .fetch_optional()
        .await
        .map_err(|e| e.to_string())?
        .map(|r| r.get("stato"))
        .ok_or_else(|| format!("Invoice not found: {id}"))?;

    let current: InvoiceStatus = stato.parse()?;
    current.transition_to(InvoiceStatus::Sent)?;

    // Build the XML and SDI-compliant filename.
    let fattura = build_fattura_from_store(&state.store, &id).await?;
    let xml = common::xml::encode::encode(&fattura).map_err(|e| e.to_string())?;
    let filename = sdi_filename(&fattura);

    let config = build_config(&state.store).await?;
    let response = pec::send_invoice(&config, &xml, &filename).await?;

    // Persist the sent XML and advance the status.
    state
        .store
        .update(
            "invoices",
            &id,
            serde_json::json!({"stato": "sent", "xml_content": &xml, "sdi_filenames": [&filename]}),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(response)
}

/// Sync PEC invoice attachments and SDI notifications with local invoices.
///
/// Returns the items processed in this run.
#[tauri::command]
pub async fn sync_pec(
    state: tauri::State<'_, TauriState>,
    log: tauri::State<'_, AppLog>,
) -> Result<Vec<NotificationSummary>, String> {
    run_sync(&state.store, &log).await
}

/// Core PEC sync routine, usable both from the command and the background cron.
pub(crate) async fn run_sync(
    store: &crate::storage::FileStore,
    log: &AppLog,
) -> Result<Vec<NotificationSummary>, String> {
    let _guard = SYNC_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await;
    log.record(LogLevel::Info, "pec", "PEC synchronization started");
    let result = run_sync_inner(store, log).await;
    match &result {
        Ok(items) => log.record(
            LogLevel::Info,
            "pec",
            &format!(
                "PEC synchronization completed: {} item(s) processed",
                items.len()
            ),
        ),
        Err(error) => {
            log.record(
                LogLevel::Error,
                "pec",
                &format!("PEC synchronization failed: {error}"),
            );
            if let Err(record_error) = record_sync_failure(store, error).await {
                log.record(
                    LogLevel::Error,
                    "pec",
                    &format!("Unable to save synchronization failure: {record_error}"),
                );
            }
        }
    }
    result
}

async fn record_sync_failure(store: &crate::storage::FileStore, error: &str) -> Result<(), String> {
    let Some(email) = store
        .read("settings", "settings", vec![], "")
        .await
        .fetch_optional()
        .await?
        .and_then(|settings| {
            serde_json::from_str::<AppSettings>(&settings.get::<String>("value")).ok()
        })
        .and_then(|settings| settings.pec.map(|pec| pec.email))
    else {
        return Ok(());
    };
    let previous = sync_record(store).await?;
    let now = chrono::Utc::now().timestamp_millis();
    if let Some(row) = previous.as_ref()
        && row.get::<String>("email").eq_ignore_ascii_case(&email)
    {
        store
            .update(
                "pec_sync",
                "inbox",
                serde_json::json!({
                    "last_attempt_at": now,
                    "last_error": error
                }),
            )
            .await?;
        return Ok(());
    }
    let state = serde_json::json!({
        "id": "inbox",
        "email": email,
        "uid_validity": null,
        "last_uid": 0,
        "last_attempt_at": now,
        "last_success_at": null,
        "last_error": error,
        "detected_sdi_address": null,
        "sdi_scan_complete": false
    });
    if previous.is_some() {
        store.update("pec_sync", "inbox", state).await?;
    } else {
        store.insert("pec_sync", state).await?;
    }
    Ok(())
}

pub(crate) async fn process_inbox_messages(
    store: &crate::storage::FileStore,
    messages: &[pec::PecMessage],
) -> Result<(Vec<NotificationSummary>, Option<String>), String> {
    let mut summaries = Vec::new();
    let mut sdi_candidates = Vec::new();

    for message in messages {
        if let Some(sender) = &message.sdi_sender {
            sdi_candidates.push(sender.clone());
        }
        let mut has_sdi_payload = false;

        for att in &message.attachments {
            let name = att.filename.to_ascii_lowercase();
            if name == "daticert.xml" || (!name.ends_with(".xml") && !name.ends_with(".xml.p7m")) {
                continue;
            }
            let (payload, signer) = if name.ends_with(".p7m") {
                let (payload, signer) = common::p7m::inspect_p7m(&att.data)
                    .map_err(|e| format!("Invalid PEC attachment {}: {e}", att.filename))?;
                (payload, Some(signer))
            } else {
                (att.data.clone(), None)
            };
            let xml = String::from_utf8(payload)
                .map_err(|e| format!("Invalid XML attachment {}: {e}", att.filename))?;
            if common::xml::decode::decode(&xml).is_ok() {
                has_sdi_payload = true;
                let detail =
                    crate::commands::invoices::import_xml_content_for_pec(store, &xml, signer)
                        .await?;
                summaries.push(NotificationSummary {
                    tipo: "FATTURA".into(),
                    identificativo_sdi: None,
                    nome_file: Some(att.filename.clone()),
                    invoice_id: Some(detail.id),
                    new_status: None,
                });
            } else if let Ok(notification) = parse_notification(&xml)
                && !matches!(
                    notification.message_type,
                    common::pec::SdiMessageType::Unknown(_)
                )
            {
                has_sdi_payload = true;
                for (linked_id, new_status) in
                    attach_sdi_notification_to_matches(store, &notification, &xml).await?
                {
                    summaries.push(NotificationSummary {
                        tipo: notification.message_type.code().to_string(),
                        identificativo_sdi: notification.identificativo_sdi.clone(),
                        nome_file: notification.nome_file.clone(),
                        invoice_id: linked_id,
                        new_status,
                    });
                }
            }
        }

        if has_sdi_payload
            && message.sdi_sender.is_none()
            && let Some(fallback) = &message.sdi_fallback
        {
            sdi_candidates.push(fallback.clone());
        }
    }

    Ok((summaries, select_preferred_sdi_address(sdi_candidates)))
}

async fn run_sync_inner(
    store: &crate::storage::FileStore,
    log: &AppLog,
) -> Result<Vec<NotificationSummary>, String> {
    let config = build_config(store).await?;
    let previous = sync_record(store).await?;
    let previous_same_mailbox = previous.as_ref().filter(|row| {
        row.get::<String>("email")
            .eq_ignore_ascii_case(&config.email)
    });
    let previous_detected = previous_same_mailbox
        .and_then(|row| row.get::<Option<String>>("detected_sdi_address"))
        .and_then(|addr| extract_sdi_pec_address(&addr));

    let cursor = previous_same_mailbox.and_then(|row| {
        let scan_complete = row
            .get::<Option<bool>>("sdi_scan_complete")
            .unwrap_or(false);
        if !scan_complete {
            return None;
        }
        Some((
            row.get::<Option<u32>>("uid_validity")?,
            row.get::<u32>("last_uid"),
        ))
    });

    log.record(
        LogLevel::Info,
        "pec",
        if cursor.is_some() {
            "Reading new PEC messages"
        } else {
            "Reading PEC mailbox history"
        },
    );
    let inbox = pec::check_inbox(&config, cursor).await?;
    log.record(
        LogLevel::Info,
        "pec",
        &format!(
            "PEC download completed: {} message(s) to process",
            inbox.messages.len()
        ),
    );
    let (summaries, newly_detected) = process_inbox_messages(store, &inbox.messages).await?;

    let detected_sdi_address = select_preferred_sdi_address(
        newly_detected
            .into_iter()
            .chain(previous_detected.into_iter()),
    );

    if let Some(ref detected) = detected_sdi_address {
        persist_detected_sdi_address(store, detected).await?;
    }

    if previous.is_none() || !summaries.is_empty() {
        log.record(
            LogLevel::Info,
            "pec",
            "Linking SDI receipts to imported invoices",
        );
        reconcile_unlinked_notifications(store).await?;
    }

    let now = chrono::Utc::now().timestamp_millis();
    let state = serde_json::json!({
        "id": "inbox",
        "email": config.email,
        "uid_validity": inbox.uid_validity,
        "last_uid": inbox.last_uid,
        "last_attempt_at": now,
        "last_success_at": now,
        "last_error": null,
        "detected_sdi_address": detected_sdi_address,
        "sdi_scan_complete": true
    });
    if previous.is_some() {
        store.update("pec_sync", "inbox", state).await?;
    } else {
        store.insert("pec_sync", state).await?;
    }

    Ok(summaries)
}

/// Read the current status of an invoice.
async fn current_status(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<InvoiceStatus, String> {
    let stato: String = store
        .read("invoices", "id", vec![serde_json::json!(id)], "")
        .await
        .fetch_one()
        .await
        .map_err(|e| e.to_string())?
        .get("stato");
    stato.parse()
}

/// Import an SDI receipt directly from XML content.
#[tauri::command]
pub async fn import_sdi_receipt(
    state: tauri::State<'_, TauriState>,
    xml_content: String,
) -> Result<NotificationSummary, String> {
    import_sdi_receipt_store(&state.store, &xml_content).await
}

pub async fn import_sdi_receipt_store(
    store: &crate::storage::FileStore,
    xml_content: &str,
) -> Result<NotificationSummary, String> {
    let notification = common::pec::parse_notification(xml_content).map_err(|e| e.to_string())?;
    let (invoice_id, new_status) =
        attach_sdi_notification_to_matches(store, &notification, xml_content)
            .await?
            .into_iter()
            .next()
            .unwrap_or((None, None));
    Ok(NotificationSummary {
        tipo: notification.message_type.code().to_string(),
        identificativo_sdi: notification.identificativo_sdi,
        nome_file: notification.nome_file,
        invoice_id,
        new_status,
    })
}

async fn attach_sdi_notification_to_matches(
    store: &crate::storage::FileStore,
    notification: &common::pec::ParsedNotification,
    raw_xml: &str,
) -> Result<Vec<(Option<String>, Option<String>)>, String> {
    let matches = if let Some(filename) = notification.nome_file.as_deref() {
        find_invoice_ids_by_sdi_reference(store, filename).await?
    } else {
        Vec::new()
    };
    if matches.is_empty() {
        return Ok(vec![
            attach_sdi_notification(store, notification, raw_xml, None).await?,
        ]);
    }
    let mut linked = Vec::with_capacity(matches.len());
    for id in matches {
        linked.push(attach_sdi_notification(store, notification, raw_xml, Some(&id)).await?);
    }
    Ok(linked)
}

/// Attach an SDI notification to an invoice, update status, and persist the receipt file.
pub async fn attach_sdi_notification(
    store: &crate::storage::FileStore,
    notification: &common::pec::ParsedNotification,
    raw_xml: &str,
    invoice_id_opt: Option<&str>,
) -> Result<(Option<String>, Option<String>), String> {
    let invoice_id = if let Some(id) = invoice_id_opt {
        Some(id.to_string())
    } else if let Some(nf) = notification.nome_file.as_ref() {
        find_invoice_by_sdi_reference(store, nf)
            .await?
            .or(find_invoice_by_sdi_id(store, notification.identificativo_sdi.as_deref()).await?)
    } else {
        find_invoice_by_sdi_id(store, notification.identificativo_sdi.as_deref()).await?
    };

    let matching_receipts: Vec<_> = store
        .read("sdi_notifications", "all", vec![], "")
        .await
        .fetch_all()
        .await?
        .into_iter()
        .filter(|row| row.get::<String>("raw_xml") == raw_xml)
        .collect();
    let existing = matching_receipts
        .iter()
        .find(|row| row.get::<Option<String>>("invoice_id") == invoice_id)
        .or_else(|| {
            invoice_id.as_ref().and_then(|_| {
                matching_receipts
                    .iter()
                    .find(|row| row.get::<Option<String>>("invoice_id").is_none())
            })
        });
    let receipt_id = existing
        .as_ref()
        .map(|row| row.get::<String>("id"))
        .unwrap_or(digest::content_id(
            &serde_json::json!({"sdi_receipt": raw_xml, "invoice_id": &invoice_id}),
        )?);
    let invoice_id = invoice_id.or_else(|| existing.as_ref().and_then(|row| row.get("invoice_id")));

    let mut new_status = None;
    let mut tx = store.begin().await?;

    if let (Some(inv_id), Some(target)) = (invoice_id.as_ref(), notification.resulting_status())
        && let Ok(cur) = current_status(&tx.snapshot(), inv_id).await
        && (cur == InvoiceStatus::Imported || cur.transition_to(target).is_ok())
    {
        tx.update(
            "invoices",
            inv_id,
            serde_json::json!({"stato": target.as_str(),
                        "imported_from_xml": cur == InvoiceStatus::Imported}),
        )
        .await
        .map_err(|e| e.to_string())?;
        new_status = Some(target.as_str().to_string());
    }

    let errori_json = serde_json::to_string(
        &notification
            .errors
            .iter()
            .map(|e| serde_json::json!({"codice": e.codice, "descrizione": e.descrizione}))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string());

    let notification_record = serde_json::json!({
        "id": receipt_id,
        "invoice_id": &invoice_id,
        "tipo": notification.message_type.code(),
        "identificativo_sdi": &notification.identificativo_sdi,
        "nome_file": &notification.nome_file,
        "esito": &notification.esito,
        "errori": &errori_json,
        "data_ora_ricezione": &notification.data_ora_ricezione,
        "destinatario": &notification.destinatario,
        "note": &notification.note,
        "raw_xml": raw_xml,
    });
    if existing.is_some() {
        tx.update("sdi_notifications", &receipt_id, notification_record)
            .await?;
    } else {
        tx.insert("sdi_notifications", notification_record).await?;
    }

    tx.commit().await?;

    // Keep the receipt beside the active vault's invoice documents.
    {
        let dir_path = store.data_dir().join("ricevute");
        std::fs::create_dir_all(&dir_path).map_err(|e| e.to_string())?;
        let base = notification.nome_file.as_deref().unwrap_or("SDI_RECEIPT");
        let base = std::path::Path::new(base)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("SDI_RECEIPT");
        let clean_base = base
            .strip_suffix(".xml.p7m")
            .or_else(|| base.strip_suffix(".p7m"))
            .or_else(|| base.strip_suffix(".xml"))
            .unwrap_or(base);
        let filename = format!(
            "{clean_base}_{}_{}.xml",
            notification.message_type.code(),
            &receipt_id[..12]
        );
        let receipt_path = dir_path.join(&filename);
        std::fs::write(receipt_path, raw_xml).map_err(|e| e.to_string())?;
    }

    Ok((invoice_id, new_status))
}

async fn find_invoice_by_sdi_id(
    store: &crate::storage::FileStore,
    sdi_id: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(sdi_id) = sdi_id else {
        return Ok(None);
    };
    let rows = store
        .read("sdi_notifications", "all", vec![], "")
        .await
        .fetch_all()
        .await?;
    let mut matched = None;
    for row in rows {
        if row.get::<Option<String>>("identificativo_sdi").as_deref() != Some(sdi_id) {
            continue;
        }
        if let Some(id) = row.get::<Option<String>>("invoice_id") {
            if matched.as_ref().is_some_and(|previous| previous != &id) {
                return Ok(None);
            }
            matched = Some(id);
        }
    }
    Ok(matched)
}

/// Revisit receipts downloaded before their invoice was imported.
pub(crate) async fn reconcile_unlinked_notifications(
    store: &crate::storage::FileStore,
) -> Result<(), String> {
    let rows = store
        .read("sdi_notifications", "all", vec![], "")
        .await
        .fetch_all()
        .await?;
    let mut processed = std::collections::HashSet::new();
    for row in rows {
        let xml: String = row.get("raw_xml");
        if !processed.insert(xml.clone()) {
            continue;
        }
        if let Ok(notification) = parse_notification(&xml) {
            attach_sdi_notification_to_matches(store, &notification, &xml).await?;
        }
    }
    Ok(())
}

/// Find an invoice ID given an SDI file reference (e.g. IT01234567890_00001.xml).
pub async fn find_invoice_by_sdi_reference(
    store: &crate::storage::FileStore,
    reference: &str,
) -> Result<Option<String>, String> {
    let matches = find_invoice_ids_by_sdi_reference(store, reference).await?;
    Ok((matches.len() == 1).then(|| matches[0].clone()))
}

async fn find_invoice_ids_by_sdi_reference(
    store: &crate::storage::FileStore,
    reference: &str,
) -> Result<Vec<String>, String> {
    let clean_ref = reference
        .strip_suffix(".xml.p7m")
        .or_else(|| reference.strip_suffix(".p7m"))
        .or_else(|| reference.strip_suffix(".xml"))
        .unwrap_or(reference);

    let rows = store
        .read("invoices", "invoice_views", vec![], "")
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let mut matched = Vec::new();
    for row in rows {
        let id: String = row.get("id");
        if id == reference || id == clean_ref {
            matched.push(id);
            continue;
        }
        let filenames = row
            .get::<Option<Vec<String>>>("sdi_filenames")
            .unwrap_or_default();
        if filenames
            .iter()
            .any(|name| same_sdi_filename(name, reference))
        {
            matched.push(id);
            continue;
        }
        // Known transmission aliases are authoritative; rebuilding every unmatched
        // invoice clones the archive for every receipt during a historical sync.
        if !filenames.is_empty() {
            continue;
        }
        if let Ok(fattura) = build_fattura_from_store(store, &id).await {
            let fn_sdi = sdi_filename(&fattura);
            if same_sdi_filename(&fn_sdi, reference) {
                matched.push(id);
            }
        }
    }
    Ok(matched)
}

fn same_sdi_filename(left: &str, right: &str) -> bool {
    fn stem(name: &str) -> &str {
        name.strip_suffix(".xml.p7m")
            .or_else(|| name.strip_suffix(".p7m"))
            .or_else(|| name.strip_suffix(".xml"))
            .unwrap_or(name)
    }
    stem(left).eq_ignore_ascii_case(stem(right))
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn failed_sync_records_start_and_error_before_returning() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::storage::FileStore::open(dir.path().join("eattura.json")).unwrap();
        let log_dir = dir.path().join("logs");
        let log = AppLog::open(log_dir.clone()).unwrap();
        assert!(run_sync(&store, &log).await.is_err());
        let entries: Vec<serde_json::Value> = std::fs::read_to_string(log_dir.join("events.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(entries[0]["message"], "PEC synchronization started");
        assert_eq!(entries.last().unwrap()["level"], "error");
        assert!(
            entries.last().unwrap()["message"]
                .as_str()
                .unwrap()
                .contains("not configured")
        );
    }

    use super::*;
    use crate::commands::settings::{
        AppSettings, PecSettings, get_settings_store, update_settings_store,
    };
    use crate::pec::{PecAttachment, PecMessage};

    #[tokio::test]
    async fn test_auto_detect_sdi_pec_address_and_persist_to_settings() {
        let dir =
            std::env::temp_dir().join(format!("eattura-pec-sdi-detect-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::storage::FileStore::open(dir.join("index.json")).unwrap();

        update_settings_store(
            &store,
            AppSettings {
                company: None,
                pec: Some(PecSettings {
                    email: "studio@pec.example.it".into(),
                    imap_host: "imaps.pec.example.it".into(),
                    imap_port: 993,
                    smtp_host: "smtps.pec.example.it".into(),
                    smtp_port: 465,
                    sdi_address: None,
                }),
            },
        )
        .await
        .unwrap();

        // Initially: not verified, falls back to sdi01@pec.fatturapa.it
        let initial_status = sync_status_store(&store).await.unwrap();
        assert!(initial_status.configured);
        assert_eq!(initial_status.sdi_address, "sdi01@pec.fatturapa.it");
        assert_eq!(initial_status.sdi_address_detected, None);
        assert!(!initial_status.sdi_address_verified);

        // Simulate receiving an SDI notification from sdi12@pec.fatturapa.it during inbox scan
        let rc_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<RicevutaConsegna>
  <IdentificativoSdI>99887766</IdentificativoSdI>
  <NomeFile>IT01234567890_00001.xml</NomeFile>
  <DataOraRicezione>2026-03-01T10:00:00</DataOraRicezione>
</RicevutaConsegna>"#;
        let messages = vec![PecMessage {
            uid: 42,
            subject: "POSTA CERTIFICATA: Ricevuta di consegna".into(),
            from: "posta-certificata@pec.aruba.it".into(),
            sdi_sender: Some("sdi12@pec.fatturapa.it".into()),
            sdi_fallback: Some("sdi12@pec.fatturapa.it".into()),
            attachments: vec![PecAttachment {
                filename: "IT01234567890_00001_RC_001.xml".into(),
                data: rc_xml.as_bytes().to_vec(),
            }],
        }];

        let (summaries, detected) = process_inbox_messages(&store, &messages).await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(detected.as_deref(), Some("sdi12@pec.fatturapa.it"));

        persist_detected_sdi_address(&store, detected.as_deref().unwrap())
            .await
            .unwrap();
        store
            .insert(
                "pec_sync",
                serde_json::json!({
                    "id": "inbox",
                    "email": "studio@pec.example.it",
                    "uid_validity": 1,
                    "last_uid": 42,
                    "last_attempt_at": 1000,
                    "last_success_at": 1000,
                    "last_error": null,
                    "detected_sdi_address": detected,
                    "sdi_scan_complete": true
                }),
            )
            .await
            .unwrap();

        let updated_settings = get_settings_store(&store).await.unwrap();
        assert_eq!(
            updated_settings.pec.unwrap().sdi_address.as_deref(),
            Some("sdi12@pec.fatturapa.it")
        );

        let verified_status = sync_status_store(&store).await.unwrap();
        assert!(verified_status.sdi_address_verified);
        assert_eq!(verified_status.sdi_address, "sdi12@pec.fatturapa.it");
        assert_eq!(
            verified_status.sdi_address_detected.as_deref(),
            Some("sdi12@pec.fatturapa.it")
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
