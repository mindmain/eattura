use axum::extract::{Path, Query, State};
use axum::Json;
use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::pagination::{Pagination, like_pattern};
use crate::models::client::{ClientDetail, ClientSummary, CreateClientRequest};
use crate::services::client_service::ClientService;
use crate::state::AppState;

/// List clients with optional search and pagination.
///
/// Query params: `?search=...&page=1&per_page=20`
pub async fn list_clients(
    State(state): State<AppState>,
    Query(params): Query<Pagination>,
) -> Result<Json<Vec<ClientSummary>>, AppError> {
    let mut qb = QueryBuilder::new(
        "SELECT id, denominazione, nome, cognome, id_paese, id_codice, comune FROM clients WHERE TRUE",
    );
    if let Some(search) = params.search.as_deref().filter(|s| !s.is_empty()) {
        let pat = like_pattern(search);
        qb.push(" AND (denominazione ILIKE ")
            .push_bind(pat.clone())
            .push(" OR cognome ILIKE ")
            .push_bind(pat.clone())
            .push(" OR id_codice ILIKE ")
            .push_bind(pat)
            .push(")");
    }
    qb.push(" ORDER BY created_at DESC LIMIT ")
        .push_bind(params.limit())
        .push(" OFFSET ")
        .push_bind(params.offset());

    let rows = qb.build().fetch_all(&state.db).await?;

    let clients: Vec<ClientSummary> = rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            ClientSummary {
                id: row.get::<String, _>("id").parse().unwrap_or_default(),
                denominazione: row.get("denominazione"),
                nome: row.get("nome"),
                cognome: row.get("cognome"),
                id_paese: row.get("id_paese"),
                id_codice: row.get("id_codice"),
                comune: row.get("comune"),
            }
        })
        .collect();

    Ok(Json(clients))
}

/// Get a single client by ID.
pub async fn get_client(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ClientDetail>, AppError> {
    let row = sqlx::query(
        "SELECT id, denominazione, nome, cognome, id_paese, id_codice,
                codice_fiscale, regime_fiscale, indirizzo, numero_civico,
                cap, comune, provincia, nazione, telefono, email, pec,
                codice_destinatario
         FROM clients WHERE id = $1",
    )
    .bind(id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Client {id} not found")))?;

    use sqlx::Row;
    let detail = ClientDetail {
        id: row.get::<String, _>("id").parse().unwrap_or_default(),
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
    };

    Ok(Json(detail))
}

/// Create a new client.
/// Checks for duplicate Partita IVA before inserting.
pub async fn create_client(
    State(state): State<AppState>,
    Json(payload): Json<CreateClientRequest>,
) -> Result<Json<ClientDetail>, AppError> {
    let svc = ClientService { db: state.db.clone() };
    let detail = svc.create(payload).await?;
    Ok(Json(detail))
}

/// Update an existing client.
pub async fn update_client(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateClientRequest>,
) -> Result<Json<ClientDetail>, AppError> {
    // Verify client exists.
    let exists = sqlx::query("SELECT id FROM clients WHERE id = $1")
        .bind(id.to_string())
        .fetch_optional(&state.db)
        .await?;

    if exists.is_none() {
        return Err(AppError::NotFound(format!("Client {id} not found")));
    }

    sqlx::query(
        "UPDATE clients SET
            denominazione = $2, nome = $3, cognome = $4, id_paese = $5,
            id_codice = $6, codice_fiscale = $7, regime_fiscale = $8,
            indirizzo = $9, numero_civico = $10, cap = $11, comune = $12,
            provincia = $13, nazione = $14, telefono = $15, email = $16,
            pec = $17, codice_destinatario = $18, updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id.to_string())
    .bind(&payload.denominazione)
    .bind(&payload.nome)
    .bind(&payload.cognome)
    .bind(&payload.id_paese)
    .bind(&payload.id_codice)
    .bind(&payload.codice_fiscale)
    .bind(&payload.regime_fiscale)
    .bind(&payload.indirizzo)
    .bind(&payload.numero_civico)
    .bind(&payload.cap)
    .bind(&payload.comune)
    .bind(&payload.provincia)
    .bind(&payload.nazione)
    .bind(&payload.telefono)
    .bind(&payload.email)
    .bind(&payload.pec)
    .bind(&payload.codice_destinatario)
    .execute(&state.db)
    .await?;

    // Return the updated client.
    let detail = ClientDetail {
        id,
        denominazione: payload.denominazione,
        nome: payload.nome,
        cognome: payload.cognome,
        id_paese: payload.id_paese,
        id_codice: payload.id_codice,
        codice_fiscale: payload.codice_fiscale,
        regime_fiscale: payload.regime_fiscale,
        indirizzo: payload.indirizzo,
        numero_civico: payload.numero_civico,
        cap: payload.cap,
        comune: payload.comune,
        provincia: payload.provincia,
        nazione: payload.nazione,
        telefono: payload.telefono,
        email: payload.email,
        pec: payload.pec,
        codice_destinatario: payload.codice_destinatario,
    };

    Ok(Json(detail))
}

/// Delete a client.
/// Fails if client has associated invoices.
pub async fn delete_client(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    // Check for associated invoices.
    let has_invoices = sqlx::query(
        "SELECT 1 FROM invoices WHERE cedente_id = $1 OR cessionario_id = $1 LIMIT 1",
    )
    .bind(id.to_string())
    .fetch_optional(&state.db)
    .await?;

    if has_invoices.is_some() {
        return Err(AppError::Conflict(
            "Cannot delete client with associated invoices".to_string(),
        ));
    }

    let result = sqlx::query("DELETE FROM clients WHERE id = $1")
        .bind(id.to_string())
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Client {id} not found")));
    }

    Ok(())
}
