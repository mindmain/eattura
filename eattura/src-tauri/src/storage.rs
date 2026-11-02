//! Normalized in-memory catalog with lazily loaded invoice documents.
pub(crate) mod digest;
mod index;
use index::Index;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record(Value);
impl Record {
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> T {
        serde_json::from_value(self.0.get(key).cloned().unwrap_or(Value::Null))
            .expect("Invalid stored record field")
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Data {
    #[serde(flatten)]
    records: BTreeMap<String, Vec<Record>>,
    #[serde(skip)]
    index: Index,
    #[serde(skip)]
    dirty: BTreeSet<String>,
    #[serde(skip)]
    loaded: BTreeSet<String>,
}
#[derive(Serialize, Deserialize)]
struct InvoiceDocument {
    invoice: Record,
    lines: Vec<Record>,
    payments: Vec<Record>,
}
#[derive(Clone)]
pub struct FileStore {
    path: Arc<std::sync::RwLock<PathBuf>>,
    data: Arc<Mutex<Data>>,
}
pub struct Transaction {
    store: FileStore,
    guard: OwnedMutexGuard<Data>,
    working: Data,
}
pub struct Change(u64);
impl Change {
    pub fn rows_affected(&self) -> u64 {
        self.0
    }
}
pub struct Records(Result<Vec<Record>, String>);
impl Records {
    pub async fn fetch_all(self) -> Result<Vec<Record>, String> {
        self.0
    }
    pub async fn fetch_optional(self) -> Result<Option<Record>, String> {
        Ok(self.0?.into_iter().next())
    }
    pub async fn fetch_one(self) -> Result<Record, String> {
        self.0?
            .into_iter()
            .next()
            .ok_or_else(|| "Record not found".into())
    }
}
impl FileStore {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let index_path = path.with_file_name("index.json");
        let needs_index = !index_path.exists();
        let mut data = match std::fs::read(&index_path) {
            Ok(bytes) => {
                let index: Index =
                    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid index: {e}"))?;
                if index.version != 1 {
                    return Err("Unsupported desktop index version".into());
                }
                for (domain, collection) in
                    [("clients", &index.clients), ("invoices", &index.invoices)]
                {
                    let ids: BTreeSet<_> = collection.ids.iter().collect();
                    if ids.len() != collection.ids.len() || ids.len() != collection.entities.len() {
                        return Err(format!("Inconsistent indexed IDs: {domain}"));
                    }
                    for id in &collection.ids {
                        let entry = collection
                            .entities
                            .get(id)
                            .ok_or("Missing indexed entity")?;
                        validate_shape(domain, &entry.0)?;
                        if entry.0["id"].as_str() != Some(id.as_str())
                            || !entry.0["search_text"].is_string()
                        {
                            return Err(format!("Invalid indexed entity: {id}"));
                        }
                    }
                }
                let auxiliary_path = resolve_file(&path, &index.auxiliary_file)?;
                let mut data: Data = serde_json::from_slice(
                    &std::fs::read(auxiliary_path).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                data.records.insert(
                    "clients".into(),
                    index
                        .clients
                        .ids
                        .iter()
                        .map(|id| {
                            index
                                .clients
                                .entities
                                .get(id)
                                .cloned()
                                .ok_or_else(|| format!("Missing indexed client: {id}"))
                        })
                        .collect::<Result<_, _>>()?,
                );
                data.records.insert(
                    "invoices".into(),
                    index
                        .invoices
                        .ids
                        .iter()
                        .map(|id| {
                            index
                                .invoices
                                .entities
                                .get(id)
                                .cloned()
                                .ok_or_else(|| format!("Missing indexed invoice: {id}"))
                        })
                        .collect::<Result<_, _>>()?,
                );
                for invoice in index.invoices.entities.values() {
                    let file = invoice.0["file"]
                        .as_str()
                        .ok_or("Missing indexed invoice file")?;
                    resolve_file(&path, file)?;
                    if !invoice.0["line_terms"]
                        .as_array()
                        .is_some_and(|terms| terms.iter().all(Value::is_string))
                    {
                        return Err("Invalid indexed line terms".into());
                    }
                }
                data.index = index;
                data
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let state_dir = path.with_file_name("state");
                if state_dir.is_dir()
                    && std::fs::read_dir(&state_dir)
                        .map_err(|e| e.to_string())?
                        .next()
                        .is_some()
                {
                    return Err("Desktop index is missing but saved state exists; restore index.json before opening the app".into());
                }
                let mut data: Data = match std::fs::read(&path) {
                    Ok(bytes) => serde_json::from_slice(&bytes)
                        .map_err(|e| format!("Invalid desktop data: {e}"))?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Data::default(),
                    Err(e) => return Err(e.to_string()),
                };
                for (domain, rows) in &data.records {
                    for row in rows {
                        validate_shape(domain, &row.0)?;
                    }
                }
                let ids: BTreeSet<String> = items(&data, "invoices")
                    .iter()
                    .map(|r| r.get("id"))
                    .collect();
                data.dirty = ids.clone();
                data.loaded = ids;
                data
            }
            Err(e) => return Err(e.to_string()),
        };
        for (domain, rows) in &data.records {
            for row in rows {
                validate_shape(domain, &row.0)?;
            }
        }
        let store = Self {
            path: Arc::new(std::sync::RwLock::new(path)),
            data: Arc::new(Mutex::new(Data::default())),
        };
        if needs_index {
            data.index = store.persist(&data)?;
            data.dirty.clear();
        }
        *store.data.try_lock().map_err(|e| e.to_string())? = data;
        Ok(store)
    }
    fn persist(&self, data: &Data) -> Result<Index, String> {
        let path_buf = self.path.read().map_err(|e| e.to_string())?.clone();
        let base = path_buf.parent().ok_or("Missing app data directory")?;
        std::fs::create_dir_all(base.join("invoices")).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(base.join("state")).map_err(|e| e.to_string())?;
        let mut files = BTreeMap::new();
        for id in &data.dirty {
            if let Some(invoice) = items(data, "invoices").iter().find(|r| r.0["id"] == *id) {
                let document = InvoiceDocument {
                    invoice: invoice.clone(),
                    lines: items(data, "invoice_lines")
                        .iter()
                        .filter(|r| r.0["invoice_id"] == *id)
                        .cloned()
                        .collect(),
                    payments: items(data, "invoice_payments")
                        .iter()
                        .filter(|r| r.0["invoice_id"] == *id)
                        .cloned()
                        .collect(),
                };
                let file = format!("invoices/{}.json", uuid::Uuid::new_v4());
                atomic_json(&base.join(&file), &document)?;
                files.insert(id.clone(), file);
            }
        }
        let auxiliary = Data {
            records: data
                .records
                .iter()
                .filter(|(domain, _)| {
                    !matches!(
                        domain.as_str(),
                        "clients" | "invoices" | "invoice_lines" | "invoice_payments"
                    )
                })
                .map(|(domain, rows)| (domain.clone(), rows.clone()))
                .collect(),
            ..Data::default()
        };
        let auxiliary_file = format!("state/{}.json", uuid::Uuid::new_v4());
        atomic_json(&base.join(&auxiliary_file), &auxiliary)?;
        let index = Index::build(data, &files, auxiliary_file);
        // Publish the catalog last: an interrupted write leaves the previous catalog usable.
        atomic_json(&base.join("index.json"), &index)?;
        Ok(index)
    }
    fn hydrate(&self, data: &mut Data, id: &str) -> Result<(), String> {
        if data.loaded.contains(id) {
            return Ok(());
        }
        let Some(entry) = data.index.invoices.entities.get(id) else {
            return Ok(());
        };
        let path_buf = self.path.read().map_err(|e| e.to_string())?.clone();
        let path = resolve_file(&path_buf, &entry.get::<String>("file"))?;
        let document: InvoiceDocument =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        validate_shape("invoices", &document.invoice.0)?;
        if document.invoice.0["id"] != id {
            return Err("Invoice document ID does not match the index".into());
        }
        for (domain, rows) in [
            ("invoice_lines", &document.lines),
            ("invoice_payments", &document.payments),
        ] {
            for row in rows {
                validate_shape(domain, &row.0)?;
                if row.0["invoice_id"] != id {
                    return Err("Invoice child does not match its document".into());
                }
            }
        }
        if let Some(header) = data
            .records
            .get_mut("invoices")
            .and_then(|rows| rows.iter_mut().find(|r| r.0["id"] == id))
        {
            *header = document.invoice;
        }
        data.records
            .entry("invoice_lines".into())
            .or_default()
            .extend(document.lines);
        data.records
            .entry("invoice_payments".into())
            .or_default()
            .extend(document.payments);
        data.loaded.insert(id.into());
        Ok(())
    }
    /// Return the active data directory.
    pub fn data_dir(&self) -> PathBuf {
        self.path
            .read()
            .unwrap()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf()
    }

    /// Freeze a consistent view for multi-record reads.
    pub async fn snapshot(&self) -> Self {
        Self {
            path: self.path.clone(),
            data: Arc::new(Mutex::new(self.data.lock().await.clone())),
        }
    }
    pub async fn begin(&self) -> Result<Transaction, String> {
        let guard = self.data.clone().lock_owned().await;
        let working = guard.clone();
        Ok(Transaction {
            store: self.clone(),
            guard,
            working,
        })
    }
    pub fn read(
        &self,
        domain: &str,
        kind: &str,
        args: Vec<Value>,
        order: &str,
    ) -> impl std::future::Future<Output = Records> + Send {
        let store = self.clone();
        let domain = domain.to_owned();
        let kind = kind.to_owned();
        let order = order.to_owned();
        async move {
            let mut data = store.data.lock().await;
            let result = (|| {
                if (domain == "invoices" && kind == "id")
                    || matches!(domain.as_str(), "invoice_lines" | "invoice_payments")
                {
                    if let Some(id) = args
                        .first()
                        .and_then(Value::as_str)
                        .filter(|_| domain == "invoices" || kind == "invoice_children")
                    {
                        store.hydrate(&mut data, id)?;
                    } else {
                        let ids = data.index.invoices.ids.clone();
                        for id in ids {
                            store.hydrate(&mut data, &id)?;
                        }
                    }
                }
                Ok(select(&data, &domain, &kind, &args, &order))
            })();
            Records(result)
        }
    }

    pub async fn insert(&self, domain: &str, value: Value) -> Result<Change, String> {
        let mut tx = self.begin().await?;
        let change = tx.insert(domain, value).await?;
        tx.commit().await?;
        Ok(change)
    }
    pub async fn update(&self, domain: &str, id: &str, value: Value) -> Result<Change, String> {
        let mut tx = self.begin().await?;
        let change = tx.update(domain, id, value).await?;
        tx.commit().await?;
        Ok(change)
    }
    pub async fn remove(&self, domain: &str, key: &str, id: &str) -> Result<Change, String> {
        let mut tx = self.begin().await?;
        let change = tx.remove(domain, key, id).await?;
        tx.commit().await?;
        Ok(change)
    }
}
impl Transaction {
    fn prepare(&mut self, domain: &str, value: &Value) -> Result<(), String> {
        if matches!(domain, "invoices" | "invoice_lines" | "invoice_payments") {
            let key = if domain == "invoices" {
                "id"
            } else {
                "invoice_id"
            };
            let id = value[key].as_str().ok_or("Missing invoice ID")?;
            self.store.hydrate(&mut self.working, id)?;
            self.working.dirty.insert(id.into());
            self.working.loaded.insert(id.into());
        }
        Ok(())
    }
    /// Change a draft invoice ID and all its local references in the same transaction.
    pub async fn rekey_invoice(&mut self, old: &str, new: &str) -> Result<(), String> {
        if old == new {
            return Ok(());
        }
        self.store.hydrate(&mut self.working, old)?;
        if items(&self.working, "invoices")
            .iter()
            .any(|r| r.0["id"] == new)
        {
            return Err("Invoice with the same content already exists".into());
        }
        let invoice = self
            .working
            .records
            .get_mut("invoices")
            .and_then(|rows| rows.iter_mut().find(|r| r.0["id"] == old))
            .ok_or("Invoice not found")?;
        invoice.0["id"] = json!(new);
        for domain in ["invoice_lines", "invoice_payments", "sdi_notifications"] {
            for row in self.working.records.entry(domain.into()).or_default() {
                if row.0["invoice_id"] == old {
                    row.0["invoice_id"] = json!(new);
                }
            }
        }
        self.working.dirty.insert(old.into());
        self.working.dirty.insert(new.into());
        self.working.loaded.remove(old);
        self.working.loaded.insert(new.into());
        Ok(())
    }
    /// Change a client's fiscal identity only when no invoice refers to it.
    pub async fn rekey_client(&mut self, old: &str, new: &str) -> Result<(), String> {
        if old == new {
            return Ok(());
        }
        if items(&self.working, "clients")
            .iter()
            .any(|r| r.0["id"] == new)
        {
            return Err("Client with the same fiscal identity already exists".into());
        }
        if items(&self.working, "invoices")
            .iter()
            .any(|r| r.0["cedente_id"] == old || r.0["cessionario_id"] == old)
        {
            return Err(
                "Cannot change the fiscal identity of a client referenced by invoices".into(),
            );
        }
        let client = self
            .working
            .records
            .get_mut("clients")
            .and_then(|rows| rows.iter_mut().find(|r| r.0["id"] == old))
            .ok_or("Client not found")?;
        client.0["id"] = json!(new);
        Ok(())
    }
    pub fn snapshot(&self) -> FileStore {
        FileStore {
            path: self.store.path.clone(),
            data: Arc::new(Mutex::new(self.working.clone())),
        }
    }
    pub async fn commit(mut self) -> Result<(), String> {
        self.working.index = self.store.persist(&self.working)?;
        self.working.dirty.clear();
        *self.guard = self.working;
        Ok(())
    }
    pub fn read(&self, domain: &str, kind: &str, args: Vec<Value>, order: &str) -> Records {
        Records(Ok(select(&self.working, domain, kind, &args, order)))
    }
    pub async fn insert(&mut self, domain: &str, value: Value) -> Result<Change, String> {
        validate_shape(domain, &value)?;
        self.prepare(domain, &value)?;
        if domain == "settings" {
            self.working
                .records
                .insert(domain.into(), vec![Record(value)]);
            return Ok(Change(1));
        }
        validate(&self.working, domain, &value, None)?;
        self.working
            .records
            .entry(domain.into())
            .or_default()
            .push(Record(value));
        Ok(Change(1))
    }
    pub async fn update(&mut self, domain: &str, id: &str, value: Value) -> Result<Change, String> {
        if domain == "invoices" {
            self.store.hydrate(&mut self.working, id)?;
            self.working.dirty.insert(id.into());
        }
        let Some(old) = items(&self.working, domain)
            .iter()
            .find(|r| r.0["id"] == id)
        else {
            return Ok(Change(0));
        };
        let mut merged = old.0.clone();
        for (k, v) in value.as_object().ok_or("Invalid record")? {
            merged[k] = v.clone();
        }
        let same_invoice_key = domain == "invoices"
            && [
                "cedente_id",
                "cessionario_id",
                "numero",
                "data",
                "tipo_documento",
                "art73",
                "soggetto_emittente",
            ]
            .iter()
            .all(|key| old.0[*key] == merged[*key])
            && (old.0["stato"] == "rejected") == (merged["stato"] == "rejected");
        if same_invoice_key {
            validate_shape(domain, &merged)?;
        } else {
            validate(&self.working, domain, &merged, Some(id))?;
        }
        let row = self
            .working
            .records
            .get_mut(domain)
            .unwrap()
            .iter_mut()
            .find(|r| r.0["id"] == id)
            .unwrap();
        row.0 = merged;
        Ok(Change(1))
    }
    pub async fn remove(&mut self, domain: &str, key: &str, id: &str) -> Result<Change, String> {
        if matches!(domain, "invoices" | "invoice_lines" | "invoice_payments") {
            self.store.hydrate(&mut self.working, id)?;
            self.working.dirty.insert(id.into());
        }
        if domain == "clients"
            && items(&self.working, "invoices")
                .iter()
                .any(|r| r.0["cedente_id"] == id || r.0["cessionario_id"] == id)
        {
            return Err("Client is referenced by invoices".into());
        }
        let rows = self.working.records.entry(domain.into()).or_default();
        let before = rows.len();
        rows.retain(|r| r.0[key] != id);
        let count = (before - rows.len()) as u64;
        if domain == "invoices" {
            for child in ["invoice_lines", "invoice_payments"] {
                self.working
                    .records
                    .entry(child.into())
                    .or_default()
                    .retain(|r| r.0["invoice_id"] != id);
            }
            for row in self
                .working
                .records
                .entry("sdi_notifications".into())
                .or_default()
            {
                if row.0["invoice_id"] == id {
                    row.0["invoice_id"] = Value::Null;
                }
            }
        }
        Ok(Change(count))
    }
}
fn items<'a>(data: &'a Data, domain: &str) -> &'a [Record] {
    data.records.get(domain).map(Vec::as_slice).unwrap_or(&[])
}

