use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Database row for the clients table.
///
/// Represents both cedente (issuer) and cessionario (recipient) entities.
/// A client can be either a company (denominazione) or a person (nome + cognome).
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ClientRow {
    pub id: Uuid,
    /// Company name (mutually exclusive with nome/cognome).
    pub denominazione: Option<String>,
    /// First name for physical persons.
    pub nome: Option<String>,
    /// Last name for physical persons.
    pub cognome: Option<String>,
    /// Country code (ISO 3166-1 alpha-2, e.g. "IT").
    pub id_paese: String,
    /// VAT number (Partita IVA).
    pub id_codice: String,
    /// Codice Fiscale (tax code, 11 or 16 chars).
    pub codice_fiscale: Option<String>,
    /// Tax regime code (e.g. "RF01" for standard).
    pub regime_fiscale: Option<String>,
    pub indirizzo: String,
    pub numero_civico: Option<String>,
    /// CAP (postal code, 5 digits).
    pub cap: String,
    pub comune: String,
    /// Province (2 uppercase letters).
    pub provincia: Option<String>,
    /// Country (ISO 3166-1 alpha-2, default "IT").
    pub nazione: String,
    pub telefono: Option<String>,
    pub email: Option<String>,
    /// PEC (Certified Email) address.
    pub pec: Option<String>,
    /// SDI destination code (6-7 alphanumeric chars).
    pub codice_destinatario: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Summary view of a client for list endpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientSummary {
    pub id: Uuid,
    pub denominazione: Option<String>,
    pub nome: Option<String>,
    pub cognome: Option<String>,
    pub id_paese: String,
    pub id_codice: String,
    pub comune: String,
}

/// Full client detail for single-record endpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientDetail {
    pub id: Uuid,
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

/// Request body for creating/updating a client.
#[derive(Debug, Serialize, Deserialize)]
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
