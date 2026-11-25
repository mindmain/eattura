//! Clean structured ZIP archive, deterministic hashing and multi-device merge for Google Drive backups.

use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::commands::payments::{
    InvoiceLedgerEntry, PaymentRecord, PaymentsLedger, load_ledger, write_ledger_without_notify,
};
use crate::storage::FileStore;

const SCHEMA_VERSION: u32 = 1;
const MAX_ZIP_ENTRIES: usize = 100_000;
const MAX_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BackupManifest {
    pub schema_version: u32,
    pub created_at: String,
    pub snapshot_sha256: String,
    pub counts: BackupCounts,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct BackupCounts {
    pub clients: usize,
    pub invoices: usize,
    pub notifications: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct InvoiceBundle {
    pub invoice: Value,
    pub lines: Vec<Value>,
    pub payments: Vec<Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct CanonicalSnapshot {
    settings: Vec<Value>,
    clients: Vec<Value>,
    invoices: Vec<InvoiceBundle>,
    sdi_notifications: Vec<Value>,
    payments_ledger: BTreeMap<String, CanonicalLedgerEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalLedgerEntry {
    invoice_id: String,
    saldo_residuo: f64,
    pagata: bool,
    pagamenti: Vec<PaymentRecord>,
    note: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct ArchiveSnapshot {
    canonical: CanonicalSnapshot,
    pub sha256: String,
}

impl ArchiveSnapshot {
    pub fn from_parts(
        mut domains: BTreeMap<String, Vec<Value>>,
        ledger: &PaymentsLedger,
    ) -> Result<Self, String> {
        let mut settings = domains.remove("settings").unwrap_or_default();
        settings.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));

        let mut clients = domains.remove("clients").unwrap_or_default();
        clients.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));

        let mut raw_invoices = domains.remove("invoices").unwrap_or_default();
        raw_invoices.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));

        let all_lines = domains.remove("invoice_lines").unwrap_or_default();
        let all_payments = domains.remove("invoice_payments").unwrap_or_default();

        let mut lines_by_invoice: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for line in all_lines {
            let Some(invoice_id) = line["invoice_id"].as_str() else {
                return Err("Riga fattura priva di riferimento nel backup.".into());
            };
            lines_by_invoice
                .entry(invoice_id.to_owned())
                .or_default()
                .push(line);
        }
        let mut payments_by_invoice: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for payment in all_payments {
            let Some(invoice_id) = payment["invoice_id"].as_str() else {
                return Err("Pagamento fattura privo di riferimento nel backup.".into());
            };
            payments_by_invoice
                .entry(invoice_id.to_owned())
                .or_default()
                .push(payment);
        }

        let mut invoices = Vec::with_capacity(raw_invoices.len());
        for invoice in raw_invoices {
            let id = invoice["id"]
                .as_str()
                .ok_or("Fattura priva di identificativo nel backup.")?
                .to_owned();
            let mut lines = lines_by_invoice.remove(&id).unwrap_or_default();
            lines.sort_by(|a, b| {
                a["numero_linea"]
                    .as_i64()
                    .cmp(&b["numero_linea"].as_i64())
                    .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
            });
            let mut payments = payments_by_invoice.remove(&id).unwrap_or_default();
            payments.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
            invoices.push(InvoiceBundle {
                invoice,
                lines,
                payments,
            });
        }

        let mut sdi_notifications = domains.remove("sdi_notifications").unwrap_or_default();
        sdi_notifications.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));

        let valid_invoice_ids: std::collections::BTreeSet<&str> = invoices
            .iter()
            .filter_map(|bundle| bundle.invoice["id"].as_str())
            .collect();
        let mut payments_ledger = BTreeMap::new();
        for (invoice_id, entry) in &ledger.entries {
            if !valid_invoice_ids.contains(invoice_id.as_str()) {
                continue;
            }
            let mut pagamenti = entry.pagamenti.clone();
            pagamenti.sort_by(|a, b| a.id.cmp(&b.id));
            payments_ledger.insert(
                invoice_id.clone(),
                CanonicalLedgerEntry {
                    invoice_id: entry.invoice_id.clone(),
                    saldo_residuo: entry.saldo_residuo,
                    pagata: entry.pagata,
                    pagamenti,
                    note: entry.note.clone(),
                },
            );
        }

        let canonical = CanonicalSnapshot {
            settings,
            clients,
            invoices,
            sdi_notifications,
            payments_ledger,
        };
        let bytes = serde_json::to_vec(&canonical).map_err(|e| e.to_string())?;
        let sha256 = hex_sha256(&bytes);
        Ok(Self { canonical, sha256 })
    }

    pub async fn capture(store: &FileStore) -> Result<Self, String> {
        let domains = store.export_archive_domains().await?;
        let ledger = load_ledger(store);
        Self::from_parts(domains, &ledger)
    }

    pub async fn apply(&self, store: &FileStore) -> Result<(), String> {
        let mut domains: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        domains.insert("settings".into(), self.canonical.settings.clone());
        domains.insert("clients".into(), self.canonical.clients.clone());

        let mut invoices = Vec::with_capacity(self.canonical.invoices.len());
        let mut lines = Vec::new();
        let mut payments = Vec::new();
        for bundle in &self.canonical.invoices {
            invoices.push(bundle.invoice.clone());
            lines.extend(bundle.lines.clone());
            payments.extend(bundle.payments.clone());
        }
        domains.insert("invoices".into(), invoices);
        domains.insert("invoice_lines".into(), lines);
        domains.insert("invoice_payments".into(), payments);
        domains.insert(
            "sdi_notifications".into(),
            self.canonical.sdi_notifications.clone(),
        );

        let mut entries = HashMap::with_capacity(self.canonical.payments_ledger.len());
        for (invoice_id, item) in &self.canonical.payments_ledger {
            entries.insert(
                invoice_id.clone(),
                InvoiceLedgerEntry {
                    invoice_id: item.invoice_id.clone(),
                    saldo_residuo: item.saldo_residuo,
                    pagata: item.pagata,
                    pagamenti: item.pagamenti.clone(),
                    note: item.note.clone(),
                },
            );
        }
        let ledger = PaymentsLedger {
            version: 1,
            entries,
        };

        store.apply_archive_domains(domains).await?;
        write_ledger_without_notify(store, &ledger)?;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.canonical.settings.is_empty()
            && self.canonical.clients.is_empty()
            && self.canonical.invoices.is_empty()
            && self.canonical.sdi_notifications.is_empty()
            && self.canonical.payments_ledger.is_empty()
    }

    pub fn counts(&self) -> BackupCounts {
        BackupCounts {
            clients: self.canonical.clients.len(),
            invoices: self.canonical.invoices.len(),
            notifications: self.canonical.sdi_notifications.len(),
        }
    }

    pub fn encode_zip(&self, created_at: &str) -> Result<Vec<u8>, String> {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));

        let manifest = BackupManifest {
            schema_version: SCHEMA_VERSION,
            created_at: created_at.to_owned(),
            snapshot_sha256: self.sha256.clone(),
            counts: self.counts(),
        };
        write_json_entry(&mut zip, options, "manifest.json", &manifest)?;
        write_json_entry(
            &mut zip,
            options,
            "settings.json",
            &self.canonical.settings,
        )?;
        write_json_entry(&mut zip, options, "clients.json", &self.canonical.clients)?;
        write_json_entry(
            &mut zip,
            options,
            "sdi_notifications.json",
            &self.canonical.sdi_notifications,
        )?;
        write_json_entry(
            &mut zip,
            options,
            "payments_ledger.json",
            &self.canonical.payments_ledger,
        )?;

        for (index, bundle) in self.canonical.invoices.iter().enumerate() {
            let id = bundle.invoice["id"].as_str().unwrap_or("invoice");
            let safe_id = safe_segment(id, index);
            write_json_entry(
                &mut zip,
                options,
                &format!("invoices/{index:06}-{safe_id}.json"),
                bundle,
            )?;
            if let Some(xml) = bundle.invoice["xml_content"]
                .as_str()
                .filter(|xml| !xml.trim().is_empty())
            {
                let year = bundle.invoice["data"]
                    .as_str()
                    .and_then(|date| date.get(..4))
                    .filter(|y| y.bytes().all(|b| b.is_ascii_digit()))
                    .unwrap_or("unknown");
                let number = safe_segment(
                    bundle.invoice["numero"].as_str().unwrap_or("doc"),
                    index,
                );
                let xml_path = format!("xml/{year}/{number}-{safe_id}.xml");
                zip.start_file(xml_path, options)
                    .map_err(|e| e.to_string())?;
                zip.write_all(xml.as_bytes()).map_err(|e| e.to_string())?;
            }
        }

        let cursor = zip.finish().map_err(|e| e.to_string())?;
        Ok(cursor.into_inner())
    }

    pub fn decode_zip(bytes: &[u8]) -> Result<(BackupManifest, Self), String> {
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|_| "Archivio di backup Drive non valido o danneggiato.".to_string())?;
        if archive.len() > MAX_ZIP_ENTRIES {
            return Err("Archivio di backup Drive contiene troppi elementi.".into());
        }

        let mut total_uncompressed = 0_u64;
        let mut manifest: Option<BackupManifest> = None;
        let mut settings: Vec<Value> = Vec::new();
        let mut clients: Vec<Value> = Vec::new();
        let mut sdi_notifications: Vec<Value> = Vec::new();
        let mut ledger_entries: BTreeMap<String, CanonicalLedgerEntry> = BTreeMap::new();
        let mut invoice_bundles: Vec<(String, InvoiceBundle)> = Vec::new();

        for index in 0..archive.len() {
            let file = archive
                .by_index(index)
                .map_err(|_| "Impossibile leggere un elemento del backup Drive.".to_string())?;
            if file.is_dir() {
                continue;
            }
            let name = file
                .name()
                .map_err(|_| "Nome file non valido nel backup Drive.".to_string())?
                .into_owned();
            let remaining = MAX_UNCOMPRESSED_BYTES
                .checked_sub(total_uncompressed)
                .ok_or("Archivio di backup Drive supera il limite di sicurezza.")?;
            let mut content = Vec::new();
            file.take(remaining + 1)
                .read_to_end(&mut content)
                .map_err(|_| "Lettura interrotta dell’archivio di backup Drive.".to_string())?;
            total_uncompressed = total_uncompressed.saturating_add(content.len() as u64);
            if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
                return Err("Archivio di backup Drive supera il limite di sicurezza.".into());
            }

            match name.as_str() {
                "manifest.json" => {
                    manifest = Some(
                        serde_json::from_slice(&content)
                            .map_err(|_| "Manifest del backup Drive non valido.".to_string())?,
                    );
                }
                "settings.json" => {
                    settings = serde_json::from_slice(&content)
                        .map_err(|_| "Impostazioni del backup Drive non valide.".to_string())?;
                }
                "clients.json" => {
                    clients = serde_json::from_slice(&content)
                        .map_err(|_| "Anagrafiche del backup Drive non valide.".to_string())?;
                }
                "sdi_notifications.json" => {
                    sdi_notifications = serde_json::from_slice(&content)
                        .map_err(|_| "Notifiche SdI del backup Drive non valide.".to_string())?;
                }
                "payments_ledger.json" => {
                    ledger_entries = serde_json::from_slice(&content)
                        .map_err(|_| "Scadenzario del backup Drive non valido.".to_string())?;
                }
                path if path.starts_with("invoices/") && path.ends_with(".json") => {
                    let bundle: InvoiceBundle = serde_json::from_slice(&content)
                        .map_err(|_| format!("Fattura non valida nel backup Drive: {path}"))?;
                    invoice_bundles.push((path.to_owned(), bundle));
                }
                _ => {}
            }
        }

        let manifest = manifest.ok_or("Manifest mancante nel backup Drive.")?;
        if manifest.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "Versione del backup Drive ({}) non supportata da questa versione di Eattura.",
                manifest.schema_version
            ));
        }

        invoice_bundles.sort_by(|a, b| a.0.cmp(&b.0));
        let mut domains: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        domains.insert("settings".into(), settings);
        domains.insert("clients".into(), clients);
        let mut invoices = Vec::with_capacity(invoice_bundles.len());
        let mut lines = Vec::new();
        let mut payments = Vec::new();
        for (_, bundle) in invoice_bundles {
            invoices.push(bundle.invoice);
            lines.extend(bundle.lines);
            payments.extend(bundle.payments);
        }
        domains.insert("invoices".into(), invoices);
        domains.insert("invoice_lines".into(), lines);
        domains.insert("invoice_payments".into(), payments);
        domains.insert("sdi_notifications".into(), sdi_notifications);

        let mut entries = HashMap::with_capacity(ledger_entries.len());
        for (invoice_id, entry) in ledger_entries {
            entries.insert(
                invoice_id,
                InvoiceLedgerEntry {
                    invoice_id: entry.invoice_id,
                    saldo_residuo: entry.saldo_residuo,
                    pagata: entry.pagata,
                    pagamenti: entry.pagamenti,
                    note: entry.note,
                },
            );
        }
        let snapshot = Self::from_parts(domains, &PaymentsLedger { version: 1, entries })?;
        if snapshot.sha256 != manifest.snapshot_sha256 {
            return Err("Controllo di integrità SHA-256 del backup Drive non superato.".into());
        }
        Ok((manifest, snapshot))
    }
}

