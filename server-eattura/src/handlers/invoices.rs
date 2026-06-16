use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::invoice::{
    CreateInvoiceRequest, InvoiceDetail, InvoiceLineDetail, InvoicePaymentDetail, InvoiceSummary,
};
use crate::services::invoice_service::InvoiceService;
use crate::services::xml_service::XmlService;
use crate::state::AppState;

/// List all invoices with optional pagination and filtering.
///
/// Query params: `?page=1&per_page=20&status=draft&search=...`
pub async fn list_invoices(
    State(state): State<AppState>,
) -> Result<Json<Vec<InvoiceSummary>>, AppError> {
    let rows = sqlx::query(
        "SELECT i.id, i.numero, i.data, i.tipo_documento, i.importo_totale, i.stato,
                COALESCE(c1.denominazione, c1.cognome || ' ' || c1.nome, '') AS cedente_denominazione,
                COALESCE(c2.denominazione, c2.cognome || ' ' || c2.nome, '') AS cessionario_denominazione
         FROM invoices i
         LEFT JOIN clients c1 ON i.cedente_id = c1.id
         LEFT JOIN clients c2 ON i.cessionario_id = c2.id
         ORDER BY i.created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let invoices: Vec<InvoiceSummary> = rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            InvoiceSummary {
                id: row.get::<String, _>("id").parse().unwrap_or_default(),
                numero: row.get("numero"),
                data: row.get("data"),
                tipo_documento: row.get("tipo_documento"),
                importo_totale: row.get("importo_totale"),
                cedente_denominazione: row.get("cedente_denominazione"),
                cessionario_denominazione: row.get("cessionario_denominazione"),
                stato: row.get("stato"),
            }
        })
        .collect();

    Ok(Json(invoices))
}

