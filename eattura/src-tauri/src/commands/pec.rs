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
use std::{sync::OnceLock, time::Duration};

use crate::app_log::{AppLog, LogLevel};
use crate::commands::invoices::build_fattura_from_store;
use crate::commands::settings::AppSettings;
use crate::pec::{self, PecConfig};
use crate::state::TauriState;
use crate::storage::{FileStore, Transaction, digest};

const SYNC_INTERVAL_MS: i64 = 300_000;
static SYNC_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
static SEND_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
const SEND_TIMEOUT: Duration = Duration::from_secs(60);

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
    let _sending = SEND_LOCK
        .try_lock()
        .map_err(|_| "È già in corso un invio PEC.")?;
    let config = build_config(&state.store).await?;
    let (message, filename) = reserve_pec_attempt(&state.store, &id, &config).await?;
    let response = match tokio::time::timeout(SEND_TIMEOUT, message.send()).await {
        Ok(Ok(response)) => response,
        Ok(Err(error)) => {
            return Err(format!("Esito PEC da verificare per {filename}: {error}"));
        }
        Err(_) => {
            return Err(format!(
                "Tempo di attesa SMTP scaduto. Verifica l'esito PEC per {filename}."
            ));
        }
    };
    resolve_pec_attempt_store(&state.store, &id, &filename, true)
        .await
        .map_err(|error| {
            format!("SMTP ha risposto positivamente per {filename}. Verifica l'archivio: {error}")
        })?;
    Ok(response)
}

async fn reserve_pec_attempt(
    store: &FileStore,
    id: &str,
    config: &PecConfig,
) -> Result<(pec::PreparedInvoice, String), String> {
    crate::commands::clients::backfill_client_destinations(store).await?;
    let mut tx = store.begin().await?;
    let row = tx
        .read("invoices", "id", vec![serde_json::json!(id)], "")
        .fetch_one()
        .await?;
    if row.get::<String>("stato") != InvoiceStatus::Validated.as_str() {
        return Err("Solo le fatture validate possono essere inviate via PEC.".into());
    }
    if row.get::<Option<String>>("pending_pec_filename").is_some() {
        return Err("Verifica l'esito del precedente tentativo PEC prima di riprovare.".into());
    }
    let mut fattura = build_fattura_from_store(&tx.snapshot(), id).await?;
    let previous_filename = sdi_filename(&fattura);
    let used_filenames = tx
        .read("invoices", "invoice_views", vec![], "")
        .fetch_all()
        .await?;
    let transmission = fattura
        .header
        .as_mut()
        .and_then(|header| header.dati_trasmissione.as_mut())
        .ok_or("Dati di trasmissione mancanti.")?;
    transmission.progressivo_invio =
        common::pec::progressivo_from_id(&uuid::Uuid::new_v4().simple().to_string());
    let filename = sdi_filename(&fattura);
    if same_sdi_filename(&previous_filename, &filename)
        || used_filenames.iter().any(|invoice| {
            invoice
                .get::<Option<String>>("pending_pec_filename")
                .is_some_and(|pending| same_sdi_filename(&pending, &filename))
                || invoice
                    .get::<Option<Vec<String>>>("sdi_filenames")
                    .unwrap_or_default()
                    .iter()
                    .any(|alias| same_sdi_filename(alias, &filename))
        })
    {
        return Err("Progressivo PEC già usato. Riprova per generarne uno nuovo.".into());
    }
    let validation = common::validation::rules::validate(&fattura);
    if !validation.is_valid() {
        return Err(format!(
            "Invio bloccato: correggi i {} errori di validazione.",
            validation.errors.len()
        ));
    }
    let xml = common::xml::encode::encode(&fattura).map_err(|e| e.to_string())?;
    let attachment = common::validation::validate_xml_attachment(&filename, xml.len());
    if let Some(error) = attachment.errors.first() {
        return Err(error.to_string());
    }
    let message = pec::prepare_invoice(config, &xml, &filename)?;
    tx.update(
        "invoices",
        id,
        serde_json::json!({
            "pending_pec_filename": filename, "pending_pec_xml": xml
        }),
    )
    .await?;
    tx.commit().await?;
    Ok((message, filename))
}

/// Resolve a previous attempt after the user checks the PEC mailbox.
#[tauri::command]
pub async fn resolve_pec_attempt(
    state: tauri::State<'_, TauriState>,
    id: String,
    filename: String,
    sent: bool,
) -> Result<(), String> {
    let _sending = SEND_LOCK
        .try_lock()
        .map_err(|_| "Attendi la fine dell'invio PEC.")?;
    resolve_pec_attempt_store(&state.store, &id, &filename, sent).await
}

async fn resolve_pec_attempt_store(
    store: &FileStore,
    id: &str,
    filename: &str,
    sent: bool,
) -> Result<(), String> {
    let mut tx = store.begin().await?;
    let row = tx
        .snapshot()
        .read("invoices", "id", vec![serde_json::json!(id)], "")
        .await
        .fetch_one()
        .await?;
    if row.get::<Option<String>>("pending_pec_filename").as_deref() != Some(filename) {
        if sent
            && recoverable_pec_filename(&tx.snapshot(), &row, id).await?.as_deref()
                == Some(filename)
        {
            let receipts = tx.snapshot()
                .read("sdi_notifications", "invoice_children", vec![serde_json::json!(id)], "")
                .await
                .fetch_all()
                .await?;
            let mut status = InvoiceStatus::Sent;
            for receipt in &receipts {
                let xml: String = receipt.get("raw_xml");
                let Ok(notification) = parse_notification(&xml) else {
                    continue;
                };
                if notification
                    .nome_file
                    .as_deref()
                    .is_some_and(|reference| same_sdi_filename(filename, reference))
                    && let Some(next) = notification.resulting_status()
                    && status.can_transition_to(next)
                {
                    status = next;
                }
            }
            tx.update(
                "invoices",
                id,
                serde_json::json!({"stato": status.as_str(), "active_pec_filename": filename}),
            )
            .await?;
            tx.commit().await?;
            return Ok(());
        }
        if sent
            && active_pec_filename(&row).is_some_and(|active| same_sdi_filename(&active, filename))
            && matches!(
                row.get::<String>("stato").as_str(),
                "sent" | "delivered" | "accepted" | "rejected"
            )
        {
            return Ok(());
        }
        return Err("Il tentativo PEC non è più in attesa di verifica.".into());
    }
    if sent {
        complete_pec_attempt(&mut tx, &row, id, filename, InvoiceStatus::Sent).await?;
    } else {
        tx.update(
            "invoices",
            id,
            serde_json::json!({
                "pending_pec_filename": null, "pending_pec_xml": null
            }),
        )
        .await?;
    }
    tx.commit().await
}

