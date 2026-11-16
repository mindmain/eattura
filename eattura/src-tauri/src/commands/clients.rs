use crate::storage::{Record, digest};
use serde::{Deserialize, Serialize};

use crate::state::TauriState;

/// Summary view of a client for list display.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSummary {
    pub id: String,
    pub denominazione: Option<String>,
    pub nome: Option<String>,
    pub cognome: Option<String>,
    pub id_paese: String,
    pub id_codice: String,
    pub comune: String,
    #[serde(default)]
    pub has_tax_id_error: bool,
    pub search_text: String,
    #[serde(default)]
    pub is_pa: bool,
    #[serde(default)]
    pub codice_destinatario: Option<String>,
}

/// Full client detail.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientDetail {
    pub id: String,
    pub denominazione: Option<String>,
    pub nome: Option<String>,
    pub cognome: Option<String>,
    pub id_paese: String,
    pub id_codice: String,
    pub codice_fiscale: Option<String>,
    pub regime_fiscale: Option<String>,
    pub indirizzo: String,
    pub numero_civico: Option<String>,
    pub cap: String,
    pub comune: String,
    pub provincia: Option<String>,
    pub nazione: String,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub pec: Option<String>,
    pub codice_destinatario: Option<String>,
    #[serde(default)]
    pub is_pa: bool,
}

/// Request body for creating or updating a client.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClientRequest {
    pub denominazione: Option<String>,
    pub nome: Option<String>,
    pub cognome: Option<String>,
    pub id_paese: String,
    #[serde(default)]
    pub id_codice: String,
    pub codice_fiscale: Option<String>,
    pub regime_fiscale: Option<String>,
    pub indirizzo: String,
    pub numero_civico: Option<String>,
    pub cap: String,
    pub comune: String,
    pub provincia: Option<String>,
    pub nazione: String,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub pec: Option<String>,
    pub codice_destinatario: Option<String>,
    #[serde(default)]
    pub is_pa: Option<bool>,
}

fn resolve_tax_identifiers(
    client: &CreateClientRequest,
) -> Result<(String, Option<String>), String> {
    let raw_id_codice = client.id_codice.trim();
    let raw_cf = client
        .codice_fiscale
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_uppercase);

    if client.id_paese.trim().eq_ignore_ascii_case("IT") {
        let vat = (!raw_id_codice.is_empty()).then_some(raw_id_codice);
        validate_italian_tax_fields(vat, raw_cf.as_deref())?;
    }

    let effective_id_codice = if !raw_id_codice.is_empty() {
        raw_id_codice.to_ascii_uppercase()
    } else if let Some(ref cf) = raw_cf {
        cf.clone()
    } else {
        return Err("Specificare Partita IVA o Codice Fiscale per l'anagrafica".into());
    };

    Ok((effective_id_codice, raw_cf))
}

pub(super) fn validate_italian_tax_fields(
    vat: Option<&str>,
    cf: Option<&str>,
) -> Result<(), String> {
    use common::validation::rules::{valid_italian_fiscal_code, valid_numeric_tax_id};

    if vat.is_some_and(|value| !valid_numeric_tax_id(value)) {
        return Err(
            "Partita IVA italiana non valida. Controlla le 11 cifre e la cifra di controllo."
                .into(),
        );
    }
    if cf.is_some_and(|value| !valid_italian_fiscal_code(value)) {
        return Err(
            "Codice fiscale italiano non valido. Controlla il formato e il carattere di controllo."
                .into(),
        );
    }
    Ok(())
}

pub(crate) fn normalize_recipient_code(code: Option<&str>) -> Option<String> {
    code.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_uppercase)
}

pub(crate) fn valid_invoice_recipient_code(code: &str, id_paese: &str) -> Option<String> {
    let normalized = normalize_recipient_code(Some(code))?;
    if !matches!(normalized.len(), 6 | 7)
        || !normalized.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return None;
    }
    if normalized == "XXXXXXX" && id_paese.trim().eq_ignore_ascii_case("IT") {
        return None;
    }
    Some(normalized)
}

pub(crate) fn valid_invoice_recipient_pec(pec: Option<&str>) -> Option<String> {
    pec.map(str::trim)
        .filter(|value| {
            !value.is_empty() && !value.to_ascii_lowercase().ends_with("@pec.fatturapa.it")
        })
        .map(str::to_owned)
}

pub(crate) fn resolve_client_recipient_code(code: Option<&str>, is_pa: bool) -> Option<String> {
    match normalize_recipient_code(code) {
        Some(normalized) => Some(normalized),
        None if !is_pa => Some("0000000".to_string()),
        None => None,
    }
}

pub(crate) fn has_explicit_recipient_destination(row: &Record) -> bool {
    if row.get::<Option<bool>>("recipient_configured") == Some(true) {
        return true;
    }
    let current_cd =
        normalize_recipient_code(row.get::<Option<String>>("codice_destinatario").as_deref());
    let current_is_pa = row
        .get::<Option<bool>>("is_pa")
        .unwrap_or_else(|| current_cd.as_deref().is_some_and(|code| code.len() == 6));
    if current_cd.as_deref().is_some_and(|code| code != "0000000") {
        return true;
    }
    let current_pec = row.get::<Option<String>>("pec");
    !current_is_pa
        && valid_invoice_recipient_pec(current_pec.as_deref()).is_some()
        && matches!(current_cd.as_deref(), None | Some("0000000"))
}

