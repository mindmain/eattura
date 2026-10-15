//! Tauri commands for PEC/SDI integration on the desktop.
//!
//! These commands send invoices to SDI over PEC, poll the PEC inbox for SDI
//! notifications, and persist them locally. Connection parameters come from the
//! app settings; the PEC password is kept in the OS keyring.

use common::invoice_status::InvoiceStatus;
use common::pec::{parse_notification, sdi_filename};
use std::sync::OnceLock;

use crate::commands::invoices::{build_fattura_from_store, import_xml_content_with_signer};
use crate::commands::settings::AppSettings;
use crate::pec::{self, PecConfig};
use crate::state::TauriState;
use crate::storage::digest;

const SYNC_INTERVAL_MS: i64 = 300_000;
static SYNC_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PecSyncStatus {
    pub configured: bool,
    pub last_attempt_at: Option<i64>,
    pub last_success_at: Option<i64>,
    pub next_sync_at: i64,
    pub last_error: Option<String>,
}

async fn sync_record(store: &crate::storage::FileStore) -> Result<Option<crate::storage::Record>, String> {
    store.read("pec_sync", "id", vec![serde_json::json!("inbox")], "")
        .await.fetch_optional().await
}

pub async fn sync_status_store(store: &crate::storage::FileStore) -> Result<PecSyncStatus, String> {
    let row = sync_record(store).await?;
    let mailbox = store.read("settings", "settings", vec![], "")
        .await.fetch_optional().await?
        .and_then(|settings| serde_json::from_str::<AppSettings>(&settings.get::<String>("value")).ok())
        .and_then(|settings| settings.pec.map(|pec| pec.email));
    let same_mailbox = row.as_ref().is_some_and(|r| {
        mailbox.as_ref().is_some_and(|email| email.eq_ignore_ascii_case(&r.get::<String>("email")))
    });
    let last_attempt_at = row.as_ref().filter(|_| same_mailbox).and_then(|r| r.get("last_attempt_at"));
    Ok(PecSyncStatus {
        configured: mailbox.is_some(),
        last_attempt_at,
        last_success_at: row.as_ref().filter(|_| same_mailbox).and_then(|r| r.get("last_success_at")),
        next_sync_at: last_attempt_at.map(|time| time + SYNC_INTERVAL_MS).unwrap_or(0),
        last_error: row.as_ref().filter(|_| same_mailbox).and_then(|r| r.get("last_error")),
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
    let sdi_address = pec.sdi_recipient().to_string();
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
) -> Result<Vec<NotificationSummary>, String> {
    run_sync(&state.store).await
}

/// Core PEC sync routine, usable both from the command and the background cron.
pub(crate) async fn run_sync(
    store: &crate::storage::FileStore,
) -> Result<Vec<NotificationSummary>, String> {
    let _guard = SYNC_LOCK.get_or_init(|| tokio::sync::Mutex::new(())).lock().await;
    let result = run_sync_inner(store).await;
    if let Err(error) = &result {
        let _ = record_sync_failure(store, error).await;
    }
    result
}

async fn record_sync_failure(store: &crate::storage::FileStore, error: &str) -> Result<(), String> {
    let Some(email) = store.read("settings", "settings", vec![], "")
        .await.fetch_optional().await?
        .and_then(|settings| serde_json::from_str::<AppSettings>(&settings.get::<String>("value")).ok())
        .and_then(|settings| settings.pec.map(|pec| pec.email)) else { return Ok(()) };
    let previous = sync_record(store).await?;
    let now = chrono::Utc::now().timestamp_millis();
    if let Some(row) = previous.as_ref() {
        if row.get::<String>("email").eq_ignore_ascii_case(&email) {
            store.update("pec_sync", "inbox", serde_json::json!({
                "last_attempt_at": now, "last_error": error
            })).await?;
            return Ok(());
        }
    }
    let state = serde_json::json!({
        "id":"inbox", "email":email, "uid_validity":null, "last_uid":0,
        "last_attempt_at":now, "last_success_at":null, "last_error":error
    });
    if previous.is_some() {
        store.update("pec_sync", "inbox", state).await?;
    } else {
        store.insert("pec_sync", state).await?;
    }
    Ok(())
}

async fn run_sync_inner(
    store: &crate::storage::FileStore,
) -> Result<Vec<NotificationSummary>, String> {
    let config = build_config(store).await?;
    let previous = sync_record(store).await?;
    let cursor = previous.as_ref().and_then(|row| {
        let email: String = row.get("email");
        if email.eq_ignore_ascii_case(&config.email) {
            Some((row.get::<Option<u32>>("uid_validity")?, row.get::<u32>("last_uid")))
        } else {
            None
        }
    });
    let inbox = pec::check_inbox(&config, cursor).await?;

    let mut summaries = Vec::new();
    for message in &inbox.messages {
        for att in &message.attachments {
            let name = att.filename.to_ascii_lowercase();
            if !name.ends_with(".xml") && !name.ends_with(".xml.p7m") {
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
                let detail = import_xml_content_with_signer(store, &xml, signer).await?;
                summaries.push(NotificationSummary {
                    tipo: "FATTURA".into(),
                    identificativo_sdi: None,
                    nome_file: Some(att.filename.clone()),
                    invoice_id: Some(detail.id),
                    new_status: None,
                });
            } else if let Ok(notification) = parse_notification(&xml) {
                if !matches!(notification.message_type, common::pec::SdiMessageType::Unknown(_)) {
                    for (linked_id, new_status) in attach_sdi_notification_to_matches(store, &notification, &xml).await? {
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
        }
    }

    if previous.is_none() || !summaries.is_empty() {
        reconcile_unlinked_notifications(store).await?;
    }

    let now = chrono::Utc::now().timestamp_millis();
    let state = serde_json::json!({
        "id": "inbox", "email": config.email,
        "uid_validity": inbox.uid_validity, "last_uid": inbox.last_uid,
        "last_attempt_at": now, "last_success_at": now, "last_error": null
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
    let (invoice_id, new_status) = attach_sdi_notification_to_matches(store, &notification, xml_content)
        .await?.into_iter().next().unwrap_or((None, None));
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
        return Ok(vec![attach_sdi_notification(store, notification, raw_xml, None).await?]);
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
        find_invoice_by_sdi_reference(store, nf).await?
            .or(find_invoice_by_sdi_id(store, notification.identificativo_sdi.as_deref()).await?)
    } else {
        find_invoice_by_sdi_id(store, notification.identificativo_sdi.as_deref()).await?
    };

    let matching_receipts: Vec<_> = store
        .read("sdi_notifications", "all", vec![], "")
        .await.fetch_all().await?
        .into_iter()
        .filter(|row| row.get::<String>("raw_xml") == raw_xml)
        .collect();
    let existing = matching_receipts.iter()
        .find(|row| row.get::<Option<String>>("invoice_id") == invoice_id)
        .or_else(|| invoice_id.as_ref().and_then(|_| matching_receipts.iter()
            .find(|row| row.get::<Option<String>>("invoice_id").is_none())));
    let receipt_id = existing.as_ref()
        .map(|row| row.get::<String>("id"))
        .unwrap_or(digest::content_id(&serde_json::json!({"sdi_receipt": raw_xml, "invoice_id": &invoice_id}))?);
    let invoice_id = invoice_id.or_else(|| existing.as_ref().and_then(|row| row.get("invoice_id")));

    let mut new_status = None;
    let mut tx = store.begin().await?;

    if let (Some(inv_id), Some(target)) = (invoice_id.as_ref(), notification.resulting_status()) {
        if let Ok(cur) = current_status(&tx.snapshot(), inv_id).await {
            if cur == InvoiceStatus::Imported || cur.transition_to(target).is_ok() {
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
        tx.update("sdi_notifications", &receipt_id, notification_record).await?;
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
            .file_name().and_then(|name| name.to_str()).unwrap_or("SDI_RECEIPT");
        let clean_base = base
            .strip_suffix(".xml.p7m")
            .or_else(|| base.strip_suffix(".p7m"))
            .or_else(|| base.strip_suffix(".xml"))
            .unwrap_or(base);
        let filename = format!("{clean_base}_{}_{}.xml", notification.message_type.code(), &receipt_id[..12]);
        let receipt_path = dir_path.join(&filename);
        std::fs::write(receipt_path, raw_xml).map_err(|e| e.to_string())?;
    }

    Ok((invoice_id, new_status))
}

async fn find_invoice_by_sdi_id(
    store: &crate::storage::FileStore,
    sdi_id: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(sdi_id) = sdi_id else { return Ok(None) };
    let rows = store.read("sdi_notifications", "all", vec![], "")
        .await.fetch_all().await?;
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
    let rows = store.read("sdi_notifications", "all", vec![], "")
        .await.fetch_all().await?;
    let mut processed = std::collections::HashSet::new();
    for row in rows {
        let xml: String = row.get("raw_xml");
        if !processed.insert(xml.clone()) { continue; }
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
        let filenames = row.get::<Option<Vec<String>>>("sdi_filenames").unwrap_or_default();
        if filenames.iter().any(|name| same_sdi_filename(name, reference)) {
            matched.push(id);
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