/// Non-destructively merges `local` and `remote` archives when both changed independently.
pub(super) fn merge_snapshots(
    local: &ArchiveSnapshot,
    remote: &ArchiveSnapshot,
) -> Result<ArchiveSnapshot, String> {
    let settings = if !remote.canonical.settings.is_empty() {
        remote.canonical.settings.clone()
    } else {
        local.canonical.settings.clone()
    };

    // 1. Merge clients by fiscal identity (id_paese + id_codice) and track ID remappings.
    let mut clients_by_tax: BTreeMap<(String, String), Value> = BTreeMap::new();
    let mut client_id_remap: HashMap<String, String> = HashMap::new();

    for source in [&remote.canonical.clients, &local.canonical.clients] {
        for client in source {
            let key = (
                client["id_paese"]
                    .as_str()
                    .unwrap_or("")
                    .to_ascii_uppercase(),
                client["id_codice"].as_str().unwrap_or("").to_owned(),
            );
            let incoming_id = client["id"].as_str().unwrap_or("").to_owned();
            if let Some(existing) = clients_by_tax.get_mut(&key) {
                let canonical_id = existing["id"].as_str().unwrap_or("").to_owned();
                if incoming_id != canonical_id {
                    client_id_remap.insert(incoming_id, canonical_id);
                }
                if let (Some(dst), Some(src)) = (existing.as_object_mut(), client.as_object()) {
                    for (field, val) in src {
                        if field != "id" && dst.get(field).is_none_or(Value::is_null) && !val.is_null() {
                            dst.insert(field.clone(), val.clone());
                        }
                    }
                }
            } else {
                clients_by_tax.insert(key, client.clone());
            }
        }
    }
    let clients: Vec<Value> = clients_by_tax.into_values().collect();

    // 2. Merge invoice bundles by ID or fiscal key, preferring higher SdI progression.
    let mut merged_invoices: Vec<InvoiceBundle> = Vec::new();
    let mut invoice_id_remap: HashMap<String, String> = HashMap::new();

    for bundle in remote
        .canonical
        .invoices
        .iter()
        .chain(local.canonical.invoices.iter())
    {
        let mut normalized = remap_invoice_clients(bundle, &client_id_remap);
        let candidate_id = normalized.invoice["id"].as_str().unwrap_or("").to_owned();

        let match_idx = merged_invoices.iter().position(|existing| {
            let existing_id = existing.invoice["id"].as_str().unwrap_or("");
            existing_id == candidate_id
                || (existing.invoice["stato"] != "rejected"
                    && normalized.invoice["stato"] != "rejected"
                    && same_fiscal_invoice_key(&existing.invoice, &normalized.invoice))
        });

        if let Some(idx) = match_idx {
            let existing = &merged_invoices[idx];
            let existing_rank = status_rank(existing.invoice["stato"].as_str());
            let candidate_rank = status_rank(normalized.invoice["stato"].as_str());
            if candidate_rank > existing_rank {
                let old_id = existing.invoice["id"].as_str().unwrap_or("").to_owned();
                if old_id != candidate_id {
                    invoice_id_remap.insert(old_id, candidate_id);
                }
                merged_invoices[idx] = normalized;
            } else {
                let kept_id = existing.invoice["id"].as_str().unwrap_or("").to_owned();
                if candidate_id != kept_id {
                    invoice_id_remap.insert(candidate_id, kept_id);
                }
                // Preserve attachments or XML if missing on the kept record.
                if let (Some(dst), Some(src)) = (
                    merged_invoices[idx].invoice.as_object_mut(),
                    normalized.invoice.as_object(),
                ) {
                    for key in ["xml_content", "allegati"] {
                        if dst.get(key).is_none_or(Value::is_null)
                            && let Some(value) = src.get(key).filter(|v| !v.is_null())
                        {
                            dst.insert(key.into(), value.clone());
                        }
                    }
                }
            }
        } else {
            merged_invoices.push(std::mem::replace(
                &mut normalized,
                InvoiceBundle {
                    invoice: Value::Null,
                    lines: Vec::new(),
                    payments: Vec::new(),
                },
            ));
        }
    }

    let valid_invoice_ids: std::collections::BTreeSet<String> = merged_invoices
        .iter()
        .filter_map(|b| b.invoice["id"].as_str().map(str::to_owned))
        .collect();

    // 3. Merge SdI notifications by ID, updating remapped invoice_ids.
    let mut notifications_by_id: BTreeMap<String, Value> = BTreeMap::new();
    for notif in remote
        .canonical
        .sdi_notifications
        .iter()
        .chain(local.canonical.sdi_notifications.iter())
    {
        let Some(id) = notif["id"].as_str() else {
            continue;
        };
        let mut entry = notif.clone();
        if let Some(inv_id) = entry["invoice_id"].as_str() {
            let resolved = resolve_id(inv_id, &invoice_id_remap);
            if valid_invoice_ids.contains(&resolved) {
                entry["invoice_id"] = json!(resolved);
            } else {
                entry["invoice_id"] = Value::Null;
            }
        }
        notifications_by_id.entry(id.to_owned()).or_insert(entry);
    }

    // 4. Merge payment ledger entries and deduplicate payment records by ID.
    let mut ledger_entries: HashMap<String, InvoiceLedgerEntry> = HashMap::new();
    for source in [&remote.canonical.payments_ledger, &local.canonical.payments_ledger] {
        for (raw_id, entry) in source {
            let resolved_id = resolve_id(raw_id, &invoice_id_remap);
            if !valid_invoice_ids.contains(&resolved_id) {
                continue;
            }
            let target = ledger_entries
                .entry(resolved_id.clone())
                .or_insert_with(|| InvoiceLedgerEntry {
                    invoice_id: resolved_id.clone(),
                    saldo_residuo: entry.saldo_residuo,
                    pagata: entry.pagata,
                    pagamenti: Vec::new(),
                    note: entry.note.clone(),
                });
            for payment in &entry.pagamenti {
                if !target.pagamenti.iter().any(|existing| existing.id == payment.id) {
                    target.pagamenti.push(payment.clone());
                }
            }
            if target.note.is_none() && entry.note.is_some() {
                target.note.clone_from(&entry.note);
            }
            target.pagata = target.pagata || entry.pagata;
            target.saldo_residuo = target.saldo_residuo.min(entry.saldo_residuo);
        }
    }

    let mut domains: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    domains.insert("settings".into(), settings);
    domains.insert("clients".into(), clients);
    let mut invoices = Vec::with_capacity(merged_invoices.len());
    let mut lines = Vec::new();
    let mut payments = Vec::new();
    for bundle in merged_invoices {
        invoices.push(bundle.invoice);
        lines.extend(bundle.lines);
        payments.extend(bundle.payments);
    }
    domains.insert("invoices".into(), invoices);
    domains.insert("invoice_lines".into(), lines);
    domains.insert("invoice_payments".into(), payments);
    domains.insert(
        "sdi_notifications".into(),
        notifications_by_id.into_values().collect(),
    );

    ArchiveSnapshot::from_parts(
        domains,
        &PaymentsLedger {
            version: 1,
            entries: ledger_entries,
        },
    )
}