pub(crate) fn same_invoice_key(existing: &Record, candidate: &Value) -> bool {
    let existing_issuer = if existing.0["soggetto_emittente"] == "CC" {
        &existing.0["cessionario_id"]
    } else {
        &existing.0["cedente_id"]
    };
    let candidate_issuer = if candidate["soggetto_emittente"] == "CC" {
        &candidate["cessionario_id"]
    } else {
        &candidate["cedente_id"]
    };
    existing_issuer == candidate_issuer
        && existing.0["numero"] == candidate["numero"]
        && (if existing.0["art73"] == true || candidate["art73"] == true {
            existing.0["data"] == candidate["data"]
        } else {
            existing.0["data"].as_str().and_then(|date| date.get(..4))
                == candidate["data"].as_str().and_then(|date| date.get(..4))
        })
        && (existing.0["tipo_documento"] == "TD04") == (candidate["tipo_documento"] == "TD04")
}
fn validate(
    data: &Data,
    domain: &str,
    value: &Value,
    excluded: Option<&str>,
) -> Result<(), String> {
    validate_shape(domain, value)?;
    for row in items(data, domain) {
        if excluded.is_some_and(|id| row.0["id"] == id) {
            continue;
        }
        let duplicate = row.0["id"] == value["id"]
            || match domain {
                "clients" => {
                    row.0["id_paese"] == value["id_paese"]
                        && row.0["id_codice"] == value["id_codice"]
                }
                "invoices" => {
                    row.0["stato"] != "rejected"
                        && value["stato"] != "rejected"
                        && same_invoice_key(row, value)
                }
                _ => false,
            };
        if duplicate {
            return Err(format!("Duplicate {domain} record"));
        }
    }
    let references: &[&str] = match domain {
        "invoices" => &["cedente_id", "cessionario_id"],
        "invoice_lines" | "invoice_payments" | "sdi_notifications" => &["invoice_id"],
        _ => &[],
    };
    for key in references {
        if !value[*key].is_null() {
            let target = if domain == "invoices" {
                "clients"
            } else {
                "invoices"
            };
            if !items(data, target).iter().any(|r| r.0["id"] == value[*key]) {
                return Err(format!("Missing reference: {key}"));
            }
        }
    }
    Ok(())
}
fn display_name(row: Option<&Record>) -> String {
    row.map(|r| {
        r.0["denominazione"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| {
                format!(
                    "{} {}",
                    r.0["cognome"].as_str().unwrap_or(""),
                    r.0["nome"].as_str().unwrap_or("")
                )
            })
    })
    .unwrap_or_default()
}
fn select(data: &Data, domain: &str, kind: &str, args: &[Value], order: &str) -> Vec<Record> {
    let source: Vec<Record> = if kind == "invoice_views" {
        data.index
            .invoices
            .ids
            .iter()
            .filter_map(|id| data.index.invoices.entities.get(id).cloned())
            .collect()
    } else if domain == "clients" && kind == "all" {
        data.index
            .clients
            .ids
            .iter()
            .filter_map(|id| data.index.clients.entities.get(id).cloned())
            .collect()
    } else {
        items(data, domain).to_vec()
    };
    let mut rows: Vec<Record> = source
        .iter()
        .filter(|r| match kind {
            "id" => r.0["id"] == args[0],
            "invoice_children" => r.0["invoice_id"] == args[0],
            "tax_identity" => r.0["id_paese"] == args[0] && r.0["id_codice"] == args[1],
            "sent" => matches!(
                r.0["stato"].as_str(),
                Some("sent" | "accepted" | "rejected")
            ),
            "client_invoice_count" => {
                r.0["cedente_id"] == args[0] || r.0["cessionario_id"] == args[1]
            }
            "number_count" => {
                r.0["cedente_id"] == args[0]
                    && r.0["data"]
                        .as_str()
                        .unwrap_or("")
                        .starts_with(args[1].as_str().unwrap_or(""))
                    && r.0["stato"] != "imported"
            }
            "invoice_views" => args.is_empty() || r.0["id"] == args[0],
            _ => true,
        })
        .cloned()
        .collect();
    if matches!(kind, "client_invoice_count" | "number_count") {
        return vec![Record(json!({"cnt":rows.len()}))];
    }
    rows.sort_by(|a, b| match order {
        "client" => display_name(Some(a)).cmp(&display_name(Some(b))),
        "invoice" => b.0["data"]
            .as_str()
            .cmp(&a.0["data"].as_str())
            .then(b.0["numero"].as_str().cmp(&a.0["numero"].as_str())),
        "line" => a.0["numero_linea"]
            .as_i64()
            .cmp(&b.0["numero_linea"].as_i64()),
        _ => std::cmp::Ordering::Equal,
    });
    rows
}