pub(crate) fn merge_client_recipient_destination(
    existing: &Record,
    candidate_cd: Option<&str>,
    candidate_pec: Option<&str>,
) -> (Option<String>, Option<String>, bool) {
    let current_cd = normalize_recipient_code(
        existing
            .get::<Option<String>>("codice_destinatario")
            .as_deref(),
    );
    let current_is_pa = existing
        .get::<Option<bool>>("is_pa")
        .unwrap_or_else(|| current_cd.as_deref().is_some_and(|code| code.len() == 6));
    let current_pec = existing
        .get::<Option<String>>("pec")
        .map(|pec| pec.trim().to_owned())
        .filter(|pec| !pec.is_empty());

    if has_explicit_recipient_destination(existing) {
        let next_cd = resolve_client_recipient_code(current_cd.as_deref(), current_is_pa);
        let next_is_pa = next_cd.as_deref().is_some_and(|code| code.len() == 6) || current_is_pa;
        return (next_cd, current_pec, next_is_pa);
    }

    let normalized_candidate_cd = normalize_recipient_code(candidate_cd);
    let next_cd = match normalized_candidate_cd.as_deref() {
        Some(inc) if inc != "0000000" => Some(inc.to_string()),
        _ => current_cd
            .or(normalized_candidate_cd)
            .or_else(|| (!current_is_pa).then(|| "0000000".to_string())),
    };
    let next_is_pa = next_cd.as_deref().is_some_and(|code| code.len() == 6) || current_is_pa;
    let next_pec = current_pec.or_else(|| valid_invoice_recipient_pec(candidate_pec));
    (next_cd, next_pec, next_is_pa)
}

#[derive(Default)]
struct DiscoveredDestination {
    explicit_channel: Option<(String, Option<String>)>,
    any_code: Option<String>,
    fallback_pec: Option<String>,
}

/// Recover missing or placeholder `codice_destinatario` and `pec` values for existing
/// clients from already-archived invoice XML headers without requiring XML re-import.
/// Explicit recipient configurations (including `"0000000"` paired with a PEC) are preserved.
/// Non-PA clients without any code in archived invoices default to `"0000000"`.
pub(crate) async fn backfill_client_destinations(
    store: &crate::storage::FileStore,
) -> Result<usize, String> {
    use std::collections::HashMap;

    let clients = store
        .read("clients", "all", vec![], "")
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let mut needs_write = false;
    let mut target_countries: HashMap<String, String> = HashMap::new();
    for row in &clients {
        let backfilled = row.get::<Option<bool>>("recipient_backfilled") == Some(true);
        let cd =
            normalize_recipient_code(row.get::<Option<String>>("codice_destinatario").as_deref());
        let is_pa = row
            .get::<Option<bool>>("is_pa")
            .unwrap_or_else(|| cd.as_deref().is_some_and(|code| code.len() == 6));
        if !backfilled || (!is_pa && cd.is_none()) {
            needs_write = true;
            if !has_explicit_recipient_destination(row) {
                target_countries.insert(row.get("id"), row.get("id_paese"));
            }
        }
    }

    if !needs_write {
        return Ok(0);
    }

    let mut discovered: HashMap<String, DiscoveredDestination> = HashMap::new();
    if !target_countries.is_empty() {
        let invoice_views = store
            .read("invoices", "invoice_views", vec![], "invoice")
            .await
            .fetch_all()
            .await
            .map_err(|e| e.to_string())?;

        for inv_view in &invoice_views {
            let buyer_id: String = inv_view.get("cessionario_id");
            let Some(country) = target_countries.get(&buyer_id) else {
                continue;
            };
            if discovered
                .get(&buyer_id)
                .is_some_and(|entry| entry.explicit_channel.is_some() && entry.fallback_pec.is_some())
            {
                continue;
            }
            let inv_id: String = inv_view.get("id");
            let Ok(Some(full_inv)) = store
                .read("invoices", "id", vec![serde_json::json!(&inv_id)], "")
                .await
                .fetch_optional()
                .await
            else {
                continue;
            };
            let Some(xml) = full_inv
                .get::<Option<String>>("xml_content")
                .filter(|xml| !xml.trim().is_empty())
            else {
                continue;
            };
            let Ok(Some(header)) = common::xml::decode::decode_header(&xml) else {
                continue;
            };
            let Some(dt) = header.dati_trasmissione.as_ref() else {
                continue;
            };
            let code = valid_invoice_recipient_code(&dt.codice_destinatario, country);
            let pec = valid_invoice_recipient_pec(dt.pec_destinatario.as_deref());
            let entry = discovered.entry(buyer_id).or_default();
            if entry.any_code.is_none() {
                entry.any_code = code.clone();
            }
            if entry.fallback_pec.is_none() {
                entry.fallback_pec = pec.clone();
            }
            if entry.explicit_channel.is_none() {
                match code.as_deref() {
                    Some(c) if c != "0000000" => {
                        entry.explicit_channel = Some((c.to_string(), pec));
                    }
                    Some("0000000") if pec.is_some() => {
                        entry.explicit_channel = Some(("0000000".to_string(), pec));
                    }
                    _ => {}
                }
            }
        }
    }

    let mut tx = store.begin().await?;
    let current_clients = tx
        .read("clients", "all", vec![], "")
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let mut wrote_any = false;
    let mut upgraded_count = 0usize;
    for row in &current_clients {
        let id: String = row.get("id");
        let backfilled = row.get::<Option<bool>>("recipient_backfilled") == Some(true);
        let raw_cd: Option<String> = row.get("codice_destinatario");
        let current_cd = normalize_recipient_code(raw_cd.as_deref());
        let current_is_pa = row
            .get::<Option<bool>>("is_pa")
            .unwrap_or_else(|| current_cd.as_deref().is_some_and(|code| code.len() == 6));
        if backfilled && (current_is_pa || current_cd.is_some()) {
            continue;
        }

        let found = discovered.get(&id);
        let (candidate_cd, candidate_pec) = match found {
            Some(entry) => match entry.explicit_channel.as_ref() {
                Some((code, pec)) if code == "0000000" => (Some(code.as_str()), pec.as_deref()),
                Some((code, pec)) => (
                    Some(code.as_str()),
                    pec.as_deref().or(entry.fallback_pec.as_deref()),
                ),
                None => (entry.any_code.as_deref(), entry.fallback_pec.as_deref()),
            },
            None => (None, None),
        };
        let (next_cd, next_pec, next_is_pa) =
            merge_client_recipient_destination(row, candidate_cd, candidate_pec);

        let data_changed = raw_cd != next_cd
            || row.get::<Option<String>>("pec") != next_pec
            || row.get::<Option<bool>>("is_pa") != Some(next_is_pa);
        if data_changed {
            upgraded_count += 1;
        }

        tx.update(
            "clients",
            &id,
            serde_json::json!({
                "codice_destinatario": next_cd,
                "pec": next_pec,
                "is_pa": next_is_pa,
                "recipient_backfilled": true,
            }),
        )
        .await
        .map_err(|e| e.to_string())?;
        wrote_any = true;
    }

    if wrote_any {
        tx.commit().await?;
    }

    Ok(upgraded_count)
}

