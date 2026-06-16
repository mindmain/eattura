use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::state::TauriState;

/// Summary view of an invoice for list display.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceSummary {
    pub id: String,
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub importo_totale: Option<f64>,
    pub cedente_denominazione: String,
    pub cessionario_denominazione: String,
    pub stato: String,
    /// Relative to the configured owner company:
    /// `sale` (owner is cedente → income), `purchase` (owner is cessionario →
    /// cost), or `unknown` (owner not configured or on neither side).
    pub direction: String,
}

/// Full invoice detail with lines and payments.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceDetail {
    pub id: String,
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub divisa: String,
    pub importo_totale: Option<f64>,
    pub stato: String,
    pub cedente_id: String,
    pub cessionario_id: String,
    pub cedente_denominazione: String,
    pub cessionario_denominazione: String,
    pub causale: Vec<String>,
    pub linee: Vec<InvoiceLineDetail>,
    pub pagamenti: Vec<PaymentDetail>,
}

/// Detail of a single invoice line item.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLineDetail {
    pub numero_linea: i32,
    pub descrizione: String,
    pub quantita: Option<f64>,
    pub unita_misura: Option<String>,
    pub prezzo_unitario: f64,
    pub prezzo_totale: f64,
    pub aliquota_iva: f64,
    pub natura: Option<String>,
}

/// Detail of a single payment entry.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDetail {
    pub modalita_pagamento: String,
    pub importo_pagamento: f64,
    pub data_scadenza_pagamento: Option<String>,
    pub iban: Option<String>,
}

/// Request to create a new invoice.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvoiceRequest {
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub divisa: String,
    pub cedente_id: String,
    pub cessionario_id: String,
    pub linee: Vec<CreateLineRequest>,
    pub pagamenti: Vec<CreatePaymentRequest>,
}

/// Request to create a single invoice line item.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLineRequest {
    pub descrizione: String,
    pub quantita: Option<f64>,
    pub unita_misura: Option<String>,
    pub prezzo_unitario: f64,
    pub aliquota_iva: f64,
    pub natura: Option<String>,
}

/// Request to create a single payment entry.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentRequest {
    pub modalita_pagamento: String,
    pub importo_pagamento: f64,
    pub data_scadenza_pagamento: Option<String>,
    pub iban: Option<String>,
}

/// Validation result returned to the frontend.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResultDto {
    pub valid: bool,
    pub errors: Vec<ValidationErrorDto>,
}

/// A single validation error.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationErrorDto {
    pub code: String,
    pub message: String,
    pub element_path: Option<String>,
}

/// List all invoices from the local SQLite database.
///
/// Joins on the clients table to include cedente and cessionario display names.
#[tauri::command]
pub async fn list_invoices(
    state: tauri::State<'_, TauriState>,
) -> Result<Vec<InvoiceSummary>, String> {
    // The owner company identity (id_paese, id_codice), if configured.
    let owner = owner_identity(&state.db).await;

    let rows = sqlx::query(
        "SELECT
            i.id, i.numero, i.data, i.tipo_documento, i.importo_totale, i.stato,
            c1.id_paese AS cedente_id_paese, c1.id_codice AS cedente_id_codice,
            c2.id_paese AS cessionario_id_paese, c2.id_codice AS cessionario_id_codice,
            COALESCE(c1.denominazione, c1.cognome || ' ' || COALESCE(c1.nome, '')) AS cedente_denominazione,
            COALESCE(c2.denominazione, c2.cognome || ' ' || COALESCE(c2.nome, '')) AS cessionario_denominazione
         FROM invoices i
         LEFT JOIN clients c1 ON i.cedente_id = c1.id
         LEFT JOIN clients c2 ON i.cessionario_id = c2.id
         ORDER BY i.data DESC, i.numero DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let invoices = rows
        .iter()
        .map(|row| {
            let cedente_id: (Option<String>, Option<String>) =
                (row.get("cedente_id_paese"), row.get("cedente_id_codice"));
            let cessionario_id: (Option<String>, Option<String>) = (
                row.get("cessionario_id_paese"),
                row.get("cessionario_id_codice"),
            );
            let direction = classify_direction(owner.as_ref(), &cedente_id, &cessionario_id);

            InvoiceSummary {
                id: row.get("id"),
                numero: row.get("numero"),
                data: row.get("data"),
                tipo_documento: row.get("tipo_documento"),
                importo_totale: row.get("importo_totale"),
                cedente_denominazione: row.get("cedente_denominazione"),
                cessionario_denominazione: row.get("cessionario_denominazione"),
                stato: row.get("stato"),
                direction,
            }
        })
        .collect();

    Ok(invoices)
}

/// Read the owner company's fiscal identity (id_paese, id_codice) from settings.
async fn owner_identity(pool: &sqlx::SqlitePool) -> Option<(String, String)> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = 'app_settings'")
        .fetch_optional(pool)
        .await
        .ok()??;
    let json: String = row.get("value");
    let settings: crate::commands::settings::AppSettings = serde_json::from_str(&json).ok()?;
    let company = settings.company?;
    Some((company.id_paese, company.id_codice))
}