/// Get a single invoice by ID with full detail (lines, payments).
pub async fn get_invoice(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<InvoiceDetail>, AppError> {
    let detail = fetch_invoice_detail(&state.db, id).await?;
    Ok(Json(detail))
}

/// Create a new invoice from request payload.
/// Validates basic structure before persisting.
pub async fn create_invoice(
    State(state): State<AppState>,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<Json<InvoiceDetail>, AppError> {
    let svc = InvoiceService { db: state.db.clone() };
    let detail = svc.create(payload).await?;
    Ok(Json(detail))
}

/// Update an existing invoice.
/// Only allowed for invoices in 'draft' status.
pub async fn update_invoice(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<Json<InvoiceDetail>, AppError> {
    // Verify invoice exists and is in draft status.
    let row = sqlx::query("SELECT stato FROM invoices WHERE id = $1")
        .bind(id.to_string())
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Invoice {id} not found")))?;

    use sqlx::Row;
    let stato: String = row.get("stato");
    if stato != "draft" {
        return Err(AppError::Internal(format!(
            "Cannot update invoice in '{stato}' status; only 'draft' invoices can be updated"
        )));
    }

    // Update the invoice header.
    sqlx::query(
        "UPDATE invoices SET
            numero = $2, data = $3, tipo_documento = $4, divisa = $5,
            importo_totale = $6, cedente_id = $7, cessionario_id = $8,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id.to_string())
    .bind(&payload.numero)
    .bind(&payload.data)
    .bind(&payload.tipo_documento)
    .bind(&payload.divisa)
    .bind(payload.importo_totale)
    .bind(payload.cedente_id.to_string())
    .bind(payload.cessionario_id.to_string())
    .execute(&state.db)
    .await?;

    // Replace line items: delete old ones and insert new ones.
    sqlx::query("DELETE FROM invoice_lines WHERE invoice_id = $1")
        .bind(id.to_string())
        .execute(&state.db)
        .await?;

    for line in &payload.linee {
        let line_id = Uuid::new_v4().to_string();
        let prezzo_totale = line.prezzo_unitario * line.quantita.unwrap_or(1.0);
        sqlx::query(
            "INSERT INTO invoice_lines
                (id, invoice_id, numero_linea, descrizione, quantita, unita_misura,
                 prezzo_unitario, prezzo_totale, aliquota_iva, natura)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&line_id)
        .bind(id.to_string())
        .bind(line.numero_linea)
        .bind(&line.descrizione)
        .bind(line.quantita)
        .bind(&line.unita_misura)
        .bind(line.prezzo_unitario)
        .bind(prezzo_totale)
        .bind(line.aliquota_iva)
        .bind(&line.natura)
        .execute(&state.db)
        .await?;
    }

    // Replace payment details.
    sqlx::query("DELETE FROM invoice_payments WHERE invoice_id = $1")
        .bind(id.to_string())
        .execute(&state.db)
        .await?;

    for payment in &payload.pagamenti {
        let payment_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO invoice_payments
                (id, invoice_id, condizioni_pagamento, modalita_pagamento,
                 importo_pagamento, data_scadenza_pagamento, iban, istituto_finanziario)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&payment_id)
        .bind(id.to_string())
        .bind(&payment.condizioni_pagamento)
        .bind(&payment.modalita_pagamento)
        .bind(payment.importo_pagamento)
        .bind(&payment.data_scadenza_pagamento)
        .bind(&payment.iban)
        .bind(&payment.istituto_finanziario)
        .execute(&state.db)
        .await?;
    }

    let detail = fetch_invoice_detail(&state.db, id).await?;
    Ok(Json(detail))
}

/// Delete an invoice (hard delete with cascading line/payment removal).
/// Only allowed for invoices in 'draft' status.
pub async fn delete_invoice(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let row = sqlx::query("SELECT stato FROM invoices WHERE id = $1")
        .bind(id.to_string())
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Invoice {id} not found")))?;

    use sqlx::Row;
    let stato: String = row.get("stato");
    if stato != "draft" {
        return Err(AppError::Internal(format!(
            "Cannot delete invoice in '{stato}' status; only 'draft' invoices can be deleted"
        )));
    }

    // ON DELETE CASCADE handles lines and payments.
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(id.to_string())
        .execute(&state.db)
        .await?;

    Ok(())
}

/// Request body for a status change.
#[derive(serde::Deserialize)]
pub struct SetStatusRequest {
    pub status: String,
}

/// Change the lifecycle status of an invoice, enforcing the shared state machine.
///
/// Transitions are validated by `common::invoice_status`. Promotion to `validated`
/// is blocked unless the invoice passes SDI validation.
pub async fn set_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<SetStatusRequest>,
) -> Result<Json<InvoiceDetail>, AppError> {
    use common::invoice_status::InvoiceStatus;
    use sqlx::Row;

    let row = sqlx::query("SELECT stato FROM invoices WHERE id = $1")
        .bind(id.to_string())
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Invoice {id} not found")))?;

    let current: InvoiceStatus = row
        .get::<String, _>("stato")
        .parse()
        .map_err(AppError::Internal)?;
    let next: InvoiceStatus = payload.status.parse().map_err(AppError::Internal)?;
    current.transition_to(next).map_err(AppError::Internal)?;

    // Block promotion to `validated` if the invoice does not pass SDI validation.
    if next == InvoiceStatus::Validated {
        let svc = InvoiceService { db: state.db.clone() };
        let xml_svc = XmlService { invoice_service: svc };
        let result = xml_svc.validate_invoice(id).await?;
        if !result.is_valid() {
            return Err(AppError::Internal(format!(
                "Cannot validate invoice: {} SDI validation error(s) must be resolved first",
                result.error_count()
            )));
        }
    }

    sqlx::query("UPDATE invoices SET stato = $2, updated_at = NOW() WHERE id = $1")
        .bind(id.to_string())
        .bind(next.as_str())
        .execute(&state.db)
        .await?;

    let detail = fetch_invoice_detail(&state.db, id).await?;
    Ok(Json(detail))
}

/// Validate an invoice against SDI rules without modifying it.
/// Returns the validation result with any errors found.
pub async fn validate_invoice(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let svc = InvoiceService { db: state.db.clone() };
    let xml_svc = XmlService { invoice_service: svc };
    let result = xml_svc.validate_invoice(id).await?;

    let errors: Vec<serde_json::Value> = result
        .errors
        .iter()
        .map(|e| {
            serde_json::json!({
                "code": e.code,
                "message": e.message,
                "element_path": e.element_path,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "valid": result.is_valid(),
        "error_count": result.error_count(),
        "errors": errors,
    })))
}

/// Export an invoice as SDI-compliant XML.
/// Returns the XML string with appropriate Content-Type header.
pub async fn export_xml(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<String, AppError> {
    let svc = InvoiceService { db: state.db.clone() };
    let xml_svc = XmlService { invoice_service: svc };
    let xml = xml_svc.export(id).await?;
    Ok(xml)
}

/// Import an invoice from SDI XML.
/// Parses the XML, creates the invoice and related records in the database.
pub async fn import_xml(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<InvoiceDetail>, AppError> {
    let svc = InvoiceService { db: state.db.clone() };
    let xml_svc = XmlService { invoice_service: svc };
    let detail = xml_svc.import(&body).await?;
    Ok(Json(detail))
}

/// Helper to fetch full invoice detail with lines and payments from the database.
async fn fetch_invoice_detail(pool: &sqlx::PgPool, id: Uuid) -> Result<InvoiceDetail, AppError> {
    use sqlx::Row;

    let row = sqlx::query(
        "SELECT i.id, i.numero, i.data, i.tipo_documento, i.divisa, i.importo_totale, i.stato,
                COALESCE(c1.denominazione, c1.cognome || ' ' || c1.nome, '') AS cedente_denominazione,
                COALESCE(c2.denominazione, c2.cognome || ' ' || c2.nome, '') AS cessionario_denominazione
         FROM invoices i
         LEFT JOIN clients c1 ON i.cedente_id = c1.id
         LEFT JOIN clients c2 ON i.cessionario_id = c2.id
         WHERE i.id = $1",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Invoice {id} not found")))?;

    let line_rows = sqlx::query(
        "SELECT numero_linea, descrizione, quantita, unita_misura,
                prezzo_unitario, prezzo_totale, aliquota_iva, natura
         FROM invoice_lines WHERE invoice_id = $1 ORDER BY numero_linea",
    )
    .bind(id.to_string())
    .fetch_all(pool)
    .await?;

    let payment_rows = sqlx::query(
        "SELECT modalita_pagamento, importo_pagamento, data_scadenza_pagamento,
                iban, istituto_finanziario
         FROM invoice_payments WHERE invoice_id = $1",
    )
    .bind(id.to_string())
    .fetch_all(pool)
    .await?;

    let linee: Vec<InvoiceLineDetail> = line_rows
        .iter()
        .map(|r| InvoiceLineDetail {
            numero_linea: r.get("numero_linea"),
            descrizione: r.get("descrizione"),
            quantita: r.get("quantita"),
            unita_misura: r.get("unita_misura"),
            prezzo_unitario: r.get("prezzo_unitario"),
            prezzo_totale: r.get("prezzo_totale"),
            aliquota_iva: r.get("aliquota_iva"),
            natura: r.get("natura"),
        })
        .collect();

    let pagamenti: Vec<InvoicePaymentDetail> = payment_rows
        .iter()
        .map(|r| InvoicePaymentDetail {
            modalita_pagamento: r.get("modalita_pagamento"),
            importo_pagamento: r.get("importo_pagamento"),
            data_scadenza_pagamento: r.get("data_scadenza_pagamento"),
            iban: r.get("iban"),
            istituto_finanziario: r.get("istituto_finanziario"),
        })
        .collect();

    Ok(InvoiceDetail {
        id: row.get::<String, _>("id").parse().unwrap_or_default(),
        numero: row.get("numero"),
        data: row.get("data"),
        tipo_documento: row.get("tipo_documento"),
        divisa: row.get("divisa"),
        importo_totale: row.get("importo_totale"),
        cedente_denominazione: row.get("cedente_denominazione"),
        cessionario_denominazione: row.get("cessionario_denominazione"),
        stato: row.get("stato"),
        causale: Vec::new(),
        linee,
        pagamenti,
    })
}
