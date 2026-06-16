use uuid::Uuid;

use crate::error::AppError;
use crate::models::client::{ClientDetail, CreateClientRequest};

/// Business logic for client operations.
///
/// Handles CRUD with deduplication by Partita IVA.
pub struct ClientService {
    pub db: sqlx::PgPool,
}

impl ClientService {
    /// Create a new client after checking for P.IVA uniqueness.
    ///
    /// Returns an error if a client with the same (id_paese, id_codice) already exists.
    pub async fn create(&self, req: CreateClientRequest) -> Result<ClientDetail, AppError> {
        // Check P.IVA uniqueness.
        let existing = sqlx::query(
            "SELECT id FROM clients WHERE id_paese = $1 AND id_codice = $2",
        )
        .bind(&req.id_paese)
        .bind(&req.id_codice)
        .fetch_optional(&self.db)
        .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(format!(
                "Client with P.IVA {}{} already exists",
                req.id_paese, req.id_codice
            )));
        }

        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO clients
                (id, denominazione, nome, cognome, id_paese, id_codice,
                 codice_fiscale, regime_fiscale, indirizzo, numero_civico,
                 cap, comune, provincia, nazione, telefono, email, pec,
                 codice_destinatario)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
        )
        .bind(id.to_string())
        .bind(&req.denominazione)
        .bind(&req.nome)
        .bind(&req.cognome)
        .bind(&req.id_paese)
        .bind(&req.id_codice)
        .bind(&req.codice_fiscale)
        .bind(&req.regime_fiscale)
        .bind(&req.indirizzo)
        .bind(&req.numero_civico)
        .bind(&req.cap)
        .bind(&req.comune)
        .bind(&req.provincia)
        .bind(&req.nazione)
        .bind(&req.telefono)
        .bind(&req.email)
        .bind(&req.pec)
        .bind(&req.codice_destinatario)
        .execute(&self.db)
        .await?;

        Ok(ClientDetail {
            id,
            denominazione: req.denominazione,
            nome: req.nome,
            cognome: req.cognome,
            id_paese: req.id_paese,
            id_codice: req.id_codice,
            codice_fiscale: req.codice_fiscale,
            regime_fiscale: req.regime_fiscale,
            indirizzo: req.indirizzo,
            numero_civico: req.numero_civico,
            cap: req.cap,
            comune: req.comune,
            provincia: req.provincia,
            nazione: req.nazione,
            telefono: req.telefono,
            email: req.email,
            pec: req.pec,
            codice_destinatario: req.codice_destinatario,
        })
    }

    /// Find or create a client by Partita IVA.
    ///
    /// Used during XML import to resolve cedente/cessionario references.
    /// If a client with the given (id_paese, id_codice) exists, returns its ID.
    /// Otherwise, creates a minimal client record and returns the new ID.
    pub async fn find_or_create_by_piva(
        &self,
        id_paese: &str,
        id_codice: &str,
    ) -> Result<Uuid, AppError> {
        use sqlx::Row;

        // Try to find existing client by P.IVA.
        let existing = sqlx::query(
            "SELECT id FROM clients WHERE id_paese = $1 AND id_codice = $2",
        )
        .bind(id_paese)
        .bind(id_codice)
        .fetch_optional(&self.db)
        .await?;

        if let Some(row) = existing {
            let id_str: String = row.get("id");
            return id_str
                .parse::<Uuid>()
                .map_err(|e| AppError::Internal(format!("Invalid UUID in database: {e}")));
        }

        // Create a minimal client record.
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO clients (id, id_paese, id_codice, indirizzo, cap, comune, nazione)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id.to_string())
        .bind(id_paese)
        .bind(id_codice)
        .bind("N/A")
        .bind("00000")
        .bind("N/A")
        .bind(id_paese)
        .execute(&self.db)
        .await?;

        Ok(id)
    }
}