/// Classify an invoice relative to the owner: `sale`, `purchase`, or `unknown`.
fn classify_direction(
    owner: Option<&(String, String)>,
    cedente: &(Option<String>, Option<String>),
    cessionario: &(Option<String>, Option<String>),
) -> String {
    let Some((owner_paese, owner_codice)) = owner else {
        return "unknown".to_string();
    };
    let matches = |id: &(Option<String>, Option<String>)| {
        id.0.as_deref() == Some(owner_paese.as_str())
            && id.1.as_deref() == Some(owner_codice.as_str())
    };
    if matches(cedente) {
        "sale".to_string()
    } else if matches(cessionario) {
        "purchase".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Get invoice detail by ID, including all line items and payments.
#[tauri::command]
pub async fn get_invoice(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<InvoiceDetail, String> {
    fetch_invoice_detail(&state.db, &id).await
}

/// Create a new invoice with its lines and payments in a single transaction.
///
/// Line totals are auto-computed as `prezzo_unitario * quantita` when quantita is provided.
/// The overall invoice total is the sum of all line totals.
#[tauri::command]
pub async fn create_invoice(
    state: tauri::State<'_, TauriState>,
    invoice: CreateInvoiceRequest,
) -> Result<InvoiceDetail, String> {
    let invoice_id = uuid::Uuid::new_v4().to_string();

    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    // Compute total from lines.
    let mut importo_totale: f64 = 0.0;

    // Insert invoice header.
    sqlx::query(
        "INSERT INTO invoices (id, numero, data, tipo_documento, divisa, importo_totale, stato, cedente_id, cessionario_id)
         VALUES (?, ?, ?, ?, ?, ?, 'draft', ?, ?)",
    )
    .bind(&invoice_id)
    .bind(&invoice.numero)
    .bind(&invoice.data)
    .bind(&invoice.tipo_documento)
    .bind(&invoice.divisa)
    .bind(0.0_f64) // placeholder, updated below
    .bind(&invoice.cedente_id)
    .bind(&invoice.cessionario_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // Insert line items.
    for (i, line) in invoice.linee.iter().enumerate() {
        let line_id = uuid::Uuid::new_v4().to_string();
        let numero_linea = (i + 1) as i32;
        let prezzo_totale = match line.quantita {
            Some(q) => line.prezzo_unitario * q,
            None => line.prezzo_unitario,
        };
        importo_totale += prezzo_totale;

        sqlx::query(
            "INSERT INTO invoice_lines (id, invoice_id, numero_linea, descrizione, quantita, unita_misura, prezzo_unitario, prezzo_totale, aliquota_iva, natura)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&line_id)
        .bind(&invoice_id)
        .bind(numero_linea)
        .bind(&line.descrizione)
        .bind(line.quantita)
        .bind(&line.unita_misura)
        .bind(line.prezzo_unitario)
        .bind(prezzo_totale)
        .bind(line.aliquota_iva)
        .bind(&line.natura)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    // Insert payment entries.
    for payment in &invoice.pagamenti {
        let payment_id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO invoice_payments (id, invoice_id, modalita_pagamento, importo_pagamento, data_scadenza_pagamento, iban)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&payment_id)
        .bind(&invoice_id)
        .bind(&payment.modalita_pagamento)
        .bind(payment.importo_pagamento)
        .bind(&payment.data_scadenza_pagamento)
        .bind(&payment.iban)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    // Update invoice total.
    sqlx::query("UPDATE invoices SET importo_totale = ? WHERE id = ?")
        .bind(importo_totale)
        .bind(&invoice_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    fetch_invoice_detail(&state.db, &invoice_id).await
}

/// Update an existing draft invoice with its lines and payments in a single transaction.
///
/// Only invoices in the `draft` state can be edited. The header is updated in place,
/// while lines and payments are fully replaced (delete + re-insert) to keep the logic
/// symmetric with `create_invoice`. Line totals and the invoice total are recomputed.
#[tauri::command]
pub async fn update_invoice(
    state: tauri::State<'_, TauriState>,
    id: String,
    invoice: CreateInvoiceRequest,
) -> Result<InvoiceDetail, String> {
    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    // Ensure the invoice exists and is still a draft.
    let stato: Option<String> = sqlx::query("SELECT stato FROM invoices WHERE id = ?")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        .map(|row| row.get("stato"));

    match stato.as_deref() {
        None => return Err(format!("Invoice not found: {id}")),
        Some("draft") => {}
        Some(other) => {
            return Err(format!(
                "Cannot edit invoice {id}: only draft invoices are editable (current state: {other})"
            ));
        }
    }

    // Update the invoice header (total recomputed below).
    sqlx::query(
        "UPDATE invoices SET
            numero = ?, data = ?, tipo_documento = ?, divisa = ?,
            cedente_id = ?, cessionario_id = ?, updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&invoice.numero)
    .bind(&invoice.data)
    .bind(&invoice.tipo_documento)
    .bind(&invoice.divisa)
    .bind(&invoice.cedente_id)
    .bind(&invoice.cessionario_id)
    .bind(&id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // Replace lines and payments.
    sqlx::query("DELETE FROM invoice_lines WHERE invoice_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM invoice_payments WHERE invoice_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let mut importo_totale: f64 = 0.0;

    for (i, line) in invoice.linee.iter().enumerate() {
        let line_id = uuid::Uuid::new_v4().to_string();
        let numero_linea = (i + 1) as i32;
        let prezzo_totale = match line.quantita {
            Some(q) => line.prezzo_unitario * q,
            None => line.prezzo_unitario,
        };
        importo_totale += prezzo_totale;

        sqlx::query(
            "INSERT INTO invoice_lines (id, invoice_id, numero_linea, descrizione, quantita, unita_misura, prezzo_unitario, prezzo_totale, aliquota_iva, natura)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&line_id)
        .bind(&id)
        .bind(numero_linea)
        .bind(&line.descrizione)
        .bind(line.quantita)
        .bind(&line.unita_misura)
        .bind(line.prezzo_unitario)
        .bind(prezzo_totale)
        .bind(line.aliquota_iva)
        .bind(&line.natura)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    for payment in &invoice.pagamenti {
        let payment_id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO invoice_payments (id, invoice_id, modalita_pagamento, importo_pagamento, data_scadenza_pagamento, iban)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&payment_id)
        .bind(&id)
        .bind(&payment.modalita_pagamento)
        .bind(payment.importo_pagamento)
        .bind(&payment.data_scadenza_pagamento)
        .bind(&payment.iban)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query("UPDATE invoices SET importo_totale = ? WHERE id = ?")
        .bind(importo_totale)
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    fetch_invoice_detail(&state.db, &id).await
}

/// Delete a draft invoice and its lines/payments.
///
/// Only invoices in the `draft` state can be deleted. The `invoice_lines` and
/// `invoice_payments` rows are removed automatically via `ON DELETE CASCADE`.
#[tauri::command]
pub async fn delete_invoice(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<(), String> {
    let stato: Option<String> = sqlx::query("SELECT stato FROM invoices WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .map(|row| row.get("stato"));

    match stato.as_deref() {
        None => return Err(format!("Invoice not found: {id}")),
        Some("draft") => {}
        Some(other) => {
            return Err(format!(
                "Cannot delete invoice {id}: only draft invoices are deletable (current state: {other})"
            ));
        }
    }

    sqlx::query("DELETE FROM invoices WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Suggest the next progressive invoice number for a cedente in a given year.
///
/// Counts the invoices already issued by `cedente_id` whose `data` falls in
/// `year` (matched by prefix) and returns `count + 1` as a plain string.
#[tauri::command]
pub async fn next_invoice_number(
    state: tauri::State<'_, TauriState>,
    cedente_id: String,
    year: String,
) -> Result<String, String> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM invoices WHERE cedente_id = ? AND data LIKE ? || '%' AND stato != 'imported'",
    )
    .bind(&cedente_id)
    .bind(&year)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let count: i64 = row.get("cnt");
    Ok((count + 1).to_string())
}

/// Change the lifecycle status of an invoice, enforcing the shared state machine.
///
/// Transitions are validated by `common::invoice_status`. Promotion to `validated`
/// is blocked unless the invoice passes SDI validation.
#[tauri::command]
pub async fn set_invoice_status(
    state: tauri::State<'_, TauriState>,
    id: String,
    status: String,
) -> Result<InvoiceDetail, String> {
    use common::invoice_status::InvoiceStatus;

    let current_str: String = sqlx::query("SELECT stato FROM invoices WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .map(|row| row.get("stato"))
        .ok_or_else(|| format!("Invoice not found: {id}"))?;

    let current: InvoiceStatus = current_str.parse()?;
    let next: InvoiceStatus = status.parse()?;
    current.transition_to(next)?;

    // Block promotion to `validated` if the invoice does not pass SDI validation.
    if next == InvoiceStatus::Validated {
        let fattura = build_fattura_from_db(&state.db, &id).await?;
        let result = common::validation::rules::validate(&fattura);
        if !result.is_valid() {
            let count = result.errors.len();
            return Err(format!(
                "Cannot validate invoice: {count} SDI validation error(s) must be resolved first"
            ));
        }
    }

    sqlx::query("UPDATE invoices SET stato = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(next.as_str())
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    fetch_invoice_detail(&state.db, &id).await
}

/// Validate an invoice against SDI rules.
///
/// Loads the invoice from the database, converts it to a `FatturaElettronica` proto,
/// and runs the validation rules from the common crate.
#[tauri::command]
pub async fn validate_invoice(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<ValidationResultDto, String> {
    let fattura = build_fattura_from_db(&state.db, &id).await?;
    let result = common::validation::rules::validate(&fattura);

    Ok(ValidationResultDto {
        valid: result.is_valid(),
        errors: result
            .errors
            .into_iter()
            .map(|e| ValidationErrorDto {
                code: e.code,
                message: e.message,
                element_path: e.element_path,
            })
            .collect(),
    })
}

/// Export an invoice as SDI-compliant XML.
///
/// Loads the invoice from the database, converts it to a `FatturaElettronica` proto,
/// and encodes it as XML. The frontend can then save the result to a file.
#[tauri::command]
pub async fn export_invoice_xml(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<String, String> {
    let fattura = build_fattura_from_db(&state.db, &id).await?;
    let xml = common::xml::encode::encode(&fattura).map_err(|e| e.to_string())?;

    // Persist the generated XML as an audit trail.
    sqlx::query("UPDATE invoices SET xml_content = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&xml)
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(xml)
}

/// Import an invoice from SDI XML content.
///
/// Decodes the XML into a `FatturaElettronica`, extracts the relevant fields,
/// and persists the invoice (with lines and payments) to the local database.
#[tauri::command]
pub async fn import_invoice_xml(
    state: tauri::State<'_, TauriState>,
    xml_content: String,
) -> Result<InvoiceDetail, String> {
    import_xml_content(&state.db, &xml_content).await
}

/// Core XML import routine: decode, ensure cedente/cessionario clients exist,
/// and persist the invoice with its lines and payments. Reused by the
/// single-file command and the recursive folder import.
pub(crate) async fn import_xml_content(
    pool: &sqlx::SqlitePool,
    xml_content: &str,
) -> Result<InvoiceDetail, String> {
    let fattura = common::xml::decode::decode(xml_content).map_err(|e| e.to_string())?;

    // Extract cedente and cessionario info to find or create clients.
    let header = fattura
        .header
        .as_ref()
        .ok_or("Missing invoice header")?;

    let cedente_id = ensure_client_from_header_cedente(pool, header).await?;
    let cessionario_id = ensure_client_from_header_cessionario(pool, header).await?;

    // A FatturaElettronica can carry multiple bodies (lotto); import each as its
    // own invoice and return the first one's detail.
    let mut imported_ids: Vec<String> = Vec::new();
    for body in &fattura.body {
        let dg = body
            .dati_generali
            .as_ref()
            .ok_or("Missing DatiGenerali")?;
        let dgd = dg
            .dati_generali_documento
            .as_ref()
            .ok_or("Missing DatiGeneraliDocumento")?;

        let invoice_id = uuid::Uuid::new_v4().to_string();
        let tipo_doc_str = common::xml::enum_map::tipo_documento_to_sdi(dgd.tipo_documento)
            .unwrap_or("TD01")
            .to_string();

        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

        // Insert invoice header.
        sqlx::query(
            "INSERT INTO invoices (id, numero, data, tipo_documento, divisa, importo_totale, stato, cedente_id, cessionario_id, xml_content)
             VALUES (?, ?, ?, ?, ?, ?, 'imported', ?, ?, ?)",
        )
        .bind(&invoice_id)
        .bind(&dgd.numero)
        .bind(&dgd.data)
        .bind(&tipo_doc_str)
        .bind(&dgd.divisa)
        .bind(dgd.importo_totale_documento)
        .bind(&cedente_id)
        .bind(&cessionario_id)
        .bind(xml_content)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        // Insert line items.
        if let Some(dbs) = &body.dati_beni_servizi {
            for linea in &dbs.dettaglio_linee {
                let line_id = uuid::Uuid::new_v4().to_string();
                let natura_str = linea
                    .natura
                    .and_then(|n| common::xml::enum_map::natura_to_sdi(n).map(String::from));

                sqlx::query(
                    "INSERT INTO invoice_lines (id, invoice_id, numero_linea, descrizione, quantita, unita_misura, prezzo_unitario, prezzo_totale, aliquota_iva, natura)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&line_id)
                .bind(&invoice_id)
                .bind(linea.numero_linea)
                .bind(&linea.descrizione)
                .bind(linea.quantita)
                .bind(&linea.unita_misura)
                .bind(linea.prezzo_unitario)
                .bind(linea.prezzo_totale)
                .bind(linea.aliquota_iva)
                .bind(&natura_str)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            }
        }

        // Insert payment entries.
        for dp in &body.dati_pagamento {
            let condizioni_str =
                common::xml::enum_map::condizioni_pagamento_to_sdi(dp.condizioni_pagamento)
                    .map(String::from);

            for det in &dp.dettaglio_pagamento {
                let payment_id = uuid::Uuid::new_v4().to_string();
                let modalita_str =
                    common::xml::enum_map::modalita_pagamento_to_sdi(det.modalita_pagamento)
                        .unwrap_or("MP01")
                        .to_string();

                sqlx::query(
                    "INSERT INTO invoice_payments (id, invoice_id, condizioni_pagamento, modalita_pagamento, importo_pagamento, data_scadenza_pagamento, iban, istituto_finanziario)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&payment_id)
                .bind(&invoice_id)
                .bind(&condizioni_str)
                .bind(&modalita_str)
                .bind(det.importo_pagamento)
                .bind(&det.data_scadenza_pagamento)
                .bind(&det.iban)
                .bind(&det.istituto_finanziario)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            }
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        imported_ids.push(invoice_id);
    }

    match imported_ids.first() {
        Some(id) => fetch_invoice_detail(pool, id).await,
        None => Err("No invoice body found in XML".to_string()),
    }
}

/// Result of a recursive folder import.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirImportSummary {
    /// Number of `.xml` files found during the walk.
    pub total_files: usize,
    /// Invoices successfully imported.
    pub imported: usize,
    /// Files skipped (not an invoice, duplicate, or parse error).
    pub skipped: usize,
    /// Per-file failures with their reason (capped to keep the payload small).
    pub errors: Vec<DirImportError>,
}

/// A single failed file during folder import.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirImportError {
    pub file: String,
    pub error: String,
}

/// Recursively import every `.xml` invoice found under `dir_path`.
///
/// Walks the directory tree, attempts to import each `.xml` file as an SDI
/// invoice (creating clients as needed), and returns a summary. Files that are
/// not invoices, duplicates, or otherwise fail are counted as skipped and the
/// first reasons are reported. Each file is independent: one failure never
/// aborts the whole run.
#[tauri::command]
pub async fn import_invoices_from_dir(
    state: tauri::State<'_, TauriState>,
    dir_path: String,
) -> Result<DirImportSummary, String> {
    let root = std::path::Path::new(&dir_path);
    if !root.is_dir() {
        return Err(format!("Not a directory: {dir_path}"));
    }

    // Collect all .xml files (recursive) before touching the DB.
    let mut xml_files = Vec::new();
    collect_xml_files(root, &mut xml_files).map_err(|e| format!("Walk failed: {e}"))?;

    let mut summary = DirImportSummary {
        total_files: xml_files.len(),
        imported: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    const MAX_REPORTED_ERRORS: usize = 50;

    for path in xml_files {
        let display = path.display().to_string();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                summary.skipped += 1;
                push_error(&mut summary, &display, format!("read error: {e}"), MAX_REPORTED_ERRORS);
                continue;
            }
        };

        match import_xml_content(&state.db, &content).await {
            Ok(_) => summary.imported += 1,
            Err(e) => {
                summary.skipped += 1;
                push_error(&mut summary, &display, e, MAX_REPORTED_ERRORS);
            }
        }
    }

    Ok(summary)
}

/// Record a per-file error, capping the reported list to keep the payload small.
fn push_error(summary: &mut DirImportSummary, file: &str, error: String, cap: usize) {
    if summary.errors.len() < cap {
        summary.errors.push(DirImportError {
            file: file.to_string(),
            error,
        });
    }
}

/// Recursively collect paths of `.xml` files under `dir`.
fn collect_xml_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_xml_files(&path, out)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("xml"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    Ok(())
}

// ===========================================================================
// Internal helpers
// ===========================================================================

/// Fetch full invoice detail from the database by ID.
async fn fetch_invoice_detail(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<InvoiceDetail, String> {
    let invoice_row = sqlx::query("SELECT * FROM invoices WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Invoice not found: {id}"))?;

    let line_rows = sqlx::query(
        "SELECT * FROM invoice_lines WHERE invoice_id = ? ORDER BY numero_linea",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let payment_rows = sqlx::query("SELECT * FROM invoice_payments WHERE invoice_id = ?")
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Resolve cedente/cessionario display names for the detail view.
    let names_row = sqlx::query(
        "SELECT
            COALESCE(c1.denominazione, c1.cognome || ' ' || COALESCE(c1.nome, ''), '') AS cedente_denominazione,
            COALESCE(c2.denominazione, c2.cognome || ' ' || COALESCE(c2.nome, ''), '') AS cessionario_denominazione
         FROM invoices i
         LEFT JOIN clients c1 ON i.cedente_id = c1.id
         LEFT JOIN clients c2 ON i.cessionario_id = c2.id
         WHERE i.id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let linee = line_rows
        .iter()
        .map(|row| InvoiceLineDetail {
            numero_linea: row.get("numero_linea"),
            descrizione: row.get("descrizione"),
            quantita: row.get("quantita"),
            unita_misura: row.get("unita_misura"),
            prezzo_unitario: row.get("prezzo_unitario"),
            prezzo_totale: row.get("prezzo_totale"),
            aliquota_iva: row.get("aliquota_iva"),
            natura: row.get("natura"),
        })
        .collect();

    let pagamenti = payment_rows
        .iter()
        .map(|row| PaymentDetail {
            modalita_pagamento: row.get("modalita_pagamento"),
            importo_pagamento: row.get("importo_pagamento"),
            data_scadenza_pagamento: row.get("data_scadenza_pagamento"),
            iban: row.get("iban"),
        })
        .collect();

    Ok(InvoiceDetail {
        id: invoice_row.get("id"),
        numero: invoice_row.get("numero"),
        data: invoice_row.get("data"),
        tipo_documento: invoice_row.get("tipo_documento"),
        divisa: invoice_row.get("divisa"),
        importo_totale: invoice_row.get("importo_totale"),
        stato: invoice_row.get("stato"),
        cedente_id: invoice_row.get("cedente_id"),
        cessionario_id: invoice_row.get("cessionario_id"),
        cedente_denominazione: names_row.get("cedente_denominazione"),
        cessionario_denominazione: names_row.get("cessionario_denominazione"),
        // `causale` is not persisted locally; expose an empty list for now.
        causale: Vec::new(),
        linee,
        pagamenti,
    })
}

/// Build a `FatturaElettronica` proto from database records.
///
/// This reconstructs the proto message structure from the invoice, its lines,
/// payments, and the associated cedente/cessionario client records.
pub(crate) async fn build_fattura_from_db(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<common::sdi::v1::FatturaElettronica, String> {
    use common::sdi::v1::*;

    // Fetch invoice.
    let inv = sqlx::query("SELECT * FROM invoices WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Invoice not found: {id}"))?;

    let cedente_id: String = inv.get("cedente_id");
    let cessionario_id: String = inv.get("cessionario_id");

    // Fetch clients.
    let cedente_row = sqlx::query("SELECT * FROM clients WHERE id = ?")
        .bind(&cedente_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let cessionario_row = sqlx::query("SELECT * FROM clients WHERE id = ?")
        .bind(&cessionario_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Fetch lines and payments.
    let line_rows = sqlx::query(
        "SELECT * FROM invoice_lines WHERE invoice_id = ? ORDER BY numero_linea",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let payment_rows = sqlx::query("SELECT * FROM invoice_payments WHERE invoice_id = ?")
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Build cedente.
    let cedente_denominazione: Option<String> = cedente_row.get("denominazione");
    let cedente_nome: Option<String> = cedente_row.get("nome");
    let cedente_cognome: Option<String> = cedente_row.get("cognome");
    let cedente_soggetto = build_anagrafica_soggetto(
        cedente_denominazione.as_deref(),
        cedente_nome.as_deref(),
        cedente_cognome.as_deref(),
    );

    let cedente_regime_str: Option<String> = cedente_row.get("regime_fiscale");
    let cedente_regime = cedente_regime_str
        .as_deref()
        .and_then(common::xml::enum_map::regime_fiscale_from_sdi)
        .unwrap_or(10); // Default RF01

    let cedente = CedentePrestatore {
        dati_anagrafici: Some(DatiAnagraficiCedente {
            id_fiscale_iva: Some(IdFiscale {
                id_paese: cedente_row.get("id_paese"),
                id_codice: cedente_row.get("id_codice"),
            }),
            codice_fiscale: cedente_row.get("codice_fiscale"),
            anagrafica: Some(Anagrafica {
                soggetto: cedente_soggetto,
                ..Default::default()
            }),
            regime_fiscale: cedente_regime,
            ..Default::default()
        }),
        sede: Some(Indirizzo {
            indirizzo: cedente_row.get("indirizzo"),
            numero_civico: cedente_row.get("numero_civico"),
            cap: cedente_row.get("cap"),
            comune: cedente_row.get("comune"),
            provincia: cedente_row.get("provincia"),
            nazione: cedente_row.get("nazione"),
        }),
        ..Default::default()
    };

    // Build cessionario.
    let cess_denominazione: Option<String> = cessionario_row.get("denominazione");
    let cess_nome: Option<String> = cessionario_row.get("nome");
    let cess_cognome: Option<String> = cessionario_row.get("cognome");
    let cess_soggetto = build_anagrafica_soggetto(
        cess_denominazione.as_deref(),
        cess_nome.as_deref(),
        cess_cognome.as_deref(),
    );

    let cessionario = CessionarioCommittente {
        dati_anagrafici: Some(DatiAnagraficiCessionario {
            id_fiscale_iva: Some(IdFiscale {
                id_paese: cessionario_row.get("id_paese"),
                id_codice: cessionario_row.get("id_codice"),
            }),
            codice_fiscale: cessionario_row.get("codice_fiscale"),
            anagrafica: Some(Anagrafica {
                soggetto: cess_soggetto,
                ..Default::default()
            }),
        }),
        sede: Some(Indirizzo {
            indirizzo: cessionario_row.get("indirizzo"),
            numero_civico: cessionario_row.get("numero_civico"),
            cap: cessionario_row.get("cap"),
            comune: cessionario_row.get("comune"),
            provincia: cessionario_row.get("provincia"),
            nazione: cessionario_row.get("nazione"),
        }),
        ..Default::default()
    };

    // Build codice destinatario.
    let codice_destinatario: Option<String> = cessionario_row.get("codice_destinatario");
    let codice_dest = codice_destinatario.unwrap_or_else(|| "0000000".to_string());

    // Build header.
    let header = FatturaElettronicaHeader {
        dati_trasmissione: Some(DatiTrasmissione {
            id_trasmittente: Some(IdFiscale {
                id_paese: cedente_row.get::<String, _>("id_paese"),
                id_codice: cedente_row.get::<String, _>("id_codice"),
            }),
            progressivo_invio: common::pec::progressivo_from_id(id),
            formato_trasmissione: 20, // FPR12 (private)
            codice_destinatario: codice_dest,
            ..Default::default()
        }),
        cedente_prestatore: Some(cedente),
        cessionario_committente: Some(cessionario),
        ..Default::default()
    };

    // Build document type.
    let tipo_doc_str: String = inv.get("tipo_documento");
    let tipo_documento = common::xml::enum_map::tipo_documento_from_sdi(&tipo_doc_str).unwrap_or(10);

    let numero: String = inv.get("numero");
    let data: String = inv.get("data");
    let divisa: String = inv.get("divisa");
    let importo_totale: Option<f64> = inv.get("importo_totale");

    // Build line items.
    let mut dettaglio_linee = Vec::new();
    for row in &line_rows {
        let natura_str: Option<String> = row.get("natura");
        let natura_val = natura_str
            .as_deref()
            .and_then(common::xml::enum_map::natura_from_sdi);

        dettaglio_linee.push(DettaglioLinee {
            numero_linea: row.get("numero_linea"),
            descrizione: row.get("descrizione"),
            quantita: row.get("quantita"),
            unita_misura: row.get("unita_misura"),
            prezzo_unitario: row.get("prezzo_unitario"),
            prezzo_totale: row.get("prezzo_totale"),
            aliquota_iva: row.get("aliquota_iva"),
            natura: natura_val,
            ..Default::default()
        });
    }

    // Build riepilogo (summary by VAT rate) from the lines.
    let mut riepilogo_map: std::collections::HashMap<String, (f64, f64, Option<i32>)> =
        std::collections::HashMap::new();
    for linea in &dettaglio_linee {
        let key = format!("{:.2}", linea.aliquota_iva);
        let entry = riepilogo_map.entry(key).or_insert((0.0, linea.aliquota_iva, linea.natura));
        entry.0 += linea.prezzo_totale;
    }

    let dati_riepilogo: Vec<DatiRiepilogo> = riepilogo_map
        .values()
        .map(|(imponibile, aliquota, natura)| {
            // Round to 2 decimals (SDI 00421): VAT must match the cent.
            let imposta = ((aliquota * imponibile / 100.0) * 100.0).round() / 100.0;
            DatiRiepilogo {
                aliquota_iva: *aliquota,
                imponibile_importo: *imponibile,
                imposta,
                natura: *natura,
                ..Default::default()
            }
        })
        .collect();

    // Build payments.
    let mut dati_pagamento = Vec::new();
    if !payment_rows.is_empty() {
        let mut dettagli = Vec::new();
        for row in &payment_rows {
            let mod_str: String = row.get("modalita_pagamento");
            let modalita =
                common::xml::enum_map::modalita_pagamento_from_sdi(&mod_str).unwrap_or(10);

            dettagli.push(DettaglioPagamento {
                modalita_pagamento: modalita,
                importo_pagamento: row.get("importo_pagamento"),
                data_scadenza_pagamento: row.get("data_scadenza_pagamento"),
                iban: row.get("iban"),
                istituto_finanziario: row.get("istituto_finanziario"),
                ..Default::default()
            });
        }

        // Use condizioni from the first payment row (all should be the same per SDI convention).
        let cond_str: Option<String> = payment_rows[0].get("condizioni_pagamento");
        let condizioni = cond_str
            .as_deref()
            .and_then(common::xml::enum_map::condizioni_pagamento_from_sdi)
            .unwrap_or(20); // TP02 (standard) as default

        dati_pagamento.push(DatiPagamento {
            condizioni_pagamento: condizioni,
            dettaglio_pagamento: dettagli,
        });
    }

    // Assemble the FatturaElettronica.
    let fattura = FatturaElettronica {
        versione: 20, // FPR12
        sistema_emittente: None,
        header: Some(header),
        body: vec![FatturaElettronicaBody {
            dati_generali: Some(DatiGenerali {
                dati_generali_documento: Some(DatiGeneraliDocumento {
                    tipo_documento,
                    divisa,
                    data,
                    numero,
                    importo_totale_documento: importo_totale,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            dati_beni_servizi: Some(DatiBeniServizi {
                dettaglio_linee,
                dati_riepilogo,
            }),
            dati_pagamento,
            ..Default::default()
        }],
    };

    Ok(fattura)
}

/// Build the `Anagrafica.soggetto` oneof from database fields.
fn build_anagrafica_soggetto(
    denominazione: Option<&str>,
    nome: Option<&str>,
    cognome: Option<&str>,
) -> Option<common::sdi::v1::anagrafica::Soggetto> {
    use common::sdi::v1::anagrafica::Soggetto;
    use common::sdi::v1::PersonaFisica;

    if let Some(denom) = denominazione {
        Some(Soggetto::Denominazione(denom.to_string()))
    } else if let (Some(nome), Some(cognome)) = (nome, cognome) {
        Some(Soggetto::PersonaFisica(PersonaFisica {
            nome: nome.to_string(),
            cognome: cognome.to_string(),
        }))
    } else { cognome.map(|cognome| Soggetto::PersonaFisica(PersonaFisica {
            nome: String::new(),
            cognome: cognome.to_string(),
        })) }
}

/// Find or create a client record from the cedente data in an XML header.
async fn ensure_client_from_header_cedente(
    pool: &sqlx::SqlitePool,
    header: &common::sdi::v1::FatturaElettronicaHeader,
) -> Result<String, String> {
    let cp = header
        .cedente_prestatore
        .as_ref()
        .ok_or("Missing CedentePrestatore")?;
    let da = cp
        .dati_anagrafici
        .as_ref()
        .ok_or("Missing DatiAnagrafici for cedente")?;
    let id_fiscale = da
        .id_fiscale_iva
        .as_ref()
        .ok_or("Missing IdFiscaleIVA for cedente")?;

    let (denominazione, nome, cognome) = extract_anagrafica_names(da.anagrafica.as_ref());
    let sede = cp.sede.as_ref();

    let regime_str = common::xml::enum_map::regime_fiscale_to_sdi(da.regime_fiscale)
        .map(String::from);

    ensure_client(
        pool,
        &id_fiscale.id_paese,
        &id_fiscale.id_codice,
        denominazione.as_deref(),
        nome.as_deref(),
        cognome.as_deref(),
        da.codice_fiscale.as_deref(),
        regime_str.as_deref(),
        sede,
    )
    .await
}

/// Find or create a client record from the cessionario data in an XML header.
async fn ensure_client_from_header_cessionario(
    pool: &sqlx::SqlitePool,
    header: &common::sdi::v1::FatturaElettronicaHeader,
) -> Result<String, String> {
    let cc = header
        .cessionario_committente
        .as_ref()
        .ok_or("Missing CessionarioCommittente")?;
    let da = cc
        .dati_anagrafici
        .as_ref()
        .ok_or("Missing DatiAnagrafici for cessionario")?;

    let id_fiscale = da.id_fiscale_iva.as_ref();
    let (id_paese, id_codice) = match id_fiscale {
        Some(id) => (id.id_paese.as_str(), id.id_codice.as_str()),
        None => ("IT", da.codice_fiscale.as_deref().unwrap_or("00000000000")),
    };

    let (denominazione, nome, cognome) = extract_anagrafica_names(da.anagrafica.as_ref());
    let sede = cc.sede.as_ref();

    ensure_client(
        pool,
        id_paese,
        id_codice,
        denominazione.as_deref(),
        nome.as_deref(),
        cognome.as_deref(),
        da.codice_fiscale.as_deref(),
        None,
        sede,
    )
    .await
}

/// Find a client by (id_paese, id_codice) or create a new one.
#[allow(clippy::too_many_arguments)]
async fn ensure_client(
    pool: &sqlx::SqlitePool,
    id_paese: &str,
    id_codice: &str,
    denominazione: Option<&str>,
    nome: Option<&str>,
    cognome: Option<&str>,
    codice_fiscale: Option<&str>,
    regime_fiscale: Option<&str>,
    sede: Option<&common::sdi::v1::Indirizzo>,
) -> Result<String, String> {
    // Try to find existing client by tax ID.
    let existing = sqlx::query("SELECT id FROM clients WHERE id_paese = ? AND id_codice = ?")
        .bind(id_paese)
        .bind(id_codice)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(row) = existing {
        return Ok(row.get("id"));
    }

    // Create a new client.
    let client_id = uuid::Uuid::new_v4().to_string();
    let (indirizzo, numero_civico, cap, comune, provincia, nazione) = match sede {
        Some(s) => (
            s.indirizzo.as_str(),
            s.numero_civico.as_deref(),
            s.cap.as_str(),
            s.comune.as_str(),
            s.provincia.as_deref(),
            s.nazione.as_str(),
        ),
        None => ("", None, "00000", "", None, "IT"),
    };

    sqlx::query(
        "INSERT INTO clients (id, denominazione, nome, cognome, id_paese, id_codice, codice_fiscale, regime_fiscale, indirizzo, numero_civico, cap, comune, provincia, nazione)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&client_id)
    .bind(denominazione)
    .bind(nome)
    .bind(cognome)
    .bind(id_paese)
    .bind(id_codice)
    .bind(codice_fiscale)
    .bind(regime_fiscale)
    .bind(indirizzo)
    .bind(numero_civico)
    .bind(cap)
    .bind(comune)
    .bind(provincia)
    .bind(nazione)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(client_id)
}

/// Extract (denominazione, nome, cognome) from an Anagrafica proto message.
fn extract_anagrafica_names(
    anagrafica: Option<&common::sdi::v1::Anagrafica>,
) -> (Option<String>, Option<String>, Option<String>) {
    use common::sdi::v1::anagrafica::Soggetto;

    match anagrafica.and_then(|a| a.soggetto.as_ref()) {
        Some(Soggetto::Denominazione(d)) => (Some(d.clone()), None, None),
        Some(Soggetto::PersonaFisica(pf)) => (None, Some(pf.nome.clone()), Some(pf.cognome.clone())),
        None => (None, None, None),
    }
}
