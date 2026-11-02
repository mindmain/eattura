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
    pub search_text: String,
    #[serde(default)]
    pub is_pa: bool,
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

    let effective_id_codice = if !raw_id_codice.is_empty() {
        raw_id_codice.to_ascii_uppercase()
    } else if let Some(ref cf) = raw_cf {
        cf.clone()
    } else {
        return Err("Specificare Partita IVA o Codice Fiscale per l'anagrafica".into());
    };

    Ok((effective_id_codice, raw_cf))
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
    let rows = store
        .read("clients", "all", vec![], "client")
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let clients = rows
        .iter()
        .map(|row| {
            let cd: Option<String> = row.get("codice_destinatario");
            let is_pa = row
                .get::<Option<bool>>("is_pa")
                .unwrap_or_else(|| cd.as_deref().map(|c| c.trim().len() == 6).unwrap_or(false));
            ClientSummary {
                id: row.get("id"),
                search_text: row.get("search_text"),
                denominazione: row.get("denominazione"),
                nome: row.get("nome"),
                cognome: row.get("cognome"),
                id_paese: row.get("id_paese"),
                id_codice: row.get("id_codice"),
                comune: row.get("comune"),
                is_pa,
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
                "pec": &client.pec,
                "codice_destinatario": &client.codice_destinatario,
                "is_pa": is_pa
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
                "pec": &client.pec,
                "codice_destinatario": &client.codice_destinatario,
                "is_pa": is_pa
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
    let cd: Option<String> = row.get("codice_destinatario");
    let is_pa = row
        .get::<Option<bool>>("is_pa")
        .unwrap_or_else(|| cd.as_deref().map(|c| c.trim().len() == 6).unwrap_or(false));
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
        let req = sample_client_request("12345678901", "Acme Srl");

        // 1. Create client
        let created = create_client_store(&store, req).await.unwrap();
        let expected_id = digest::client_id("IT", "12345678901").unwrap();
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
        let mut update_req = sample_client_request("12345678901", "Acme SpA");
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
    async fn test_client_duplicate_fiscal_id_rejected() {
        let (store, _dir) = test_store("duplicate");
        let req1 = sample_client_request("12345678901", "First Company");
        create_client_store(&store, req1).await.unwrap();

        let req2 = sample_client_request("12345678901", "Second Company");
        let err = create_client_store(&store, req2).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_delete_client_referenced_by_invoice_fails() {
        let (store, _dir) = test_store("ref-protect");
        let seller = create_client_store(&store, sample_client_request("11111111111", "Seller"))
            .await
            .unwrap();
        let buyer = create_client_store(&store, sample_client_request("22222222222", "Buyer"))
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
            create_client_store(&store, sample_client_request("11111111111", "Original"))
                .await
                .unwrap();

        // Update with new P.IVA
        let update_req = sample_client_request("33333333333", "Renamed & Rekeyed");
        let updated = update_client_store(&store, &original.id, update_req)
            .await
            .unwrap();

        let new_expected_id = digest::client_id("IT", "33333333333").unwrap();
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
}