fn remap_invoice_clients(
    bundle: &InvoiceBundle,
    client_remap: &HashMap<String, String>,
) -> InvoiceBundle {
    let mut cloned = bundle.clone();
    for key in ["cedente_id", "cessionario_id"] {
        if let Some(current) = cloned.invoice[key].as_str() {
            let mapped = resolve_id(current, client_remap);
            if mapped != current {
                cloned.invoice[key] = json!(mapped);
            }
        }
    }
    cloned
}

fn resolve_id(initial: &str, map: &HashMap<String, String>) -> String {
    let mut current = initial;
    for _ in 0..8 {
        match map.get(current) {
            Some(next) if next != current => current = next,
            _ => break,
        }
    }
    current.to_owned()
}

fn status_rank(status: Option<&str>) -> u8 {
    match status {
        Some("delivered" | "accepted") => 5,
        Some("rejected") => 4,
        Some("sent") => 3,
        Some("imported") => 2,
        Some("validated") => 1,
        _ => 0,
    }
}

fn same_fiscal_invoice_key(existing: &Value, candidate: &Value) -> bool {
    let existing_issuer = if existing["soggetto_emittente"] == "CC" {
        &existing["cessionario_id"]
    } else {
        &existing["cedente_id"]
    };
    let candidate_issuer = if candidate["soggetto_emittente"] == "CC" {
        &candidate["cessionario_id"]
    } else {
        &candidate["cedente_id"]
    };
    existing_issuer == candidate_issuer
        && existing["numero"] == candidate["numero"]
        && (if existing["art73"] == true || candidate["art73"] == true {
            existing["data"] == candidate["data"]
        } else {
            existing["data"].as_str().and_then(|date| date.get(..4))
                == candidate["data"].as_str().and_then(|date| date.get(..4))
        })
        && (existing["tipo_documento"] == "TD04") == (candidate["tipo_documento"] == "TD04")
}