/// Reject malformed snapshots before commands can access their fields.
fn validate_shape(domain: &str, value: &Value) -> Result<(), String> {
    let required: &[&str] = match domain {
        "clients" => &[
            "id",
            "id_paese",
            "id_codice",
            "indirizzo",
            "cap",
            "comune",
            "nazione",
        ],
        "invoices" => &[
            "id",
            "numero",
            "data",
            "tipo_documento",
            "divisa",
            "stato",
            "cedente_id",
            "cessionario_id",
        ],
        "invoice_lines" => &["id", "invoice_id", "descrizione"],
        "invoice_payments" => &["id", "invoice_id", "modalita_pagamento"],
        "sdi_notifications" => &["id", "tipo", "raw_xml"],
        "pec_sync" => &["id", "email"],
        "settings" => &["key", "value"],
        _ => return Err(format!("Unknown desktop data domain: {domain}")),
    };
    if !value.is_object() || required.iter().any(|key| !value[*key].is_string()) {
        return Err(format!("Invalid required fields in {domain}"));
    }
    for key in [
        "denominazione",
        "nome",
        "cognome",
        "codice_fiscale",
        "regime_fiscale",
        "numero_civico",
        "provincia",
        "telefono",
        "email",
        "pec",
        "codice_destinatario",
        "xml_content",
        "unita_misura",
        "natura",
        "condizioni_pagamento",
        "data_scadenza_pagamento",
        "iban",
        "istituto_finanziario",
        "identificativo_sdi",
        "nome_file",
        "esito",
        "errori",
    ] {
        if !value[key].is_null() && !value[key].is_string() {
            return Err(format!("Invalid string field: {domain}.{key}"));
        }
    }
    for key in ["importo_totale", "quantita"] {
        if !value[key].is_null() && !value[key].is_number() {
            return Err(format!("Invalid number field: {domain}.{key}"));
        }
    }
    let numbers: &[&str] = match domain {
        "invoice_lines" => &["prezzo_unitario", "prezzo_totale", "aliquota_iva"],
        "invoice_payments" => &["importo_pagamento"],
        _ => &[],
    };
    if numbers.iter().any(|key| !value[*key].is_number()) {
        return Err(format!("Invalid numeric fields in {domain}"));
    }
    if domain == "invoice_lines"
        && value["numero_linea"]
            .as_i64()
            .is_none_or(|n| i32::try_from(n).is_err())
    {
        return Err("Invalid line number".into());
    }
    if domain == "settings" {
        serde_json::from_str::<crate::commands::settings::AppSettings>(
            value["value"].as_str().unwrap(),
        )
        .map_err(|e| format!("Invalid settings: {e}"))?;
    }
    Ok(())
}

