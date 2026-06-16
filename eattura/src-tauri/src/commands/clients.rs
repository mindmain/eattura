use serde::{Deserialize, Serialize};
use sqlx::Row;

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
}

/// Request body for creating or updating a client.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClientRequest {
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
}

/// List all clients from the local SQLite database.
///
/// Returns a summary view with key fields, ordered by company name or surname.
#[tauri::command]
pub async fn list_clients(
    state: tauri::State<'_, TauriState>,
) -> Result<Vec<ClientSummary>, String> {
    let rows = sqlx::query(
        "SELECT id, denominazione, nome, cognome, id_paese, id_codice, comune
         FROM clients
         ORDER BY COALESCE(denominazione, cognome)",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let clients = rows
        .iter()
        .map(|row| ClientSummary {
            id: row.get("id"),
            denominazione: row.get("denominazione"),
            nome: row.get("nome"),
            cognome: row.get("cognome"),
            id_paese: row.get("id_paese"),
            id_codice: row.get("id_codice"),
            comune: row.get("comune"),
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
    let row = sqlx::query("SELECT * FROM clients WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Client not found: {id}"))?;

    Ok(row_to_client_detail(&row))
}

/// Create a new client.
///
/// Generates a UUID for the new client and inserts it into the database.
/// The UNIQUE(id_paese, id_codice) constraint prevents duplicate tax IDs.
#[tauri::command]
pub async fn create_client(
    state: tauri::State<'_, TauriState>,
    client: CreateClientRequest,
) -> Result<ClientDetail, String> {
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO clients (
            id, denominazione, nome, cognome, id_paese, id_codice,
            codice_fiscale, regime_fiscale, indirizzo, numero_civico,
            cap, comune, provincia, nazione, telefono, email, pec, codice_destinatario
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&client.denominazione)
    .bind(&client.nome)
    .bind(&client.cognome)
    .bind(&client.id_paese)
    .bind(&client.id_codice)
    .bind(&client.codice_fiscale)
    .bind(&client.regime_fiscale)
    .bind(&client.indirizzo)
    .bind(&client.numero_civico)
    .bind(&client.cap)
    .bind(&client.comune)
    .bind(&client.provincia)
    .bind(&client.nazione)
    .bind(&client.telefono)
    .bind(&client.email)
    .bind(&client.pec)
    .bind(&client.codice_destinatario)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Fetch the newly created record to return it with default timestamps.
    get_client(state, id).await
}

/// Update an existing client by ID.
///
/// Replaces all mutable fields. The `updated_at` timestamp is refreshed automatically.
#[tauri::command]
pub async fn update_client(
    state: tauri::State<'_, TauriState>,
    id: String,
    client: CreateClientRequest,
) -> Result<ClientDetail, String> {
    let result = sqlx::query(
        "UPDATE clients SET
            denominazione = ?, nome = ?, cognome = ?, id_paese = ?, id_codice = ?,
            codice_fiscale = ?, regime_fiscale = ?, indirizzo = ?, numero_civico = ?,
            cap = ?, comune = ?, provincia = ?, nazione = ?,
            telefono = ?, email = ?, pec = ?, codice_destinatario = ?,
            updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&client.denominazione)
    .bind(&client.nome)
    .bind(&client.cognome)
    .bind(&client.id_paese)
    .bind(&client.id_codice)
    .bind(&client.codice_fiscale)
    .bind(&client.regime_fiscale)
    .bind(&client.indirizzo)
    .bind(&client.numero_civico)
    .bind(&client.cap)
    .bind(&client.comune)
    .bind(&client.provincia)
    .bind(&client.nazione)
    .bind(&client.telefono)
    .bind(&client.email)
    .bind(&client.pec)
    .bind(&client.codice_destinatario)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Client not found: {id}"));
    }

    get_client(state, id).await
}

/// Delete a client by ID.
///
/// Fails if the client is referenced by any invoices (as cedente or cessionario).
#[tauri::command]
pub async fn delete_client(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<(), String> {
    // Check for associated invoices before deleting.
    let row = sqlx::query(
        "SELECT COUNT(*) as cnt FROM invoices WHERE cedente_id = ? OR cessionario_id = ?",
    )
    .bind(&id)
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let count: i64 = row.get("cnt");
    if count > 0 {
        return Err(format!(
            "Cannot delete client {id}: referenced by {count} invoice(s)"
        ));
    }

    let result = sqlx::query("DELETE FROM clients WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Client not found: {id}"));
    }

    Ok(())
}

/// Map a SQLite row to a `ClientDetail` struct.
fn row_to_client_detail(row: &sqlx::sqlite::SqliteRow) -> ClientDetail {
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
        codice_destinatario: row.get("codice_destinatario"),
    }
}