fn write_json_entry<W: Write + std::io::Seek, T: Serialize>(
    zip: &mut ZipWriter<W>,
    options: SimpleFileOptions,
    path: &str,
    value: &T,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    zip.start_file(path, options).map_err(|e| e.to_string())?;
    zip.write_all(&bytes).map_err(|e| e.to_string())
}

fn safe_segment(value: &str, fallback_index: usize) -> String {
    let sanitized: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        format!("item_{fallback_index}")
    } else {
        trimmed.to_owned()
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn zip_roundtrip_preserves_deterministic_hash_and_all_domains() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::open(dir.path().join("eattura.json")).unwrap();
        store
            .insert(
                "clients",
                json!({
                    "id": "client-1",
                    "id_paese": "IT",
                    "id_codice": "12345678903",
                    "denominazione": "Studio Rossi",
                    "indirizzo": "Via Nazionale 10",
                    "cap": "00100",
                    "comune": "Roma",
                    "nazione": "IT"
                }),
            )
            .await
            .unwrap();
        store
            .insert(
                "invoices",
                json!({
                    "id": "inv-1",
                    "numero": "1/2026",
                    "data": "2026-03-10",
                    "tipo_documento": "TD01",
                    "divisa": "EUR",
                    "stato": "delivered",
                    "cedente_id": "client-1",
                    "cessionario_id": "client-1",
                    "importo_totale": 122.0,
                    "xml_content": "<FatturaElettronica/>"
                }),
            )
            .await
            .unwrap();
        store
            .insert(
                "invoice_lines",
                json!({
                    "id": "line-1",
                    "invoice_id": "inv-1",
                    "numero_linea": 1,
                    "descrizione": "Consulenza",
                    "prezzo_unitario": 100.0,
                    "prezzo_totale": 100.0,
                    "aliquota_iva": 22.0
                }),
            )
            .await
            .unwrap();
        store
            .insert(
                "sdi_notifications",
                json!({
                    "id": "rc-1",
                    "invoice_id": "inv-1",
                    "tipo": "RC",
                    "raw_xml": "<RicevutaConsegna/>"
                }),
            )
            .await
            .unwrap();

        let captured = ArchiveSnapshot::capture(&store).await.unwrap();
        let zip_bytes = captured.encode_zip("2026-11-25T11:00:00Z").unwrap();
        let (manifest, decoded) = ArchiveSnapshot::decode_zip(&zip_bytes).unwrap();
        assert_eq!(manifest.snapshot_sha256, captured.sha256);
        assert_eq!(manifest.counts.invoices, 1);
        assert_eq!(manifest.counts.clients, 1);
        assert_eq!(manifest.counts.notifications, 1);
        assert_eq!(decoded.sha256, captured.sha256);

        let target_dir = tempfile::tempdir().unwrap();
        let restored = FileStore::open(target_dir.path().join("eattura.json")).unwrap();
        decoded.apply(&restored).await.unwrap();
        let re_captured = ArchiveSnapshot::capture(&restored).await.unwrap();
        assert_eq!(re_captured.sha256, captured.sha256);
    }

    #[tokio::test]
    async fn merge_keeps_distinct_invoices_and_highest_sdi_status() {
        let dir_a = tempfile::tempdir().unwrap();
        let store_a = FileStore::open(dir_a.path().join("eattura.json")).unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        let store_b = FileStore::open(dir_b.path().join("eattura.json")).unwrap();

        let client = json!({
            "id": "client-1",
            "id_paese": "IT",
            "id_codice": "12345678903",
            "denominazione": "Cliente Comune",
            "indirizzo": "Via Roma 1",
            "cap": "00100",
            "comune": "Roma",
            "nazione": "IT"
        });
        store_a.insert("clients", client.clone()).await.unwrap();
        store_b.insert("clients", client).await.unwrap();

        // Store A has inv-1 as draft + inv-local-draft
        store_a
            .insert(
                "invoices",
                json!({
                    "id": "inv-1",
                    "numero": "1",
                    "data": "2026-05-01",
                    "tipo_documento": "TD01",
                    "divisa": "EUR",
                    "stato": "draft",
                    "cedente_id": "client-1",
                    "cessionario_id": "client-1"
                }),
            )
            .await
            .unwrap();
        store_a
            .insert(
                "invoices",
                json!({
                    "id": "inv-local",
                    "numero": "2",
                    "data": "2026-05-02",
                    "tipo_documento": "TD01",
                    "divisa": "EUR",
                    "stato": "draft",
                    "cedente_id": "client-1",
                    "cessionario_id": "client-1"
                }),
            )
            .await
            .unwrap();

        // Store B upgraded inv-1 to delivered + added inv-remote
        store_b
            .insert(
                "invoices",
                json!({
                    "id": "inv-1",
                    "numero": "1",
                    "data": "2026-05-01",
                    "tipo_documento": "TD01",
                    "divisa": "EUR",
                    "stato": "delivered",
                    "cedente_id": "client-1",
                    "cessionario_id": "client-1"
                }),
            )
            .await
            .unwrap();
        store_b
            .insert(
                "invoices",
                json!({
                    "id": "inv-remote",
                    "numero": "3",
                    "data": "2026-05-03",
                    "tipo_documento": "TD01",
                    "divisa": "EUR",
                    "stato": "imported",
                    "cedente_id": "client-1",
                    "cessionario_id": "client-1"
                }),
            )
            .await
            .unwrap();

        let snap_a = ArchiveSnapshot::capture(&store_a).await.unwrap();
        let snap_b = ArchiveSnapshot::capture(&store_b).await.unwrap();
        let merged = merge_snapshots(&snap_a, &snap_b).unwrap();
        assert_eq!(merged.counts().invoices, 3);
        merged.apply(&store_a).await.unwrap();

        let inv1 = store_a
            .read("invoices", "id", vec![json!("inv-1")], "")
            .await
            .fetch_one()
            .await
            .unwrap();
        assert_eq!(inv1.get::<String>("stato"), "delivered");
    }
}