async fn complete_pec_attempt(
    tx: &mut Transaction,
    row: &crate::storage::Record,
    id: &str,
    filename: &str,
    status: InvoiceStatus,
) -> Result<(), String> {
    let mut aliases = row
        .get::<Option<Vec<String>>>("sdi_filenames")
        .unwrap_or_default();
    if !aliases
        .iter()
        .any(|alias| same_sdi_filename(alias, filename))
    {
        aliases.push(filename.into());
    }
    tx.update(
        "invoices",
        id,
        serde_json::json!({
        "stato": status.as_str(), "xml_content": row.get::<Option<String>>("pending_pec_xml")
            .ok_or("XML del tentativo PEC mancante dall'archivio.")?,
            "sdi_filenames": aliases, "active_pec_filename": filename,
            "pending_pec_filename": null, "pending_pec_xml": null
        }),
    )
    .await?;
    Ok(())
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
    let _backup_batch = store.begin_backup_batch();
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

const SDI_SCAN_VERSION: u32 = 2;

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
        "sdi_scan_complete": false,
        "sdi_scan_version": 0
    });
    if previous.is_some() {
        store.update("pec_sync", "inbox", state).await?;
    } else {
        store.insert("pec_sync", state).await?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) async fn process_inbox_messages(
    store: &crate::storage::FileStore,
    messages: &[pec::PecMessage],
) -> Result<(Vec<NotificationSummary>, Option<String>), String> {
    process_inbox_messages_with_log(store, messages, &AppLog::memory()).await
}