/// List all clients from the local file store.
///
/// Returns a summary view with key fields, ordered by company name or surname.
#[tauri::command]
pub async fn list_clients(
    state: tauri::State<'_, TauriState>,
) -> Result<Vec<ClientSummary>, String> {
    list_clients_store(&state.store).await
}

pub async fn list_clients_store(
    store: &crate::storage::FileStore,
) -> Result<Vec<ClientSummary>, String> {
    backfill_client_destinations(store).await?;

    let rows = store
        .read("clients", "all", vec![], "client")
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let clients = rows
        .iter()
        .map(|row| {
            let cd = normalize_recipient_code(
                row.get::<Option<String>>("codice_destinatario").as_deref(),
            );
            let is_pa = row
                .get::<Option<bool>>("is_pa")
                .unwrap_or_else(|| cd.as_deref().map(|c| c.len() == 6).unwrap_or(false));
            let country: String = row.get("id_paese");
            let id: String = row.get("id_codice");
            let cf: Option<String> = row.get("codice_fiscale");
            let has_tax_id_error = country.eq_ignore_ascii_case("IT") && {
                let cf_only = id.len() == 16 && id.bytes().any(|byte| byte.is_ascii_alphabetic());
                let vat = (!cf_only).then_some(id.as_str());
                let cf = cf
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .or_else(|| cf_only.then_some(id.as_str()));
                validate_italian_tax_fields(vat, cf).is_err()
            };
            ClientSummary {
                id: row.get("id"),
                search_text: row.get("search_text"),
                denominazione: row.get("denominazione"),
                nome: row.get("nome"),
                cognome: row.get("cognome"),
                id_paese: country,
                id_codice: id,
                has_tax_id_error,
                comune: row.get("comune"),
                is_pa,
                codice_destinatario: cd,
            }
        })
        .collect();

    Ok(clients)
}

