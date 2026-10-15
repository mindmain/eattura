use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::commands::invoices::{export_invoice_xml_store, import_xml_content, list_invoices_store};
use crate::storage::FileStore;
use crate::TauriState;
use common::crypto::{self, EncryptedEnvelope};
use common::sync::{SyncInvoiceItem, SyncPacket};

const SYNC_SALT: &str = "eattura_vault_sync_salt_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    pub relay_url: String,
    pub passphrase: String,
    pub device_id: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusDto {
    pub topic: String,
    pub pending_messages: usize,
    pub server_healthy: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPushResult {
    pub success: bool,
    pub invoices_pushed: usize,
    pub clients_pushed: usize,
    pub topic: String,
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPullResult {
    pub success: bool,
    pub envelopes_received: usize,
    pub invoices_processed: usize,
    pub clients_processed: usize,
    pub errors: Vec<String>,
}

/// Generate a human-friendly multi-device pairing code / passphrase.
#[tauri::command]
pub fn generate_sync_pairing_code() -> String {
    const WORDS: &[&str] = &[
        "aquila", "bosco", "castello", "colle", "drago", "falco", "faro", "fiume",
        "foresta", "gabbiano", "isola", "lago", "luna", "mare", "monte", "notte",
        "nuvola", "onda", "pietra", "ponte", "quercia", "roccia", "sole", "spada",
        "stella", "terra", "torre", "valle", "vento", "vulcano",
    ];

    let u = uuid::Uuid::new_v4();
    let random_bytes = u.as_bytes();

    let w1 = WORDS[(random_bytes[0] as usize) % WORDS.len()];
    let w2 = WORDS[(random_bytes[1] as usize) % WORDS.len()];
    let w3 = WORDS[(random_bytes[2] as usize) % WORDS.len()];
    let num = 1000 + ((random_bytes[3] as u32) * 35) % 9000;

    format!("{w1}-{w2}-{w3}-{num}")
}

/// Build HTTP headers with optional API key.
fn build_headers(api_key: &Option<String>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(key) = api_key {
        if let Ok(val) = HeaderValue::from_str(key) {
            headers.insert("x-api-key", val);
        }
    }
    headers
}

/// Helper to gather all SDI notifications for an invoice.
async fn fetch_sdi_receipts_for_invoice(
    store: &FileStore,
    invoice_id: &str,
) -> Vec<(String, String)> {
    let Ok(rows) = store
        .read(
            "sdi_notifications",
            "invoice_children",
            vec![json!(invoice_id)],
            "",
        )
        .await
        .fetch_all()
        .await
    else {
        return Vec::new();
    };

    rows.into_iter()
        .filter_map(|r| {
            let tipo: String = r.get("tipo");
            let raw_xml: String = r.get("raw_xml");
            if !raw_xml.is_empty() {
                Some((tipo, raw_xml))
            } else {
                None
            }
        })
        .collect()
}

/// Build a SyncPacket from the local store.
pub async fn build_sync_packet(store: &FileStore, device_id: &str) -> Result<SyncPacket, String> {
    let mut packet = SyncPacket::new(device_id.to_string());

    // 1. Gather all invoices
    let summaries = list_invoices_store(store).await?;
    for s in summaries {
        if let Ok(xml_content) = export_invoice_xml_store(store, &s.id).await {
            let notifs = fetch_sdi_receipts_for_invoice(store, &s.id).await;
            packet.invoices.push(SyncInvoiceItem {
                id: s.id,
                numero: s.numero,
                data: s.data,
                tipo_documento: s.tipo_documento,
                direction: s.direction,
                xml_content,
                sdi_notifications: notifs,
            });
        }
    }

    // 2. Gather all clients
    if let Ok(clients) = store.read("clients", "all", vec![], "").await.fetch_all().await {
        for c in clients {
            if let Ok(v) = serde_json::to_value(&c) { packet.clients.push(v); }
        }
    }

    // 3. Settings (if any)
    if let Ok(Some(row)) = store
        .read("settings", "settings", vec![], "")
        .await
        .fetch_optional()
        .await
    {
        packet.settings = serde_json::to_value(&row).ok();
    }

    Ok(packet)
}

/// Apply received SyncPacket into local store.
pub async fn apply_sync_packet(
    store: &FileStore,
    packet: &SyncPacket,
) -> Result<(usize, usize), String> {
    let mut clients_applied = 0;
    let mut invoices_applied = 0;

    // 1. Apply Clients
    for client_val in &packet.clients {
        let Some(client_id) = client_val.get("id").and_then(|v| v.as_str()) else {
            continue;
        };

        let existing = store
            .read("clients", "id", vec![json!(client_id)], "")
            .await
            .fetch_optional()
            .await
            .map_err(|e| e.to_string())?;

        let mut tx = store.begin().await.map_err(|e| e.to_string())?;
        if existing.is_none() {
            tx.insert("clients", client_val.clone())
                .await
                .map_err(|e| e.to_string())?;
        } else {
            tx.update("clients", client_id, client_val.clone())
                .await
                .map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        clients_applied += 1;
    }

    // 2. Apply Invoices
    for inv in &packet.invoices {
        // Idempotent import via XML
        if let Ok(detail) = import_xml_content(store, &inv.xml_content).await {
            // Re-attach notifications if any
            for (_tipo, raw_xml) in &inv.sdi_notifications {
                if let Ok(notif) = common::pec::parse_notification(raw_xml) {
                    let _ = crate::commands::pec::attach_sdi_notification(
                        store,
                        &notif,
                        raw_xml,
                        Some(&detail.id),
                    )
                    .await;
                }
            }
            invoices_applied += 1;
        }
    }

    Ok((invoices_applied, clients_applied))
}

/// Push local vault state to the ephemeral relay encrypted with the user passphrase.
#[tauri::command]
pub async fn sync_push_changes(
    state: tauri::State<'_, TauriState>,
    config: SyncConfig,
) -> Result<SyncPushResult, String> {
    sync_push_changes_store(&state.store, config).await
}

pub async fn sync_push_changes_store(
    store: &FileStore,
    config: SyncConfig,
) -> Result<SyncPushResult, String> {
    if config.passphrase.trim().is_empty() {
        return Err("La passphrase di sincronizzazione E2EE non può essere vuota".to_string());
    }

    let packet = build_sync_packet(store, &config.device_id).await?;
    let invoices_count = packet.invoices.len();
    let clients_count = packet.clients.len();

    let envelope = packet
        .encrypt(&config.passphrase, SYNC_SALT)
        .map_err(|e| format!("Errore crittografico cifratura E2EE: {e}"))?;

    let client = reqwest::Client::new();
    let push_url = format!("{}/api/relay/push", config.relay_url.trim_end_matches('/'));

    let payload_str = serde_json::to_string(&envelope)
        .map_err(|e| format!("Errore serializzazione busta cifrata: {e}"))?;

    let res = client
        .post(&push_url)
        .headers(build_headers(&config.api_key))
        .json(&json!({
            "topic": envelope.topic,
            "payload": payload_str,
            "sender_id": config.device_id
        }))
        .send()
        .await
        .map_err(|e| format!("Impossibile contattare il server relay: {e}"))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!("Errore dal server relay: {err_text}"));
    }

    let res_json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Risposta non valida dal server relay: {e}"))?;

    let message_id = res_json
        .get("message_id")
        .and_then(|v| v.as_str())
        .unwrap_or("ok")
        .to_string();

    Ok(SyncPushResult {
        success: true,
        invoices_pushed: invoices_count,
        clients_pushed: clients_count,
        topic: envelope.topic,
        message_id,
    })
}

/// Pull and apply encrypted changes from the ephemeral relay.
#[tauri::command]
pub async fn sync_pull_changes(
    state: tauri::State<'_, TauriState>,
    config: SyncConfig,
) -> Result<SyncPullResult, String> {
    sync_pull_changes_store(&state.store, config).await
}

pub async fn sync_pull_changes_store(
    store: &FileStore,
    config: SyncConfig,
) -> Result<SyncPullResult, String> {
    if config.passphrase.trim().is_empty() {
        return Err("La passphrase di sincronizzazione E2EE non può essere vuota".to_string());
    }

    let topic = crypto::derive_topic(&config.passphrase);
    let client = reqwest::Client::new();
    let pull_url = format!(
        "{}/api/relay/pull/{}",
        config.relay_url.trim_end_matches('/'),
        topic
    );

    let res = client
        .get(&pull_url)
        .headers(build_headers(&config.api_key))
        .send()
        .await
        .map_err(|e| format!("Impossibile contattare il server relay: {e}"))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!("Errore dal server relay: {err_text}"));
    }

    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Risposta non valida dal server relay: {e}"))?;

    let messages = body
        .get("messages")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut envelopes_received = 0;
    let mut total_invoices = 0;
    let mut total_clients = 0;
    let mut errors = Vec::new();

    for msg in messages {
        let sender_id = msg
            .get("sender_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        // Skip our own broadcasts to prevent redundant re-imports
        if sender_id == config.device_id {
            continue;
        }

        let Some(payload_str) = msg.get("payload").and_then(|v| v.as_str()) else {
            continue;
        };

        let Ok(envelope) = serde_json::from_str::<EncryptedEnvelope>(payload_str) else {
            errors.push("Busta cifrata non valida nel messaggio relay".to_string());
            continue;
        };

        match SyncPacket::decrypt(&envelope, &config.passphrase, SYNC_SALT) {
            Ok(packet) => {
                envelopes_received += 1;
                match apply_sync_packet(store, &packet).await {
                    Ok((inv_count, cli_count)) => {
                        total_invoices += inv_count;
                        total_clients += cli_count;
                    }
                    Err(e) => {
                        errors.push(format!("Errore applicazione pacchetto sincronizzato: {e}"));
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Decifratura fallita: passphrase non corretta o dati corrotti ({e})"));
            }
        }
    }

    Ok(SyncPullResult {
        success: errors.is_empty(),
        envelopes_received,
        invoices_processed: total_invoices,
        clients_processed: total_clients,
        errors,
    })
}

/// Check status and pending message count on the relay server without consuming them.
#[tauri::command]
pub async fn sync_check_relay(
    config: SyncConfig,
) -> Result<SyncStatusDto, String> {
    if config.passphrase.trim().is_empty() {
        return Err("Passphrase mancante".to_string());
    }

    let topic = crypto::derive_topic(&config.passphrase);
    let client = reqwest::Client::new();
    let peek_url = format!(
        "{}/api/relay/peek/{}",
        config.relay_url.trim_end_matches('/'),
        topic
    );

    let res = client
        .get(&peek_url)
        .headers(build_headers(&config.api_key))
        .send()
        .await
        .map_err(|e| format!("Connessione fallita: {e}"))?;

    let is_ok = res.status().is_success();
    let count = if is_ok {
        let body: serde_json::Value = res.json().await.unwrap_or_default();
        body.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize
    } else {
        0
    };

    Ok(SyncStatusDto {
        topic,
        pending_messages: count,
        server_healthy: is_ok,
        message: if is_ok {
            format!("{count} pacchetti in attesa nel relay")
        } else {
            "Server non raggiungibile".to_string()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_between_two_vault_stores() {
        let dir_a = tempfile::tempdir().unwrap();
        let store_a = FileStore::open(dir_a.path().join("eattura.json")).unwrap();

        let dir_b = tempfile::tempdir().unwrap();
        let store_b = FileStore::open(dir_b.path().join("eattura.json")).unwrap();

        // 1. Add client to store_a
        let client_id = "cli-test-01";
        let mut tx_a = store_a.begin().await.unwrap();
        tx_a.insert("clients", json!({
            "id": client_id,
            "denominazione": "Studio Tecnico Alfa SRL",
            "id_codice": "01234567890",
            "id_paese": "IT",
            "indirizzo": "Via Roma 1",
            "cap": "00100",
            "comune": "Roma",
            "nazione": "IT",
            "codice_destinatario": "M5UXCR1",
            "pec": "studioalfa@pec.it"
        })).await.unwrap();
        tx_a.commit().await.unwrap();

        // 2. Build sync packet from store_a
        let packet_a = build_sync_packet(&store_a, "device-desktop").await.unwrap();
        assert_eq!(packet_a.clients.len(), 1);

        // 3. Encrypt envelope with passphrase
        let passphrase = "falco-torre-bosco-4521";
        let envelope = packet_a.encrypt(passphrase, SYNC_SALT).unwrap();

        // 4. Decrypt on device B
        let recovered_packet = SyncPacket::decrypt(&envelope, passphrase, SYNC_SALT).unwrap();
        assert_eq!(recovered_packet.sender_device_id, "device-desktop");

        // 5. Apply to store_b
        let (invs, clis) = apply_sync_packet(&store_b, &recovered_packet).await.unwrap();
        assert_eq!(clis, 1);
        assert_eq!(invs, 0);

        // 6. Verify client exists in store_b
        let found = store_b.read("clients", "id", vec![json!(client_id)], "").await.fetch_optional().await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().get::<String>("denominazione"), "Studio Tecnico Alfa SRL");
    }

    #[test]
    fn test_generate_pairing_code() {
        let code1 = generate_sync_pairing_code();
        let code2 = generate_sync_pairing_code();
        assert!(!code1.is_empty());
        assert!(!code2.is_empty());
        assert_ne!(code1, code2);
        assert!(code1.split('-').count() >= 4);
    }
}