pub(crate) async fn process_inbox_messages_with_log(
    store: &crate::storage::FileStore,
    messages: &[pec::PecMessage],
    log: &AppLog,
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
                match common::p7m::inspect_p7m(&att.data) {
                    Ok((payload, signer)) => (payload, Some(signer)),
                    Err(e) => {
                        log.record(
                            LogLevel::Warn,
                            "pec",
                            &format!(
                                "Skipping unreadable PEC attachment {} (UID {}): {e}",
                                att.filename, message.uid
                            ),
                        );
                        continue;
                    }
                }
            } else {
                (att.data.clone(), None)
            };
            let xml = match String::from_utf8(payload) {
                Ok(xml) => xml,
                Err(e) => {
                    log.record(
                        LogLevel::Warn,
                        "pec",
                        &format!(
                            "Skipping non-UTF-8 XML attachment {} (UID {}): {e}",
                            att.filename, message.uid
                        ),
                    );
                    continue;
                }
            };
            if common::xml::decode::decode(&xml).is_ok() {
                has_sdi_payload = true;
                match crate::commands::invoices::import_xml_content_for_pec(store, &xml, signer)
                    .await
                {
                    Ok(detail) => {
                        summaries.push(NotificationSummary {
                            tipo: "FATTURA".into(),
                            identificativo_sdi: None,
                            nome_file: Some(att.filename.clone()),
                            invoice_id: Some(detail.id),
                            new_status: None,
                        });
                    }
                    Err(e) => {
                        log.record(
                            LogLevel::Warn,
                            "pec",
                            &format!(
                                "Skipping PEC invoice attachment {} (UID {}): {e}",
                                att.filename, message.uid
                            ),
                        );
                    }
                }
            } else if let Ok(notification) = parse_notification(&xml)
                && !matches!(
                    notification.message_type,
                    common::pec::SdiMessageType::Unknown(_)
                )
            {
                has_sdi_payload = true;
                match attach_sdi_notification_to_matches(store, &notification, &xml).await {
                    Ok(matches) => {
                        for (linked_id, new_status) in matches {
                            summaries.push(NotificationSummary {
                                tipo: notification.message_type.code().to_string(),
                                identificativo_sdi: notification.identificativo_sdi.clone(),
                                nome_file: notification.nome_file.clone(),
                                invoice_id: linked_id,
                                new_status,
                            });
                        }
                    }
                    Err(e) => {
                        log.record(
                            LogLevel::Warn,
                            "pec",
                            &format!(
                                "Skipping PEC receipt attachment {} (UID {}): {e}",
                                att.filename, message.uid
                            ),
                        );
                    }
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
        let scan_version = row.get::<Option<u32>>("sdi_scan_version").unwrap_or(0);
        if !scan_complete || scan_version < SDI_SCAN_VERSION {
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
    let (mut summaries, newly_detected) =
        process_inbox_messages_with_log(store, &inbox.messages, log).await?;

    let detected_sdi_address = select_preferred_sdi_address(
        newly_detected
            .into_iter()
            .chain(previous_detected.into_iter()),
    );

    if let Some(ref detected) = detected_sdi_address {
        persist_detected_sdi_address(store, detected).await?;
    }

    if cursor.is_none() || previous.is_none() || !summaries.is_empty() {
        log.record(
            LogLevel::Info,
            "pec",
            "Linking SDI receipts to imported invoices",
        );
        for reconciled in reconcile_unlinked_notifications(store).await? {
            if !summaries.iter().any(|existing| {
                existing.invoice_id == reconciled.invoice_id
                    && existing.tipo == reconciled.tipo
                    && existing.new_status == reconciled.new_status
            }) {
                summaries.push(reconciled);
            }
        }
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
        "sdi_scan_complete": true,
        "sdi_scan_version": SDI_SCAN_VERSION
    });
    if previous.is_some() {
        store.update("pec_sync", "inbox", state).await?;
    } else {
        store.insert("pec_sync", state).await?;
    }

    Ok(summaries)
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

    let mut tx = store.begin().await?;
    let snapshot = tx.snapshot();
    let matching_receipts: Vec<_> = snapshot
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
    let invoice = if let Some(inv_id) = invoice_id.as_deref() {
        snapshot
            .read("invoices", "id", vec![serde_json::json!(inv_id)], "")
            .await
            .fetch_optional()
            .await?
    } else {
        None
    };
    if let (Some(inv_id), Some(row)) = (invoice_id.as_deref(), invoice) {
        if let Some(pending) = row.get::<Option<String>>("pending_pec_filename") {
            if !matches!(
                notification.message_type,
                common::pec::SdiMessageType::Unknown(_)
            ) && notification
                .nome_file
                .as_deref()
                .is_some_and(|filename| same_sdi_filename(&pending, filename))
            {
                let target = notification
                    .resulting_status()
                    .unwrap_or(InvoiceStatus::Sent);
                complete_pec_attempt(&mut tx, &row, inv_id, &pending, target).await?;
                new_status = Some(target.as_str().to_string());
            }
        } else if let Some(target) = notification.resulting_status()
            && let Ok(cur) = row.get::<String>("stato").parse::<InvoiceStatus>()
        {
            let active = active_pec_filename(&row);
            let matches_attempt = match active.as_deref() {
                Some(active) => {
                    if let Some(filename) = notification.nome_file.as_deref() {
                        same_sdi_filename(active, filename)
                    } else {
                        matches_sdi_id_for_attempt(&snapshot, inv_id, active, notification)
                            .await?
                    }
                }
                None => cur == InvoiceStatus::Imported
                    || notification.nome_file.as_deref().is_some_and(|filename| {
                        let aliases = row.get::<Option<Vec<String>>>("sdi_filenames").unwrap_or_default();
                        aliases.len() == 1 && same_sdi_filename(&aliases[0], filename)
                    }),
            };
            let legacy_rc_upgrade = cur == InvoiceStatus::Accepted
                && matches!(target, InvoiceStatus::Delivered | InvoiceStatus::Rejected)
                && matches_attempt
                && !has_attempt_pa_acceptance(&snapshot, inv_id, active.as_deref()).await?;
            let can_apply = cur.transition_to(target).is_ok()
                || legacy_rc_upgrade
                || (matches!(cur, InvoiceStatus::Draft | InvoiceStatus::Rejected)
                    && matches_attempt
                    && recoverable_pec_filename(&snapshot, &row, inv_id).await?.is_some());
            if matches_attempt && can_apply {
                tx.update(
                    "invoices",
                    inv_id,
                    serde_json::json!({"stato": target.as_str(),
                                "imported_from_xml": cur == InvoiceStatus::Imported}),
                )
                .await?;
                new_status = Some(target.as_str().to_string());
            }
        }
    }

    let errori_json = serde_json::to_string(&notification.errors).map_err(|e| e.to_string())?;

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

/// Revisit stored receipts to link newly imported invoices and upgrade legacy statuses.
pub(crate) async fn reconcile_unlinked_notifications(
    store: &crate::storage::FileStore,
) -> Result<Vec<NotificationSummary>, String> {
    let rows = store
        .read("sdi_notifications", "all", vec![], "")
        .await
        .fetch_all()
        .await?;
    let mut processed = std::collections::HashSet::new();
    let mut updated = Vec::new();
    for row in rows {
        let xml: String = row.get("raw_xml");
        if !processed.insert(xml.clone()) {
            continue;
        }
        if let Ok(notification) = parse_notification(&xml) {
            for (linked_id, new_status) in
                attach_sdi_notification_to_matches(store, &notification, &xml).await?
            {
                if new_status.is_some() {
                    updated.push(NotificationSummary {
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
    Ok(updated)
}

pub(crate) fn filter_attempt_receipts<'a>(
    receipts: &'a [crate::storage::Record],
    active: Option<&str>,
) -> Vec<&'a crate::storage::Record> {
    let attempt_sdi_ids: std::collections::HashSet<String> = match active {
        Some(active_name) => receipts
            .iter()
            .filter(|row| {
                row.get::<Option<String>>("nome_file")
                    .is_some_and(|name| same_sdi_filename(active_name, &name))
            })
            .filter_map(|row| row.get::<Option<String>>("identificativo_sdi"))
            .collect(),
        None => std::collections::HashSet::new(),
    };
    receipts
        .iter()
        .filter(|row| match active {
            Some(active_name) => match row.get::<Option<String>>("nome_file") {
                Some(name) => same_sdi_filename(active_name, &name),
                None => row
                    .get::<Option<String>>("identificativo_sdi")
                    .is_some_and(|id| attempt_sdi_ids.contains(&id)),
            },
            None => true,
        })
        .collect()
}

pub(crate) fn records_have_attempt_mc(
    receipts: &[crate::storage::Record],
    active: Option<&str>,
) -> bool {
    filter_attempt_receipts(receipts, active)
        .into_iter()
        .any(|row| row.get::<String>("tipo") == "MC")
}

pub(crate) async fn has_attempt_pa_refusal(
    store: &FileStore,
    row: &crate::storage::Record,
    invoice_id: &str,
) -> Result<bool, String> {
    let receipts = store
        .read(
            "sdi_notifications",
            "invoice_children",
            vec![serde_json::json!(invoice_id)],
            "",
        )
        .await
        .fetch_all()
        .await?;
    let active = active_pec_filename(row);
    Ok(filter_attempt_receipts(&receipts, active.as_deref())
        .into_iter()
        .any(|receipt| {
            let tipo: String = receipt.get("tipo");
            tipo == "RC"
                || (tipo == "NE"
                    && receipt.get::<Option<String>>("esito").as_deref() == Some("EC02"))
        }))
}

async fn has_attempt_pa_acceptance(
    store: &FileStore,
    invoice_id: &str,
    active: Option<&str>,
) -> Result<bool, String> {
    let receipts = store
        .read(
            "sdi_notifications",
            "invoice_children",
            vec![serde_json::json!(invoice_id)],
            "",
        )
        .await
        .fetch_all()
        .await?;
    Ok(filter_attempt_receipts(&receipts, active)
        .into_iter()
        .any(|row| {
            let tipo: String = row.get("tipo");
            tipo == "DT"
                || (tipo == "NE"
                    && row.get::<Option<String>>("esito").as_deref() == Some("EC01"))
        }))
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
        if row
            .get::<Option<String>>("pending_pec_filename")
            .is_some_and(|pending| same_sdi_filename(&pending, reference))
            || filenames
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

pub(crate) fn active_pec_filename(row: &crate::storage::Record) -> Option<String> {
    row.get::<Option<String>>("active_pec_filename")
        .or_else(|| stored_xml_filename(row))
}

fn stored_xml_filename(row: &crate::storage::Record) -> Option<String> {
    // Before active attempts were stored explicitly, the sent XML held the progressivo.
    let xml: String = row.get::<Option<String>>("xml_content")?;
    let invoice = common::xml::decode::decode(&xml).ok()?;
    Some(sdi_filename(&invoice))
}

/// A wrongly rectified resend can be restored only when its transmitted XML is still available.
pub(crate) async fn recoverable_pec_filename(
    store: &FileStore,
    row: &crate::storage::Record,
    invoice_id: &str,
) -> Result<Option<String>, String> {
    if !matches!(row.get::<String>("stato").as_str(), "draft" | "rejected")
        || row.get::<Option<String>>("pending_pec_filename").is_some()
    {
        return Ok(None);
    }
    let aliases = row.get::<Option<Vec<String>>>("sdi_filenames").unwrap_or_default();
    let Some(filename) = active_pec_filename(row) else {
        return Ok(None);
    };
    let Some(active_index) = aliases.iter().position(|alias| same_sdi_filename(alias, &filename)) else {
        return Ok(None);
    };
    if !stored_xml_filename(row).is_some_and(|stored| same_sdi_filename(&stored, &filename)) {
        return Ok(None);
    }
    let receipts = store
        .read("sdi_notifications", "invoice_children", vec![serde_json::json!(invoice_id)], "")
        .await
        .fetch_all()
        .await?;
    let mut earlier_rejection = false;
    for receipt in receipts {
        if !matches!(receipt.get::<String>("tipo").as_str(), "NS" | "NE") {
            continue;
        }
        if receipt.get::<String>("tipo") == "NE"
            && receipt.get::<Option<String>>("esito").as_deref() != Some("EC02")
        {
            continue;
        }
        let Some(reference) = receipt.get::<Option<String>>("nome_file") else {
            return Ok(None);
        };
        if same_sdi_filename(&filename, &reference) {
            return Ok(None);
        }
        earlier_rejection |= aliases[..active_index]
            .iter()
            .any(|alias| same_sdi_filename(alias, &reference));
    }
    Ok(earlier_rejection.then_some(filename))
}

async fn matches_sdi_id_for_attempt(
    store: &FileStore,
    invoice_id: &str,
    active: &str,
    notification: &common::pec::ParsedNotification,
) -> Result<bool, String> {
    let Some(sdi_id) = notification.identificativo_sdi.as_deref() else {
        return Ok(false);
    };
    let receipts = store
        .read("sdi_notifications", "invoice_children", vec![serde_json::json!(invoice_id)], "")
        .await
        .fetch_all()
        .await?;
    Ok(receipts.iter().any(|row| {
        row.get::<Option<String>>("identificativo_sdi").as_deref() == Some(sdi_id)
            && row.get::<Option<String>>("nome_file")
                .is_some_and(|filename| same_sdi_filename(active, &filename))
    }))
}

pub(crate) fn same_sdi_filename(left: &str, right: &str) -> bool {
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
    async fn oversized_xml_is_rejected_before_reserving_pec_attempt() {
        use crate::commands::invoices::{create_invoice_store, set_invoice_status_store};
        use base64::Engine as _;
        use serde_json::json;

        let directory = tempfile::tempdir().unwrap();
        let store = FileStore::open(directory.path().join("eattura.json")).unwrap();
        for (id, vat) in [("seller", "01234567897"), ("buyer", "98765432103")] {
            store
                .insert("clients", json!({
                    "id": id, "id_paese": "IT", "id_codice": vat, "denominazione": id,
                    "indirizzo": "Via Roma", "cap": "00100", "comune": "Roma",
                    "nazione": "IT", "regime_fiscale": "RF01"
                }))
                .await
                .unwrap();
        }
        let request = serde_json::from_value(json!({
            "numero": "1", "data": "2024-01-01", "tipoDocumento": "TD01", "divisa": "EUR",
            "cedenteId": "seller", "cessionarioId": "buyer", "pagamenti": [],
            "linee": [{"descrizione": "Consulting", "quantita": 1, "prezzoUnitario": 100, "aliquotaIva": 22}]
        })).unwrap();
        let invoice = create_invoice_store(&store, request).await.unwrap();
        set_invoice_status_store(&store, &invoice.id, "validated").await.unwrap();
        let size_bytes = 3_800_000;
        let attachment = crate::commands::invoices::AttachmentDetail {
            nome_attachment: "attachment.bin".into(),
            size_bytes,
            attachment_base64: base64::engine::general_purpose::STANDARD.encode(vec![0; size_bytes]),
            ..Default::default()
        };
        store
            .update("invoices", &invoice.id, json!({"allegati": [attachment]}))
            .await
            .unwrap();

        let config = PecConfig {
            email: "test@example.com".into(),
            password: String::new(),
            sdi_address: String::new(),
            smtp_host: "localhost".into(),
            smtp_port: 465,
            imap_host: "localhost".into(),
            imap_port: 993,
        };
        let error = reserve_pec_attempt(&store, &invoice.id, &config)
            .await
            .err()
            .expect("PEC must reject the oversized encoded XML");
        assert!(error.contains("[00003]"), "{error}");
        let detail = crate::commands::invoices::fetch_invoice_detail(&store, &invoice.id)
            .await
            .unwrap();
        assert_eq!(detail.pending_pec_filename, None);
    }

    #[tokio::test]
    async fn pending_send_survives_restart_and_blocks_resend_until_reconciled() {
        use crate::commands::invoices::{
            create_invoice_store, fetch_invoice_detail, set_invoice_status_store,
            validate_invoice_store,
        };
        use serde_json::json;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("eattura.json");
        let store = FileStore::open(path.clone()).unwrap();
        for (id, vat) in [("seller", "01234567897"), ("buyer", "98765432103")] {
            store
                .insert(
                    "clients",
                    json!({
                        "id": id, "id_paese": "IT", "id_codice": vat, "denominazione": id,
                        "indirizzo": "Via Roma", "cap": "00100", "comune": "Roma",
                        "nazione": "IT", "regime_fiscale": "RF01", "codice_destinatario": "ABC1234"
                    }),
                )
                .await
                .unwrap();
        }
        let mut request = json!({
            "numero": "1", "data": "2026-01-01", "tipoDocumento": "TD01", "divisa": "EUR",
            "cedenteId": "seller", "cessionarioId": "buyer", "pagamenti": [],
            "linee": [{"descrizione": "Aggiornamento dell’immagine", "quantita": 1, "prezzoUnitario": 100, "aliquotaIva": 22}]
        });
        let error = create_invoice_store(&store, serde_json::from_value(request.clone()).unwrap())
            .await
            .expect_err("Draft creation must reject the invalid description");
        assert!(error.contains("U+2019"));
        request["linee"][0]["descrizione"] = json!("Aggiornamento dell'immagine");
        let invoice = create_invoice_store(&store, serde_json::from_value(request).unwrap())
            .await
            .unwrap();
        let line_id: String = store
            .read("invoice_lines", "invoice_children", vec![json!(invoice.id)], "line")
            .await
            .fetch_one()
            .await
            .unwrap()
            .get("id");
        // Simulate a saved draft from an earlier version.
        store
            .update(
                "invoice_lines",
                &line_id,
                json!({"descrizione": "Aggiornamento dell’immagine"}),
            )
            .await
            .unwrap();
        let config = PecConfig {
            email: "test@example.com".into(),
            password: String::new(),
            sdi_address: String::new(),
            smtp_host: "localhost".into(),
            smtp_port: 465,
            imap_host: "localhost".into(),
            imap_port: 993,
        };
        assert_eq!(invoice.codice_destinatario.as_deref(), Some("ABC1234"));
        let validation = validate_invoice_store(&store, &invoice.id).await.unwrap();
        assert!(!validation.valid);
        assert!(validation.errors.iter().any(|error| error.code == "00200"));
        assert!(
            set_invoice_status_store(&store, &invoice.id, "validated")
                .await
                .is_err()
        );
        // Simulate an invoice validated before the description check was introduced.
        store
            .update("invoices", &invoice.id, json!({"stato": "validated"}))
            .await
            .unwrap();
        let error = reserve_pec_attempt(&store, &invoice.id, &config)
            .await
            .err()
            .expect("PEC must reject a previously validated invoice with an invalid description");
        assert!(error.contains("errori di validazione"));
        let detail = fetch_invoice_detail(&store, &invoice.id).await.unwrap();
        assert_eq!(detail.pending_pec_filename, None);
        set_invoice_status_store(&store, &invoice.id, "draft")
            .await
            .unwrap();
        store
            .update(
                "invoice_lines",
                &line_id,
                json!({"descrizione": "Aggiornamento dell'immagine"}),
            )
            .await
            .unwrap();
        assert!(
            reserve_pec_attempt(&store, &invoice.id, &config)
                .await
                .is_err()
        );
        set_invoice_status_store(&store, &invoice.id, "validated")
            .await
            .unwrap();
        let (first, second) = tokio::join!(
            reserve_pec_attempt(&store, &invoice.id, &config),
            reserve_pec_attempt(&store, &invoice.id, &config),
        );
        assert_ne!(first.is_ok(), second.is_ok());
        let (_, filename) = first.or(second).unwrap();
        assert!(
            !std::fs::read_to_string(directory.path().join("index.json"))
                .unwrap()
                .contains("pending_pec_xml")
        );
        drop(store);

        let reopened = FileStore::open(path).unwrap();
        assert!(
            reserve_pec_attempt(&reopened, &invoice.id, &config)
                .await
                .is_err()
        );
        assert!(
            set_invoice_status_store(&reopened, &invoice.id, "draft")
                .await
                .is_err()
        );
        assert!(
            resolve_pec_attempt_store(&reopened, &invoice.id, "different.xml", false)
                .await
                .is_err()
        );
        resolve_pec_attempt_store(&reopened, &invoice.id, &filename, false)
            .await
            .unwrap();
        let (_, retry_filename) = reserve_pec_attempt(&reopened, &invoice.id, &config)
            .await
            .unwrap();
        assert_ne!(filename, retry_filename);
        assert_eq!(
            retry_filename.split_once('_').unwrap().1.len(),
            "00000.xml".len()
        );
        assert!(
            resolve_pec_attempt_store(&reopened, &invoice.id, &filename, true)
                .await
                .is_err()
        );
        let receipt = format!(
            "<RicevutaScarto><IdentificativoSdI>111</IdentificativoSdI><NomeFile>{retry_filename}</NomeFile></RicevutaScarto>"
        );
        let result = import_sdi_receipt_store(&reopened, &receipt).await.unwrap();
        assert_eq!(result.new_status.as_deref(), Some("rejected"));
        resolve_pec_attempt_store(&reopened, &invoice.id, &retry_filename, true)
            .await
            .unwrap();
        let detail = fetch_invoice_detail(&reopened, &invoice.id).await.unwrap();
        assert_eq!(detail.stato, "rejected");
        assert_eq!(detail.pending_pec_filename, None);
        let sent = build_fattura_from_store(&reopened, &invoice.id)
            .await
            .unwrap();
        assert_eq!(sdi_filename(&sent), retry_filename);

        use crate::commands::invoices::rectify_rejected_invoice_store;
        let rectified = rectify_rejected_invoice_store(&reopened, &invoice.id).await.unwrap();
        assert_eq!(rectified.recoverable_pec_filename, None);
        set_invoice_status_store(&reopened, &invoice.id, "validated").await.unwrap();
        let (_, corrected_filename) = reserve_pec_attempt(&reopened, &invoice.id, &config)
            .await
            .unwrap();
        resolve_pec_attempt_store(&reopened, &invoice.id, &corrected_filename, true)
            .await
            .unwrap();
        assert_ne!(corrected_filename, retry_filename);

        // Exercise legacy records and a full replay of historical receipts.
        reopened.update("invoices", &invoice.id, json!({"active_pec_filename": null}))
            .await.unwrap();
        reconcile_unlinked_notifications(&reopened).await.unwrap();
        let replayed = import_sdi_receipt_store(&reopened, &receipt).await.unwrap();
        assert_eq!(replayed.new_status, None);
        let detail = fetch_invoice_detail(&reopened, &invoice.id).await.unwrap();
        assert_eq!(detail.stato, "sent");
        assert_eq!(detail.notifications.len(), 1);

        // Restore a resend already pushed back to draft by the old behavior.
        reopened.update("invoices", &invoice.id, json!({"stato": "draft"}))
            .await.unwrap();
        let draft = fetch_invoice_detail(&reopened, &invoice.id).await.unwrap();
        assert_eq!(draft.recoverable_pec_filename.as_deref(), Some(corrected_filename.as_str()));
        assert!(set_invoice_status_store(&reopened, &invoice.id, "validated").await.is_err());
        assert!(resolve_pec_attempt_store(&reopened, &invoice.id, &retry_filename, true).await.is_err());
        resolve_pec_attempt_store(&reopened, &invoice.id, &corrected_filename, true)
            .await.unwrap();
        assert_eq!(fetch_invoice_detail(&reopened, &invoice.id).await.unwrap().stato, "sent");
        let outcome = format!("<RicevutaConsegna><IdentificativoSdI>222</IdentificativoSdI><NomeFile>{corrected_filename}</NomeFile></RicevutaConsegna>");
        let result = import_sdi_receipt_store(&reopened, &outcome).await.unwrap();
        assert_eq!(result.new_status.as_deref(), Some("delivered"));
        let detail = fetch_invoice_detail(&reopened, &invoice.id).await.unwrap();
        assert_eq!(detail.notifications.len(), 2);

        // The actual outcome can also recover a draft without manual confirmation.
        reopened.update("invoices", &invoice.id, json!({"stato": "draft"}))
            .await.unwrap();
        let replayed = import_sdi_receipt_store(&reopened, &receipt).await.unwrap();
        assert_eq!(replayed.new_status, None);
        let result = import_sdi_receipt_store(&reopened, &outcome).await.unwrap();
        assert_eq!(result.new_status.as_deref(), Some("delivered"));
    }

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

    #[tokio::test]
    async fn upgrades_legacy_accepted_invoice_to_delivered_and_attaches_b2b_mc() {
        use crate::commands::invoices::{
            create_invoice_store, fetch_invoice_detail, set_invoice_status_store,
        };
        use serde_json::json;

        async fn create_test_fixture() -> (tempfile::TempDir, FileStore, crate::commands::invoices::InvoiceDetail) {
            let directory = tempfile::tempdir().unwrap();
            let store = FileStore::open(directory.path().join("eattura.json")).unwrap();
            for (id, vat) in [("seller", "01234567897"), ("buyer", "98765432103")] {
                store
                    .insert(
                        "clients",
                        json!({
                            "id": id, "id_paese": "IT", "id_codice": vat, "denominazione": id,
                            "indirizzo": "Via Roma", "cap": "00100", "comune": "Roma",
                            "nazione": "IT", "regime_fiscale": "RF01", "codice_destinatario": "ABC1234"
                        }),
                    )
                    .await
                    .unwrap();
            }
            let request = serde_json::from_value(json!({
                "numero": "1", "data": "2026-01-01", "tipoDocumento": "TD01", "divisa": "EUR",
                "cedenteId": "seller", "cessionarioId": "buyer", "pagamenti": [],
                "linee": [{"descrizione": "Consulenza", "quantita": 1, "prezzoUnitario": 100, "aliquotaIva": 22}]
            }))
            .unwrap();
            let invoice = create_invoice_store(&store, request).await.unwrap();
            (directory, store, invoice)
        }

        let config = PecConfig {
            email: "studio@pec.example.it".into(),
            password: String::new(),
            sdi_address: String::new(),
            smtp_host: "localhost".into(),
            smtp_port: 465,
            imap_host: "localhost".into(),
            imap_port: 993,
        };

        let (_dir, store, invoice) = create_test_fixture().await;
        set_invoice_status_store(&store, &invoice.id, "validated")
            .await
            .unwrap();
        let (_, filename) = reserve_pec_attempt(&store, &invoice.id, &config)
            .await
            .unwrap();
        resolve_pec_attempt_store(&store, &invoice.id, &filename, true)
            .await
            .unwrap();

        // Simulate an invoice that was marked `accepted` by an earlier version on `RC`.
        let rc_xml = format!(
            "<RicevutaConsegna><IdentificativoSdI>777111</IdentificativoSdI><NomeFile>{filename}</NomeFile></RicevutaConsegna>"
        );
        import_sdi_receipt_store(&store, &rc_xml).await.unwrap();
        store
            .update("invoices", &invoice.id, json!({"stato": "accepted"}))
            .await
            .unwrap();
        assert_eq!(
            fetch_invoice_detail(&store, &invoice.id).await.unwrap().stato,
            "accepted"
        );

        // Reconciling or re-scanning PEC upgrades legacy `accepted` -> `delivered`.
        let upgraded = reconcile_unlinked_notifications(&store).await.unwrap();
        assert_eq!(upgraded.len(), 1);
        assert_eq!(upgraded[0].new_status.as_deref(), Some("delivered"));
        assert_eq!(
            fetch_invoice_detail(&store, &invoice.id).await.unwrap().stato,
            "delivered"
        );

        // A PA invoice that later receives NE EC01 transitions `delivered` -> `accepted`
        // and stays `accepted` even if `RC` is replayed again.
        let ne_accept = format!(
            "<NotificaEsito><IdentificativoSdI>777111</IdentificativoSdI><NomeFile>{filename}</NomeFile><Esito>EC01</Esito></NotificaEsito>"
        );
        let ne_res = import_sdi_receipt_store(&store, &ne_accept).await.unwrap();
        assert_eq!(ne_res.new_status.as_deref(), Some("accepted"));
        let replay_rc = import_sdi_receipt_store(&store, &rc_xml).await.unwrap();
        assert_eq!(replay_rc.new_status, None);
        assert_eq!(
            fetch_invoice_detail(&store, &invoice.id).await.unwrap().stato,
            "accepted"
        );

        // Create a second invoice to verify B2B `RicevutaImpossibilitaRecapito` (`MC`)
        // and PA `RC -> NE EC02` (`delivered -> rejected`).
        let (_dir2, store2, invoice2) = create_test_fixture().await;
        set_invoice_status_store(&store2, &invoice2.id, "validated")
            .await
            .unwrap();
        let (_, filename2) = reserve_pec_attempt(&store2, &invoice2.id, &config)
            .await
            .unwrap();
        resolve_pec_attempt_store(&store2, &invoice2.id, &filename2, true)
            .await
            .unwrap();

        let b2b_mc = format!(
            r#"<ns2:RicevutaImpossibilitaRecapito xmlns:ns2="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fattura/messaggi/v1.0" versione="1.0">
                <IdentificativoSdI>888222</IdentificativoSdI>
                <NomeFile>{filename2}</NomeFile>
                <DataOraRicezione>2026-06-16T10:00:00.000+02:00</DataOraRicezione>
                <DataMessaADisposizione>2026-06-16</DataMessaADisposizione>
                <Descrizione>Impossibilita' di recapito</Descrizione>
            </ns2:RicevutaImpossibilitaRecapito>"#
        );
        let messages = vec![PecMessage {
            uid: 10,
            subject: "POSTA CERTIFICATA: Invio File".into(),
            from: "posta-certificata@pec.aruba.it".into(),
            sdi_sender: Some("sdi11@pec.fatturapa.it".into()),
            sdi_fallback: None,
            attachments: vec![PecAttachment {
                filename: format!(
                    "{}_MC_001.xml",
                    filename2.trim_end_matches(".xml")
                ),
                data: b2b_mc.as_bytes().to_vec(),
            }],
        }];
        let (summaries, _) = process_inbox_messages(&store2, &messages).await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].tipo, "MC");
        assert_eq!(summaries[0].invoice_id.as_deref(), Some(invoice2.id.as_str()));
        let detail2 = fetch_invoice_detail(&store2, &invoice2.id).await.unwrap();
        assert_eq!(detail2.stato, "sent");
        assert!(detail2.has_mc_notice);
        assert_eq!(detail2.notifications.len(), 1);
        assert_eq!(detail2.notifications[0].tipo, "MC");
        let summaries2 = crate::commands::invoices::list_invoices_store(&store2)
            .await
            .unwrap();
        assert!(
            summaries2
                .iter()
                .find(|inv| inv.id == invoice2.id)
                .unwrap()
                .has_mc_notice
        );

        // Now verify PA `delivered` -> `rejected` on NE EC02 and ensure draft rectification is blocked.
        let rc2 = format!(
            "<RicevutaConsegna><IdentificativoSdI>888222</IdentificativoSdI><NomeFile>{filename2}</NomeFile></RicevutaConsegna>"
        );
        assert_eq!(
            import_sdi_receipt_store(&store2, &rc2)
                .await
                .unwrap()
                .new_status
                .as_deref(),
            Some("delivered")
        );
        let ne_refuse = format!(
            "<NotificaEsito><IdentificativoSdI>888222</IdentificativoSdI><NomeFile>{filename2}</NomeFile><Esito>EC02</Esito></NotificaEsito>"
        );
        assert_eq!(
            import_sdi_receipt_store(&store2, &ne_refuse)
                .await
                .unwrap()
                .new_status
                .as_deref(),
            Some("rejected")
        );
        let rectify_err =
            crate::commands::invoices::rectify_rejected_invoice_store(&store2, &invoice2.id)
                .await
                .unwrap_err();
        assert!(rectify_err.contains("NE EC02"));
        assert!(
            set_invoice_status_store(&store2, &invoice2.id, "draft")
                .await
                .is_err()
        );
        assert_eq!(
            fetch_invoice_detail(&store2, &invoice2.id)
                .await
                .unwrap()
                .stato,
            "rejected"
        );
    }

    #[tokio::test]
    async fn process_inbox_messages_logs_bad_attachments_and_continues_batch() {
        use crate::commands::invoices::{
            create_invoice_store, fetch_invoice_detail, set_invoice_status_store,
        };
        use serde_json::json;

        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::open(dir.path().join("eattura.json")).unwrap();
        let log_dir = dir.path().join("logs");
        let log = AppLog::open(log_dir.clone()).unwrap();

        for (id, vat) in [("seller", "01234567897"), ("buyer", "98765432103")] {
            store
                .insert(
                    "clients",
                    json!({
                        "id": id,
                        "id_paese": "IT",
                        "id_codice": vat,
                        "denominazione": id,
                        "indirizzo": "Via Roma 1",
                        "cap": "00100",
                        "comune": "Roma",
                        "nazione": "IT",
                        "regime_fiscale": "RF01"
                    }),
                )
                .await
                .unwrap();
        }
        let req = serde_json::from_value(json!({
            "numero": "1",
            "data": "2026-03-01",
            "tipoDocumento": "TD01",
            "divisa": "EUR",
            "cedenteId": "seller",
            "cessionarioId": "buyer",
            "pagamenti": [],
            "linee": [{"descrizione": "Servizio", "quantita": 1, "prezzoUnitario": 100.0, "aliquotaIva": 22.0}]
        }))
        .unwrap();
        let invoice = create_invoice_store(&store, req).await.unwrap();
        set_invoice_status_store(&store, &invoice.id, "validated")
            .await
            .unwrap();
        let config = crate::pec::PecConfig {
            email: "pec@example.it".into(),
            password: "secret".into(),
            imap_host: "imap.example.it".into(),
            imap_port: 993,
            smtp_host: "smtp.example.it".into(),
            smtp_port: 465,
            sdi_address: "sdi01@pec.fatturapa.it".into(),
        };
        let (_, filename) = reserve_pec_attempt(&store, &invoice.id, &config)
            .await
            .unwrap();
        resolve_pec_attempt_store(&store, &invoice.id, &filename, true)
            .await
            .unwrap();

        let valid_rc = format!(
            "<RicevutaConsegna><IdentificativoSdI>999001</IdentificativoSdI><NomeFile>{filename}</NomeFile></RicevutaConsegna>"
        );
        let messages = vec![
            PecMessage {
                uid: 1,
                subject: "Old broken P7M".into(),
                from: "archive@pec.example.it".into(),
                sdi_sender: None,
                sdi_fallback: None,
                attachments: vec![PecAttachment {
                    filename: "IT01234567897_BROKEN.xml.p7m".into(),
                    data: b"corrupted-der-bytes".to_vec(),
                }],
            },
            PecMessage {
                uid: 2,
                subject: "Old Latin-1 XML".into(),
                from: "archive@pec.example.it".into(),
                sdi_sender: None,
                sdi_fallback: None,
                attachments: vec![PecAttachment {
                    filename: "IT01234567897_LATIN1.xml".into(),
                    data: b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><Fattura>Caff\xe8</Fattura>"
                        .to_vec(),
                }],
            },
            PecMessage {
                uid: 3,
                subject: "POSTA CERTIFICATA: Ricevuta di consegna".into(),
                from: "posta-certificata@pec.aruba.it".into(),
                sdi_sender: Some("sdi09@pec.fatturapa.it".into()),
                sdi_fallback: None,
                attachments: vec![PecAttachment {
                    filename: format!("{}_RC_001.xml", filename.trim_end_matches(".xml")),
                    data: valid_rc.into_bytes(),
                }],
            },
        ];

        let (summaries, detected) =
            process_inbox_messages_with_log(&store, &messages, &log)
                .await
                .unwrap();
        assert_eq!(detected.as_deref(), Some("sdi09@pec.fatturapa.it"));
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].tipo, "RC");
        assert_eq!(summaries[0].new_status.as_deref(), Some("delivered"));
        assert_eq!(
            fetch_invoice_detail(&store, &invoice.id).await.unwrap().stato,
            "delivered"
        );

        let log_entries: Vec<serde_json::Value> =
            std::fs::read_to_string(log_dir.join("events.jsonl"))
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        assert_eq!(log_entries.len(), 2);
        assert!(log_entries.iter().all(|entry| entry["level"] == "warn"));
        assert!(
            log_entries[0]["message"]
                .as_str()
                .unwrap()
                .contains("IT01234567897_BROKEN.xml.p7m")
        );
        assert!(
            log_entries[1]["message"]
                .as_str()
                .unwrap()
                .contains("IT01234567897_LATIN1.xml")
        );
    }
}