/// Get a single client by ID with all fields.
#[tauri::command]
pub async fn get_client(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<ClientDetail, String> {
    get_client_store(&state.store, &id).await
}

pub async fn get_client_store(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<ClientDetail, String> {
    backfill_client_destinations(store).await?;

    let row = store
        .read("clients", "id", vec![serde_json::json!(id)], "")
        .await
        .fetch_optional()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Client not found: {id}"))?;

    Ok(row_to_client_detail(&row))
}

/// Create a new client.
///
/// Generates a deterministic ID for the new client and inserts it into the file store.
/// Fiscal identity uniqueness prevents duplicate tax IDs.
#[tauri::command]
pub async fn create_client(
    state: tauri::State<'_, TauriState>,
    client: CreateClientRequest,
) -> Result<ClientDetail, String> {
    create_client_store(&state.store, client).await
}

pub async fn create_client_store(
    store: &crate::storage::FileStore,
    client: CreateClientRequest,
) -> Result<ClientDetail, String> {
    let (effective_id_codice, effective_cf) = resolve_tax_identifiers(&client)?;
    let is_pa = client.is_pa.unwrap_or_else(|| {
        client
            .codice_destinatario
            .as_deref()
            .map(|c| c.trim().len() == 6)
            .unwrap_or(false)
    });
    if let Some(cd) = client.codice_destinatario.as_deref() {
        let trimmed = cd.trim();
        if is_pa
            && !trimmed.is_empty()
            && (trimmed.len() != 6 || !trimmed.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            return Err(format!(
                "Codice Destinatario IPA per Pubblica Amministrazione deve essere di 6 caratteri alfanumerici, ricevuto '{}'",
                trimmed
            ));
        }
    }
    let resolved_cd = resolve_client_recipient_code(client.codice_destinatario.as_deref(), is_pa);
    let resolved_pec = client
        .pec
        .as_deref()
        .map(str::trim)
        .filter(|pec| !pec.is_empty())
        .map(str::to_owned);

    let id = digest::client_id(&client.id_paese, &effective_id_codice)?;

    store
        .insert(
            "clients",
            serde_json::json!({
                "id": &id,
                "denominazione": &client.denominazione,
                "nome": &client.nome,
                "cognome": &client.cognome,
                "id_paese": &client.id_paese,
                "id_codice": &effective_id_codice,
                "codice_fiscale": &effective_cf,
                "regime_fiscale": &client.regime_fiscale,
                "indirizzo": &client.indirizzo,
                "numero_civico": &client.numero_civico,
                "cap": &client.cap,
                "comune": &client.comune,
                "provincia": &client.provincia,
                "nazione": &client.nazione,
                "telefono": &client.telefono,
                "email": &client.email,
                "pec": &resolved_pec,
                "codice_destinatario": &resolved_cd,
                "is_pa": is_pa,
                "recipient_backfilled": true,
                "recipient_configured": true
            }),
        )
        .await
        .map_err(|e| e.to_string())?;

    // Fetch the newly created record to return it with persisted fields.
    get_client_store(store, &id).await
}

/// Update an existing client by ID.
///
/// Replaces all mutable fields and persists the updated snapshot.
#[tauri::command]
pub async fn update_client(
    state: tauri::State<'_, TauriState>,
    id: String,
    client: CreateClientRequest,
) -> Result<ClientDetail, String> {
    update_client_store(&state.store, &id, client).await
}

pub async fn update_client_store(
    store: &crate::storage::FileStore,
    id: &str,
    client: CreateClientRequest,
) -> Result<ClientDetail, String> {
    let (effective_id_codice, effective_cf) = resolve_tax_identifiers(&client)?;
    let mut tx = store.begin().await?;
    let existing = tx
        .read("clients", "id", vec![serde_json::json!(id)], "")
        .fetch_optional()
        .await?
        .ok_or_else(|| format!("Client not found: {id}"))?;
    let same_fiscal_identity = existing.get::<String>("id_paese") == client.id_paese
        && existing.get::<String>("id_codice") == effective_id_codice;
    let is_pa = client.is_pa.unwrap_or_else(|| {
        client
            .codice_destinatario
            .as_deref()
            .map(|c| c.trim().len() == 6)
            .unwrap_or(false)
    });
    if let Some(cd) = client.codice_destinatario.as_deref() {
        let trimmed = cd.trim();
        if is_pa
            && !trimmed.is_empty()
            && (trimmed.len() != 6 || !trimmed.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            return Err(format!(
                "Codice Destinatario IPA per Pubblica Amministrazione deve essere di 6 caratteri alfanumerici, ricevuto '{}'",
                trimmed
            ));
        }
    }
    let resolved_cd = resolve_client_recipient_code(client.codice_destinatario.as_deref(), is_pa);
    let resolved_pec = client
        .pec
        .as_deref()
        .map(str::trim)
        .filter(|pec| !pec.is_empty())
        .map(str::to_owned);

    let new_id = if same_fiscal_identity {
        id.to_string()
    } else {
        digest::client_id(&client.id_paese, &effective_id_codice)?
    };
    let result = tx
        .update(
            "clients",
            id,
            serde_json::json!({
                "denominazione": &client.denominazione,
                "nome": &client.nome,
                "cognome": &client.cognome,
                "id_paese": &client.id_paese,
                "id_codice": &effective_id_codice,
                "codice_fiscale": &effective_cf,
                "regime_fiscale": &client.regime_fiscale,
                "indirizzo": &client.indirizzo,
                "numero_civico": &client.numero_civico,
                "cap": &client.cap,
                "comune": &client.comune,
                "provincia": &client.provincia,
                "nazione": &client.nazione,
                "telefono": &client.telefono,
                "email": &client.email,
                "pec": &resolved_pec,
                "codice_destinatario": &resolved_cd,
                "is_pa": is_pa,
                "recipient_backfilled": true,
                "recipient_configured": true
            }),
        )
        .await
        .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Client not found: {id}"));
    }

    tx.rekey_client(id, &new_id).await?;
    tx.commit().await?;
    get_client_store(store, &new_id).await
}

/// Delete a client by ID.
///
/// Fails if the client is referenced by any invoices (as cedente or cessionario).
#[tauri::command]
pub async fn delete_client(state: tauri::State<'_, TauriState>, id: String) -> Result<(), String> {
    delete_client_store(&state.store, &id).await
}

pub async fn delete_client_store(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<(), String> {
    // Check for associated invoices before deleting.
    let row = store
        .read(
            "invoices",
            "client_invoice_count",
            vec![serde_json::json!(id), serde_json::json!(id)],
            "",
        )
        .await
        .fetch_one()
        .await
        .map_err(|e| e.to_string())?;

    let count: i64 = row.get("cnt");
    if count > 0 {
        return Err(format!(
            "Cannot delete client {id}: referenced by {count} invoice(s)"
        ));
    }

    let result = store
        .remove("clients", "id", id)
        .await
        .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Client not found: {id}"));
    }

    Ok(())
}

/// Query the official European Commission VIES REST API to validate a VAT number
/// and autodiscover company details without local or remote persistence.
#[tauri::command]
pub async fn lookup_vat_vies(
    country_code: String,
    vat_number: String,
) -> Result<common::vies::VatLookupResponse, String> {
    if country_code.trim().is_empty() || vat_number.trim().is_empty() {
        return Err("Entrambi i campi 'Codice Paese' e 'Partita IVA' sono obbligatori".into());
    }

    let (country, vat) = common::vies::normalize_vat(&country_code, &vat_number);
    let url = format!("https://ec.europa.eu/taxation_customs/vies/rest-api/ms/{country}/vat/{vat}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Impossibile creare client HTTP: {e}"))?;

    let res = client
        .get(&url)
        .header("User-Agent", "eattura/0.1.0")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Connessione a VIES fallita: {e}"))?;

    let status = res.status();
    if status.is_success() {
        let raw: common::vies::RawViesResponse = res
            .json()
            .await
            .map_err(|e| format!("Risposta VIES non valida: {e}"))?;
        Ok(raw.into_vat_lookup_response(&country_code, &vat))
    } else {
        let body = res.text().await.unwrap_or_default();
        if let Ok(raw) = serde_json::from_str::<common::vies::RawViesResponse>(&body) {
            Ok(raw.into_vat_lookup_response(&country_code, &vat))
        } else {
            Ok(common::vies::VatLookupResponse {
                valid: false,
                country_code: country_code.to_uppercase(),
                vat_number: vat,
                name: None,
                address: None,
                street: None,
                postal_code: None,
                city: None,
                province: None,
                user_error: Some(format!("VIES HTTP {status}: {body}")),
            })
        }
    }
}