fn resolve_file(base: &std::path::Path, relative: &str) -> Result<PathBuf, String> {
    let path = std::path::Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("Invalid indexed file path".into());
    }
    Ok(base
        .parent()
        .ok_or("Missing app data directory")?
        .join(path))
}
fn atomic_json(path: &std::path::Path, value: &impl Serialize) -> Result<(), String> {
    use std::io::Write;
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("json.tmp");
    let mut file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        dir: PathBuf,
        store: FileStore,
    }
    impl Fixture {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("eattura-store-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            let store = FileStore::open(dir.join("data.json")).unwrap();
            Self { dir, store }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
    fn client(id: &str) -> Value {
        json!({"id":id,"id_paese":"IT","id_codice":id,"indirizzo":"Via Roma","cap":"00100","comune":"Roma","nazione":"IT"})
    }
    fn invoice() -> Value {
        json!({"id":"invoice","numero":"1","data":"2026-01-01","tipo_documento":"TD01","divisa":"EUR","stato":"draft","cedente_id":"client","cessionario_id":"client"})
    }
    #[tokio::test]
    async fn persists_and_reloads_committed_records() {
        let f = Fixture::new();
        f.store.insert("clients", client("client")).await.unwrap();
        let mut tx = f.store.begin().await.unwrap();
        tx.insert("invoices", invoice()).await.unwrap();
        tx.insert("invoice_payments",json!({"id":"payment","invoice_id":"invoice","modalita_pagamento":"MP05","importo_pagamento":12.5})).await.unwrap();
        tx.commit().await.unwrap();
        let reopened = FileStore::open(f.dir.join("data.json")).unwrap();
        let row = reopened
            .read("invoice_payments", "id", vec![json!("payment")], "")
            .await
            .fetch_one()
            .await
            .unwrap();
        assert_eq!(row.get::<f64>("importo_pagamento"), 12.5);
    }
    #[tokio::test]
    async fn dropped_transaction_and_failed_write_leave_memory_unchanged() {
        let f = Fixture::new();
        {
            let mut tx = f.store.begin().await.unwrap();
            tx.insert("clients", client("discarded")).await.unwrap();
        }
        std::fs::create_dir(f.dir.join("index.json.tmp")).unwrap();
        assert!(f.store.insert("clients", client("failed")).await.is_err());
        assert!(
            f.store
                .read("clients", "all", vec![], "")
                .await
                .fetch_all()
                .await
                .unwrap()
                .is_empty()
        );
        assert!(!f.dir.join("data.json").exists());
    }
    #[tokio::test]
    async fn enforces_uniqueness_references_and_cascade() {
        let f = Fixture::new();
        assert!(f.store.insert("invoices", invoice()).await.is_err());
        f.store.insert("clients", client("client")).await.unwrap();
        let mut duplicate = client("other");
        duplicate["id_codice"] = json!("client");
        assert!(f.store.insert("clients", duplicate).await.is_err());
        f.store.insert("invoices", invoice()).await.unwrap();
        let mut duplicate = invoice();
        duplicate["id"] = json!("other");
        assert!(f.store.insert("invoices", duplicate).await.is_err());
        let mut same_year = invoice();
        same_year["id"] = json!("same-year");
        same_year["data"] = json!("2026-12-31");
        assert!(f.store.insert("invoices", same_year).await.is_err());
        let mut credit_note = invoice();
        credit_note["id"] = json!("credit-note");
        credit_note["tipo_documento"] = json!("TD04");
        f.store.insert("invoices", credit_note).await.unwrap();
        assert!(f.store.remove("clients", "id", "client").await.is_err());
        f.store.insert("invoice_payments",json!({"id":"payment","invoice_id":"invoice","modalita_pagamento":"MP05","importo_pagamento":12.5})).await.unwrap();
        f.store.remove("invoices", "id", "invoice").await.unwrap();
        f.store
            .remove("invoices", "id", "credit-note")
            .await
            .unwrap();
        assert!(
            f.store
                .read("invoice_payments", "all", vec![], "")
                .await
                .fetch_all()
                .await
                .unwrap()
                .is_empty()
        );
        f.store.remove("clients", "id", "client").await.unwrap();
    }
    #[tokio::test]
    async fn invoice_identity_handles_art73_and_customer_issued_documents() {
        let f = Fixture::new();
        f.store.insert("clients", client("seller")).await.unwrap();
        f.store.insert("clients", client("buyer")).await.unwrap();
        let mut first = invoice();
        first["cedente_id"] = json!("seller");
        first["cessionario_id"] = json!("buyer");
        first["art73"] = json!(true);
        f.store.insert("invoices", first).await.unwrap();

        let mut later = invoice();
        later["id"] = json!("later");
        later["cedente_id"] = json!("seller");
        later["cessionario_id"] = json!("buyer");
        later["data"] = json!("2026-02-01");
        later["art73"] = json!(true);
        f.store.insert("invoices", later).await.unwrap();

        let mut same_date = invoice();
        same_date["id"] = json!("same-date");
        same_date["cedente_id"] = json!("seller");
        same_date["cessionario_id"] = json!("buyer");
        same_date["art73"] = json!(true);
        assert!(f.store.insert("invoices", same_date).await.is_err());

        let mut customer_issued = invoice();
        customer_issued["id"] = json!("customer-issued");
        customer_issued["cedente_id"] = json!("seller");
        customer_issued["cessionario_id"] = json!("buyer");
        customer_issued["soggetto_emittente"] = json!("CC");
        f.store.insert("invoices", customer_issued).await.unwrap();
    }
    #[tokio::test]
    async fn concurrent_writers_preserve_all_records() {
        let f = Fixture::new();
        let a = f.store.clone();
        let b = f.store.clone();
        let (a, b) = tokio::join!(
            a.insert("clients", client("a")),
            b.insert("clients", client("b"))
        );
        a.unwrap();
        b.unwrap();
        assert_eq!(
            f.store
                .read("clients", "all", vec![], "")
                .await
                .fetch_all()
                .await
                .unwrap()
                .len(),
            2
        );
        let reopened = FileStore::open(f.dir.join("data.json")).unwrap();
        assert_eq!(
            reopened
                .read("clients", "all", vec![], "")
                .await
                .fetch_all()
                .await
                .unwrap()
                .len(),
            2
        );
    }
    #[tokio::test]
    async fn startup_and_search_use_only_the_catalog() {
        let f = Fixture::new();
        let mut c = client("client");
        c["denominazione"] = json!("Acme Roma");
        c["codice_fiscale"] = json!("ABC123");
        f.store.insert("clients", c).await.unwrap();
        let mut tx = f.store.begin().await.unwrap();
        tx.insert("invoices", invoice()).await.unwrap();
        tx.insert("invoice_lines", json!({"id":"line","invoice_id":"invoice","numero_linea":1,"descrizione":"Consulenza software","prezzo_unitario":100.0,"prezzo_totale":100.0,"aliquota_iva":22.0})).await.unwrap();
        tx.commit().await.unwrap();
        let reopened = FileStore::open(f.dir.join("data.json")).unwrap();
        assert!(reopened.data.lock().await.loaded.is_empty());
        std::fs::rename(f.dir.join("invoices"), f.dir.join("hidden-invoices")).unwrap();
        let rows = reopened
            .read("invoices", "invoice_views", vec![], "invoice")
            .await
            .fetch_all()
            .await
            .unwrap();
        let text: String = rows[0].get("search_text");
        assert!(text.contains("consulenza software"));
        assert!(text.contains("acme roma"));
        assert!(text.contains("abc123"));
        let state = reopened.data.lock().await;
        assert_eq!(
            state.index.invoices_by_client["client"].issued,
            vec!["invoice"]
        );
        assert_eq!(state.index.invoices_by_status["draft"], vec!["invoice"]);
        assert_eq!(state.index.invoices_by_year["2026"], vec!["invoice"]);
        assert!(state.loaded.is_empty());
        drop(state);
        assert!(
            reopened
                .read("invoices", "id", vec![json!("invoice")], "")
                .await
                .fetch_one()
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn metadata_changes_refresh_index_without_loading_documents() {
        let f = Fixture::new();
        f.store.insert("clients", client("client")).await.unwrap();
        f.store.insert("invoices", invoice()).await.unwrap();
        let reopened = FileStore::open(f.dir.join("data.json")).unwrap();
        let before: Index =
            serde_json::from_slice(&std::fs::read(f.dir.join("index.json")).unwrap()).unwrap();
        reopened
            .update(
                "clients",
                "client",
                json!({"denominazione":"Updated Company"}),
            )
            .await
            .unwrap();
        assert!(reopened.data.lock().await.loaded.is_empty());
        let after: Index =
            serde_json::from_slice(&std::fs::read(f.dir.join("index.json")).unwrap()).unwrap();
        assert_eq!(
            before.invoices.entities["invoice"].0["file"],
            after.invoices.entities["invoice"].0["file"]
        );
        assert!(
            after.invoices.entities["invoice"]
                .get::<String>("search_text")
                .contains("updated company")
        );
        reopened
            .update("invoices", "invoice", json!({"stato":"validated"}))
            .await
            .unwrap();
        let reopened = FileStore::open(f.dir.join("data.json")).unwrap();
        assert_eq!(
            reopened.data.lock().await.index.invoices_by_status["validated"],
            vec!["invoice"]
        );
        reopened.remove("invoices", "id", "invoice").await.unwrap();
        let data = reopened.data.lock().await;
        assert!(data.index.invoices.ids.is_empty());
        assert!(data.index.invoices_by_client["client"].issued.is_empty());
    }
    #[tokio::test]
    async fn failed_catalog_publish_keeps_previous_documents_and_index() {
        let f = Fixture::new();
        f.store.insert("clients", client("client")).await.unwrap();
        f.store.insert("invoices", invoice()).await.unwrap();
        let original = std::fs::read(f.dir.join("index.json")).unwrap();
        std::fs::create_dir(f.dir.join("index.json.tmp")).unwrap();
        assert!(
            f.store
                .update("invoices", "invoice", json!({"numero":"changed"}))
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(f.dir.join("index.json")).unwrap(), original);
        let reopened = FileStore::open(f.dir.join("data.json")).unwrap();
        let row = reopened
            .read("invoices", "id", vec![json!("invoice")], "")
            .await
            .fetch_one()
            .await
            .unwrap();
        assert_eq!(row.get::<String>("numero"), "1");
    }
    #[test]
    fn migrates_the_previous_snapshot_without_deleting_it() {
        let f = Fixture::new();
        std::fs::remove_file(f.dir.join("index.json")).unwrap();
        std::fs::remove_dir_all(f.dir.join("state")).unwrap();
        let old = json!({"clients":[client("client")], "invoices":[invoice()], "invoice_lines":[], "invoice_payments":[]});
        let bytes = serde_json::to_vec(&old).unwrap();
        std::fs::write(f.dir.join("data.json"), &bytes).unwrap();
        let reopened = FileStore::open(f.dir.join("data.json")).unwrap();
        assert_eq!(std::fs::read(f.dir.join("data.json")).unwrap(), bytes);
        assert!(f.dir.join("index.json").exists());
        assert!(
            reopened
                .data
                .try_lock()
                .unwrap()
                .index
                .invoices
                .entities
                .contains_key("invoice")
        );
    }
    #[test]
    fn missing_catalog_does_not_reset_existing_storage() {
        let f = Fixture::new();
        std::fs::remove_file(f.dir.join("index.json")).unwrap();
        assert!(FileStore::open(f.dir.join("data.json")).is_err());
        assert!(!f.dir.join("index.json").exists());
    }
    #[tokio::test]
    async fn rekeying_a_draft_updates_children_and_notifications() {
        let f = Fixture::new();
        f.store.insert("clients", client("client")).await.unwrap();
        f.store.insert("invoices", invoice()).await.unwrap();
        f.store.insert("invoice_lines", json!({"id":"line","invoice_id":"invoice","numero_linea":1,"descrizione":"Service","prezzo_unitario":2.0,"prezzo_totale":2.0,"aliquota_iva":22.0})).await.unwrap();
        f.store
            .insert(
                "sdi_notifications",
                json!({"id":"notice","invoice_id":"invoice","tipo":"RC","raw_xml":"<RC/>"}),
            )
            .await
            .unwrap();
        let mut tx = f.store.begin().await.unwrap();
        let new_id = digest::content_id(&json!({"document":"changed invoice"})).unwrap();
        tx.rekey_invoice("invoice", &new_id).await.unwrap();
        tx.commit().await.unwrap();
        let reopened = FileStore::open(f.dir.join("data.json")).unwrap();
        assert!(
            reopened
                .read("invoices", "id", vec![json!("invoice")], "")
                .await
                .fetch_optional()
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            reopened
                .read("invoices", "id", vec![json!(&new_id)], "")
                .await
                .fetch_one()
                .await
                .is_ok()
        );
        assert_eq!(
            reopened
                .read("invoice_lines", "id", vec![json!("line")], "")
                .await
                .fetch_one()
                .await
                .unwrap()
                .get::<String>("invoice_id"),
            new_id
        );
        assert_eq!(
            reopened
                .read("sdi_notifications", "id", vec![json!("notice")], "")
                .await
                .fetch_one()
                .await
                .unwrap()
                .get::<String>("invoice_id"),
            new_id
        );
        assert_eq!(
            reopened.data.lock().await.index.invoices_by_client["client"].issued,
            vec![new_id]
        );
    }
    #[tokio::test]
    async fn referenced_client_identity_cannot_be_rekeyed() {
        let f = Fixture::new();
        f.store.insert("clients", client("client")).await.unwrap();
        f.store.insert("invoices", invoice()).await.unwrap();
        let mut tx = f.store.begin().await.unwrap();
        assert!(tx.rekey_client("client", "changed").await.is_err());
        drop(tx);
        assert!(
            f.store
                .read("clients", "id", vec![json!("client")], "")
                .await
                .fetch_one()
                .await
                .is_ok()
        );
    }
    #[test]
    fn corrupt_snapshot_is_rejected_without_overwriting_it() {
        let f = Fixture::new();
        let path = f.dir.join("index.json");
        let bytes = br#"{"clients":[{"id":12}]}"#;
        std::fs::write(&path, bytes).unwrap();
        assert!(FileStore::open(path.clone()).is_err());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
