//! Normalized desktop catalog. Invoice documents are loaded only for details.
use super::{Data, Record, display_name, items};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Entities {
    pub ids: Vec<String>,
    pub entities: BTreeMap<String, Record>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClientInvoices {
    pub issued: Vec<String>,
    pub received: Vec<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Index {
    pub version: u32,
    pub clients: Entities,
    pub invoices: Entities,
    pub invoices_by_client: BTreeMap<String, ClientInvoices>,
    pub invoices_by_status: BTreeMap<String, Vec<String>>,
    pub invoices_by_year: BTreeMap<String, Vec<String>>,
    pub client_by_tax_identity: BTreeMap<String, String>,
    pub auxiliary_file: String,
}

pub(super) fn search_text(value: &Value, fields: &[&str]) -> String {
    fields
        .iter()
        .filter_map(|key| match &value[*key] {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl Index {
    pub fn build(data: &Data, files: &BTreeMap<String, String>, auxiliary_file: String) -> Self {
        let mut index = Self {
            version: 1,
            auxiliary_file,
            ..Self::default()
        };
        for client in items(data, "clients") {
            let id: String = client.get("id");
            let mut entry = client.clone();
            entry.0["search_text"] = json!(search_text(
                &client.0,
                &[
                    "denominazione",
                    "nome",
                    "cognome",
                    "id_paese",
                    "id_codice",
                    "codice_fiscale",
                    "comune",
                    "indirizzo",
                    "cap",
                    "provincia",
                    "email",
                    "pec",
                    "telefono",
                ]
            ));
            // Both name orders remain searchable.
            let text = format!(
                "{} {}",
                entry.0["search_text"].as_str().unwrap_or(""),
                display_name(Some(client)).to_lowercase()
            );
            entry.0["search_text"] = json!(text);
            index.client_by_tax_identity.insert(
                format!(
                    "{}:{}",
                    client.get::<String>("id_paese"),
                    client.get::<String>("id_codice")
                ),
                id.clone(),
            );
            index.clients.ids.push(id.clone());
            index.clients.entities.insert(id.clone(), entry);
            index
                .invoices_by_client
                .insert(id, ClientInvoices::default());
        }
        for invoice in items(data, "invoices") {
            let id: String = invoice.get("id");
            let mut entry = invoice.clone();
            let fields = entry.0.as_object_mut().unwrap();
            fields.remove("xml_content");
            fields.remove("pending_pec_xml");
            let previous = data.index.invoices.entities.get(&id);
            let file = files
                .get(&id)
                .cloned()
                .or_else(|| previous.and_then(|r| r.0["file"].as_str().map(str::to_owned)))
                .expect("Invoice document path must exist");
            entry.0["file"] = json!(file);
            let line_terms = if data.dirty.contains(&id) {
                items(data, "invoice_lines")
                    .iter()
                    .filter(|r| r.0["invoice_id"] == id)
                    .map(|r| r.0["descrizione"].as_str().unwrap_or("").to_owned())
                    .collect::<Vec<_>>()
            } else {
                previous
                    .map(|r| r.get::<Vec<String>>("line_terms"))
                    .unwrap_or_default()
            };
            let mut terms = vec![
                search_text(
                    &invoice.0,
                    &[
                        "numero",
                        "data",
                        "tipo_documento",
                        "stato",
                        "importo_totale",
                        "divisa",
                    ],
                ),
                line_terms.join(" ").to_lowercase(),
            ];
            entry.0["line_terms"] = json!(line_terms);
            for (side, key) in [("cedente", "cedente_id"), ("cessionario", "cessionario_id")] {
                let client_id: String = invoice.get(key);
                let client = index.clients.entities.get(&client_id);
                entry.0[format!("{side}_denominazione")] = json!(display_name(client));
                for field in ["id_paese", "id_codice"] {
                    entry.0[format!("{side}_{field}")] =
                        client.map(|c| c.0[field].clone()).unwrap_or(Value::Null);
                }
                if let Some(client) = client {
                    terms.push(client.get("search_text"));
                }
                if let Some(links) = index.invoices_by_client.get_mut(&client_id) {
                    if side == "cedente" {
                        links.issued.push(id.clone());
                    } else {
                        links.received.push(id.clone());
                    }
                }
            }
            entry.0["search_text"] = json!(terms.join(" "));
            index
                .invoices_by_status
                .entry(invoice.get("stato"))
                .or_default()
                .push(id.clone());
            let date: String = invoice.get("data");
            index
                .invoices_by_year
                .entry(date.chars().take(4).collect())
                .or_default()
                .push(id.clone());
            index.invoices.ids.push(id.clone());
            index.invoices.entities.insert(id, entry);
        }
        index
    }
}