fn row_to_client_detail(row: &Record) -> ClientDetail {
    let cd = normalize_recipient_code(row.get::<Option<String>>("codice_destinatario").as_deref());
    let is_pa = row
        .get::<Option<bool>>("is_pa")
        .unwrap_or_else(|| cd.as_deref().map(|c| c.len() == 6).unwrap_or(false));
    ClientDetail {
        id: row.get("id"),
        denominazione: row.get("denominazione"),
        nome: row.get("nome"),
        cognome: row.get("cognome"),
        id_paese: row.get("id_paese"),
        id_codice: row.get("id_codice"),
        codice_fiscale: row.get("codice_fiscale"),
        regime_fiscale: row.get("regime_fiscale"),
        indirizzo: row.get("indirizzo"),
        numero_civico: row.get("numero_civico"),
        cap: row.get("cap"),
        comune: row.get("comune"),
        provincia: row.get("provincia"),
        nazione: row.get("nazione"),
        telefono: row.get("telefono"),
        email: row.get("email"),
        pec: row.get("pec"),
        codice_destinatario: cd,
        is_pa,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store(name: &str) -> (crate::storage::FileStore, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "eattura-test-clients-{}-{}",
            name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.json");
        let store = crate::storage::FileStore::open(path).unwrap();
        (store, dir)
    }

    fn sample_client_request(tax_code: &str, name: &str) -> CreateClientRequest {
        CreateClientRequest {
            denominazione: Some(name.to_string()),
            nome: None,
            cognome: None,
            id_paese: "IT".to_string(),
            id_codice: tax_code.to_string(),
            codice_fiscale: Some(tax_code.to_string()),
            regime_fiscale: Some("RF01".to_string()),
            indirizzo: "Via Roma 1".to_string(),
            numero_civico: Some("1".to_string()),
            cap: "00100".to_string(),
            comune: "Roma".to_string(),
            provincia: Some("RM".to_string()),
            nazione: "IT".to_string(),
            telefono: Some("06123456".to_string()),
            email: Some("info@example.it".to_string()),
            pec: Some("info@pec.example.it".to_string()),
            codice_destinatario: Some("0000000".to_string()),
            is_pa: Some(false),
        }
    }

    #[tokio::test]
    async fn test_client_crud_lifecycle() {
        let (store, _dir) = test_store("crud");
        let req = sample_client_request("12345678903", "Acme Srl");

        // 1. Create client
        let created = create_client_store(&store, req).await.unwrap();
        let expected_id = digest::client_id("IT", "12345678903").unwrap();
        assert_eq!(created.id, expected_id);
        assert_eq!(created.denominazione.as_deref(), Some("Acme Srl"));
        assert_eq!(created.comune, "Roma");

        // 2. Get client
        let fetched = get_client_store(&store, &created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.pec.as_deref(), Some("info@pec.example.it"));

        // 3. List clients
        let list = list_clients_store(&store).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, created.id);
        assert!(list[0].search_text.contains("acme"));

        // 4. Update client (same fiscal identity)
        let mut update_req = sample_client_request("12345678903", "Acme SpA");
        update_req.comune = "Milano".to_string();
        let updated = update_client_store(&store, &created.id, update_req)
            .await
            .unwrap();
        assert_eq!(updated.id, created.id); // Same ID because tax identity is unchanged
        assert_eq!(updated.denominazione.as_deref(), Some("Acme SpA"));
        assert_eq!(updated.comune, "Milano");

        // 5. Delete client
        delete_client_store(&store, &created.id).await.unwrap();
        assert!(get_client_store(&store, &created.id).await.is_err());
        let list_empty = list_clients_store(&store).await.unwrap();
        assert_eq!(list_empty.len(), 0);
    }

    #[tokio::test]
    async fn test_create_private_individual_with_codice_fiscale_only() {
        let (store, _dir) = test_store("cf-only");
        let mut req = sample_client_request("", "");
        req.denominazione = None;
        req.nome = Some("Mario".to_string());
        req.cognome = Some("Rossi".to_string());
        req.id_codice = String::new();
        req.codice_fiscale = Some("rssmra80a01h501u".to_string());

        let created = create_client_store(&store, req).await.unwrap();
        assert_eq!(created.id_codice, "RSSMRA80A01H501U");
        assert_eq!(created.codice_fiscale.as_deref(), Some("RSSMRA80A01H501U"));
    }

    #[tokio::test]
    async fn rejects_italian_tax_id_typos_but_accepts_foreign_formats() {
        let (store, _dir) = test_store("tax-ids");
        let request = sample_client_request("12345678901", "Example");
        let error = create_client_store(&store, request).await.unwrap_err();
        assert!(error.contains("Partita IVA"));

        let request = sample_client_request("12345678903", "Example");
        let created = create_client_store(&store, request).await.unwrap();
        let mut request = sample_client_request("12345678903", "Example");
        request.codice_fiscale = Some("RSSMRA80A01H501V".into());
        let error = update_client_store(&store, &created.id, request).await.unwrap_err();
        assert!(error.contains("Codice fiscale"));

        let mut request = sample_client_request("FOREIGN-VAT", "Foreign example");
        request.id_paese = "DE".into();
        create_client_store(&store, request).await.unwrap();
    }

    #[tokio::test]
    async fn test_client_duplicate_fiscal_id_rejected() {
        let (store, _dir) = test_store("duplicate");
        let req1 = sample_client_request("12345678903", "First Company");
        create_client_store(&store, req1).await.unwrap();

        let req2 = sample_client_request("12345678903", "Second Company");
        let err = create_client_store(&store, req2).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_delete_client_referenced_by_invoice_fails() {
        let (store, _dir) = test_store("ref-protect");
        let seller = create_client_store(&store, sample_client_request("11111111115", "Seller"))
            .await
            .unwrap();
        let buyer = create_client_store(&store, sample_client_request("22222222220", "Buyer"))
            .await
            .unwrap();

        // Create an invoice referencing seller and buyer
        let mut tx = store.begin().await.unwrap();
        tx.insert(
            "invoices",
            serde_json::json!({
                "id": "inv-001",
                "numero": "1",
                "data": "2026-01-01",
                "tipo_documento": "TD01",
                "divisa": "EUR",
                "importo_totale": 100.0,
                "stato": "draft",
                "cedente_id": &seller.id,
                "cessionario_id": &buyer.id
            }),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        // Attempting to delete the seller must fail
        let del_res = delete_client_store(&store, &seller.id).await;
        assert!(del_res.is_err());
        assert!(del_res.unwrap_err().contains("referenced by 1 invoice"));

        // Delete invoice first
        store.remove("invoices", "id", "inv-001").await.unwrap();

        // Now deletion succeeds
        delete_client_store(&store, &seller.id).await.unwrap();
        assert!(get_client_store(&store, &seller.id).await.is_err());
    }

    #[tokio::test]
    async fn test_client_rekeying_when_tax_identity_changes() {
        let (store, _dir) = test_store("rekey");
        let original =
            create_client_store(&store, sample_client_request("11111111115", "Original"))
                .await
                .unwrap();

        // Update with new P.IVA
        let update_req = sample_client_request("33333333335", "Renamed & Rekeyed");
        let updated = update_client_store(&store, &original.id, update_req)
            .await
            .unwrap();

        let new_expected_id = digest::client_id("IT", "33333333335").unwrap();
        assert_eq!(updated.id, new_expected_id);
        assert_ne!(updated.id, original.id);

        // Old ID no longer exists
        assert!(get_client_store(&store, &original.id).await.is_err());
        // New ID exists
        let fetched = get_client_store(&store, &new_expected_id).await.unwrap();
        assert_eq!(fetched.denominazione.as_deref(), Some("Renamed & Rekeyed"));
    }

    #[tokio::test]
    async fn test_vies_vat_lookup_empty_params_error() {
        let res = lookup_vat_vies("".into(), "".into()).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("obbligatori"));
    }

    fn minimal_fattura_xml(
        formato: &str,
        codice_destinatario: &str,
        pec_destinatario: Option<&str>,
        (cedente_vat, cedente_name): (&str, &str),
        (cessionario_vat, cessionario_name): (&str, &str),
        (numero, data): (&str, &str),
    ) -> String {
        let pec_tag = pec_destinatario
            .map(|pec| format!("<PECDestinatario>{pec}</PECDestinatario>"))
            .unwrap_or_default();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<p:FatturaElettronica versione="{formato}" xmlns:p="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2">
  <FatturaElettronicaHeader>
    <DatiTrasmissione>
      <IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>{cedente_vat}</IdCodice></IdTrasmittente>
      <ProgressivoInvio>00001</ProgressivoInvio>
      <FormatoTrasmissione>{formato}</FormatoTrasmissione>
      <CodiceDestinatario>{codice_destinatario}</CodiceDestinatario>
      {pec_tag}
    </DatiTrasmissione>
    <CedentePrestatore>
      <DatiAnagrafici>
        <IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>{cedente_vat}</IdCodice></IdFiscaleIVA>
        <Anagrafica><Denominazione>{cedente_name}</Denominazione></Anagrafica>
        <RegimeFiscale>RF01</RegimeFiscale>
      </DatiAnagrafici>
      <Sede><Indirizzo>Via Roma 1</Indirizzo><CAP>00100</CAP><Comune>Roma</Comune><Provincia>RM</Provincia><Nazione>IT</Nazione></Sede>
    </CedentePrestatore>
    <CessionarioCommittente>
      <DatiAnagrafici>
        <IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>{cessionario_vat}</IdCodice></IdFiscaleIVA>
        <Anagrafica><Denominazione>{cessionario_name}</Denominazione></Anagrafica>
      </DatiAnagrafici>
      <Sede><Indirizzo>Via Milano 2</Indirizzo><CAP>20100</CAP><Comune>Milano</Comune><Provincia>MI</Provincia><Nazione>IT</Nazione></Sede>
    </CessionarioCommittente>
  </FatturaElettronicaHeader>
  <FatturaElettronicaBody>
    <DatiGenerali>
      <DatiGeneraliDocumento>
        <TipoDocumento>TD01</TipoDocumento>
        <Divisa>EUR</Divisa>
        <Data>{data}</Data>
        <Numero>{numero}</Numero>
        <ImportoTotaleDocumento>122.00</ImportoTotaleDocumento>
      </DatiGeneraliDocumento>
    </DatiGenerali>
    <DatiBeniServizi>
      <DettaglioLinee>
        <NumeroLinea>1</NumeroLinea>
        <Descrizione>Consulenza</Descrizione>
        <PrezzoUnitario>100.00</PrezzoUnitario>
        <PrezzoTotale>100.00</PrezzoTotale>
        <AliquotaIVA>22.00</AliquotaIVA>
      </DettaglioLinee>
      <DatiRiepilogo>
        <AliquotaIVA>22.00</AliquotaIVA>
        <ImponibileImporto>100.00</ImponibileImporto>
        <Imposta>22.00</Imposta>
      </DatiRiepilogo>
    </DatiBeniServizi>
  </FatturaElettronicaBody>
</p:FatturaElettronica>"#
        )
    }

    #[tokio::test]
    async fn backfills_missing_and_placeholder_recipient_codes_from_archived_invoices() {
        let (store, dir) = test_store("backfill-destinations");
        let seller_id = digest::client_id("IT", "11111111115").unwrap();
        let buyer_sdi_id = digest::client_id("IT", "22222222220").unwrap();
        let buyer_pa_id = digest::client_id("IT", "33333333335").unwrap();
        let supplier_only_id = digest::client_id("IT", "44444444440").unwrap();
        let manual_id = digest::client_id("IT", "55555555555").unwrap();

        let mut tx = store.begin().await.unwrap();
        for (id, vat, name, cd) in [
            (&seller_id, "11111111115", "Emittente Srl", None),
            (&buyer_sdi_id, "22222222220", "Cliente SDI Srl", None),
            (&buyer_pa_id, "33333333335", "Comune di Prova", None),
            (&supplier_only_id, "44444444440", "Fornitore Srl", None),
            (&manual_id, "55555555555", "Cliente Manuale Srl", Some("ABC1234")),
        ] {
            tx.insert(
                "clients",
                serde_json::json!({
                    "id": id,
                    "denominazione": name,
                    "id_paese": "IT",
                    "id_codice": vat,
                    "indirizzo": "Via Test 1",
                    "cap": "00100",
                    "comune": "Roma",
                    "nazione": "IT",
                    "codice_destinatario": cd,
                    "pec": null
                }),
            )
            .await
            .unwrap();
        }

        let xml_older = minimal_fattura_xml(
            "FPR12",
            "0000000",
            Some("amm@pec.clientesdi.it"),
            ("11111111115", "Emittente Srl"),
            ("22222222220", "Cliente SDI Srl"),
            ("1", "2026-01-10"),
        );
        let xml_newer = minimal_fattura_xml(
            "FPR12",
            "SUBM70N",
            None,
            ("11111111115", "Emittente Srl"),
            ("22222222220", "Cliente SDI Srl"),
            ("2", "2026-02-15"),
        );
        let xml_pa = minimal_fattura_xml(
            "FPA12",
            "UF12AB",
            None,
            ("11111111115", "Emittente Srl"),
            ("33333333335", "Comune di Prova"),
            ("3", "2026-02-20"),
        );
        let xml_purchase = minimal_fattura_xml(
            "FPR12",
            "0000000",
            None,
            ("44444444440", "Fornitore Srl"),
            ("11111111115", "Emittente Srl"),
            ("99", "2026-02-01"),
        );
        let xml_manual = minimal_fattura_xml(
            "FPR12",
            "0000000",
            None,
            ("11111111115", "Emittente Srl"),
            ("55555555555", "Cliente Manuale Srl"),
            ("4", "2026-02-25"),
        );

        for (inv_id, num, date, cedente, cessionario, xml) in [
            ("inv-old", "1", "2026-01-10", &seller_id, &buyer_sdi_id, &xml_older),
            ("inv-new", "2", "2026-02-15", &seller_id, &buyer_sdi_id, &xml_newer),
            ("inv-pa", "3", "2026-02-20", &seller_id, &buyer_pa_id, &xml_pa),
            ("inv-pur", "99", "2026-02-01", &supplier_only_id, &seller_id, &xml_purchase),
            ("inv-man", "4", "2026-02-25", &seller_id, &manual_id, &xml_manual),
        ] {
            tx.insert(
                "invoices",
                serde_json::json!({
                    "id": inv_id,
                    "numero": num,
                    "data": date,
                    "tipo_documento": "TD01",
                    "divisa": "EUR",
                    "importo_totale": 122.0,
                    "stato": "imported",
                    "cedente_id": cedente,
                    "cessionario_id": cessionario,
                    "xml_content": xml
                }),
            )
            .await
            .unwrap();
        }
        tx.commit().await.unwrap();

        // Trigger backfill automatically via list_clients_store without re-importing XMLs.
        let summaries = list_clients_store(&store).await.unwrap();
        let buyer_sdi_summary = summaries.iter().find(|c| c.id == buyer_sdi_id).unwrap();
        assert_eq!(buyer_sdi_summary.codice_destinatario.as_deref(), Some("SUBM70N"));
        assert!(buyer_sdi_summary.search_text.contains("subm70n"));

        let buyer_sdi = get_client_store(&store, &buyer_sdi_id).await.unwrap();
        assert_eq!(buyer_sdi.codice_destinatario.as_deref(), Some("SUBM70N"));
        assert_eq!(buyer_sdi.pec.as_deref(), Some("amm@pec.clientesdi.it"));
        assert!(!buyer_sdi.is_pa);

        let buyer_pa = get_client_store(&store, &buyer_pa_id).await.unwrap();
        assert_eq!(buyer_pa.codice_destinatario.as_deref(), Some("UF12AB"));
        assert!(buyer_pa.is_pa);

        let supplier_only = get_client_store(&store, &supplier_only_id).await.unwrap();
        assert_eq!(supplier_only.codice_destinatario.as_deref(), Some("0000000"));

        let manual = get_client_store(&store, &manual_id).await.unwrap();
        assert_eq!(manual.codice_destinatario.as_deref(), Some("ABC1234"));

        // Second invocation is an idempotent no-op.
        assert_eq!(backfill_client_destinations(&store).await.unwrap(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn import_upgrades_supplier_client_when_seen_as_buyer_with_sdi_code() {
        let (store, dir) = test_store("import-upgrade-sdi");

        // 1. Import a purchase invoice where Partner Srl is CedentePrestatore (no destination code).
        let purchase_xml = minimal_fattura_xml(
            "FPR12",
            "0000000",
            None,
            ("22222222220", "Partner Srl"),
            ("11111111115", "MyCompany Srl"),
            ("P-1", "2026-01-15"),
        );
        crate::commands::invoices::import_xml_content(&store, &purchase_xml)
            .await
            .unwrap();

        let partner_id = digest::client_id("IT", "22222222220").unwrap();
        let after_purchase = get_client_store(&store, &partner_id).await.unwrap();
        assert_eq!(
            after_purchase.codice_destinatario.as_deref(),
            Some("0000000")
        );
        assert_eq!(after_purchase.pec, None);

        // 2. Import a sale invoice where Partner Srl is CessionarioCommittente with SDI code + PEC.
        let sale_xml = minimal_fattura_xml(
            "FPR12",
            "M5UXCR1",
            Some("partner@pec.it"),
            ("11111111115", "MyCompany Srl"),
            ("22222222220", "Partner Srl"),
            ("S-1", "2026-02-01"),
        );
        crate::commands::invoices::import_xml_content(&store, &sale_xml)
            .await
            .unwrap();

        let after_sale = get_client_store(&store, &partner_id).await.unwrap();
        assert_eq!(after_sale.codice_destinatario.as_deref(), Some("M5UXCR1"));
        assert_eq!(after_sale.pec.as_deref(), Some("partner@pec.it"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn preserves_explicit_pec_destination_across_reimport_backfill_and_xml_generation() {
        let (store, dir) = test_store("preserve-pec-destination");

        // 1. User configures seller and a PEC-routed buyer (0000000 + PEC).
        let seller = create_client_store(
            &store,
            sample_client_request("11111111115", "MyCompany Srl"),
        )
        .await
        .unwrap();
        let mut buyer_req = sample_client_request("22222222220", "Cliente PEC Srl");
        buyer_req.codice_destinatario = Some("0000000".to_string());
        buyer_req.pec = Some("amministrazione@pec.cliente.it".to_string());
        let buyer = create_client_store(&store, buyer_req).await.unwrap();
        assert_eq!(buyer.codice_destinatario.as_deref(), Some("0000000"));
        assert_eq!(
            buyer.pec.as_deref(),
            Some("amministrazione@pec.cliente.it")
        );

        // 2. Also insert an existing pre-upgrade client (no `recipient_configured` flag)
        // that already had `0000000` + PEC on disk alongside an older archived invoice with a specific SDI code.
        let legacy_pec_id = digest::client_id("IT", "33333333335").unwrap();
        let old_legacy_xml = minimal_fattura_xml(
            "FPR12",
            "SUBM70N",
            None,
            ("11111111115", "MyCompany Srl"),
            ("33333333335", "Legacy PEC Srl"),
            ("OLD-1", "2024-05-10"),
        );
        let mut tx = store.begin().await.unwrap();
        tx.insert(
            "clients",
            serde_json::json!({
                "id": &legacy_pec_id,
                "denominazione": "Legacy PEC Srl",
                "id_paese": "IT",
                "id_codice": "33333333335",
                "indirizzo": "Via Vecchia 10",
                "cap": "00100",
                "comune": "Roma",
                "nazione": "IT",
                "codice_destinatario": "0000000",
                "pec": "legacy@pec.cliente.it"
            }),
        )
        .await
        .unwrap();
        tx.insert(
            "invoices",
            serde_json::json!({
                "id": "inv-legacy-old",
                "numero": "OLD-1",
                "data": "2024-05-10",
                "tipo_documento": "TD01",
                "divisa": "EUR",
                "importo_totale": 122.0,
                "stato": "imported",
                "cedente_id": &seller.id,
                "cessionario_id": &legacy_pec_id,
                "xml_content": &old_legacy_xml
            }),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        // Backfill from archive must preserve `0000000` + PEC on the existing client.
        backfill_client_destinations(&store).await.unwrap();
        let legacy_after_backfill = get_client_store(&store, &legacy_pec_id).await.unwrap();
        assert_eq!(
            legacy_after_backfill.codice_destinatario.as_deref(),
            Some("0000000")
        );
        assert_eq!(
            legacy_after_backfill.pec.as_deref(),
            Some("legacy@pec.cliente.it")
        );

        // 3. Re-importing an old invoice with a specific SDI code (`M5UXCR1`) for `buyer`
        // must NOT overwrite `0000000` or drop PEC from subsequent generated XMLs.
        let old_sale_xml = minimal_fattura_xml(
            "FPR12",
            "M5UXCR1",
            None,
            ("11111111115", "MyCompany Srl"),
            ("22222222220", "Cliente PEC Srl"),
            ("OLD-2", "2024-06-01"),
        );
        crate::commands::invoices::import_xml_content(&store, &old_sale_xml)
            .await
            .unwrap();

        let buyer_after_import = get_client_store(&store, &buyer.id).await.unwrap();
        assert_eq!(
            buyer_after_import.codice_destinatario.as_deref(),
            Some("0000000")
        );
        assert_eq!(
            buyer_after_import.pec.as_deref(),
            Some("amministrazione@pec.cliente.it")
        );

        let new_inv_req: crate::commands::invoices::CreateInvoiceRequest =
            serde_json::from_value(serde_json::json!({
                "numero": "2026-10",
                "data": "2026-03-15",
                "tipoDocumento": "TD01",
                "divisa": "EUR",
                "cedenteId": &seller.id,
                "cessionarioId": &buyer.id,
                "pagamenti": [],
                "linee": [{"descrizione": "Servizio", "quantita": 1, "prezzoUnitario": 100.0, "aliquotaIva": 22.0}]
            }))
            .unwrap();
        let created_inv = crate::commands::invoices::create_invoice_store(&store, new_inv_req)
            .await
            .unwrap();
        let generated_xml =
            crate::commands::invoices::export_invoice_xml_store(&store, &created_inv.id)
                .await
                .unwrap();
        assert!(generated_xml.contains("<CodiceDestinatario>0000000</CodiceDestinatario>"));
        assert!(
            generated_xml.contains("<PECDestinatario>amministrazione@pec.cliente.it</PECDestinatario>")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
