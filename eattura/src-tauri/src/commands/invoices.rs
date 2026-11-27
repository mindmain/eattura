use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::state::TauriState;
use crate::storage::digest;

/// Summary view of an invoice for list display.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceSummary {
    pub id: String,
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub importo_totale: Option<f64>,
    pub total_calculated: bool,
    pub cedente_denominazione: String,
    pub cessionario_denominazione: String,
    pub stato: String,
    #[serde(default)]
    pub tax_id_errors: Vec<ValidationErrorDto>,
    /// Relative to the configured owner company:
    /// `sale` (owner is cedente → income), `purchase` (owner is cessionario →
    /// cost), or `unknown` (owner not configured or on neither side).
    pub direction: String,
    pub search_text: String,
    #[serde(default)]
    pub is_signed: bool,
    #[serde(default)]
    pub is_autofattura: bool,
    #[serde(default)]
    pub is_pa: bool,
    #[serde(default)]
    pub is_split_payment: bool,
    #[serde(default)]
    pub has_mc_notice: bool,
}

/// Summary of an attached SDI notification in the invoice fascicolo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdiNotificationDetail {
    pub id: String,
    pub tipo: String,
    pub identificativo_sdi: Option<String>,
    pub nome_file: Option<String>,
    pub esito: Option<String>,
    pub data_ora_ricezione: Option<String>,
    pub destinatario: Option<String>,
    pub note: Option<String>,
    pub errori: Vec<common::pec::SdiError>,
    pub raw_xml: String,
}

/// Withholding tax detail (DatiRitenuta)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(schemars::JsonSchema)]
pub struct DatiRitenutaDetail {
    pub tipo_ritenuta: String,
    pub importo_ritenuta: f64,
    pub aliquota_ritenuta: f64,
    pub causale_pagamento: String,
}

/// Virtual stamp duty detail (DatiBollo)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(schemars::JsonSchema)]
pub struct DatiBolloDetail {
    pub bollo_virtuale: String,
    pub importo_bollo: Option<f64>,
}

/// Linked invoice reference for autofatture / reverse charge integration (DatiFattureCollegate)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(schemars::JsonSchema)]
pub struct DatiFatturaCollegataDetail {
    pub id_documento: String,
    pub data: Option<String>,
    pub num_item: Option<String>,
    pub codice_cig: Option<String>,
    pub codice_cup: Option<String>,
}

/// Procurement / financial traceability detail for PA and contracts (DatiOrdineAcquisto, DatiContratto, DatiConvenzione)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(schemars::JsonSchema)]
pub struct DatiDocumentoTracciabilitaDetail {
    #[serde(default)]
    pub tipo_documento_rif: Option<String>,
    pub id_documento: String,
    pub data: Option<String>,
    pub num_item: Option<String>,
    pub codice_commessa_convenzione: Option<String>,
    pub codice_cup: Option<String>,
    pub codice_cig: Option<String>,
}

/// Social security / professional fund contribution detail (DatiCassaPrevidenziale)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(schemars::JsonSchema)]
pub struct DatiCassaDetail {
    pub tipo_cassa: String,
    pub al_cassa: f64,
    pub importo_contributo_cassa: f64,
    pub imponibile_cassa: Option<f64>,
    pub aliquota_iva: f64,
    pub ritenuta: Option<String>,
    pub natura: Option<String>,
}

/// Attachment detail embedded inside the XML invoice (2.5 <Allegati>)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDetail {
    pub nome_attachment: String,
    pub algoritmo_compressione: Option<String>,
    pub formato_attachment: Option<String>,
    pub descrizione_attachment: Option<String>,
    pub size_bytes: usize,
    pub attachment_base64: String,
}

/// Request to attach a file to an invoice (2.5 <Allegati>)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(schemars::JsonSchema)]
pub struct CreateAttachmentRequest {
    pub nome_attachment: String,
    #[serde(default)]
    pub algoritmo_compressione: Option<String>,
    #[serde(default)]
    pub formato_attachment: Option<String>,
    #[serde(default)]
    pub descrizione_attachment: Option<String>,
    pub attachment_base64: String,
}

/// Transport document detail (2.1.8 <DatiDDT>) for deferred invoices (TD24)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(schemars::JsonSchema)]
pub struct DatiDdtDetail {
    pub numero_ddt: String,
    pub data_ddt: String,
    #[serde(default)]
    pub riferimento_numero_linea: Vec<i32>,
}

/// Full invoice detail with lines and payments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceDetail {
    pub id: String,
    #[serde(default)]
    pub revision: Option<String>,
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub divisa: String,
    pub importo_totale: Option<f64>,
    pub total_calculated: bool,
    pub stato: String,
    #[serde(default)]
    pub tax_id_errors: Vec<ValidationErrorDto>,
    pub cedente_id: String,
    pub cessionario_id: String,
    pub cedente_denominazione: String,
    pub cessionario_denominazione: String,
    pub causale: Vec<String>,
    pub linee: Vec<InvoiceLineDetail>,
    pub pagamenti: Vec<PaymentDetail>,
    #[serde(default)]
    pub is_signed: bool,
    #[serde(default)]
    pub signer_info: Option<common::p7m::SignerInfo>,
    #[serde(default)]
    pub notifications: Vec<SdiNotificationDetail>,
    #[serde(default)]
    pub netto_a_pagare: Option<f64>,
    #[serde(default)]
    pub dati_ritenuta: Vec<DatiRitenutaDetail>,
    #[serde(default)]
    pub dati_bollo: Option<DatiBolloDetail>,
    #[serde(default)]
    pub dati_cassa_previdenziale: Vec<DatiCassaDetail>,
    #[serde(default)]
    pub dati_fatture_collegate: Vec<DatiFatturaCollegataDetail>,
    #[serde(default)]
    pub soggetto_emittente: Option<String>,
    #[serde(default)]
    pub is_autofattura: bool,
    #[serde(default)]
    pub is_pa: bool,
    #[serde(default)]
    pub is_split_payment: bool,
    #[serde(default)]
    pub esigibilita_iva: Option<String>,
    #[serde(default)]
    pub formato_trasmissione: Option<String>,
    #[serde(default)]
    pub codice_destinatario: Option<String>,
    #[serde(default)]
    pub pec_destinatario: Option<String>,
    #[serde(default)]
    pub pending_pec_filename: Option<String>,
    #[serde(default)]
    pub active_pec_filename: Option<String>,
    #[serde(default)]
    pub recoverable_pec_filename: Option<String>,
    #[serde(default)]
    pub has_mc_notice: bool,
    #[serde(default)]
    pub dati_ordine_acquisto: Vec<DatiDocumentoTracciabilitaDetail>,
    #[serde(default)]
    pub dati_contratto: Vec<DatiDocumentoTracciabilitaDetail>,
    #[serde(default)]
    pub dati_convenzione: Vec<DatiDocumentoTracciabilitaDetail>,
    #[serde(default)]
    pub dati_ddt: Vec<DatiDdtDetail>,
    #[serde(default)]
    pub allegati: Vec<AttachmentDetail>,
}

/// Detail of a single invoice line item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLineDetail {
    pub numero_linea: i32,
    pub descrizione: String,
    pub quantita: Option<f64>,
    pub unita_misura: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_inizio_periodo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_fine_periodo: Option<String>,
    pub prezzo_unitario: f64,
    pub prezzo_totale: f64,
    pub aliquota_iva: f64,
    pub natura: Option<String>,
    #[serde(flatten)]
    pub fields: LineFields,
}

/// Detail of a single payment entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDetail {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condizioni_pagamento: Option<String>,
    pub modalita_pagamento: String,
    pub importo_pagamento: f64,
    pub data_scadenza_pagamento: Option<String>,
    pub iban: Option<String>,
}

/// Request to create a new invoice.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(schemars::JsonSchema)]
pub struct CreateInvoiceRequest {
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub divisa: String,
    /// Exact issuer ID from the local client catalog.
    pub cedente_id: String,
    /// Exact customer ID from the local client catalog.
    pub cessionario_id: String,
    #[serde(default)]
    pub causale: Vec<String>,
    pub linee: Vec<CreateLineRequest>,
    pub pagamenti: Vec<CreatePaymentRequest>,
    #[serde(default)]
    pub dati_ritenuta: Option<Vec<DatiRitenutaDetail>>,
    #[serde(default)]
    pub dati_bollo: Option<DatiBolloDetail>,
    #[serde(default)]
    pub dati_cassa_previdenziale: Option<Vec<DatiCassaDetail>>,
    #[serde(default)]
    pub dati_fatture_collegate: Option<Vec<DatiFatturaCollegataDetail>>,
    #[serde(default)]
    pub soggetto_emittente: Option<String>,
    #[serde(default)]
    pub is_pa: Option<bool>,
    #[serde(default)]
    pub is_split_payment: Option<bool>,
    #[serde(default)]
    pub esigibilita_iva: Option<String>,
    #[serde(default)]
    pub dati_ordine_acquisto: Option<Vec<DatiDocumentoTracciabilitaDetail>>,
    #[serde(default)]
    pub dati_contratto: Option<Vec<DatiDocumentoTracciabilitaDetail>>,
    #[serde(default)]
    pub dati_convenzione: Option<Vec<DatiDocumentoTracciabilitaDetail>>,
    #[serde(default)]
    pub dati_ddt: Option<Vec<DatiDdtDetail>>,
    #[serde(default)]
    pub allegati: Option<Vec<CreateAttachmentRequest>>,
}

/// Request to create a single invoice line item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(schemars::JsonSchema)]
pub struct CreateLineRequest {
    /// SdI line description. Use Basic Latin or Latin-1 Supplement.
    #[schemars(length(min = 1, max = 1000))]
    pub descrizione: String,
    pub quantita: Option<f64>,
    pub unita_misura: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_inizio_periodo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_fine_periodo: Option<String>,
    pub prezzo_unitario: f64,
    pub aliquota_iva: f64,
    pub natura: Option<String>,
    #[serde(flatten)]
    pub fields: LineFields,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct LineFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tipo_cessione_prestazione: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub codice_articolo: Vec<LineArticleCode>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sconto_maggiorazione: Vec<LineAdjustment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ritenuta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub riferimento_amministrazione: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub altri_dati_gestionali: Vec<LineManagementData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LineArticleCode {
    pub codice_tipo: String,
    pub codice_valore: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LineAdjustment {
    pub tipo: String,
    pub percentuale: Option<f64>,
    pub importo: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LineManagementData {
    pub tipo_dato: String,
    pub riferimento_testo: Option<String>,
    pub riferimento_numero: Option<f64>,
    pub riferimento_data: Option<String>,
}

impl LineFields {
    fn adjustments(&self) -> Vec<common::sdi::v1::ScontoMaggiorazione> {
        self.sconto_maggiorazione
            .iter()
            .filter_map(|item| {
                Some(common::sdi::v1::ScontoMaggiorazione {
                    tipo: common::xml::enum_map::tipo_sconto_maggiorazione_from_sdi(&item.tipo)?,
                    percentuale: item.percentuale,
                    importo: item.importo,
                })
            })
            .collect()
    }

    fn from_document(line: &common::sdi::v1::DettaglioLinee) -> Self {
        use common::xml::enum_map::*;
        Self {
            tipo_cessione_prestazione: line
                .tipo_cessione_prestazione
                .and_then(tipo_cessione_prestazione_to_sdi)
                .map(str::to_owned),
            codice_articolo: line
                .codice_articolo
                .iter()
                .map(|item| LineArticleCode {
                    codice_tipo: item.codice_tipo.clone(),
                    codice_valore: item.codice_valore.clone(),
                })
                .collect(),
            sconto_maggiorazione: line
                .sconto_maggiorazione
                .iter()
                .filter_map(|item| {
                    Some(LineAdjustment {
                        tipo: tipo_sconto_maggiorazione_to_sdi(item.tipo)?.to_owned(),
                        percentuale: item.percentuale,
                        importo: item.importo,
                    })
                })
                .collect(),
            ritenuta: line.ritenuta.and_then(ritenuta_to_sdi).map(str::to_owned),
            riferimento_amministrazione: line.riferimento_amministrazione.clone(),
            altri_dati_gestionali: line
                .altri_dati_gestionali
                .iter()
                .map(|item| LineManagementData {
                    tipo_dato: item.tipo_dato.clone(),
                    riferimento_testo: item.riferimento_testo.clone(),
                    riferimento_numero: item.riferimento_numero,
                    riferimento_data: item.riferimento_data.clone(),
                })
                .collect(),
        }
    }
}

impl CreateLineRequest {
    fn total(&self) -> f64 {
        common::validation::rules::apply_sconto_maggiorazione(
            self.prezzo_unitario,
            &self.fields.adjustments(),
        ) * self.quantita.unwrap_or(1.0)
    }
}

async fn insert_draft_lines(
    tx: &mut crate::storage::Transaction,
    invoice_id: &str,
    lines: &[CreateLineRequest],
) -> Result<(), String> {
    for (index, line) in lines.iter().enumerate() {
        tx.insert(
            "invoice_lines",
            serde_json::json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "invoice_id": invoice_id,
                "numero_linea": index + 1,
                "descrizione": line.descrizione,
                "quantita": line.quantita,
                "unita_misura": line.unita_misura,
                "data_inizio_periodo": line.data_inizio_periodo,
                "data_fine_periodo": line.data_fine_periodo,
                "prezzo_unitario": line.prezzo_unitario,
                "prezzo_totale": line.total(),
                "aliquota_iva": line.aliquota_iva,
                "natura": line.natura,
                "line_fields": line.fields,
            }),
        )
        .await?;
    }
    Ok(())
}

fn parse_line_date(number: usize, value: &str, field: &str) -> Result<chrono::NaiveDate, String> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .filter(|date| date.format("%Y-%m-%d").to_string() == value && value.len() == 10)
        .ok_or_else(|| {
            format!(
                "Line {}: {field} must be a date in YYYY-MM-DD format",
                number
            )
        })
}

fn check_line_period(number: usize, start: Option<&str>, end: Option<&str>) -> Result<(), String> {
    let start = start
        .map(|value| parse_line_date(number, value, "dataInizioPeriodo"))
        .transpose()?;
    let end = end
        .map(|value| parse_line_date(number, value, "dataFinePeriodo"))
        .transpose()?;
    if let (Some(start), Some(end)) = (start, end)
        && start > end
    {
        return Err(format!(
            "Line {number}: dataFinePeriodo precedes dataInizioPeriodo"
        ));
    }
    Ok(())
}

fn check_draft_lines(lines: &[CreateLineRequest]) -> Result<(), String> {
    for (index, line) in lines.iter().enumerate() {
        use common::xml::enum_map::*;
        common::validation::rules::validate_line_description(&line.descrizione)
            .map_err(|error| format!("00200, line {}: {error}", index + 1))?;
        let fields = &line.fields;
        if fields
            .tipo_cessione_prestazione
            .as_deref()
            .is_some_and(|code| tipo_cessione_prestazione_from_sdi(code).is_none())
            || fields
                .ritenuta
                .as_deref()
                .is_some_and(|code| ritenuta_from_sdi(code).is_none())
            || fields.sconto_maggiorazione.iter().any(|item| {
                tipo_sconto_maggiorazione_from_sdi(&item.tipo).is_none()
                    || (item.percentuale.is_none() && item.importo.is_none())
                    || item
                        .percentuale
                        .is_some_and(|pct| !(0.0..=100.0).contains(&pct))
            })
        {
            return Err(format!(
                "Line {}: invalid transaction type, withholding or adjustment",
                index + 1
            ));
        }
        for item in &fields.altri_dati_gestionali {
            if let Some(date) = &item.riferimento_data {
                parse_line_date(index + 1, date, "riferimentoData")?;
            }
        }
        check_line_period(
            index + 1,
            line.data_inizio_periodo.as_deref(),
            line.data_fine_periodo.as_deref(),
        )?;
    }
    Ok(())
}

/// Request to create a single payment entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(schemars::JsonSchema)]
pub struct CreatePaymentRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condizioni_pagamento: Option<String>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationErrorDto {
    pub code: String,
    pub message: String,
    pub element_path: Option<String>,
}

impl From<common::validation::ValidationError> for ValidationErrorDto {
    fn from(error: common::validation::ValidationError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            element_path: error.element_path,
        }
    }
}

fn tax_id_errors(row: &crate::storage::Record) -> Vec<ValidationErrorDto> {
    row.get::<Option<Vec<common::validation::ValidationError>>>("tax_id_errors")
        .unwrap_or_default()
        .into_iter()
        .map(ValidationErrorDto::from)
        .collect()
}

/// List all invoices from the local file store.
///
/// Resolves associated clients to include cedente and cessionario display names.
#[tauri::command]
pub async fn list_invoices(
    state: tauri::State<'_, TauriState>,
) -> Result<Vec<InvoiceSummary>, String> {
    list_invoices_store(&state.store).await
}

pub async fn list_invoices_store(
    store: &crate::storage::FileStore,
) -> Result<Vec<InvoiceSummary>, String> {
    // The owner company identity (id_paese, id_codice), if configured.
    let owner = owner_identity(store).await;

    let rows = store
        .read("invoices", "invoice_views", vec![], "invoice")
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    // Backfill older imports once. Keep the original XML field absent while
    // caching a separate amount for lists, exports, and the dashboard.
    let mut backfilled = std::collections::HashMap::new();
    for row in &rows {
        if row.get::<Option<f64>>("importo_totale").is_some()
            || row.get::<Option<f64>>("importo_totale_calcolato").is_some()
        {
            continue;
        }
        let id: String = row.get("id");
        let Some(stored) = store
            .read("invoices", "id", vec![serde_json::json!(&id)], "")
            .await
            .fetch_optional()
            .await?
        else {
            continue;
        };
        if let Some(xml) = stored.get::<Option<String>>("xml_content")
            && let Some(total) = inferred_total_for_imported_body(&id, &xml)
        {
            backfilled.insert(id, total);
        }
    }
    if !backfilled.is_empty() {
        let mut tx = store.begin().await?;
        for (id, total) in &backfilled {
            tx.update(
                "invoices",
                id,
                serde_json::json!({"importo_totale_calcolato": total}),
            )
            .await?;
        }
        tx.commit().await?;
    }

    let all_notifications = store
        .read("sdi_notifications", "all", vec![], "")
        .await
        .fetch_all()
        .await
        .unwrap_or_default();
    let mut notifications_by_invoice: std::collections::HashMap<
        String,
        Vec<crate::storage::Record>,
    > = std::collections::HashMap::new();
    for notif in all_notifications {
        if let Some(inv_id) = notif.get::<Option<String>>("invoice_id") {
            notifications_by_invoice
                .entry(inv_id)
                .or_default()
                .push(notif);
        }
    }

    let mut mc_invoice_ids = std::collections::HashSet::new();
    for row in &rows {
        let stato: String = row.get("stato");
        if matches!(stato.as_str(), "draft" | "validated") {
            continue;
        }
        let id: String = row.get("id");
        let Some(receipts) = notifications_by_invoice.get(&id) else {
            continue;
        };
        if !receipts.iter().any(|r| r.get::<String>("tipo") == "MC") {
            continue;
        }
        let mut active = row.get::<Option<String>>("active_pec_filename");
        let mut recoverable = false;
        if (active.is_none() || stato == "rejected")
            && let Some(full_row) = store
                .read("invoices", "id", vec![serde_json::json!(&id)], "")
                .await
                .fetch_optional()
                .await?
        {
            if active.is_none() {
                active = crate::commands::pec::active_pec_filename(&full_row);
            }
            if stato == "rejected" {
                recoverable = crate::commands::pec::recoverable_pec_filename(store, &full_row, &id)
                    .await?
                    .is_some();
            }
        }
        if !recoverable && crate::commands::pec::records_have_attempt_mc(receipts, active.as_deref())
        {
            mc_invoice_ids.insert(id);
        }
    }

    let invoices = rows
        .iter()
        .map(|row| {
            let id: String = row.get("id");
            let cedente_id: (Option<String>, Option<String>) =
                (row.get("cedente_id_paese"), row.get("cedente_id_codice"));
            let cessionario_id: (Option<String>, Option<String>) = (
                row.get("cessionario_id_paese"),
                row.get("cessionario_id_codice"),
            );
            let direction = classify_direction(owner.as_ref(), &cedente_id, &cessionario_id);
            let tipo_doc_val: String = row.get("tipo_documento");
            let is_autofattura = row.get::<Option<bool>>("is_autofattura").unwrap_or({
                matches!(
                    tipo_doc_val.as_str(),
                    "TD16" | "TD17" | "TD18" | "TD19" | "TD20" | "TD21" | "TD27"
                )
            });

            let is_pa = row.get::<Option<bool>>("is_pa").unwrap_or(false);
            let is_split_payment = row.get::<Option<bool>>("is_split_payment").unwrap_or(false);
            let has_mc_notice = mc_invoice_ids.contains(&id);

            InvoiceSummary {
                search_text: row.get("search_text"),
                numero: row.get("numero"),
                data: row.get("data"),
                tipo_documento: row.get("tipo_documento"),
                importo_totale: row
                    .get::<Option<f64>>("importo_totale")
                    .or_else(|| row.get("importo_totale_calcolato"))
                    .or_else(|| backfilled.get(&id).copied()),
                total_calculated: row.get::<Option<f64>>("importo_totale").is_none()
                    && (row.get::<Option<f64>>("importo_totale_calcolato").is_some()
                        || backfilled.contains_key(&id)),
                cedente_denominazione: row.get("cedente_denominazione"),
                cessionario_denominazione: row.get("cessionario_denominazione"),
                stato: row.get("stato"),
                tax_id_errors: tax_id_errors(row),
                direction,
                is_signed: row.get::<Option<bool>>("is_signed").unwrap_or(false),
                is_autofattura,
                is_pa,
                is_split_payment,
                has_mc_notice,
                id,
            }
        })
        .collect();

    Ok(invoices)
}

fn inferred_total_for_imported_body(id: &str, xml: &str) -> Option<f64> {
    imported_body_for_id(id, xml)
        .as_ref()
        .and_then(common::invoice_total::inferred_gross_total)
}

fn imported_body_for_id(id: &str, xml: &str) -> Option<common::sdi::v1::FatturaElettronicaBody> {
    let invoice = common::xml::decode::decode(xml).ok()?;
    let header = invoice.header.as_ref()?;
    invoice.body.into_iter().find_map(|body| {
        let body_id = digest::content_id(
            &serde_json::json!({"kind":"imported_invoice","header":header,"body":&body}),
        )
        .ok()?;
        (body_id == id).then_some(body)
    })
}

/// Read the owner company's fiscal identity (id_paese, id_codice) from settings.
pub(crate) async fn owner_identity(store: &crate::storage::FileStore) -> Option<(String, String)> {
    let row = store
        .read("settings", "settings", vec![], "")
        .await
        .fetch_optional()
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
    fetch_invoice_detail(&state.store, &id).await
}

/// Create a new invoice with its lines and payments in a single transaction.
///
/// Line totals include quantity and line discounts or surcharges.
/// The overall invoice total is the sum of all line totals.
#[tauri::command]
pub async fn create_invoice(
    state: tauri::State<'_, TauriState>,
    invoice: CreateInvoiceRequest,
) -> Result<InvoiceDetail, String> {
    create_invoice_store(&state.store, invoice).await
}

/// Compute total invoice gross amount and net amount to pay, taking into account
/// line items, pension funds (casse previdenziali), VAT, virtual stamp duty, and withholding taxes.
fn compute_invoice_financials(
    linee: &[CreateLineRequest],
    casse: Option<&[DatiCassaDetail]>,
    bollo: Option<&DatiBolloDetail>,
    ritenute: Option<&[DatiRitenutaDetail]>,
    is_split_payment: bool,
) -> (f64, f64) {
    let mut imponibile_linee: f64 = 0.0;
    let mut iva_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

    for line in linee {
        let pt = line.total();
        imponibile_linee += pt;
        if line.aliquota_iva > 0.0 {
            let k = format!("{:.2}", line.aliquota_iva);
            *iva_map.entry(k).or_insert(0.0) += pt;
        }
    }

    let mut tot_cassa = 0.0;
    let mut cassa_iva = 0.0;
    if let Some(casse) = casse {
        for c in casse {
            tot_cassa += c.importo_contributo_cassa;
            if c.aliquota_iva > 0.0 {
                let imp =
                    ((c.aliquota_iva * c.importo_contributo_cassa / 100.0) * 100.0).round() / 100.0;
                cassa_iva += imp;
            }
        }
    }

    let mut tot_iva = 0.0;
    for (k, imp) in &iva_map {
        let al: f64 = k.parse().unwrap_or(0.0);
        tot_iva += ((al * imp / 100.0) * 100.0).round() / 100.0;
    }
    tot_iva += cassa_iva;

    // Virtual stamp duty (2.00)
    let mut bollo_amt = 0.0;
    let has_bollo_line = linee
        .iter()
        .any(|l| l.descrizione.to_lowercase().contains("bollo"));
    if let Some(b) = bollo
        && b.bollo_virtuale == "SI"
        && !has_bollo_line
    {
        bollo_amt = b.importo_bollo.unwrap_or(2.00);
    }

    let importo_totale =
        ((imponibile_linee + tot_cassa + tot_iva + bollo_amt) * 100.0).round() / 100.0;

    let mut tot_ritenuta = 0.0;
    if let Some(ritenute) = ritenute {
        for r in ritenute {
            tot_ritenuta += r.importo_ritenuta;
        }
    }

    // In Split Payment, VAT is remitted directly to the State by PA, so it is deducted from Netto a pagare
    let split_deduction = if is_split_payment { tot_iva } else { 0.0 };
    let netto_a_pagare =
        ((importo_totale - tot_ritenuta - split_deduction).max(0.0) * 100.0).round() / 100.0;
    (importo_totale, netto_a_pagare)
}

pub async fn create_invoice_store(
    store: &crate::storage::FileStore,
    invoice: CreateInvoiceRequest,
) -> Result<InvoiceDetail, String> {
    check_draft_lines(&invoice.linee)?;
    let invoice_id =
        digest::content_id(&serde_json::json!({"kind":"created_invoice","document":&invoice}))?;

    let mut tx = store.begin().await.map_err(|e| e.to_string())?;

    let is_split_payment = invoice.is_split_payment.unwrap_or(false)
        || invoice.esigibilita_iva.as_deref() == Some("S");
    let is_pa = invoice.is_pa.unwrap_or(false);

    let (importo_totale, netto_a_pagare) = compute_invoice_financials(
        &invoice.linee,
        invoice.dati_cassa_previdenziale.as_deref(),
        invoice.dati_bollo.as_ref(),
        invoice.dati_ritenuta.as_deref(),
        is_split_payment,
    );

    let is_autofattura = matches!(
        invoice.tipo_documento.as_str(),
        "TD16" | "TD17" | "TD18" | "TD19" | "TD20" | "TD21" | "TD27"
    );
    let soggetto_emittente = if is_autofattura {
        invoice
            .soggetto_emittente
            .clone()
            .or_else(|| Some("CC".to_string()))
    } else {
        invoice.soggetto_emittente.clone()
    };
    let esigibilita_iva = if is_split_payment {
        Some("S".to_string())
    } else {
        invoice.esigibilita_iva.clone()
    };

    // Insert invoice header.
    let processed_ddt = invoice.dati_ddt.clone().unwrap_or_default();
    let processed_allegati: Vec<AttachmentDetail> = invoice
        .allegati
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|a| {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(&a.attachment_base64)
                .unwrap_or_default();
            let size_bytes = decoded.len();
            let formato = a.formato_attachment.or_else(|| {
                std::path::Path::new(&a.nome_attachment)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s.to_uppercase())
            });
            AttachmentDetail {
                nome_attachment: a.nome_attachment,
                algoritmo_compressione: a.algoritmo_compressione,
                formato_attachment: formato,
                descrizione_attachment: a.descrizione_attachment,
                size_bytes,
                attachment_base64: a.attachment_base64,
            }
        })
        .collect();

    tx.insert(
        "invoices",
        serde_json::json!({
            "id": &invoice_id,
            "numero": &invoice.numero,
            "data": &invoice.data,
            "tipo_documento": &invoice.tipo_documento,
            "divisa": &invoice.divisa,
            "importo_totale": importo_totale,
            "netto_a_pagare": netto_a_pagare,
            "stato": "draft",
            "cedente_id": &invoice.cedente_id,
            "cessionario_id": &invoice.cessionario_id,
            "causale": &invoice.causale,
            "dati_ritenuta": &invoice.dati_ritenuta,
            "dati_bollo": &invoice.dati_bollo,
            "dati_cassa_previdenziale": &invoice.dati_cassa_previdenziale,
            "dati_fatture_collegate": &invoice.dati_fatture_collegate,
            "soggetto_emittente": &soggetto_emittente,
            "is_autofattura": is_autofattura,
            "is_pa": is_pa,
            "is_split_payment": is_split_payment,
            "esigibilita_iva": &esigibilita_iva,
            "dati_ordine_acquisto": &invoice.dati_ordine_acquisto,
            "dati_contratto": &invoice.dati_contratto,
            "dati_convenzione": &invoice.dati_convenzione,
            "dati_ddt": &processed_ddt,
            "allegati": &processed_allegati,
        }),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Insert line items.
    insert_draft_lines(&mut tx, &invoice_id, &invoice.linee).await?;

    // Insert payment entries.
    for payment in &invoice.pagamenti {
        let payment_id = uuid::Uuid::new_v4().to_string();

        tx.insert("invoice_payments", serde_json::json!({"id": &payment_id, "invoice_id": &invoice_id, "condizioni_pagamento": &payment.condizioni_pagamento, "modalita_pagamento": &payment.modalita_pagamento, "importo_pagamento": payment.importo_pagamento, "data_scadenza_pagamento": &payment.data_scadenza_pagamento, "iban": &payment.iban})).await
        .map_err(|e| e.to_string())?;
    }

    // Update invoice total.
    tx.update(
        "invoices",
        &invoice_id,
        serde_json::json!({"importo_totale": importo_totale}),
    )
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    fetch_invoice_detail(store, &invoice_id).await
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
    expected_revision: Option<String>,
) -> Result<InvoiceDetail, String> {
    update_invoice_store(
        &state.store,
        &id,
        invoice,
        expected_revision.as_deref(),
        false,
    )
    .await
}

fn effective_issuing_subject(invoice: &CreateInvoiceRequest) -> Option<String> {
    invoice
        .soggetto_emittente
        .clone()
        .or_else(|| is_autofattura_type(&invoice.tipo_documento).then(|| "CC".to_owned()))
}

fn is_autofattura_type(tipo_documento: &str) -> bool {
    matches!(
        tipo_documento,
        "TD16" | "TD17" | "TD18" | "TD19" | "TD20" | "TD21" | "TD27"
    )
}

async fn editable_draft(
    tx: &crate::storage::Transaction,
    id: &str,
    expected_revision: Option<&str>,
) -> Result<crate::storage::Record, String> {
    let row = tx
        .read("invoices", "invoice_views", vec![serde_json::json!(id)], "")
        .fetch_optional()
        .await?
        .ok_or("Draft not found. Reload the invoice list before editing.")?;
    let stato: String = row.get("stato");
    if stato != "draft" {
        return Err(format!(
            "Cannot edit invoice {id}: only draft invoices are editable (current state: {stato})"
        ));
    }
    if crate::commands::pec::recoverable_pec_filename(&tx.snapshot(), &row, id)
        .await?
        .is_some()
    {
        return Err("Verifica in PEC il reinvio già registrato prima di modificare la fattura.".into());
    }
    if let Some(expected) = expected_revision
        && invoice_revision(&row).as_deref() != Some(expected)
    {
        return Err("Draft changed. Reload the invoice before editing.".into());
    }
    Ok(row)
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DraftLinePatch {
    #[schemars(range(min = 1, max = 9999))]
    numero_linea: i32,
    /// SdI line description. Use Basic Latin or Latin-1 Supplement.
    #[schemars(length(min = 1, max = 1000))]
    descrizione: Option<String>,
    /// Omit to preserve the start date. Use null to clear it.
    #[serde(default, deserialize_with = "present_value")]
    #[schemars(with = "Option<String>")]
    data_inizio_periodo: Option<Option<String>>,
    /// Omit to preserve the end date. Use null to clear it.
    #[serde(default, deserialize_with = "present_value")]
    #[schemars(with = "Option<String>")]
    data_fine_periodo: Option<Option<String>>,
}

fn present_value<'de, D: serde::Deserializer<'de>>(
    value: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(value).map(Some)
}

pub(crate) async fn update_draft_lines(
    store: &crate::storage::FileStore,
    id: &str,
    expected_revision: &str,
    patches: Vec<DraftLinePatch>,
) -> Result<InvoiceDetail, String> {
    if !(1..=9999).contains(&patches.len()) {
        return Err("Supply between 1 and 9999 line updates.".into());
    }
    let mut tx = store.begin().await?;
    editable_draft(&tx, id, Some(expected_revision)).await?;
    let snapshot = tx.snapshot();
    let rows = snapshot
        .read(
            "invoice_lines",
            "invoice_children",
            vec![serde_json::json!(id)],
            "line",
        )
        .await
        .fetch_all()
        .await?;
    let mut seen = std::collections::HashSet::new();
    let mut updates = Vec::with_capacity(patches.len());

    for patch in patches {
        let number = patch.numero_linea;
        if !seen.insert(number) {
            return Err(format!("Line {number}: supply only one update per line."));
        }
        let row = rows
            .iter()
            .find(|row| row.get::<i32>("numero_linea") == number)
            .ok_or_else(|| format!("Line {number} does not exist in this draft."))?;
        let mut values = serde_json::Map::new();
        if let Some(description) = patch.descrizione {
            common::validation::rules::validate_line_description(&description)
                .map_err(|error| format!("00200, line {number}: {error}"))?;
            values.insert("descrizione".into(), serde_json::json!(description));
        }
        if patch.data_inizio_periodo.is_some() || patch.data_fine_periodo.is_some() {
            let start = patch
                .data_inizio_periodo
                .unwrap_or_else(|| row.get("data_inizio_periodo"));
            let end = patch
                .data_fine_periodo
                .unwrap_or_else(|| row.get("data_fine_periodo"));
            check_line_period(number as usize, start.as_deref(), end.as_deref())?;
            values.insert("data_inizio_periodo".into(), serde_json::json!(start));
            values.insert("data_fine_periodo".into(), serde_json::json!(end));
        }
        if values.is_empty() {
            return Err(format!(
                "Line {number}: supply a description or period date."
            ));
        }
        updates.push((row.get::<String>("id"), serde_json::Value::Object(values)));
    }

    // Updating the header hydrates the document and marks it for persistence.
    tx.update(
        "invoices",
        id,
        serde_json::json!({"xml_content": null, "imported_from_xml": false}),
    )
    .await?;
    for (line_id, values) in updates {
        tx.update("invoice_lines", &line_id, values).await?;
    }
    tx.commit().await?;
    fetch_invoice_detail(store, id).await
}

pub async fn update_invoice_store(
    store: &crate::storage::FileStore,
    id: &str,
    mut invoice: CreateInvoiceRequest,
    expected_revision: Option<&str>,
    via_mcp: bool,
) -> Result<InvoiceDetail, String> {
    let mut tx = store.begin().await.map_err(|e| e.to_string())?;

    let row = editable_draft(&tx, id, expected_revision).await?;
    if via_mcp
        && (row.get::<String>("cedente_id") != invoice.cedente_id
            || row.get::<String>("cessionario_id") != invoice.cessionario_id
            || is_autofattura_type(&row.get::<String>("tipo_documento"))
                != is_autofattura_type(&invoice.tipo_documento)
            || row.get::<Option<String>>("soggetto_emittente")
                != effective_issuing_subject(&invoice))
    {
        return Err(
            "MCP cannot change the issuer, customer or issuing subject of an existing draft."
                .into(),
        );
    }
    if via_mcp && invoice.allegati.is_none() {
        invoice.allegati =
            row.get::<Option<Vec<AttachmentDetail>>>("allegati")
                .map(|attachments| {
                    attachments
                        .into_iter()
                        .map(|attachment| CreateAttachmentRequest {
                            nome_attachment: attachment.nome_attachment,
                            algoritmo_compressione: attachment.algoritmo_compressione,
                            formato_attachment: attachment.formato_attachment,
                            descrizione_attachment: attachment.descrizione_attachment,
                            attachment_base64: attachment.attachment_base64,
                        })
                        .collect()
                });
    }

    check_draft_lines(&invoice.linee)?;
    let new_id =
        digest::content_id(&serde_json::json!({"kind":"created_invoice","document":&invoice}))?;
    tx.rekey_invoice(id, &new_id).await?;
    let id = new_id;

    let is_split_payment = invoice.is_split_payment.unwrap_or(false)
        || invoice.esigibilita_iva.as_deref() == Some("S");
    let is_pa = invoice.is_pa.unwrap_or(false);

    let (importo_totale, netto_a_pagare) = compute_invoice_financials(
        &invoice.linee,
        invoice.dati_cassa_previdenziale.as_deref(),
        invoice.dati_bollo.as_ref(),
        invoice.dati_ritenuta.as_deref(),
        is_split_payment,
    );

    let is_autofattura = is_autofattura_type(&invoice.tipo_documento);
    let soggetto_emittente = effective_issuing_subject(&invoice);
    let esigibilita_iva = if is_split_payment {
        Some("S".to_string())
    } else {
        invoice.esigibilita_iva.clone()
    };

    let processed_ddt = invoice.dati_ddt.clone().unwrap_or_default();
    let processed_allegati: Vec<AttachmentDetail> = invoice
        .allegati
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|a| {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(&a.attachment_base64)
                .unwrap_or_default();
            let size_bytes = decoded.len();
            let formato = a.formato_attachment.or_else(|| {
                std::path::Path::new(&a.nome_attachment)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s.to_uppercase())
            });
            AttachmentDetail {
                nome_attachment: a.nome_attachment,
                algoritmo_compressione: a.algoritmo_compressione,
                formato_attachment: formato,
                descrizione_attachment: a.descrizione_attachment,
                size_bytes,
                attachment_base64: a.attachment_base64,
            }
        })
        .collect();

    // Update the invoice header.
    tx.update(
        "invoices",
        &id,
        serde_json::json!({
            "numero": &invoice.numero,
            "data": &invoice.data,
            "tipo_documento": &invoice.tipo_documento,
            "divisa": &invoice.divisa,
            "cedente_id": &invoice.cedente_id,
            "cessionario_id": &invoice.cessionario_id,
            "causale": &invoice.causale,
            "dati_ritenuta": &invoice.dati_ritenuta,
            "dati_bollo": &invoice.dati_bollo,
            "dati_cassa_previdenziale": &invoice.dati_cassa_previdenziale,
            "dati_fatture_collegate": &invoice.dati_fatture_collegate,
            "soggetto_emittente": &soggetto_emittente,
            "is_autofattura": is_autofattura,
            "importo_totale": importo_totale,
            "netto_a_pagare": netto_a_pagare,
            "is_pa": is_pa,
            "is_split_payment": is_split_payment,
            "esigibilita_iva": &esigibilita_iva,
            "dati_ordine_acquisto": &invoice.dati_ordine_acquisto,
            "dati_contratto": &invoice.dati_contratto,
            "dati_convenzione": &invoice.dati_convenzione,
            "dati_ddt": &processed_ddt,
            "allegati": &processed_allegati,
            "imported_from_xml": false,
            "xml_content": null,
        }),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Replace lines and payments.
    tx.remove("invoice_lines", "invoice_id", &id)
        .await
        .map_err(|e| e.to_string())?;
    tx.remove("invoice_payments", "invoice_id", &id)
        .await
        .map_err(|e| e.to_string())?;

    insert_draft_lines(&mut tx, &id, &invoice.linee).await?;

    for payment in &invoice.pagamenti {
        let payment_id = uuid::Uuid::new_v4().to_string();

        tx.insert("invoice_payments", serde_json::json!({"id": &payment_id, "invoice_id": &id, "condizioni_pagamento": &payment.condizioni_pagamento, "modalita_pagamento": &payment.modalita_pagamento, "importo_pagamento": payment.importo_pagamento, "data_scadenza_pagamento": &payment.data_scadenza_pagamento, "iban": &payment.iban})).await
        .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    fetch_invoice_detail(store, &id).await
}

/// Delete a draft invoice and its lines/payments.
///
/// Only invoices in the `draft` state can be deleted. The `invoice_lines` and
/// `invoice_payments` rows are removed automatically via the file store cascade.
#[tauri::command]
pub async fn delete_invoice(state: tauri::State<'_, TauriState>, id: String) -> Result<(), String> {
    delete_invoice_store(&state.store, &id).await
}

pub async fn delete_invoice_store(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<(), String> {
    let mut tx = store.begin().await?;
    let row = tx
        .read("invoices", "id", vec![serde_json::json!(id)], "")
        .fetch_optional()
        .await?
        .ok_or_else(|| format!("Invoice not found: {id}"))?;
    let status: String = row.get("stato");
    if status != "draft" {
        return Err(format!(
            "Cannot delete invoice {id}: only draft invoices are deletable (current state: {status})"
        ));
    }
    if crate::commands::pec::recoverable_pec_filename(&tx.snapshot(), &row, id)
        .await?
        .is_some()
    {
        return Err("Verifica in PEC il reinvio già registrato prima di eliminare la fattura.".into());
    }

    tx.remove("invoices", "id", id)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await?;
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
    next_invoice_number_store(&state.store, &cedente_id, &year).await
}

pub async fn next_invoice_number_store(
    store: &crate::storage::FileStore,
    cedente_id: &str,
    year: &str,
) -> Result<String, String> {
    let row = store
        .read(
            "invoices",
            "number_count",
            vec![serde_json::json!(cedente_id), serde_json::json!(year)],
            "",
        )
        .await
        .fetch_one()
        .await
        .map_err(|e| e.to_string())?;

    let count: i64 = row.get("cnt");
    let rows = store
        .read("invoices", "invoice_views", vec![], "")
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let max_existing = rows
        .iter()
        .filter(|r| {
            r.get::<String>("cedente_id") == cedente_id
                && r.get::<String>("data").starts_with(year)
                && !r.get::<Option<bool>>("imported_from_xml").unwrap_or(false)
        })
        .filter_map(|r| {
            let num = r.get::<String>("numero");
            let trimmed = num.trim();
            trimmed.parse::<i64>().ok().or_else(|| {
                trimmed
                    .split(['/', '-'])
                    .next()
                    .and_then(|part| part.trim().parse::<i64>().ok())
            })
        })
        .max()
        .unwrap_or(0);

    Ok((count + 1).max(max_existing + 1).to_string())
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
    set_invoice_status_store(&state.store, &id, &status).await
}

pub async fn set_invoice_status_store(
    store: &crate::storage::FileStore,
    id: &str,
    status: &str,
) -> Result<InvoiceDetail, String> {
    use common::invoice_status::InvoiceStatus;

    let mut tx = store.begin().await?;
    let row = tx
        .read("invoices", "id", vec![serde_json::json!(id)], "")
        .fetch_optional()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Invoice not found: {id}"))?;

    if row.get::<Option<String>>("pending_pec_filename").is_some() {
        return Err("Verifica l'esito PEC prima di cambiare lo stato della fattura.".into());
    }
    let current_str: String = row.get("stato");
    let current: InvoiceStatus = current_str.parse()?;
    let next: InvoiceStatus = status.parse()?;
    current.transition_to(next)?;
    if next == InvoiceStatus::Sent {
        return Err("Usa il comando di invio PEC per registrare la trasmissione.".into());
    }
    if current == InvoiceStatus::Rejected && next == InvoiceStatus::Draft {
        if crate::commands::pec::recoverable_pec_filename(&tx.snapshot(), &row, id)
            .await?
            .is_some()
        {
            return Err(
                "Lo scarto è di un invio precedente. Verifica in PEC il reinvio prima di rettificare."
                    .into(),
            );
        }
        if crate::commands::pec::has_attempt_pa_refusal(&tx.snapshot(), &row, id).await? {
            return Err(
                "La fattura è stata rifiutata dalla PA (NE EC02) dopo la consegna SdI e non può essere riportata in bozza come scarto."
                    .into(),
            );
        }
    }

    // Block promotion to `validated` if the invoice does not pass SDI validation.
    if next == InvoiceStatus::Validated {
        if crate::commands::pec::recoverable_pec_filename(&tx.snapshot(), &row, id)
            .await?
            .is_some()
        {
            return Err("Verifica in PEC il reinvio già registrato prima di procedere.".into());
        }
        let fattura = build_fattura_from_store(&tx.snapshot(), id).await?;
        let result = common::validation::rules::validate(&fattura);
        if !result.is_valid() {
            let count = result.errors.len();
            return Err(format!(
                "Cannot validate invoice: {count} SDI validation error(s) must be resolved first"
            ));
        }
    }

    tx.update("invoices", id, serde_json::json!({"stato": next.as_str()}))
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await?;
    fetch_invoice_detail(store, id).await
}

/// Validate an invoice against SDI rules.
///
/// Loads the invoice from the file store, converts it to a `FatturaElettronica` proto,
/// and runs the validation rules from the common crate.
#[tauri::command]
pub async fn validate_invoice(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<ValidationResultDto, String> {
    validate_invoice_store(&state.store, &id).await
}

pub async fn validate_invoice_store(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<ValidationResultDto, String> {
    let fattura = build_fattura_from_store(store, id).await?;
    let result = common::validation::rules::validate(&fattura);

    Ok(ValidationResultDto {
        valid: result.is_valid(),
        errors: result
            .errors
            .into_iter()
            .map(ValidationErrorDto::from)
            .collect(),
    })
}

/// Export an invoice as SDI-compliant XML.
///
/// Loads the invoice from the file store, converts it to a `FatturaElettronica` proto,
/// and encodes it as XML. The frontend can then save the result to a file.
#[tauri::command]
pub async fn export_invoice_xml(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<String, String> {
    export_invoice_xml_store(&state.store, &id).await
}

pub async fn export_invoice_xml_store(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<String, String> {
    crate::commands::clients::backfill_client_destinations(store).await?;
    let mut tx = store.begin().await?;
    let snapshot = tx.snapshot();
    let row = snapshot
        .read("invoices", "id", vec![serde_json::json!(id)], "")
        .await
        .fetch_one()
        .await?;
    if crate::commands::pec::recoverable_pec_filename(&snapshot, &row, id)
        .await?
        .is_some()
    {
        return Err("Verifica in PEC il reinvio già registrato prima di esportare la fattura.".into());
    }
    let fattura = build_fattura_from_store(&snapshot, id).await?;
    if matches!(row.get::<String>("stato").as_str(), "draft" | "validated") {
        common::validation::rules::validate_transmitter(
            fattura.header.as_ref().and_then(|header| header.dati_trasmissione.as_ref()),
        )
        .map_err(|error| error.to_string())?;
    }
    let xml = common::xml::encode::encode(&fattura).map_err(|e| e.to_string())?;

    // Persist the generated XML as an audit trail.
    tx.update("invoices", id, serde_json::json!({"xml_content": &xml})).await?;
    tx.commit().await?;

    Ok(xml)
}

/// Generate a PDF courtesy copy directly from the invoice record in the file store.
#[tauri::command]
pub async fn render_invoice_pdf(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<Vec<u8>, String> {
    render_invoice_pdf_store(&state.store, &id).await
}

pub async fn render_invoice_pdf_store(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<Vec<u8>, String> {
    let fattura = build_fattura_from_store(store, id).await?;
    common::pdf::generate_courtesy_pdf(&fattura).map_err(|e| e.to_string())
}

/// Import an invoice from SDI XML or .p7m content.
///
/// Decodes the XML into a `FatturaElettronica`, extracts the relevant fields,
/// and persists the invoice (with lines and payments) to the local file store.
#[tauri::command]
pub async fn import_invoice_xml(
    state: tauri::State<'_, TauriState>,
    xml_content: String,
) -> Result<InvoiceDetail, String> {
    let bytes = xml_content.as_bytes();
    let (payload, signer_info) = if common::p7m::is_p7m(bytes) {
        let (xml, info) = common::p7m::inspect_p7m(bytes).map_err(|e| e.to_string())?;
        (xml, if info.is_signed { Some(info) } else { None })
    } else if let Ok(decoded) =
        base64::engine::general_purpose::STANDARD.decode(xml_content.trim().as_bytes())
    {
        if common::p7m::is_p7m(&decoded) {
            let (xml, info) = common::p7m::inspect_p7m(&decoded).map_err(|e| e.to_string())?;
            (xml, if info.is_signed { Some(info) } else { None })
        } else {
            (bytes.to_vec(), None)
        }
    } else {
        (bytes.to_vec(), None)
    };

    let xml_str = std::str::from_utf8(&payload).map_err(|e| format!("Invalid UTF-8 XML: {e}"))?;

    // Check if the payload is an SDI notification rather than an invoice
    if let Ok(notif) = common::pec::parse_notification(xml_str)
        && !matches!(notif.message_type, common::pec::SdiMessageType::Unknown(_))
    {
        let (linked_id, _) =
            crate::commands::pec::attach_sdi_notification(&state.store, &notif, xml_str, None)
                .await?;
        if let Some(id) = linked_id {
            return fetch_invoice_detail(&state.store, &id).await;
        }
    }

    import_xml_content_with_signer(&state.store, xml_str, signer_info).await
}

#[cfg(test)]
pub(crate) async fn import_xml_content(
    store: &crate::storage::FileStore,
    xml_content: &str,
) -> Result<InvoiceDetail, String> {
    import_xml_content_with_signer(store, xml_content, None).await
}

pub(crate) async fn import_xml_content_with_signer(
    store: &crate::storage::FileStore,
    xml_content: &str,
    signer_info: Option<common::p7m::SignerInfo>,
) -> Result<InvoiceDetail, String> {
    import_xml_content_inner(store, xml_content, signer_info, true).await
}

/// PEC batches reconcile receipts once after every invoice has been imported.
pub(crate) async fn import_xml_content_for_pec(
    store: &crate::storage::FileStore,
    xml_content: &str,
    signer_info: Option<common::p7m::SignerInfo>,
) -> Result<InvoiceDetail, String> {
    import_xml_content_inner(store, xml_content, signer_info, false).await
}

async fn import_xml_content_inner(
    store: &crate::storage::FileStore,
    xml_content: &str,
    signer_info: Option<common::p7m::SignerInfo>,
    reconcile_receipts: bool,
) -> Result<InvoiceDetail, String> {
    let fattura = common::xml::decode::decode(xml_content).map_err(|e| e.to_string())?;

    // Extract cedente and cessionario info to find or create clients.
    let header = fattura.header.as_ref().ok_or("Missing invoice header")?;

    let cedente_id = ensure_client_from_header_cedente(store, header).await?;
    let cessionario_id = ensure_client_from_header_cessionario(store, header).await?;
    let is_signed = signer_info.as_ref().map(|s| s.is_signed).unwrap_or(false);

    // A FatturaElettronica can carry multiple bodies (lotto); import each as its
    // own invoice and return the first one's detail.
    let mut imported_ids: Vec<String> = Vec::new();
    for body in &fattura.body {
        let dg = body.dati_generali.as_ref().ok_or("Missing DatiGenerali")?;
        let dgd = dg
            .dati_generali_documento
            .as_ref()
            .ok_or("Missing DatiGeneraliDocumento")?;

        let invoice_id = digest::content_id(
            &serde_json::json!({"kind":"imported_invoice","header":header,"body":body}),
        )?;
        let tipo_doc_str = common::xml::enum_map::tipo_documento_to_sdi(dgd.tipo_documento)
            .unwrap_or("TD01")
            .to_string();
        let imported_soggetto_emittente = header
            .soggetto_emittente
            .and_then(common::xml::enum_map::soggetto_emittente_to_sdi)
            .map(String::from);
        let art73 = dgd.art73 == Some(10);
        let fiscal_key = serde_json::json!({
            "cedente_id": &cedente_id, "cessionario_id": &cessionario_id,
            "soggetto_emittente": &imported_soggetto_emittente,
            "numero": &dgd.numero, "data": &dgd.data,
            "tipo_documento": &tipo_doc_str, "art73": art73
        });

        // The transport header can differ between copies of the same invoice.
        // Reuse the fiscal document instead of creating a second record.
        let existing_invoice = store
            .read("invoices", "invoice_views", vec![], "")
            .await
            .fetch_all()
            .await?
            .into_iter()
            .find(|row| {
                !matches!(row.get::<String>("stato").as_str(), "draft" | "rejected")
                    && crate::storage::same_invoice_key(row, &fiscal_key)
            });
        if let Some(existing) = existing_invoice {
            let id: String = existing.get("id");
            let filename = common::pec::sdi_filename(&fattura);
            let mut filenames = existing
                .get::<Option<Vec<String>>>("sdi_filenames")
                .unwrap_or_default();
            if !filenames.contains(&filename) {
                filenames.push(filename);
                store
                    .update(
                        "invoices",
                        &id,
                        serde_json::json!({"sdi_filenames": filenames}),
                    )
                    .await?;
            }
            imported_ids.push(id);
            continue;
        }

        let mut tx = store.begin().await.map_err(|e| e.to_string())?;
        if tx
            .read("invoices", "id", vec![serde_json::json!(&invoice_id)], "")
            .fetch_optional()
            .await?
            .is_some()
        {
            imported_ids.push(invoice_id);
            continue;
        }

        let imported_causale = dgd.causale.clone();
        let imported_ritenute: Vec<DatiRitenutaDetail> = dgd
            .dati_ritenuta
            .iter()
            .map(|r| {
                let tr_str = common::xml::enum_map::tipo_ritenuta_to_sdi(r.tipo_ritenuta)
                    .unwrap_or("RT01")
                    .to_string();
                let cp_str = common::xml::enum_map::causale_pagamento_to_sdi(r.causale_pagamento)
                    .unwrap_or("A")
                    .to_string();
                DatiRitenutaDetail {
                    tipo_ritenuta: tr_str,
                    importo_ritenuta: r.importo_ritenuta,
                    aliquota_ritenuta: r.aliquota_ritenuta,
                    causale_pagamento: cp_str,
                }
            })
            .collect();

        let imported_bollo: Option<DatiBolloDetail> = dgd.dati_bollo.as_ref().map(|b| {
            let bv_str = common::xml::enum_map::bollo_virtuale_to_sdi(b.bollo_virtuale)
                .unwrap_or("SI")
                .to_string();
            DatiBolloDetail {
                bollo_virtuale: bv_str,
                importo_bollo: b.importo_bollo,
            }
        });

        let imported_casse: Vec<DatiCassaDetail> = dgd
            .dati_cassa_previdenziale
            .iter()
            .map(|c| {
                let tc_str = common::xml::enum_map::tipo_cassa_to_sdi(c.tipo_cassa)
                    .unwrap_or("TC01")
                    .to_string();
                let rit_str = c
                    .ritenuta
                    .and_then(common::xml::enum_map::ritenuta_to_sdi)
                    .map(String::from);
                let nat_str = c
                    .natura
                    .and_then(common::xml::enum_map::natura_to_sdi)
                    .map(String::from);
                DatiCassaDetail {
                    tipo_cassa: tc_str,
                    al_cassa: c.al_cassa,
                    importo_contributo_cassa: c.importo_contributo_cassa,
                    imponibile_cassa: c.imponibile_cassa,
                    aliquota_iva: c.aliquota_iva,
                    ritenuta: rit_str,
                    natura: nat_str,
                }
            })
            .collect();

        let mut tot_rit = 0.0;
        for r in &imported_ritenute {
            tot_rit += r.importo_ritenuta;
        }

        let mut tot_split_iva = 0.0;
        let mut has_split_payment = false;
        let mut extracted_esigibilita = None;
        if let Some(dbs) = &body.dati_beni_servizi {
            for riep in &dbs.dati_riepilogo {
                if riep.esigibilita_iva == Some(30) {
                    has_split_payment = true;
                    tot_split_iva += riep.imposta;
                    extracted_esigibilita = Some("S".to_string());
                } else if extracted_esigibilita.is_none() {
                    extracted_esigibilita = riep
                        .esigibilita_iva
                        .and_then(common::xml::enum_map::esigibilita_iva_to_sdi)
                        .map(String::from);
                }
            }
        }

        let is_pa_invoice = header
            .dati_trasmissione
            .as_ref()
            .map(|dt| dt.formato_trasmissione == 10)
            .unwrap_or(false);

        let inferred_total = if dgd.importo_totale_documento.is_none() {
            common::invoice_total::inferred_gross_total(body)
        } else {
            None
        };
        let imported_netto = dgd
            .importo_totale_documento
            .or(inferred_total)
            .map(|t| (((t - tot_rit - tot_split_iva).max(0.0)) * 100.0).round() / 100.0);

        let imported_collegate: Vec<DatiFatturaCollegataDetail> = dg
            .dati_fatture_collegate
            .iter()
            .map(|fc| DatiFatturaCollegataDetail {
                id_documento: fc.id_documento.clone(),
                data: fc.data.clone(),
                num_item: fc.num_item.clone(),
                codice_cig: fc.codice_cig.clone(),
                codice_cup: fc.codice_cup.clone(),
            })
            .collect();

        let imported_ordini: Vec<DatiDocumentoTracciabilitaDetail> = dg
            .dati_ordine_acquisto
            .iter()
            .map(|ord| DatiDocumentoTracciabilitaDetail {
                tipo_documento_rif: Some("ordine".into()),
                id_documento: ord.id_documento.clone(),
                data: ord.data.clone(),
                num_item: ord.num_item.clone(),
                codice_commessa_convenzione: ord.codice_commessa_convenzione.clone(),
                codice_cup: ord.codice_cup.clone(),
                codice_cig: ord.codice_cig.clone(),
            })
            .collect();

        let imported_contratti: Vec<DatiDocumentoTracciabilitaDetail> = dg
            .dati_contratto
            .iter()
            .map(|con| DatiDocumentoTracciabilitaDetail {
                tipo_documento_rif: Some("contratto".into()),
                id_documento: con.id_documento.clone(),
                data: con.data.clone(),
                num_item: con.num_item.clone(),
                codice_commessa_convenzione: con.codice_commessa_convenzione.clone(),
                codice_cup: con.codice_cup.clone(),
                codice_cig: con.codice_cig.clone(),
            })
            .collect();

        let imported_convenzioni: Vec<DatiDocumentoTracciabilitaDetail> = dg
            .dati_convenzione
            .iter()
            .map(|conv| DatiDocumentoTracciabilitaDetail {
                tipo_documento_rif: Some("convenzione".into()),
                id_documento: conv.id_documento.clone(),
                data: conv.data.clone(),
                num_item: conv.num_item.clone(),
                codice_commessa_convenzione: conv.codice_commessa_convenzione.clone(),
                codice_cup: conv.codice_cup.clone(),
                codice_cig: conv.codice_cig.clone(),
            })
            .collect();

        let imported_ddt: Vec<DatiDdtDetail> = dg
            .dati_ddt
            .iter()
            .map(|d| DatiDdtDetail {
                numero_ddt: d.numero_ddt.clone(),
                data_ddt: d.data_ddt.clone(),
                riferimento_numero_linea: d.riferimento_numero_linea.clone(),
            })
            .collect();

        let imported_allegati: Vec<AttachmentDetail> = body
            .allegati
            .iter()
            .map(|a| {
                let base64_str = base64::engine::general_purpose::STANDARD.encode(&a.attachment);
                let size_bytes = a.attachment.len();
                AttachmentDetail {
                    nome_attachment: a.nome_attachment.clone(),
                    algoritmo_compressione: a.algoritmo_compressione.clone(),
                    formato_attachment: a.formato_attachment.clone().or_else(|| {
                        std::path::Path::new(&a.nome_attachment)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|s| s.to_uppercase())
                    }),
                    descrizione_attachment: a.descrizione_attachment.clone(),
                    size_bytes,
                    attachment_base64: base64_str,
                }
            })
            .collect();

        let is_autofattura = matches!(
            tipo_doc_str.as_str(),
            "TD16" | "TD17" | "TD18" | "TD19" | "TD20" | "TD21" | "TD27"
        );

        // Insert invoice header.
        tx.insert(
            "invoices",
            serde_json::json!({
                "id": &invoice_id,
                "numero": &dgd.numero,
                "data": &dgd.data,
                "tipo_documento": &tipo_doc_str,
                "divisa": &dgd.divisa,
                "importo_totale": dgd.importo_totale_documento,
                "importo_totale_calcolato": inferred_total,
                "netto_a_pagare": imported_netto,
                "stato": "imported",
                "imported_from_xml": true,
                "cedente_id": &cedente_id,
                "cessionario_id": &cessionario_id,
                "xml_content": xml_content,
                "is_signed": is_signed,
                "signer_info": &signer_info,
                "causale": &imported_causale,
                "dati_ritenuta": &imported_ritenute,
                "dati_bollo": &imported_bollo,
                "dati_cassa_previdenziale": &imported_casse,
                "dati_fatture_collegate": &imported_collegate,
                "soggetto_emittente": &imported_soggetto_emittente,
                "art73": art73,
                "sdi_filenames": [common::pec::sdi_filename(&fattura)],
                "is_autofattura": is_autofattura,
                "is_pa": is_pa_invoice,
                "is_split_payment": has_split_payment,
                "esigibilita_iva": &extracted_esigibilita,
                "dati_ordine_acquisto": &imported_ordini,
                "dati_contratto": &imported_contratti,
                "dati_convenzione": &imported_convenzioni,
                "dati_ddt": &imported_ddt,
                "allegati": &imported_allegati,
            }),
        )
        .await
        .map_err(|e| e.to_string())?;

        // Insert line items.
        if let Some(dbs) = &body.dati_beni_servizi {
            for linea in &dbs.dettaglio_linee {
                let line_id = uuid::Uuid::new_v4().to_string();
                let natura_str = linea
                    .natura
                    .and_then(|n| common::xml::enum_map::natura_to_sdi(n).map(String::from));

                tx.insert("invoice_lines", serde_json::json!({"id": &line_id, "invoice_id": &invoice_id, "numero_linea": linea.numero_linea, "descrizione": &linea.descrizione, "quantita": linea.quantita, "unita_misura": &linea.unita_misura, "data_inizio_periodo": &linea.data_inizio_periodo, "data_fine_periodo": &linea.data_fine_periodo, "prezzo_unitario": linea.prezzo_unitario, "prezzo_totale": linea.prezzo_totale, "aliquota_iva": linea.aliquota_iva, "natura": &natura_str, "line_fields": LineFields::from_document(linea)})).await
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

                tx.insert("invoice_payments", serde_json::json!({"id": &payment_id, "invoice_id": &invoice_id, "condizioni_pagamento": &condizioni_str, "modalita_pagamento": &modalita_str, "importo_pagamento": det.importo_pagamento, "data_scadenza_pagamento": &det.data_scadenza_pagamento, "iban": &det.iban, "istituto_finanziario": &det.istituto_finanziario})).await
                .map_err(|e| e.to_string())?;
            }
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        imported_ids.push(invoice_id);
    }

    match imported_ids.first() {
        Some(id) => {
            if reconcile_receipts {
                crate::commands::pec::reconcile_unlinked_notifications(store).await?;
            }
            fetch_invoice_detail(store, id).await
        }
        None => Err("No invoice body found in XML".to_string()),
    }
}

/// Result of a recursive folder import.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirImportSummary {
    /// Number of files found during the walk.
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

/// Recursively import every `.xml` and `.p7m` invoice found under `dir_path`.
#[tauri::command]
pub async fn import_invoices_from_dir(
    state: tauri::State<'_, TauriState>,
    dir_path: String,
) -> Result<DirImportSummary, String> {
    let root = std::path::Path::new(&dir_path);
    if !root.is_dir() {
        return Err(format!("Not a directory: {dir_path}"));
    }

    // Collect all .xml and .p7m files (recursive) before touching the DB.
    let mut invoice_files = Vec::new();
    collect_invoice_files(root, &mut invoice_files).map_err(|e| format!("Walk failed: {e}"))?;
    let _backup_batch = state.store.begin_backup_batch();

    let mut summary = DirImportSummary {
        total_files: invoice_files.len(),
        imported: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    const MAX_REPORTED_ERRORS: usize = 50;

    for path in invoice_files {
        let display = path.display().to_string();
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                summary.skipped += 1;
                push_error(
                    &mut summary,
                    &display,
                    format!("read error: {e}"),
                    MAX_REPORTED_ERRORS,
                );
                continue;
            }
        };

        let (payload, signer_info) = match common::p7m::inspect_p7m(&bytes) {
            Ok((xml, info)) => (xml, if info.is_signed { Some(info) } else { None }),
            Err(e) => {
                summary.skipped += 1;
                push_error(
                    &mut summary,
                    &display,
                    format!("p7m/xml extraction error: {e}"),
                    MAX_REPORTED_ERRORS,
                );
                continue;
            }
        };

        let content = match std::str::from_utf8(&payload) {
            Ok(c) => c,
            Err(e) => {
                summary.skipped += 1;
                push_error(
                    &mut summary,
                    &display,
                    format!("invalid utf8: {e}"),
                    MAX_REPORTED_ERRORS,
                );
                continue;
            }
        };

        // Check if file is an SDI receipt/notification (RC, MC, NS, NE, DT)
        if let Ok(notif) = common::pec::parse_notification(content)
            && !matches!(notif.message_type, common::pec::SdiMessageType::Unknown(_))
        {
            match crate::commands::pec::attach_sdi_notification(&state.store, &notif, content, None)
                .await
            {
                Ok(_) => {
                    summary.imported += 1;
                    continue;
                }
                Err(e) => {
                    summary.skipped += 1;
                    push_error(
                        &mut summary,
                        &display,
                        format!("SDI receipt link error: {e}"),
                        MAX_REPORTED_ERRORS,
                    );
                    continue;
                }
            }
        }

        match import_xml_content_with_signer(&state.store, content, signer_info).await {
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

/// Recursively collect paths of `.xml` and `.p7m` files under `dir`.
fn collect_invoice_files(
    dir: &std::path::Path,
    out: &mut Vec<std::path::PathBuf>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_invoice_files(&path, out)?;
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            if ext.eq_ignore_ascii_case("xml")
                || ext.eq_ignore_ascii_case("p7m")
                || file_name.to_lowercase().ends_with(".xml.p7m")
            {
                out.push(path);
            }
        }
    }
    Ok(())
}

/// Verify a PKCS#7 / CAdES .p7m signature and extract signer certificate metadata.
#[tauri::command]
pub async fn verify_p7m_signature(
    p7m_base64: Option<String>,
    p7m_path: Option<String>,
) -> Result<common::p7m::SignerInfo, String> {
    let bytes = if let Some(path) = p7m_path {
        std::fs::read(&path).map_err(|e| format!("Cannot read file: {e}"))?
    } else if let Some(b64) = p7m_base64 {
        base64::engine::general_purpose::STANDARD
            .decode(b64.trim().as_bytes())
            .map_err(|e| format!("Invalid base64: {e}"))?
    } else {
        return Err("Neither p7m_path nor p7m_base64 provided".to_string());
    };

    let (_, info) = common::p7m::inspect_p7m(&bytes).map_err(|e| e.to_string())?;
    Ok(info)
}

// ===========================================================================
// Internal helpers
// ===========================================================================

/// Fetch full invoice detail from the file store by ID.
pub(crate) async fn fetch_invoice_detail(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<InvoiceDetail, String> {
    let snapshot = store.snapshot().await;
    let store = &snapshot;
    let invoice_row = store
        .read("invoices", "id", vec![serde_json::json!(id)], "")
        .await
        .fetch_optional()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Invoice not found: {id}"))?;

    let line_rows = store
        .read(
            "invoice_lines",
            "invoice_children",
            vec![serde_json::json!(id)],
            "line",
        )
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let payment_rows = store
        .read(
            "invoice_payments",
            "invoice_children",
            vec![serde_json::json!(id)],
            "",
        )
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    // Resolve cedente/cessionario display names for the detail view.
    let names_row = store
        .read("invoices", "invoice_views", vec![serde_json::json!(id)], "")
        .await
        .fetch_one()
        .await
        .map_err(|e| e.to_string())?;

    // Older imports kept the original XML but did not index advanced line fields.
    let original_body = if invoice_row.get::<Option<bool>>("imported_from_xml") == Some(true)
        && line_rows
            .iter()
            .any(|row| row.get::<Option<LineFields>>("line_fields").is_none())
    {
        invoice_row
            .get::<Option<String>>("xml_content")
            .and_then(|xml| imported_body_for_id(id, &xml))
            .and_then(|body| body.dati_beni_servizi)
    } else {
        None
    };
    let linee = line_rows
        .iter()
        .map(|row| {
            let numero_linea: i32 = row.get("numero_linea");
            let original = original_body.as_ref().and_then(|body| {
                body.dettaglio_linee
                    .iter()
                    .find(|line| line.numero_linea == numero_linea)
            });
            InvoiceLineDetail {
                numero_linea,
                descrizione: row.get("descrizione"),
                quantita: row.get("quantita"),
                unita_misura: row.get("unita_misura"),
                data_inizio_periodo: row
                    .get::<Option<String>>("data_inizio_periodo")
                    .or_else(|| original.and_then(|line| line.data_inizio_periodo.clone())),
                data_fine_periodo: row
                    .get::<Option<String>>("data_fine_periodo")
                    .or_else(|| original.and_then(|line| line.data_fine_periodo.clone())),
                prezzo_unitario: row.get("prezzo_unitario"),
                prezzo_totale: row.get("prezzo_totale"),
                aliquota_iva: row.get("aliquota_iva"),
                natura: row.get("natura"),
                fields: row
                    .get::<Option<LineFields>>("line_fields")
                    .or_else(|| original.map(LineFields::from_document))
                    .unwrap_or_default(),
            }
        })
        .collect();

    let pagamenti = payment_rows
        .iter()
        .map(|row| PaymentDetail {
            condizioni_pagamento: row.get("condizioni_pagamento"),
            modalita_pagamento: row.get("modalita_pagamento"),
            importo_pagamento: row.get("importo_pagamento"),
            data_scadenza_pagamento: row.get("data_scadenza_pagamento"),
            iban: row.get("iban"),
        })
        .collect();

    let notif_rows = store
        .read(
            "sdi_notifications",
            "invoice_children",
            vec![serde_json::json!(id)],
            "",
        )
        .await
        .fetch_all()
        .await
        .unwrap_or_default();

    let mut notifications = Vec::new();
    for row in &notif_rows {
        let errs_raw: String = row
            .get::<Option<String>>("errori")
            .unwrap_or_else(|| "[]".into());
        let errori: Vec<common::pec::SdiError> =
            serde_json::from_str(&errs_raw).unwrap_or_default();
        notifications.push(SdiNotificationDetail {
            id: row.get("id"),
            tipo: row.get("tipo"),
            identificativo_sdi: row.get("identificativo_sdi"),
            nome_file: row.get("nome_file"),
            esito: row.get("esito"),
            data_ora_ricezione: row.get("data_ora_ricezione"),
            destinatario: row.get("destinatario"),
            note: row.get("note"),
            errori,
            raw_xml: row.get("raw_xml"),
        });
    }

    let causale: Vec<String> = invoice_row
        .get::<Option<Vec<String>>>("causale")
        .unwrap_or_default();
    let dati_ritenuta: Vec<DatiRitenutaDetail> = invoice_row
        .get::<Option<Vec<DatiRitenutaDetail>>>("dati_ritenuta")
        .unwrap_or_default();
    let dati_bollo: Option<DatiBolloDetail> =
        invoice_row.get::<Option<DatiBolloDetail>>("dati_bollo");
    let dati_cassa_previdenziale: Vec<DatiCassaDetail> = invoice_row
        .get::<Option<Vec<DatiCassaDetail>>>("dati_cassa_previdenziale")
        .unwrap_or_default();
    let netto_a_pagare: Option<f64> = invoice_row.get::<Option<f64>>("netto_a_pagare");
    let dati_fatture_collegate: Vec<DatiFatturaCollegataDetail> = invoice_row
        .get::<Option<Vec<DatiFatturaCollegataDetail>>>("dati_fatture_collegate")
        .unwrap_or_default();
    let soggetto_emittente: Option<String> =
        invoice_row.get::<Option<String>>("soggetto_emittente");
    let tipo_doc_val: String = invoice_row.get("tipo_documento");
    let is_autofattura = invoice_row
        .get::<Option<bool>>("is_autofattura")
        .unwrap_or({
            matches!(
                tipo_doc_val.as_str(),
                "TD16" | "TD17" | "TD18" | "TD19" | "TD20" | "TD21" | "TD27"
            )
        });
    let pending_xml = invoice_row.get::<Option<String>>("pending_pec_xml");
    let stored = if let Some(xml) = pending_xml {
        Some(common::xml::decode::decode(&xml).map_err(|error| error.to_string())?)
    } else {
        stored_transmission_document(&invoice_row)?
    };
    let transmission = if let Some(fattura) = stored {
        fattura.header.and_then(|header| header.dati_trasmissione)
    } else {
        let buyer_id: String = invoice_row.get("cessionario_id");
        store
            .read("clients", "id", vec![serde_json::json!(buyer_id)], "")
            .await
            .fetch_optional()
            .await?
            .map(|buyer| destination(&invoice_row, &buyer, is_autofattura))
    };
    let formato_trasmissione = transmission
        .as_ref()
        .and_then(|data| {
            common::xml::enum_map::formato_trasmissione_to_sdi(data.formato_trasmissione)
        })
        .map(str::to_owned);
    let is_pa = formato_trasmissione.as_deref() == Some("FPA12");
    let codice_destinatario = transmission
        .as_ref()
        .map(|data| data.codice_destinatario.clone());
    let pec_destinatario = transmission.and_then(|data| data.pec_destinatario);
    let is_split_payment = invoice_row
        .get::<Option<bool>>("is_split_payment")
        .unwrap_or(false);
    let esigibilita_iva: Option<String> = invoice_row.get("esigibilita_iva");
    let dati_ordine_acquisto: Vec<DatiDocumentoTracciabilitaDetail> = invoice_row
        .get::<Option<Vec<DatiDocumentoTracciabilitaDetail>>>("dati_ordine_acquisto")
        .unwrap_or_default();
    let dati_contratto: Vec<DatiDocumentoTracciabilitaDetail> = invoice_row
        .get::<Option<Vec<DatiDocumentoTracciabilitaDetail>>>("dati_contratto")
        .unwrap_or_default();
    let dati_convenzione: Vec<DatiDocumentoTracciabilitaDetail> = invoice_row
        .get::<Option<Vec<DatiDocumentoTracciabilitaDetail>>>("dati_convenzione")
        .unwrap_or_default();
    let dati_ddt: Vec<DatiDdtDetail> = invoice_row
        .get::<Option<Vec<DatiDdtDetail>>>("dati_ddt")
        .unwrap_or_default();
    let allegati: Vec<AttachmentDetail> = invoice_row
        .get::<Option<Vec<AttachmentDetail>>>("allegati")
        .unwrap_or_default();
    let declared_total: Option<f64> = invoice_row.get("importo_totale");
    let inferred_total = if declared_total.is_none() {
        invoice_row
            .get::<Option<f64>>("importo_totale_calcolato")
            .or_else(|| {
                invoice_row
                    .get::<Option<String>>("xml_content")
                    .and_then(|xml| {
                        inferred_total_for_imported_body(&invoice_row.get::<String>("id"), &xml)
                    })
            })
    } else {
        None
    };
    let displayed_total = declared_total.or(inferred_total);
    let displayed_netto = netto_a_pagare.or_else(|| {
        displayed_total.map(|total| {
            let ritenuta: f64 = dati_ritenuta.iter().map(|item| item.importo_ritenuta).sum();
            let split_iva: f64 = if is_split_payment {
                let xml = invoice_row.get::<Option<String>>("xml_content");
                xml.and_then(|xml| imported_body_for_id(&invoice_row.get::<String>("id"), &xml))
                    .and_then(|body| body.dati_beni_servizi)
                    .map(|dbs| {
                        dbs.dati_riepilogo
                            .iter()
                            .filter(|summary| summary.esigibilita_iva == Some(30))
                            .map(|summary| summary.imposta)
                            .sum()
                    })
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            ((total - ritenuta - split_iva).max(0.0) * 100.0).round() / 100.0
        })
    });

    let stato: String = invoice_row.get("stato");
    let active_pec_filename = crate::commands::pec::active_pec_filename(&invoice_row);
    let recoverable_pec_filename =
        crate::commands::pec::recoverable_pec_filename(store, &invoice_row, id).await?;
    let has_mc_notice = !matches!(stato.as_str(), "draft" | "validated")
        && recoverable_pec_filename.is_none()
        && crate::commands::pec::records_have_attempt_mc(
            &notif_rows,
            active_pec_filename.as_deref(),
        );

    Ok(InvoiceDetail {
        id: invoice_row.get("id"),
        revision: invoice_revision(&names_row),
        numero: invoice_row.get("numero"),
        data: invoice_row.get("data"),
        tipo_documento: invoice_row.get("tipo_documento"),
        divisa: invoice_row.get("divisa"),
        importo_totale: displayed_total,
        total_calculated: declared_total.is_none() && inferred_total.is_some(),
        netto_a_pagare: displayed_netto,
        stato,
        tax_id_errors: tax_id_errors(&names_row),
        cedente_id: invoice_row.get("cedente_id"),
        cessionario_id: invoice_row.get("cessionario_id"),
        cedente_denominazione: names_row.get("cedente_denominazione"),
        cessionario_denominazione: names_row.get("cessionario_denominazione"),
        causale,
        linee,
        pagamenti,
        is_signed: invoice_row
            .get::<Option<bool>>("is_signed")
            .unwrap_or(false),
        signer_info: invoice_row.get("signer_info"),
        notifications,
        dati_ritenuta,
        dati_bollo,
        dati_cassa_previdenziale,
        dati_fatture_collegate,
        soggetto_emittente,
        is_autofattura,
        is_pa,
        is_split_payment,
        esigibilita_iva,
        formato_trasmissione,
        codice_destinatario,
        pec_destinatario,
        pending_pec_filename: invoice_row.get("pending_pec_filename"),
        active_pec_filename,
        recoverable_pec_filename,
        has_mc_notice,
        dati_ordine_acquisto,
        dati_contratto,
        dati_convenzione,
        dati_ddt,
        allegati,
    })
}

fn invoice_revision(row: &crate::storage::Record) -> Option<String> {
    row.get::<Option<String>>("file").and_then(|file| {
        std::path::Path::new(&file)
            .file_stem()?
            .to_str()
            .map(str::to_owned)
    })
}

fn stored_transmission_document(
    row: &crate::storage::Record,
) -> Result<Option<common::sdi::v1::FatturaElettronica>, String> {
    let status: String = row.get("stato");
    if matches!(
        status.as_str(),
        "imported" | "sent" | "delivered" | "accepted" | "rejected"
    ) || row.get::<Option<bool>>("imported_from_xml") == Some(true)
    {
        return row
            .get::<Option<String>>("xml_content")
            .filter(|xml| !xml.trim().is_empty())
            .map(|xml| common::xml::decode::decode(&xml).map_err(|error| error.to_string()))
            .transpose();
    }
    Ok(None)
}

fn destination(
    invoice: &crate::storage::Record,
    buyer: &crate::storage::Record,
    is_autofattura: bool,
) -> common::sdi::v1::DatiTrasmissione {
    let codice_destinatario = crate::commands::clients::normalize_recipient_code(
        buyer
            .get::<Option<String>>("codice_destinatario")
            .as_deref(),
    )
    .unwrap_or_else(|| "0000000".to_owned());
    let pec_destinatario = if !is_autofattura && codice_destinatario == "0000000" {
        buyer
            .get::<Option<String>>("pec")
            .map(|pec| pec.trim().to_owned())
            .filter(|pec| !pec.is_empty())
    } else {
        None
    };
    let is_pa = buyer
        .get::<Option<bool>>("is_pa")
        .unwrap_or(codice_destinatario.len() == 6)
        || invoice.get::<Option<bool>>("is_pa").unwrap_or(false);
    common::sdi::v1::DatiTrasmissione {
        codice_destinatario,
        pec_destinatario,
        formato_trasmissione: if is_pa { 10 } else { 20 },
        ..Default::default()
    }
}

/// Build a `FatturaElettronica` proto from stored records.
///
/// This reconstructs the proto message structure from the invoice, its lines,
/// payments, and the associated cedente/cessionario client records.
pub(crate) async fn build_fattura_from_store(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<common::sdi::v1::FatturaElettronica, String> {
    let snapshot = store.snapshot().await;
    let store = &snapshot;
    use common::sdi::v1::*;

    // Fetch invoice.
    let inv = store
        .read("invoices", "id", vec![serde_json::json!(id)], "")
        .await
        .fetch_optional()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Invoice not found: {id}"))?;

    if let Some(fattura) = stored_transmission_document(&inv)? {
        return Ok(fattura);
    }

    let cedente_id: String = inv.get("cedente_id");
    let cessionario_id: String = inv.get("cessionario_id");

    // Fetch clients.
    let cedente_row = store
        .read("clients", "id", vec![serde_json::json!(&cedente_id)], "")
        .await
        .fetch_one()
        .await
        .map_err(|e| e.to_string())?;

    let cessionario_row = store
        .read(
            "clients",
            "id",
            vec![serde_json::json!(&cessionario_id)],
            "",
        )
        .await
        .fetch_one()
        .await
        .map_err(|e| e.to_string())?;

    // Fetch lines and payments.
    let line_rows = store
        .read(
            "invoice_lines",
            "invoice_children",
            vec![serde_json::json!(id)],
            "line",
        )
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let payment_rows = store
        .read(
            "invoice_payments",
            "invoice_children",
            vec![serde_json::json!(id)],
            "",
        )
        .await
        .fetch_all()
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

    let cess_id_paese: String = cessionario_row.get("id_paese");
    let cess_id_codice: String = cessionario_row.get("id_codice");
    let cess_cf: Option<String> = cessionario_row.get("codice_fiscale");
    let cess_is_cf_only = cess_id_paese.eq_ignore_ascii_case("IT")
        && cess_id_codice.trim().len() == 16
        && cess_id_codice
            .trim()
            .chars()
            .any(|c| c.is_ascii_alphabetic());
    let (cess_id_fiscale_iva, cess_codice_fiscale) = if cess_is_cf_only {
        (
            None,
            cess_cf
                .filter(|s| !s.trim().is_empty())
                .or_else(|| Some(cess_id_codice.trim().to_ascii_uppercase())),
        )
    } else if cess_id_codice.trim().is_empty() || cess_id_codice.trim() == "00000000000" {
        (None, cess_cf)
    } else {
        (
            Some(IdFiscale {
                id_paese: cess_id_paese,
                id_codice: cess_id_codice,
            }),
            cess_cf,
        )
    };

    let cessionario = CessionarioCommittente {
        dati_anagrafici: Some(DatiAnagraficiCessionario {
            id_fiscale_iva: cess_id_fiscale_iva,
            codice_fiscale: cess_codice_fiscale,
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

    // Build document type.
    let tipo_doc_str: String = inv.get("tipo_documento");
    let tipo_documento =
        common::xml::enum_map::tipo_documento_from_sdi(&tipo_doc_str).unwrap_or(10);
    let is_autofattura = matches!(tipo_documento, 160 | 170 | 180 | 190 | 200 | 210 | 270);

    let soggetto_emittente = if is_autofattura {
        Some(10) // CC: CessionarioCommittente
    } else {
        inv.get::<Option<String>>("soggetto_emittente")
            .as_deref()
            .and_then(common::xml::enum_map::soggetto_emittente_from_sdi)
    };

    let transmitter = if is_autofattura {
        &cessionario_row
    } else {
        &cedente_row
    };
    let transmitter_country: String = transmitter.get("id_paese");
    let transmitter_vat: String = transmitter.get("id_codice");
    let transmitter_cf: Option<String> = transmitter.get("codice_fiscale");
    let transmitter_id = if transmitter_country == "IT" {
        let is_person = ["nome", "cognome"].iter().any(|field| {
            transmitter.get::<Option<String>>(field)
                .is_some_and(|value| !value.trim().is_empty())
        });
        transmitter_cf.as_deref().map(str::trim).filter(|cf| !cf.is_empty())
            .unwrap_or_else(|| {
                if is_person && transmitter_vat.trim().len() != 16 {
                    ""
                } else {
                    transmitter_vat.trim()
                }
            })
            .to_ascii_uppercase()
    } else {
        transmitter_vat
    };
    let destination = destination(&inv, &cessionario_row, is_autofattura);
    let formato_trasmissione = destination.formato_trasmissione;

    // Build header.
    let header = FatturaElettronicaHeader {
        dati_trasmissione: Some(DatiTrasmissione {
            id_trasmittente: Some(IdFiscale {
                id_paese: transmitter_country,
                id_codice: transmitter_id,
            }),
            progressivo_invio: common::pec::progressivo_from_id(id),
            ..destination
        }),
        cedente_prestatore: Some(cedente),
        cessionario_committente: Some(cessionario),
        soggetto_emittente,
        ..Default::default()
    };

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

        use common::xml::enum_map::*;
        let fields = row
            .get::<Option<LineFields>>("line_fields")
            .unwrap_or_default();
        let adjustments = fields.adjustments();
        dettaglio_linee.push(DettaglioLinee {
            numero_linea: row.get("numero_linea"),
            descrizione: row.get("descrizione"),
            quantita: row.get("quantita"),
            unita_misura: row.get("unita_misura"),
            data_inizio_periodo: row.get("data_inizio_periodo"),
            data_fine_periodo: row.get("data_fine_periodo"),
            prezzo_unitario: row.get("prezzo_unitario"),
            prezzo_totale: row.get("prezzo_totale"),
            aliquota_iva: row.get("aliquota_iva"),
            natura: natura_val,
            tipo_cessione_prestazione: fields
                .tipo_cessione_prestazione
                .as_deref()
                .and_then(tipo_cessione_prestazione_from_sdi),
            codice_articolo: fields
                .codice_articolo
                .into_iter()
                .map(|item| CodiceArticolo {
                    codice_tipo: item.codice_tipo,
                    codice_valore: item.codice_valore,
                })
                .collect(),
            sconto_maggiorazione: adjustments,
            ritenuta: fields.ritenuta.as_deref().and_then(ritenuta_from_sdi),
            riferimento_amministrazione: fields.riferimento_amministrazione,
            altri_dati_gestionali: fields
                .altri_dati_gestionali
                .into_iter()
                .map(|item| AltriDatiGestionali {
                    tipo_dato: item.tipo_dato,
                    riferimento_testo: item.riferimento_testo,
                    riferimento_numero: item.riferimento_numero,
                    riferimento_data: item.riferimento_data,
                })
                .collect(),
        });
    }

    // Build riepilogo (summary by VAT rate and Natura) from the lines.
    type VatSummary = std::collections::BTreeMap<(String, Option<i32>), (f64, f64, Option<i32>)>;
    let mut riepilogo_map: VatSummary = std::collections::BTreeMap::new();
    for linea in &dettaglio_linee {
        let key = (format!("{:.2}", linea.aliquota_iva), linea.natura);
        let entry = riepilogo_map
            .entry(key)
            .or_insert((0.0, linea.aliquota_iva, linea.natura));
        entry.0 += linea.prezzo_totale;
    }

    // DatiRiepilogo will be finalized after incorporating cassa previdenziale below.

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

    // Reconstruct Causale (chunked into <=200 chars per XSD String200LatinType), DatiRitenuta, DatiBollo, DatiCassaPrevidenziale
    let causale: Vec<String> = inv
        .get::<Option<Vec<String>>>("causale")
        .unwrap_or_default()
        .into_iter()
        .flat_map(|line| split_sdi_causale(&line))
        .collect();
    let dati_ritenuta: Vec<DatiRitenuta> = inv
        .get::<Option<Vec<DatiRitenutaDetail>>>("dati_ritenuta")
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            let tipo =
                common::xml::enum_map::tipo_ritenuta_from_sdi(&r.tipo_ritenuta).unwrap_or(10);
            let causale_pag =
                common::xml::enum_map::causale_pagamento_from_sdi(&r.causale_pagamento)
                    .unwrap_or(10);
            DatiRitenuta {
                tipo_ritenuta: tipo,
                importo_ritenuta: r.importo_ritenuta,
                aliquota_ritenuta: r.aliquota_ritenuta,
                causale_pagamento: causale_pag,
            }
        })
        .collect();

    let dati_bollo: Option<DatiBollo> =
        inv.get::<Option<DatiBolloDetail>>("dati_bollo")
            .and_then(|b| {
                let bv = common::xml::enum_map::bollo_virtuale_from_sdi(&b.bollo_virtuale)?;
                Some(DatiBollo {
                    bollo_virtuale: bv,
                    importo_bollo: b.importo_bollo,
                })
            });

    let dati_cassa_previdenziale: Vec<DatiCassaPrevidenziale> = inv
        .get::<Option<Vec<DatiCassaDetail>>>("dati_cassa_previdenziale")
        .unwrap_or_default()
        .into_iter()
        .map(|c| {
            let tc = common::xml::enum_map::tipo_cassa_from_sdi(&c.tipo_cassa).unwrap_or(40);
            let ritenuta = c
                .ritenuta
                .as_deref()
                .and_then(common::xml::enum_map::ritenuta_from_sdi);
            let natura = c
                .natura
                .as_deref()
                .and_then(common::xml::enum_map::natura_from_sdi);
            DatiCassaPrevidenziale {
                tipo_cassa: tc,
                al_cassa: c.al_cassa,
                importo_contributo_cassa: c.importo_contributo_cassa,
                imponibile_cassa: c.imponibile_cassa,
                aliquota_iva: c.aliquota_iva,
                ritenuta,
                natura,
                ..Default::default()
            }
        })
        .collect();

    // Include cassa contributions in riepilogo
    for cassa in &dati_cassa_previdenziale {
        let key = (format!("{:.2}", cassa.aliquota_iva), cassa.natura);
        let entry = riepilogo_map
            .entry(key)
            .or_insert((0.0, cassa.aliquota_iva, cassa.natura));
        entry.0 += cassa.importo_contributo_cassa;
    }

    let is_split_payment = inv.get::<Option<bool>>("is_split_payment").unwrap_or(false)
        || inv.get::<Option<String>>("esigibilita_iva").as_deref() == Some("S")
        || formato_trasmissione == 10;

    // Recalculate dati_riepilogo with cassa contributions included and split payment flag
    let dati_riepilogo: Vec<DatiRiepilogo> = riepilogo_map
        .values()
        .map(|(imponibile, aliquota, natura)| {
            let imposta = ((aliquota * imponibile / 100.0) * 100.0).round() / 100.0;
            let esig_val = if is_split_payment && *aliquota > 0.0 {
                Some(30) // S = Split Payment
            } else {
                None
            };
            DatiRiepilogo {
                aliquota_iva: *aliquota,
                imponibile_importo: *imponibile,
                imposta,
                natura: *natura,
                esigibilita_iva: esig_val,
                ..Default::default()
            }
        })
        .collect();

    let dati_fatture_collegate: Vec<common::sdi::v1::DatiDocumentiCorrelati> = inv
        .get::<Option<Vec<DatiFatturaCollegataDetail>>>("dati_fatture_collegate")
        .unwrap_or_default()
        .into_iter()
        .map(|df| common::sdi::v1::DatiDocumentiCorrelati {
            id_documento: df.id_documento,
            data: df.data,
            num_item: df.num_item,
            codice_cup: df.codice_cup,
            codice_cig: df.codice_cig,
            ..Default::default()
        })
        .collect();

    let dati_ordine_acquisto: Vec<common::sdi::v1::DatiDocumentiCorrelati> = inv
        .get::<Option<Vec<DatiDocumentoTracciabilitaDetail>>>("dati_ordine_acquisto")
        .unwrap_or_default()
        .into_iter()
        .map(|d| common::sdi::v1::DatiDocumentiCorrelati {
            id_documento: d.id_documento,
            data: d.data,
            num_item: d.num_item,
            codice_commessa_convenzione: d.codice_commessa_convenzione,
            codice_cup: d.codice_cup,
            codice_cig: d.codice_cig,
            ..Default::default()
        })
        .collect();

    let dati_contratto: Vec<common::sdi::v1::DatiDocumentiCorrelati> = inv
        .get::<Option<Vec<DatiDocumentoTracciabilitaDetail>>>("dati_contratto")
        .unwrap_or_default()
        .into_iter()
        .map(|d| common::sdi::v1::DatiDocumentiCorrelati {
            id_documento: d.id_documento,
            data: d.data,
            num_item: d.num_item,
            codice_commessa_convenzione: d.codice_commessa_convenzione,
            codice_cup: d.codice_cup,
            codice_cig: d.codice_cig,
            ..Default::default()
        })
        .collect();

    let dati_convenzione: Vec<common::sdi::v1::DatiDocumentiCorrelati> = inv
        .get::<Option<Vec<DatiDocumentoTracciabilitaDetail>>>("dati_convenzione")
        .unwrap_or_default()
        .into_iter()
        .map(|d| common::sdi::v1::DatiDocumentiCorrelati {
            id_documento: d.id_documento,
            data: d.data,
            num_item: d.num_item,
            codice_commessa_convenzione: d.codice_commessa_convenzione,
            codice_cup: d.codice_cup,
            codice_cig: d.codice_cig,
            ..Default::default()
        })
        .collect();

    let dati_ddt: Vec<common::sdi::v1::DatiDdt> = inv
        .get::<Option<Vec<DatiDdtDetail>>>("dati_ddt")
        .unwrap_or_default()
        .into_iter()
        .map(|d| common::sdi::v1::DatiDdt {
            numero_ddt: d.numero_ddt,
            data_ddt: d.data_ddt,
            riferimento_numero_linea: d.riferimento_numero_linea,
        })
        .collect();

    let allegati: Vec<common::sdi::v1::Allegati> = inv
        .get::<Option<Vec<AttachmentDetail>>>("allegati")
        .unwrap_or_default()
        .into_iter()
        .filter_map(|a| {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&a.attachment_base64)
                .ok()?;
            Some(common::sdi::v1::Allegati {
                nome_attachment: a.nome_attachment,
                algoritmo_compressione: a.algoritmo_compressione,
                formato_attachment: a.formato_attachment,
                descrizione_attachment: a.descrizione_attachment,
                attachment: bytes,
            })
        })
        .collect();

    // Assemble the FatturaElettronica.
    let fattura = FatturaElettronica {
        versione: formato_trasmissione,
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
                    dati_ritenuta,
                    dati_bollo,
                    dati_cassa_previdenziale,
                    causale,
                    ..Default::default()
                }),
                dati_ordine_acquisto,
                dati_contratto,
                dati_convenzione,
                dati_fatture_collegate,
                dati_ddt,
                ..Default::default()
            }),
            dati_beni_servizi: Some(DatiBeniServizi {
                dettaglio_linee,
                dati_riepilogo,
            }),
            dati_pagamento,
            allegati,
            ..Default::default()
        }],
    };

    Ok(fattura)
}

/// Build the `Anagrafica.soggetto` oneof from stored fields.
fn build_anagrafica_soggetto(
    denominazione: Option<&str>,
    nome: Option<&str>,
    cognome: Option<&str>,
) -> Option<common::sdi::v1::anagrafica::Soggetto> {
    use common::sdi::v1::PersonaFisica;
    use common::sdi::v1::anagrafica::Soggetto;

    if let Some(denom) = denominazione {
        Some(Soggetto::Denominazione(denom.to_string()))
    } else if let (Some(nome), Some(cognome)) = (nome, cognome) {
        Some(Soggetto::PersonaFisica(PersonaFisica {
            nome: nome.to_string(),
            cognome: cognome.to_string(),
        }))
    } else {
        cognome.map(|cognome| {
            Soggetto::PersonaFisica(PersonaFisica {
                nome: String::new(),
                cognome: cognome.to_string(),
            })
        })
    }
}

/// Find or create a client record from the cedente data in an XML header.
async fn ensure_client_from_header_cedente(
    store: &crate::storage::FileStore,
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

    let regime_str =
        common::xml::enum_map::regime_fiscale_to_sdi(da.regime_fiscale).map(String::from);

    ensure_client(
        store,
        &id_fiscale.id_paese,
        &id_fiscale.id_codice,
        denominazione.as_deref(),
        nome.as_deref(),
        cognome.as_deref(),
        da.codice_fiscale.as_deref(),
        regime_str.as_deref(),
        sede,
        None,
        None,
    )
    .await
}

/// Find or create a client record from the cessionario data in an XML header.
async fn ensure_client_from_header_cessionario(
    store: &crate::storage::FileStore,
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
    let codice_destinatario = header
        .dati_trasmissione
        .as_ref()
        .map(|dt| dt.codice_destinatario.trim())
        .filter(|cd| !cd.is_empty());
    let pec_destinatario = header
        .dati_trasmissione
        .as_ref()
        .and_then(|dt| dt.pec_destinatario.as_deref())
        .map(str::trim)
        .filter(|pec| !pec.is_empty());

    ensure_client(
        store,
        id_paese,
        id_codice,
        denominazione.as_deref(),
        nome.as_deref(),
        cognome.as_deref(),
        da.codice_fiscale.as_deref(),
        None,
        sede,
        codice_destinatario,
        pec_destinatario,
    )
    .await
}

/// Find a client by (id_paese, id_codice) or create a new one.
#[allow(clippy::too_many_arguments)]
async fn ensure_client(
    store: &crate::storage::FileStore,
    id_paese: &str,
    id_codice: &str,
    denominazione: Option<&str>,
    nome: Option<&str>,
    cognome: Option<&str>,
    codice_fiscale: Option<&str>,
    regime_fiscale: Option<&str>,
    sede: Option<&common::sdi::v1::Indirizzo>,
    codice_destinatario: Option<&str>,
    pec: Option<&str>,
) -> Result<String, String> {
    use crate::commands::clients::{
        merge_client_recipient_destination, resolve_client_recipient_code,
        valid_invoice_recipient_code, valid_invoice_recipient_pec,
    };

    let incoming_cd = codice_destinatario.and_then(|cd| valid_invoice_recipient_code(cd, id_paese));
    let incoming_pec = valid_invoice_recipient_pec(pec);
    let incoming_cf = codice_fiscale
        .map(str::trim)
        .filter(|cf| !cf.is_empty())
        .map(str::to_ascii_uppercase);

    // Try to find existing client by tax ID.
    let existing = store
        .read(
            "clients",
            "tax_identity",
            vec![serde_json::json!(id_paese), serde_json::json!(id_codice)],
            "",
        )
        .await
        .fetch_optional()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(row) = existing {
        let id: String = row.get("id");
        let raw_existing_cd: Option<String> = row.get("codice_destinatario");
        let (next_cd, next_pec, next_is_pa) = merge_client_recipient_destination(
            &row,
            incoming_cd.as_deref(),
            incoming_pec.as_deref(),
        );
        let existing_cf = row
            .get::<Option<String>>("codice_fiscale")
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let next_cf = existing_cf.clone().or(incoming_cf);

        if raw_existing_cd != next_cd
            || row.get::<Option<String>>("pec") != next_pec
            || row.get::<Option<String>>("codice_fiscale") != next_cf
            || row.get::<Option<bool>>("is_pa") != Some(next_is_pa)
        {
            store
                .update(
                    "clients",
                    &id,
                    serde_json::json!({
                        "codice_destinatario": next_cd,
                        "pec": next_pec,
                        "codice_fiscale": next_cf,
                        "is_pa": next_is_pa,
                        "recipient_backfilled": true
                    }),
                )
                .await
                .map_err(|e| e.to_string())?;
        }
        return Ok(id);
    }

    // Create a new client.
    let client_id = digest::client_id(id_paese, id_codice)?;
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
    let is_pa = incoming_cd.as_deref().is_some_and(|cd| cd.len() == 6);
    let resolved_cd = resolve_client_recipient_code(incoming_cd.as_deref(), is_pa);

    store
        .insert(
            "clients",
            serde_json::json!({
                "id": &client_id,
                "denominazione": denominazione,
                "nome": nome,
                "cognome": cognome,
                "id_paese": id_paese,
                "id_codice": id_codice,
                "codice_fiscale": incoming_cf,
                "regime_fiscale": regime_fiscale,
                "indirizzo": indirizzo,
                "numero_civico": numero_civico,
                "cap": cap,
                "comune": comune,
                "provincia": provincia,
                "nazione": nazione,
                "pec": incoming_pec,
                "codice_destinatario": resolved_cd,
                "is_pa": is_pa,
                "recipient_backfilled": true
            }),
        )
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
        Some(Soggetto::PersonaFisica(pf)) => {
            (None, Some(pf.nome.clone()), Some(pf.cognome.clone()))
        }
        None => (None, None, None),
    }
}

/// Rectify a rejected invoice (SDI scarto / NS) by returning it to draft state
/// per Circolare AdE 13/E 2018 (re-sending within 5 days with same number/date).
#[tauri::command]
pub async fn rectify_rejected_invoice(
    state: tauri::State<'_, TauriState>,
    id: String,
) -> Result<InvoiceDetail, String> {
    rectify_rejected_invoice_store(&state.store, &id).await
}

pub async fn rectify_rejected_invoice_store(
    store: &crate::storage::FileStore,
    id: &str,
) -> Result<InvoiceDetail, String> {
    let mut tx = store.begin().await?;
    let row = tx
        .snapshot()
        .read("invoices", "id", vec![serde_json::json!(id)], "")
        .await
        .fetch_one()
        .await?;
    let stato: String = row.get("stato");

    if stato != "rejected" {
        return Err(format!(
            "Only rejected invoices can be rectified (current status: {stato})"
        ));
    }

    if crate::commands::pec::recoverable_pec_filename(&tx.snapshot(), &row, id)
        .await?
        .is_some()
    {
        return Err("Lo scarto è di un invio precedente. Verifica in PEC il reinvio prima di rettificare.".into());
    }
    if crate::commands::pec::has_attempt_pa_refusal(&tx.snapshot(), &row, id).await? {
        return Err(
            "La fattura è stata rifiutata dalla PA (NE EC02) dopo la consegna SdI e non può essere riportata in bozza come scarto."
                .into(),
        );
    }
    tx.update(
        "invoices",
        id,
        serde_json::json!({"stato": "draft", "imported_from_xml": false}),
    )
    .await?;
    tx.commit().await?;
    fetch_invoice_detail(store, id).await
}

/// Generate a draft autofattura / reverse charge integration request from an existing purchase invoice.
#[tauri::command]
pub async fn generate_autofattura_draft(
    state: tauri::State<'_, TauriState>,
    purchase_invoice_id: String,
    autofattura_type: Option<String>,
) -> Result<CreateInvoiceRequest, String> {
    generate_autofattura_draft_store(&state.store, &purchase_invoice_id, autofattura_type).await
}

pub async fn generate_autofattura_draft_store(
    store: &crate::storage::FileStore,
    purchase_invoice_id: &str,
    autofattura_type: Option<String>,
) -> Result<CreateInvoiceRequest, String> {
    let purchase = fetch_invoice_detail(store, purchase_invoice_id).await?;

    let cedente_row = store
        .read(
            "clients",
            "id",
            vec![serde_json::json!(&purchase.cedente_id)],
            "",
        )
        .await
        .fetch_optional()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Cedente client not found: {}", purchase.cedente_id))?;

    let cedente_paese: String = cedente_row.get("id_paese");
    let target_tipo = match autofattura_type {
        Some(t) if !t.is_empty() => t,
        _ => {
            if cedente_paese == "IT" {
                "TD16".to_string() // Reverse charge interno
            } else {
                "TD17".to_string() // Servizi estero
            }
        }
    };

    let current_year = chrono::Utc::now().format("%Y").to_string();
    let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let count_row = store
        .read(
            "invoices",
            "number_count",
            vec![
                serde_json::json!(&purchase.cessionario_id),
                serde_json::json!(&current_year),
            ],
            "",
        )
        .await
        .fetch_one()
        .await
        .map_err(|e| e.to_string())?;
    let cnt: i64 = count_row.get("cnt");
    let autofattura_num = format!("{}/{}", cnt + 1, target_tipo);

    let mut autofattura_linee = Vec::new();
    let mut total_imponibile = 0.0;
    for l in &purchase.linee {
        let pt = match l.quantita {
            Some(q) => l.prezzo_unitario * q,
            None => l.prezzo_unitario,
        };
        total_imponibile += pt;
        autofattura_linee.push(CreateLineRequest {
            descrizione: format!("Integrazione acquisto: {}", l.descrizione),
            quantita: l.quantita,
            unita_misura: l.unita_misura.clone(),
            fields: Default::default(),
            data_inizio_periodo: l.data_inizio_periodo.clone(),
            data_fine_periodo: l.data_fine_periodo.clone(),
            prezzo_unitario: l.prezzo_unitario,
            aliquota_iva: 22.0,
            natura: None,
        });
    }

    let iva_integrata = ((total_imponibile * 22.0 / 100.0) * 100.0).round() / 100.0;
    let totale_lordo = total_imponibile + iva_integrata;

    let collegata = DatiFatturaCollegataDetail {
        id_documento: purchase.numero.clone(),
        data: Some(purchase.data.clone()),
        num_item: None,
        codice_cig: None,
        codice_cup: None,
    };

    let causale = vec![format!(
        "Integrazione documento {} ex art. 17 c. 2 DPR 633/72 rif. fattura {} del {}",
        target_tipo, purchase.numero, purchase.data
    )];

    let pagamenti = vec![CreatePaymentRequest {
        condizioni_pagamento: Some("TP02".to_string()),
        modalita_pagamento: "MP05".to_string(),
        importo_pagamento: totale_lordo,
        data_scadenza_pagamento: None,
        iban: None,
    }];

    Ok(CreateInvoiceRequest {
        numero: autofattura_num,
        data: current_date,
        tipo_documento: target_tipo,
        divisa: purchase.divisa.clone(),
        cedente_id: purchase.cedente_id.clone(),
        cessionario_id: purchase.cessionario_id.clone(),
        causale,
        linee: autofattura_linee,
        pagamenti,
        dati_ritenuta: None,
        dati_bollo: None,
        dati_cassa_previdenziale: None,
        dati_fatture_collegate: Some(vec![collegata]),
        soggetto_emittente: Some("CC".to_string()),
        ..Default::default()
    })
}

fn split_sdi_causale(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.chars().count() <= 200 {
        return vec![trimmed.to_owned()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for word in trimmed.split_whitespace() {
        let word_len = word.chars().count();
        let cur_len = current.chars().count();
        if cur_len > 0 && cur_len + 1 + word_len > 200 {
            chunks.push(std::mem::take(&mut current));
        }
        if word_len > 200 {
            let mut buf = String::new();
            for ch in word.chars() {
                buf.push(ch);
                if buf.chars().count() == 200 {
                    chunks.push(std::mem::take(&mut buf));
                }
            }
            current = buf;
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn known_transmission_aliases_prevent_matching_rebuilt_invoice_names() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::storage::FileStore::open(dir.path().join("eattura.json")).unwrap();
        let (seller_id, buyer_id) = setup_test_parties(&store).await;
        let created =
            create_invoice_store(&store, sample_invoice_request(&seller_id, &buyer_id, "1"))
                .await
                .unwrap();
        let rebuilt = common::pec::sdi_filename(
            &build_fattura_from_store(&store, &created.id).await.unwrap(),
        );
        let known = "IT12345678901_TRANSMITTED.xml";
        assert_ne!(rebuilt, known);
        store
            .update(
                "invoices",
                &created.id,
                serde_json::json!({"sdi_filenames": [known]}),
            )
            .await
            .unwrap();

        assert_eq!(
            crate::commands::pec::find_invoice_by_sdi_reference(&store, known)
                .await
                .unwrap(),
            Some(created.id)
        );
        assert_eq!(
            crate::commands::pec::find_invoice_by_sdi_reference(&store, &rebuilt)
                .await
                .unwrap(),
            None
        );
    }

    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn file_backed_invoice_preserves_detail_and_xml_round_trip() {
        let dir = std::env::temp_dir().join(format!("eattura-invoice-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("data.json");
        let store = crate::storage::FileStore::open(path.clone()).unwrap();
        for (id, tax_id, name) in [
            ("seller", "12345678901", "Seller"),
            ("buyer", "98765432109", "Buyer"),
        ] {
            store
                .insert(
                    "clients",
                    json!({
                        "id": id, "id_paese": "IT", "id_codice": tax_id,
                        "denominazione": name, "regime_fiscale": "RF01",
                        "indirizzo": "Via Roma", "cap": "00100", "comune": "Roma", "nazione": "IT"
                    }),
                )
                .await
                .unwrap();
        }
        let mut tx = store.begin().await.unwrap();
        tx.insert(
            "invoices",
            json!({
                "id": "invoice", "numero": "1", "data": "2026-01-01", "tipo_documento": "TD01",
                "divisa": "EUR", "importo_totale": 100.0, "stato": "draft",
                "cedente_id": "seller", "cessionario_id": "buyer"
            }),
        )
        .await
        .unwrap();
        tx.insert(
            "invoice_lines",
            json!({
                "id": "line", "invoice_id": "invoice", "numero_linea": 1,
                "descrizione": "Service", "quantita": 2.0, "prezzo_unitario": 50.0,
                "prezzo_totale": 100.0, "aliquota_iva": 22.0
            }),
        )
        .await
        .unwrap();
        tx.insert(
            "invoice_payments",
            json!({
                "id": "payment", "invoice_id": "invoice", "modalita_pagamento": "MP05",
                "importo_pagamento": 122.0
            }),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let reopened = crate::storage::FileStore::open(path).unwrap();
        let detail = fetch_invoice_detail(&reopened, "invoice").await.unwrap();
        assert_eq!(detail.cedente_denominazione, "Seller");
        assert_eq!(detail.cessionario_denominazione, "Buyer");
        assert_eq!(detail.linee[0].prezzo_totale, 100.0);
        assert_eq!(detail.pagamenti[0].importo_pagamento, 122.0);
        let fattura = build_fattura_from_store(&reopened, "invoice")
            .await
            .unwrap();
        let xml = common::xml::encode::encode(&fattura).unwrap();
        let import_dir = dir.join("imported");
        std::fs::create_dir_all(&import_dir).unwrap();
        let imported_store = crate::storage::FileStore::open(import_dir.join("data.json")).unwrap();
        let imported = import_xml_content(&imported_store, &xml).await.unwrap();
        let preserved = build_fattura_from_store(&imported_store, &imported.id)
            .await
            .unwrap();
        assert_eq!(
            common::pec::sdi_filename(&preserved),
            common::pec::sdi_filename(&fattura)
        );

        let spaced_xml = xml.replace("><", ">\n  <");
        let duplicate = import_xml_content(&imported_store, &spaced_xml)
            .await
            .unwrap();
        assert_eq!(imported.id, duplicate.id);
        assert_eq!(imported.id.len(), 64);

        let reopened_import =
            crate::storage::FileStore::open(import_dir.join("data.json")).unwrap();
        let indexed = reopened_import
            .read("invoices", "invoice_views", vec![], "invoice")
            .await
            .fetch_all()
            .await
            .unwrap();
        assert_eq!(indexed[0].get::<String>("id"), imported.id);
        assert_eq!(indexed.len(), 1);
        assert!(indexed[0].get::<String>("search_text").contains("service"));
        let decoded = common::xml::decode::decode(&xml).unwrap();
        let document = decoded.body[0]
            .dati_generali
            .as_ref()
            .unwrap()
            .dati_generali_documento
            .as_ref()
            .unwrap();
        assert_eq!(document.numero, "1");
        assert_eq!(document.importo_totale_documento, Some(100.0));
        std::fs::remove_dir_all(dir).unwrap();
    }

    async fn setup_test_parties(store: &crate::storage::FileStore) -> (String, String) {
        let seller_id = digest::client_id("IT", "11111111115").unwrap();
        let buyer_id = digest::client_id("IT", "22222222220").unwrap();

        store
            .insert(
                "clients",
                json!({
                    "id": &seller_id, "id_paese": "IT", "id_codice": "11111111115",
                    "denominazione": "Seller Srl", "regime_fiscale": "RF01",
                    "indirizzo": "Via Roma 1", "cap": "00100", "comune": "Roma", "nazione": "IT"
                }),
            )
            .await
            .unwrap();

        store
            .insert(
                "clients",
                json!({
                    "id": &buyer_id, "id_paese": "IT", "id_codice": "22222222220",
                    "denominazione": "Buyer SpA", "regime_fiscale": "RF01",
                    "indirizzo": "Corso Milano 2", "cap": "20100", "comune": "Milano", "nazione": "IT"
                }),
            )
            .await
            .unwrap();

        (seller_id, buyer_id)
    }

    fn sample_invoice_request(
        seller_id: &str,
        buyer_id: &str,
        numero: &str,
    ) -> CreateInvoiceRequest {
        CreateInvoiceRequest {
            numero: numero.to_string(),
            data: "2026-03-01".to_string(),
            tipo_documento: "TD01".to_string(),
            divisa: "EUR".to_string(),
            cedente_id: seller_id.to_string(),
            cessionario_id: buyer_id.to_string(),
            linee: vec![
                CreateLineRequest {
                    descrizione: "Development Services".to_string(),
                    quantita: Some(2.0),
                    unita_misura: Some("H".to_string()),
                    fields: Default::default(),
                    data_inizio_periodo: None,
                    data_fine_periodo: None,
                    prezzo_unitario: 50.0,
                    aliquota_iva: 22.0,
                    natura: None,
                },
                CreateLineRequest {
                    descrizione: "Documentation".to_string(),
                    quantita: Some(1.0),
                    unita_misura: None,
                    fields: Default::default(),
                    data_inizio_periodo: None,
                    data_fine_periodo: None,
                    prezzo_unitario: 40.0,
                    aliquota_iva: 22.0,
                    natura: None,
                },
            ],
            pagamenti: vec![CreatePaymentRequest {
                condizioni_pagamento: None,
                modalita_pagamento: "MP05".to_string(),
                importo_pagamento: 170.80,
                data_scadenza_pagamento: Some("2026-04-01".to_string()),
                iban: Some("IT60X0542811101000000123456".to_string()),
            }],
            causale: Vec::new(),
            dati_ritenuta: None,
            dati_bollo: None,
            dati_cassa_previdenziale: None,
            dati_fatture_collegate: None,
            soggetto_emittente: None,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn fixed_line_discount_matches_the_validated_xml_and_saved_total() {
        let directory = tempfile::tempdir().unwrap();
        let store = crate::storage::FileStore::open(directory.path().join("data.json")).unwrap();
        let (seller, buyer) = setup_test_parties(&store).await;
        let mut request = sample_invoice_request(&seller, &buyer, "1");
        request.linee.truncate(1);
        request.linee[0].fields.sconto_maggiorazione = vec![LineAdjustment {
            tipo: "SC".into(),
            percentuale: Some(5.0),
            importo: Some(10.0),
        }];
        request.pagamenti[0].importo_pagamento = 97.60;

        let invoice = create_invoice_store(&store, request).await.unwrap();
        assert_eq!(invoice.linee[0].prezzo_totale, 80.0);
        assert_eq!(invoice.importo_totale, Some(97.60));
        let validation = validate_invoice_store(&store, &invoice.id).await.unwrap();
        assert!(validation.valid, "{:?}", validation.errors);
        let xml = export_invoice_xml_store(&store, &invoice.id).await.unwrap();
        assert!(xml.contains("<PrezzoTotale>80.00</PrezzoTotale>"));
    }

    #[tokio::test]
    async fn italian_individual_transmitter_requires_and_uses_fiscal_code() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::storage::FileStore::open(dir.path().join("index.json")).unwrap();
        let (seller, buyer) = setup_test_parties(&store).await;
        store.update("clients", &seller, json!({
            "denominazione": null, "nome": "Mario", "cognome": "Rossi",
        })).await.unwrap();
        let invoice = create_invoice_store(&store, sample_invoice_request(&seller, &buyer, "1"))
            .await.unwrap();
        let validation = validate_invoice_store(&store, &invoice.id).await.unwrap();
        assert!(!validation.valid);
        assert!(validation.errors.iter().any(|error| error.code == "00300"));
        assert!(set_invoice_status_store(&store, &invoice.id, "validated").await.is_err());
        assert!(export_invoice_xml_store(&store, &invoice.id).await.is_err());

        store.update("clients", &seller, json!({"codice_fiscale": "RSSMRA80A01H501U"}))
            .await.unwrap();
        assert!(validate_invoice_store(&store, &invoice.id).await.unwrap().valid);
        let xml = export_invoice_xml_store(&store, &invoice.id).await.unwrap();
        assert!(xml.contains("<IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>RSSMRA80A01H501U</IdCodice></IdTrasmittente>"));
        assert!(xml.contains("<IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>11111111115</IdCodice></IdFiscaleIVA>"));
        let document = common::xml::decode::decode(&xml).unwrap();
        assert!(common::pec::sdi_filename(&document).starts_with("ITRSSMRA80A01H501U_"));
    }

    #[tokio::test]
    async fn test_create_invoice_computes_totals_and_persists() {
        let dir = std::env::temp_dir().join(format!("eattura-inv-create-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::storage::FileStore::open(dir.join("index.json")).unwrap();

        let (seller_id, buyer_id) = setup_test_parties(&store).await;
        let req = sample_invoice_request(&seller_id, &buyer_id, "10/2026");

        let invoice = create_invoice_store(&store, req).await.unwrap();

        // 2 lines: (2.0 * 50.0) + (1.0 * 40.0) = 140.0 + 22% VAT (30.80) = 170.80 gross
        assert_eq!(invoice.importo_totale, Some(170.80));
        assert_eq!(invoice.netto_a_pagare, Some(170.80));
        assert_eq!(invoice.stato, "draft");
        assert_eq!(invoice.linee.len(), 2);
        assert_eq!(invoice.pagamenti.len(), 1);
        assert_eq!(invoice.cedente_denominazione, "Seller Srl");
        assert_eq!(invoice.cessionario_denominazione, "Buyer SpA");

        // Verify direct read from store
        let fetched = fetch_invoice_detail(&store, &invoice.id).await.unwrap();
        assert_eq!(fetched.id, invoice.id);
        assert_eq!(fetched.numero, "10/2026");
    }

    #[tokio::test]
    async fn test_update_draft_invoice_rekeys_and_updates_content() {
        let dir = std::env::temp_dir().join(format!("eattura-inv-update-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::storage::FileStore::open(dir.join("index.json")).unwrap();

        let (seller_id, buyer_id) = setup_test_parties(&store).await;
        let req = sample_invoice_request(&seller_id, &buyer_id, "20/2026");
        let created = create_invoice_store(&store, req).await.unwrap();

        // Update with changed line items
        let mut update_req = sample_invoice_request(&seller_id, &buyer_id, "20/2026");
        update_req.linee = vec![CreateLineRequest {
            descrizione: "Single Consulting Day".to_string(),
            quantita: Some(1.0),
            unita_misura: Some("DAY".to_string()),
            fields: Default::default(),
            data_inizio_periodo: None,
            data_fine_periodo: None,
            prezzo_unitario: 500.0,
            aliquota_iva: 22.0,
            natura: None,
        }];

        let updated = update_invoice_store(&store, &created.id, update_req, None, false)
            .await
            .unwrap();

        assert_ne!(updated.id, created.id); // Content-based ID changes
        // 500.0 taxable + 22% VAT (110.0) = 610.0 gross
        assert_eq!(updated.importo_totale, Some(610.0));
        assert_eq!(updated.linee.len(), 1);
        assert_eq!(updated.linee[0].descrizione, "Single Consulting Day");

        // Old ID no longer exists
        assert!(fetch_invoice_detail(&store, &created.id).await.is_err());
    }

    #[tokio::test]
    async fn test_ritenuta_cassa_bollo_and_forfettario_lifecycle() {
        let dir = std::env::temp_dir().join(format!("eattura-ritenute-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::storage::FileStore::open(dir.join("index.json")).unwrap();

        let (seller_id, buyer_id) = setup_test_parties(&store).await;

        // Create an invoice with:
        // - Professional service: € 1000.00 (22% VAT)
        // - Cassa Inarcassa (TC04 4%): € 40.00 (22% VAT)
        // - Bollo Virtuale (€ 2.00)
        // - Ritenuta d'acconto (RT01 20% on € 1000.00): € 200.00
        let req = CreateInvoiceRequest {
            numero: "FPA/2026/01".to_string(),
            data: "2026-03-01".to_string(),
            tipo_documento: "TD01".to_string(),
            divisa: "EUR".to_string(),
            cedente_id: seller_id,
            cessionario_id: buyer_id,
            causale: vec![
                "Operazione svolta da professionista abilitato".to_string(),
                "Causale ritenuta d'acconto codice A".to_string(),
            ],
            linee: vec![CreateLineRequest {
                descrizione: "Progettazione architettonica".to_string(),
                quantita: Some(1.0),
                unita_misura: None,
                fields: Default::default(),
                data_inizio_periodo: None,
                data_fine_periodo: None,
                prezzo_unitario: 1000.0,
                aliquota_iva: 22.0,
                natura: None,
            }],
            pagamenti: vec![CreatePaymentRequest {
                condizioni_pagamento: None,
                modalita_pagamento: "MP05".to_string(),
                importo_pagamento: 1070.80, // Netto a pagare
                data_scadenza_pagamento: Some("2026-04-01".to_string()),
                iban: Some("IT60X0542811101000000123456".to_string()),
            }],
            dati_cassa_previdenziale: Some(vec![DatiCassaDetail {
                tipo_cassa: "TC04".to_string(),
                al_cassa: 4.0,
                importo_contributo_cassa: 40.0,
                imponibile_cassa: Some(1000.0),
                aliquota_iva: 22.0,
                ritenuta: None,
                natura: None,
            }]),
            dati_bollo: Some(DatiBolloDetail {
                bollo_virtuale: "SI".to_string(),
                importo_bollo: Some(2.0),
            }),
            dati_ritenuta: Some(vec![DatiRitenutaDetail {
                tipo_ritenuta: "RT01".to_string(),
                importo_ritenuta: 200.0,
                aliquota_ritenuta: 20.0,
                causale_pagamento: "A".to_string(),
            }]),
            dati_fatture_collegate: None,
            soggetto_emittente: None,
            ..Default::default()
        };

        let invoice = create_invoice_store(&store, req).await.unwrap();

        // Imponibile = 1000 + 40 (cassa) = 1040
        // IVA 22% su 1040 = 228.80
        // Bollo = 2.00
        // Totale lordo = 1040 + 228.80 + 2.00 = 1270.80
        // Netto a pagare = 1270.80 - 200.00 (ritenuta) = 1070.80
        assert_eq!(invoice.importo_totale, Some(1270.80));
        assert_eq!(invoice.netto_a_pagare, Some(1070.80));
        assert_eq!(invoice.dati_cassa_previdenziale.len(), 1);
        assert_eq!(invoice.dati_cassa_previdenziale[0].tipo_cassa, "TC04");
        assert_eq!(invoice.dati_ritenuta.len(), 1);
        assert_eq!(invoice.dati_ritenuta[0].tipo_ritenuta, "RT01");
        assert!(invoice.dati_bollo.is_some());

        // Validate XML export builds proto with full DatiCassa, DatiRitenuta, and DatiBollo
        let xml = export_invoice_xml_store(&store, &invoice.id).await.unwrap();
        assert!(xml.contains("<DatiCassaPrevidenziale>"));
        assert!(xml.contains("<TipoCassa>TC04</TipoCassa>"));
        assert!(xml.contains("<ImportoContributoCassa>40.00</ImportoContributoCassa>"));
        assert!(xml.contains("<DatiRitenuta>"));
        assert!(xml.contains("<TipoRitenuta>RT01</TipoRitenuta>"));
        assert!(xml.contains("<AliquotaRitenuta>20.00</AliquotaRitenuta>"));
        assert!(xml.contains("<DatiBollo>"));
        assert!(xml.contains("<BolloVirtuale>SI</BolloVirtuale>"));

        // Render courtesy PDF verifying it generates cleanly with all notes and breakdown
        let pdf_bytes = render_invoice_pdf_store(&store, &invoice.id).await.unwrap();
        assert!(!pdf_bytes.is_empty());
        assert_eq!(&pdf_bytes[0..4], b"%PDF");
    }

    #[tokio::test]
    async fn test_cannot_delete_or_update_non_draft_invoice() {
        let dir = std::env::temp_dir().join(format!("eattura-inv-lock-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::storage::FileStore::open(dir.join("index.json")).unwrap();

        let (seller_id, buyer_id) = setup_test_parties(&store).await;
        let req = sample_invoice_request(&seller_id, &buyer_id, "30/2026");
        let created = create_invoice_store(&store, req).await.unwrap();

        // Directly set to non-draft state (sent) in storage
        store
            .update("invoices", &created.id, json!({"stato": "sent"}))
            .await
            .unwrap();

        // Attempt to update must fail
        let mut update_req = sample_invoice_request(&seller_id, &buyer_id, "30/2026");
        update_req.linee[0].prezzo_unitario = 999.0;
        let update_res = update_invoice_store(&store, &created.id, update_req, None, false).await;
        assert!(update_res.is_err());
        assert!(
            update_res
                .unwrap_err()
                .contains("only draft invoices are editable")
        );

        // Attempt to delete must fail
        let del_res = delete_invoice_store(&store, &created.id).await;
        assert!(del_res.is_err());
        assert!(
            del_res
                .unwrap_err()
                .contains("only draft invoices are deletable")
        );
    }

    #[tokio::test]
    async fn test_status_transitions_and_illegal_jump() {
        let dir = std::env::temp_dir().join(format!("eattura-inv-status-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::storage::FileStore::open(dir.join("index.json")).unwrap();

        let (seller_id, buyer_id) = setup_test_parties(&store).await;
        let req = sample_invoice_request(&seller_id, &buyer_id, "40/2026");
        let created = create_invoice_store(&store, req).await.unwrap();

        // Illegal: draft -> sent (must pass through validated)
        let jump_res = set_invoice_status_store(&store, &created.id, "sent").await;
        assert!(jump_res.is_err());
        assert!(
            jump_res
                .unwrap_err()
                .contains("Illegal status transition: draft → sent")
        );

        // Illegal: draft -> accepted
        let jump_res2 = set_invoice_status_store(&store, &created.id, "accepted").await;
        assert!(jump_res2.is_err());
        assert!(
            jump_res2
                .unwrap_err()
                .contains("Illegal status transition: draft → accepted")
        );

        // Now set state to `sent`
        store
            .update("invoices", &created.id, json!({"stato": "sent"}))
            .await
            .unwrap();

        // Legal: sent -> accepted
        let accepted = set_invoice_status_store(&store, &created.id, "accepted")
            .await
            .unwrap();
        assert_eq!(accepted.stato, "accepted");

        // Illegal: accepted -> draft (cannot revert terminal state)
        let back_res = set_invoice_status_store(&store, &created.id, "draft").await;
        assert!(back_res.is_err());
        assert!(
            back_res
                .unwrap_err()
                .contains("Illegal status transition: accepted → draft")
        );
    }

    #[tokio::test]
    async fn archived_vat_warning_uses_the_original_xml_after_reopening_an_older_catalog() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("index.json");
        let store = crate::storage::FileStore::open(path.clone()).unwrap();
        let (seller, buyer) = setup_test_parties(&store).await;
        let invoice = create_invoice_store(&store, sample_invoice_request(&seller, &buyer, "1"))
            .await
            .unwrap();
        let mut document = build_fattura_from_store(&store, &invoice.id).await.unwrap();
        document.header.as_mut().unwrap().cessionario_committente.as_mut().unwrap()
            .dati_anagrafici.as_mut().unwrap().id_fiscale_iva.as_mut().unwrap()
            .id_codice = "09876543210".into();
        let xml = common::xml::encode::encode(&document).unwrap();
        store.update("invoices", &invoice.id, json!({"stato": "imported", "xml_content": xml}))
            .await.unwrap();
        let listed = list_invoices_store(&store).await.unwrap();
        assert_eq!(listed[0].tax_id_errors.len(), 1);
        assert_eq!(listed[0].tax_id_errors[0].code, "00305");
        drop(store);

        let mut catalog: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        catalog["invoices"]["entities"][&invoice.id].as_object_mut().unwrap().remove("tax_id_errors");
        let legacy_catalog = serde_json::to_vec(&catalog).unwrap();
        std::fs::write(&path, &legacy_catalog).unwrap();

        let reopened = crate::storage::FileStore::open(path.clone()).unwrap();
        let listed = list_invoices_store(&reopened).await.unwrap();
        assert_eq!(listed[0].tax_id_errors[0].code, "00305");
        let detail = fetch_invoice_detail(&reopened, &invoice.id).await.unwrap();
        assert_eq!(detail.stato, "imported");
        assert_eq!(detail.tax_id_errors[0].code, "00305");
        assert!(!validate_invoice_store(&reopened, &invoice.id).await.unwrap().valid);
        assert!(set_invoice_status_store(&reopened, &invoice.id, "validated").await.is_err());
        assert_eq!(std::fs::read(path).unwrap(), legacy_catalog);
    }

    #[tokio::test]
    async fn test_render_invoice_pdf() {
        let temp = std::env::temp_dir().join(format!("test_render_pdf_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        let store = crate::storage::FileStore::open(temp.join("eattura.json")).unwrap();

        let sample_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:FatturaElettronica versione="FPR12" xmlns:p="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2">
  <FatturaElettronicaHeader>
    <DatiTrasmissione><IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdTrasmittente><ProgressivoInvio>00001</ProgressivoInvio><FormatoTrasmissione>FPR12</FormatoTrasmissione><CodiceDestinatario>0000000</CodiceDestinatario></DatiTrasmissione>
    <CedentePrestatore><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Alpha Srl</Denominazione></Anagrafica><RegimeFiscale>RF01</RegimeFiscale></DatiAnagrafici><Sede><Indirizzo>Via Roma 1</Indirizzo><CAP>00100</CAP><Comune>Roma</Comune><Nazione>IT</Nazione></Sede></CedentePrestatore>
    <CessionarioCommittente><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>09876543210</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Beta Spa</Denominazione></Anagrafica></DatiAnagrafici><Sede><Indirizzo>Via Milano 2</Indirizzo><CAP>20100</CAP><Comune>Milano</Comune><Nazione>IT</Nazione></Sede></CessionarioCommittente>
  </FatturaElettronicaHeader>
  <FatturaElettronicaBody>
    <DatiGenerali><DatiGeneraliDocumento><TipoDocumento>TD01</TipoDocumento><Divisa>EUR</Divisa><Data>2026-01-15</Data><Numero>1</Numero><ImportoTotaleDocumento>122.00</ImportoTotaleDocumento></DatiGeneraliDocumento></DatiGenerali>
    <DatiBeniServizi><DettaglioLinee><NumeroLinea>1</NumeroLinea><Descrizione>Consulenza</Descrizione><Quantita>1.00</Quantita><PrezzoUnitario>100.00</PrezzoUnitario><PrezzoTotale>100.00</PrezzoTotale><AliquotaIVA>22.00</AliquotaIVA></DettaglioLinee><DatiRiepilogo><AliquotaIVA>22.00</AliquotaIVA><ImponibileImporto>100.00</ImponibileImporto><Imposta>22.00</Imposta></DatiRiepilogo></DatiBeniServizi>
  </FatturaElettronicaBody>
</p:FatturaElettronica>"#;

        let detail = import_xml_content(&store, sample_xml).await.unwrap();
        let pdf_bytes = render_invoice_pdf_store(&store, &detail.id).await.unwrap();
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-"));

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[tokio::test]
    async fn test_import_and_sign_p7m() {
        let temp = std::env::temp_dir().join(format!("test_p7m_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        let store = crate::storage::FileStore::open(temp.join("eattura.json")).unwrap();

        let sample_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:FatturaElettronica versione="FPR12" xmlns:p="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2">
  <FatturaElettronicaHeader>
    <DatiTrasmissione><IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdTrasmittente><ProgressivoInvio>00002</ProgressivoInvio><FormatoTrasmissione>FPR12</FormatoTrasmissione><CodiceDestinatario>0000000</CodiceDestinatario></DatiTrasmissione>
    <CedentePrestatore><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Alpha Srl</Denominazione></Anagrafica><RegimeFiscale>RF01</RegimeFiscale></DatiAnagrafici><Sede><Indirizzo>Via Roma 1</Indirizzo><CAP>00100</CAP><Comune>Roma</Comune><Nazione>IT</Nazione></Sede></CedentePrestatore>
    <CessionarioCommittente><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>09876543210</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Beta Spa</Denominazione></Anagrafica></DatiAnagrafici><Sede><Indirizzo>Via Milano 2</Indirizzo><CAP>20100</CAP><Comune>Milano</Comune><Nazione>IT</Nazione></Sede></CessionarioCommittente>
  </FatturaElettronicaHeader>
  <FatturaElettronicaBody>
    <DatiGenerali><DatiGeneraliDocumento><TipoDocumento>TD01</TipoDocumento><Divisa>EUR</Divisa><Data>2026-01-16</Data><Numero>2</Numero><ImportoTotaleDocumento>122.00</ImportoTotaleDocumento></DatiGeneraliDocumento></DatiGenerali>
    <DatiBeniServizi><DettaglioLinee><NumeroLinea>1</NumeroLinea><Descrizione>Consulenza</Descrizione><Quantita>1.00</Quantita><PrezzoUnitario>100.00</PrezzoUnitario><PrezzoTotale>100.00</PrezzoTotale><AliquotaIVA>22.00</AliquotaIVA></DettaglioLinee><DatiRiepilogo><AliquotaIVA>22.00</AliquotaIVA><ImponibileImporto>100.00</ImponibileImporto><Imposta>22.00</Imposta></DatiRiepilogo></DatiBeniServizi>
  </FatturaElettronicaBody>
</p:FatturaElettronica>"#;

        let detail = import_xml_content(&store, sample_xml).await.unwrap();
        assert!(!detail.is_signed);

        let mut p7m_envelope = vec![0x30, 0x82, 0x05, 0x00];
        p7m_envelope.extend_from_slice(sample_xml.as_bytes());
        let (_, info) = common::p7m::inspect_p7m(&p7m_envelope).unwrap();
        assert!(info.is_signed);

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[tokio::test]
    async fn imported_invoice_without_declared_total_uses_vat_summaries() {
        let temp = tempfile::tempdir().unwrap();
        let store = crate::storage::FileStore::open(temp.path().join("eattura.json")).unwrap();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:FatturaElettronica versione="FPR12" xmlns:p="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2">
  <FatturaElettronicaHeader>
    <DatiTrasmissione><IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdTrasmittente><ProgressivoInvio>00004</ProgressivoInvio><FormatoTrasmissione>FPR12</FormatoTrasmissione><CodiceDestinatario>0000000</CodiceDestinatario></DatiTrasmissione>
    <CedentePrestatore><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Alpha Srl</Denominazione></Anagrafica><RegimeFiscale>RF01</RegimeFiscale></DatiAnagrafici><Sede><Indirizzo>Via Roma 1</Indirizzo><CAP>00100</CAP><Comune>Roma</Comune><Nazione>IT</Nazione></Sede></CedentePrestatore>
    <CessionarioCommittente><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>09876543210</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Beta Spa</Denominazione></Anagrafica></DatiAnagrafici><Sede><Indirizzo>Via Milano 2</Indirizzo><CAP>20100</CAP><Comune>Milano</Comune><Nazione>IT</Nazione></Sede></CessionarioCommittente>
  </FatturaElettronicaHeader>
  <FatturaElettronicaBody>
    <DatiGenerali><DatiGeneraliDocumento><TipoDocumento>TD01</TipoDocumento><Divisa>EUR</Divisa><Data>2026-01-18</Data><Numero>4</Numero></DatiGeneraliDocumento></DatiGenerali>
    <DatiBeniServizi><DettaglioLinee><NumeroLinea>1</NumeroLinea><Descrizione>Consulenza</Descrizione><Quantita>1.00</Quantita><PrezzoUnitario>100.00</PrezzoUnitario><PrezzoTotale>100.00</PrezzoTotale><AliquotaIVA>22.00</AliquotaIVA></DettaglioLinee><DatiRiepilogo><AliquotaIVA>22.00</AliquotaIVA><ImponibileImporto>100.00</ImponibileImporto><Imposta>22.00</Imposta></DatiRiepilogo></DatiBeniServizi>
  </FatturaElettronicaBody>
</p:FatturaElettronica>"#;

        let receipt = "<RicevutaConsegna><IdentificativoSdI>12345</IdentificativoSdI><NomeFile>IT01234567890_00004.xml</NomeFile></RicevutaConsegna>";
        let parsed_receipt = common::pec::parse_notification(receipt).unwrap();
        let unlinked =
            crate::commands::pec::attach_sdi_notification(&store, &parsed_receipt, receipt, None)
                .await
                .unwrap();
        assert_eq!(unlinked.0, None);

        let detail = import_xml_content(&store, xml).await.unwrap();
        assert_eq!(detail.importo_totale, Some(122.0));
        assert_eq!(detail.stato, "delivered");
        assert!(detail.total_calculated);
        let listed = list_invoices_store(&store).await.unwrap();
        assert_eq!(listed[0].importo_totale, Some(122.0));
        assert!(listed[0].total_calculated);
        let linked = store
            .read("sdi_notifications", "all", vec![], "")
            .await
            .fetch_all()
            .await
            .unwrap();
        assert_eq!(linked.len(), 1);
        assert_eq!(
            linked[0].get::<Option<String>>("invoice_id"),
            Some(detail.id.clone())
        );
        crate::commands::pec::attach_sdi_notification(&store, &parsed_receipt, receipt, None)
            .await
            .unwrap();
        assert_eq!(
            store
                .read("sdi_notifications", "all", vec![], "")
                .await
                .fetch_all()
                .await
                .unwrap()
                .len(),
            1
        );

        let retransmitted = xml
            .replace("<ProgressivoInvio>00004", "<ProgressivoInvio>00005")
            .replace("<Data>2026-01-18", "<Data>2026-02-18");
        let same_document = import_xml_content(&store, &retransmitted).await.unwrap();
        assert_eq!(same_document.id, detail.id);
        assert_eq!(list_invoices_store(&store).await.unwrap().len(), 1);
        assert_eq!(
            crate::commands::pec::find_invoice_by_sdi_reference(&store, "IT01234567890_00005.xml")
                .await
                .unwrap(),
            Some(detail.id.clone())
        );

        // Simulate an invoice imported before the inferred amount was stored.
        store
            .update(
                "invoices",
                &detail.id,
                serde_json::json!({
                    "importo_totale_calcolato": null,
                    "netto_a_pagare": null
                }),
            )
            .await
            .unwrap();
        let backfilled = list_invoices_store(&store).await.unwrap();
        assert_eq!(backfilled[0].importo_totale, Some(122.0));
        assert_eq!(
            fetch_invoice_detail(&store, &detail.id)
                .await
                .unwrap()
                .netto_a_pagare,
            Some(122.0)
        );

        let reopened = crate::storage::FileStore::open(temp.path().join("eattura.json")).unwrap();
        let persisted = list_invoices_store(&reopened).await.unwrap();
        assert_eq!(persisted[0].importo_totale, Some(122.0));
        assert!(persisted[0].total_calculated);

        let exported = export_invoice_xml_store(&store, &detail.id).await.unwrap();
        assert!(!exported.contains("ImportoTotaleDocumento"));

        let body = xml
            .split("<FatturaElettronicaBody>")
            .nth(1)
            .unwrap()
            .split("</FatturaElettronicaBody>")
            .next()
            .unwrap();
        let second_body = format!(
            "<FatturaElettronicaBody>{}</FatturaElettronicaBody>",
            body.replace("<Numero>4</Numero>", "<Numero>5</Numero>")
        );
        let batch = xml.replace(
            "</p:FatturaElettronica>",
            &format!("{second_body}</p:FatturaElettronica>"),
        );
        import_xml_content(&store, &batch).await.unwrap();
        let receipts = store
            .read("sdi_notifications", "all", vec![], "")
            .await
            .fetch_all()
            .await
            .unwrap();
        assert_eq!(receipts.len(), 2);
        assert_ne!(
            receipts[0].get::<Option<String>>("invoice_id"),
            receipts[1].get::<Option<String>>("invoice_id")
        );

        store
            .update(
                "invoices",
                &detail.id,
                serde_json::json!({"stato":"rejected"}),
            )
            .await
            .unwrap();
        let corrected = xml
            .replace("<ProgressivoInvio>00004", "<ProgressivoInvio>00006")
            .replace(
                "<Descrizione>Consulenza</Descrizione>",
                "<Descrizione>Consulenza corretta</Descrizione>",
            );
        let reissued = import_xml_content(&store, &corrected).await.unwrap();
        assert_ne!(reissued.id, detail.id);
    }

    #[tokio::test]
    async fn test_sdi_receipt_and_rectify() {
        let temp = std::env::temp_dir().join(format!("test_sdi_notif_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        let store = crate::storage::FileStore::open(temp.join("eattura.json")).unwrap();

        let sample_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:FatturaElettronica versione="FPR12" xmlns:p="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2">
  <FatturaElettronicaHeader>
    <DatiTrasmissione><IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdTrasmittente><ProgressivoInvio>00003</ProgressivoInvio><FormatoTrasmissione>FPR12</FormatoTrasmissione><CodiceDestinatario>0000000</CodiceDestinatario></DatiTrasmissione>
    <CedentePrestatore><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Alpha Srl</Denominazione></Anagrafica><RegimeFiscale>RF01</RegimeFiscale></DatiAnagrafici><Sede><Indirizzo>Via Roma 1</Indirizzo><CAP>00100</CAP><Comune>Roma</Comune><Nazione>IT</Nazione></Sede></CedentePrestatore>
    <CessionarioCommittente><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>09876543210</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Beta Spa</Denominazione></Anagrafica></DatiAnagrafici><Sede><Indirizzo>Via Milano 2</Indirizzo><CAP>20100</CAP><Comune>Milano</Comune><Nazione>IT</Nazione></Sede></CessionarioCommittente>
  </FatturaElettronicaHeader>
  <FatturaElettronicaBody>
    <DatiGenerali><DatiGeneraliDocumento><TipoDocumento>TD01</TipoDocumento><Divisa>EUR</Divisa><Data>2026-01-17</Data><Numero>3</Numero><ImportoTotaleDocumento>122.00</ImportoTotaleDocumento></DatiGeneraliDocumento></DatiGenerali>
    <DatiBeniServizi><DettaglioLinee><NumeroLinea>1</NumeroLinea><Descrizione>Consulenza</Descrizione><Quantita>1.00</Quantita><PrezzoUnitario>100.00</PrezzoUnitario><PrezzoTotale>100.00</PrezzoTotale><AliquotaIVA>22.00</AliquotaIVA></DettaglioLinee><DatiRiepilogo><AliquotaIVA>22.00</AliquotaIVA><ImponibileImporto>100.00</ImponibileImporto><Imposta>22.00</Imposta></DatiRiepilogo></DatiBeniServizi>
  </FatturaElettronicaBody>
</p:FatturaElettronica>"#;

        let detail = import_xml_content(&store, sample_xml).await.unwrap();
        assert_eq!(detail.notifications.len(), 0);

        // Mark invoice as sent first
        store
            .update("invoices", &detail.id, json!({"stato": "sent"}))
            .await
            .unwrap();

        // Process a B2B/B2C rejection through the same path as a PEC sync.
        let scarto_xml = r#"<sdi:RicevutaScarto versione="1.0" xmlns:sdi="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fattura/messaggi/v1.0">
            <IdentificativoSdI>777888</IdentificativoSdI>
            <NomeFile>IT01234567890_00003.xml</NomeFile>
            <DataOraRicezione>2026-01-18T10:00:00</DataOraRicezione>
            <ListaErrori>
                <Errore>
                    <Codice>00400</Codice>
                    <Descrizione>Natura mancante</Descrizione>
                    <Suggerimento>Indicare la Natura per aliquota IVA pari a zero</Suggerimento>
                </Errore>
            </ListaErrori>
        </sdi:RicevutaScarto>"#;

        let messages = [crate::pec::PecMessage {
            uid: 1,
            subject: "Ricevuta di scarto".into(),
            from: "sdi01@pec.fatturapa.it".into(),
            sdi_sender: Some("sdi01@pec.fatturapa.it".into()),
            sdi_fallback: None,
            attachments: vec![crate::pec::PecAttachment {
                filename: "IT01234567890_00003_NS_001.xml".into(),
                data: scarto_xml.as_bytes().to_vec(),
            }],
        }];
        let (summaries, _) = crate::commands::pec::process_inbox_messages(&store, &messages)
            .await
            .unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].invoice_id.as_deref(), Some(detail.id.as_str()));
        assert_eq!(summaries[0].new_status.as_deref(), Some("rejected"));

        // Fetch invoice detail: should now be rejected and contain 1 notification
        let updated = fetch_invoice_detail(&store, &detail.id).await.unwrap();
        assert_eq!(updated.stato, "rejected");
        assert_eq!(updated.notifications.len(), 1);
        assert_eq!(updated.notifications[0].tipo, "NS");
        assert_eq!(updated.notifications[0].errori.len(), 1);
        assert_eq!(updated.notifications[0].errori[0].codice, "00400");
        assert_eq!(
            updated.notifications[0].errori[0].suggerimento.as_deref(),
            Some("Indicare la Natura per aliquota IVA pari a zero")
        );

        // Rectify rejected invoice: resets status back to draft for 5-day correction
        let rectified = rectify_rejected_invoice_store(&store, &detail.id)
            .await
            .unwrap();
        assert_eq!(rectified.stato, "draft");

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[tokio::test]
    async fn test_autofattura_td17_and_td16_lifecycle() {
        let temp = std::env::temp_dir().join(format!("test_autofattura_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        let store = crate::storage::FileStore::open(temp.join("eattura.json")).unwrap();

        // 1. Create a US vendor (Google LLC) and Italian buyer (MyCompany SRL)
        store
            .insert(
                "clients",
                json!({
                    "id": "foreign_vendor",
                    "id_paese": "US",
                    "id_codice": "99999999999",
                    "denominazione": "Google Cloud EMEA Ltd",
                    "regime_fiscale": "RF01",
                    "indirizzo": "1600 Amphitheatre Parkway",
                    "cap": "94043",
                    "comune": "Mountain View",
                    "nazione": "US"
                }),
            )
            .await
            .unwrap();

        store
            .insert(
                "clients",
                json!({
                    "id": "buyer_company",
                    "id_paese": "IT",
                    "id_codice": "01234567897",
                    "denominazione": "MyCompany Srl",
                    "regime_fiscale": "RF01",
                    "indirizzo": "Via Roma 10",
                    "cap": "00100",
                    "comune": "Roma",
                    "nazione": "IT",
                    "codice_destinatario": "M5UXCR1"
                }),
            )
            .await
            .unwrap();

        // 2. Register an incoming purchase invoice from foreign vendor (e.g. $100 software service)
        let purchase_req = CreateInvoiceRequest {
            numero: "INV-US-2026-001".to_string(),
            data: "2026-02-01".to_string(),
            tipo_documento: "TD01".to_string(),
            divisa: "EUR".to_string(),
            cedente_id: "foreign_vendor".to_string(),
            cessionario_id: "buyer_company".to_string(),
            causale: vec!["Google Cloud monthly subscription".to_string()],
            linee: vec![CreateLineRequest {
                descrizione: "Cloud hosting and Kubernetes compute".to_string(),
                quantita: Some(1.0),
                unita_misura: Some("mese".to_string()),
                fields: Default::default(),
                data_inizio_periodo: None,
                data_fine_periodo: None,
                prezzo_unitario: 100.0,
                aliquota_iva: 0.0,
                natura: Some("N2.2".to_string()),
            }],
            pagamenti: vec![CreatePaymentRequest {
                condizioni_pagamento: None,
                modalita_pagamento: "MP08".to_string(), // Carta di credito
                importo_pagamento: 100.0,
                data_scadenza_pagamento: None,
                iban: None,
            }],
            dati_ritenuta: None,
            dati_bollo: None,
            dati_cassa_previdenziale: None,
            dati_fatture_collegate: None,
            soggetto_emittente: None,
            ..Default::default()
        };

        let purchase_inv = create_invoice_store(&store, purchase_req).await.unwrap();

        // 3. Generate autofattura / integration draft (TD17) from this purchase
        let auto_draft = generate_autofattura_draft_store(&store, &purchase_inv.id, None)
            .await
            .unwrap();

        // Check autofattura draft contents
        assert_eq!(auto_draft.tipo_documento, "TD17");
        assert_eq!(auto_draft.cedente_id, "foreign_vendor");
        assert_eq!(auto_draft.cessionario_id, "buyer_company");
        assert_eq!(auto_draft.soggetto_emittente.as_deref(), Some("CC"));
        assert!(auto_draft.dati_fatture_collegate.is_some());
        let collegate = auto_draft.dati_fatture_collegate.as_ref().unwrap();
        assert_eq!(collegate.len(), 1);
        assert_eq!(collegate[0].id_documento, "INV-US-2026-001");
        assert_eq!(collegate[0].data.as_deref(), Some("2026-02-01"));

        // Check integrated line items: 22% VAT applied, total 122.00
        assert_eq!(auto_draft.linee.len(), 1);
        assert_eq!(auto_draft.linee[0].aliquota_iva, 22.0);
        assert_eq!(auto_draft.linee[0].natura, None);

        // 4. Create and persist the autofattura
        let created_auto = create_invoice_store(&store, auto_draft).await.unwrap();
        assert!(created_auto.is_autofattura);
        assert_eq!(created_auto.tipo_documento, "TD17");
        assert_eq!(created_auto.soggetto_emittente.as_deref(), Some("CC"));
        assert_eq!(created_auto.importo_totale, Some(122.0));
        assert_eq!(created_auto.dati_fatture_collegate.len(), 1);

        // 5. Test XML serialization and SDI validation
        let xml_str = export_invoice_xml_store(&store, &created_auto.id)
            .await
            .unwrap();
        assert!(xml_str.contains("<TipoDocumento>TD17</TipoDocumento>"));
        assert!(xml_str.contains("<SoggettoEmittente>CC</SoggettoEmittente>"));
        assert!(xml_str.contains("<DatiFattureCollegate>"));
        assert!(xml_str.contains("<IdDocumento>INV-US-2026-001</IdDocumento>"));
        // Trasmittente is Cessionario (IT), not foreign Cedente (US)
        assert!(xml_str.contains(
            "<IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>01234567897</IdCodice></IdTrasmittente>"
        ));

        let val_res = validate_invoice_store(&store, &created_auto.id)
            .await
            .unwrap();
        assert!(
            val_res.valid,
            "TD17 autofattura must pass SDI validation: {:?}",
            val_res.errors
        );

        // 6. Test PDF courtesy rendering
        let pdf_bytes = render_invoice_pdf_store(&store, &created_auto.id)
            .await
            .unwrap();
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-"));

        let _ = std::fs::remove_dir_all(&temp);
    }
    #[tokio::test]
    async fn b2g_pa_invoice_lifecycle_and_split_payment_test() {
        let temp = std::env::temp_dir().join(format!("eattura-b2g-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        let store = crate::storage::FileStore::open(temp.join("data.json")).unwrap();

        // 1. Create seller (private Italian SME) and buyer (Public Administration with 6-char IPA code)
        let seller_id = "seller_sme";
        let buyer_pa_id = "buyer_pa_entity";

        store
            .insert(
                "clients",
                serde_json::json!({
                    "id": seller_id,
                    "denominazione": "Azienda Fornitrice SRL",
                    "id_paese": "IT",
                    "id_codice": "01234567897",
                    "regime_fiscale": "RF01",
                    "indirizzo": "Via Roma 10",
                    "cap": "00100",
                    "comune": "Roma",
                    "provincia": "RM",
                    "nazione": "IT",
                    "codice_destinatario": "0000000",
                    "is_pa": false,
                }),
            )
            .await
            .unwrap();

        store
            .insert(
                "clients",
                serde_json::json!({
                    "id": buyer_pa_id,
                    "denominazione": "Comune di Test",
                    "id_paese": "IT",
                    "id_codice": "98765432103",
                    "codice_fiscale": "98765432103",
                    "indirizzo": "Piazza del Comune 1",
                    "cap": "00100",
                    "comune": "Roma",
                    "provincia": "RM",
                    "nazione": "IT",
                    "codice_destinatario": "UF12AB", // 6 chars IPA code
                    "is_pa": true,
                }),
            )
            .await
            .unwrap();

        // 2. Create B2G invoice with Split Payment and CIG / CUP procurement codes
        let pa_invoice_req = CreateInvoiceRequest {
            numero: "PA/2026/001".to_string(),
            data: "2026-03-01".to_string(),
            tipo_documento: "TD01".to_string(),
            divisa: "EUR".to_string(),
            cedente_id: seller_id.to_string(),
            cessionario_id: buyer_pa_id.to_string(),
            causale: vec!["Fornitura arredi scolastici - Determina dirigenziale N. 42".to_string()],
            linee: vec![CreateLineRequest {
                descrizione: "Banchi modulari e sedie ergonomiche".to_string(),
                quantita: Some(10.0),
                unita_misura: Some("PZ".to_string()),
                fields: Default::default(),
                data_inizio_periodo: None,
                data_fine_periodo: None,
                prezzo_unitario: 100.0, // 1000.00
                aliquota_iva: 22.0,     // 220.00 VAT
                natura: None,
            }],
            pagamenti: vec![CreatePaymentRequest {
                condizioni_pagamento: None,
                modalita_pagamento: "MP05".to_string(),
                importo_pagamento: 1000.00, // Netto a pagare = 1000 (taxable base), VAT remitted directly to State
                data_scadenza_pagamento: Some("2026-04-30".to_string()),
                iban: Some("IT60X0542811101000000123456".to_string()),
            }],
            is_pa: Some(true),
            is_split_payment: Some(true),
            esigibilita_iva: Some("S".to_string()),
            dati_ordine_acquisto: Some(vec![DatiDocumentoTracciabilitaDetail {
                tipo_documento_rif: Some("ordine".into()),
                id_documento: "DET-2026-42".to_string(),
                data: Some("2026-02-15".to_string()),
                num_item: None,
                codice_commessa_convenzione: None,
                codice_cup: Some("B12C34000050006".to_string()), // 15 chars
                codice_cig: Some("1234567890".to_string()),      // 10 chars
            }]),
            ..Default::default()
        };

        let created = create_invoice_store(&store, pa_invoice_req).await.unwrap();

        // Verify financial calculations:
        // Gross: 1220.00, Split Payment VAT: 220.00, Net payable to supplier: 1000.00
        assert_eq!(created.importo_totale, Some(1220.00));
        assert_eq!(created.netto_a_pagare, Some(1000.00));
        assert!(created.is_pa);
        assert!(created.is_split_payment);
        assert_eq!(created.esigibilita_iva.as_deref(), Some("S"));
        assert_eq!(created.formato_trasmissione.as_deref(), Some("FPA12"));
        assert_eq!(created.dati_ordine_acquisto.len(), 1);
        assert_eq!(
            created.dati_ordine_acquisto[0].codice_cig.as_deref(),
            Some("1234567890")
        );
        assert_eq!(
            created.dati_ordine_acquisto[0].codice_cup.as_deref(),
            Some("B12C34000050006")
        );

        // 3. Validate the PA invoice and its procurement data.
        let val_res = validate_invoice_store(&store, &created.id).await.unwrap();
        assert!(
            val_res.valid,
            "PA invoice must pass all SDI validation rules: {:?}",
            val_res.errors
        );

        // 4. Export XML and verify tags
        let xml_str = export_invoice_xml_store(&store, &created.id).await.unwrap();
        assert!(xml_str.contains("<FormatoTrasmissione>FPA12</FormatoTrasmissione>"));
        assert!(xml_str.contains("<CodiceDestinatario>UF12AB</CodiceDestinatario>"));
        assert!(xml_str.contains("<EsigibilitaIVA>S</EsigibilitaIVA>"));
        assert!(xml_str.contains("<DatiOrdineAcquisto>"));
        assert!(xml_str.contains("<IdDocumento>DET-2026-42</IdDocumento>"));
        assert!(xml_str.contains("<CodiceCIG>1234567890</CodiceCIG>"));
        assert!(xml_str.contains("<CodiceCUP>B12C34000050006</CodiceCUP>"));

        // 5. Test PDF cortesia rendering
        let pdf_bytes = render_invoice_pdf_store(&store, &created.id).await.unwrap();
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-"));

        let _ = std::fs::remove_dir_all(&temp);
    }
    #[tokio::test]
    async fn test_ddt_and_allegati_lifecycle() {
        let temp = std::env::temp_dir().join(format!(
            "eattura-ddt-allegati-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp).unwrap();
        let store = crate::storage::FileStore::open(temp.join("data.json")).unwrap();

        let seller_id = "seller_ddt";
        let buyer_id = "buyer_ddt";

        store
            .insert(
                "clients",
                serde_json::json!({
                    "id": seller_id,
                    "denominazione": "Grossista Materiali SPA",
                    "id_paese": "IT",
                    "id_codice": "01234567897",
                    "regime_fiscale": "RF01",
                    "indirizzo": "Via Merci 1",
                    "cap": "20100",
                    "comune": "Milano",
                    "provincia": "MI",
                    "nazione": "IT",
                    "codice_destinatario": "0000000",
                }),
            )
            .await
            .unwrap();

        store
            .insert(
                "clients",
                serde_json::json!({
                    "id": buyer_id,
                    "denominazione": "Cliente Negozio SRL",
                    "id_paese": "IT",
                    "id_codice": "09876543217",
                    "indirizzo": "Corso Commercio 20",
                    "cap": "10100",
                    "comune": "Torino",
                    "provincia": "TO",
                    "nazione": "IT",
                    "codice_destinatario": "M5UXCR1",
                }),
            )
            .await
            .unwrap();

        let dummy_pdf_content = b"%PDF-1.4 dummy attached delivery note binary content";
        let b64_attachment = base64::engine::general_purpose::STANDARD.encode(dummy_pdf_content);

        let req = CreateInvoiceRequest {
            numero: "DIFF-2026/01".to_string(),
            data: "2026-03-31".to_string(),
            tipo_documento: "TD24".to_string(),
            divisa: "EUR".to_string(),
            cedente_id: seller_id.to_string(),
            cessionario_id: buyer_id.to_string(),
            causale: vec!["Riepilogo consegne del mese di Marzo 2026".to_string()],
            linee: vec![CreateLineRequest {
                descrizione: "Fornitura merci varie da DDT 45 e DDT 46".to_string(),
                quantita: Some(5.0),
                unita_misura: Some("PZ".to_string()),
                fields: Default::default(),
                data_inizio_periodo: None,
                data_fine_periodo: None,
                prezzo_unitario: 100.0,
                aliquota_iva: 22.0,
                natura: None,
            }],
            pagamenti: vec![CreatePaymentRequest {
                condizioni_pagamento: None,
                modalita_pagamento: "MP05".to_string(),
                importo_pagamento: 610.0,
                data_scadenza_pagamento: Some("2026-04-30".to_string()),
                iban: None,
            }],
            dati_ddt: Some(vec![
                DatiDdtDetail {
                    numero_ddt: "DDT-45".to_string(),
                    data_ddt: "2026-03-10".to_string(),
                    riferimento_numero_linea: vec![1],
                },
                DatiDdtDetail {
                    numero_ddt: "DDT-46".to_string(),
                    data_ddt: "2026-03-20".to_string(),
                    riferimento_numero_linea: vec![1],
                },
            ]),
            allegati: Some(vec![CreateAttachmentRequest {
                nome_attachment: "DDT_45_firmato.pdf".to_string(),
                algoritmo_compressione: None,
                formato_attachment: Some("PDF".to_string()),
                descrizione_attachment: Some("Scansione copia firmata DDT 45".to_string()),
                attachment_base64: b64_attachment.clone(),
            }]),
            ..Default::default()
        };

        // 1. Create and verify in-memory & file store state
        let created = create_invoice_store(&store, req).await.unwrap();
        assert_eq!(created.tipo_documento, "TD24");
        assert_eq!(created.dati_ddt.len(), 2);
        assert_eq!(created.dati_ddt[0].numero_ddt, "DDT-45");
        assert_eq!(created.dati_ddt[0].data_ddt, "2026-03-10");
        assert_eq!(created.dati_ddt[0].riferimento_numero_linea, vec![1]);
        assert_eq!(created.dati_ddt[1].numero_ddt, "DDT-46");

        assert_eq!(created.allegati.len(), 1);
        assert_eq!(created.allegati[0].nome_attachment, "DDT_45_firmato.pdf");
        assert_eq!(
            created.allegati[0].formato_attachment.as_deref(),
            Some("PDF")
        );
        assert_eq!(created.allegati[0].size_bytes, dummy_pdf_content.len());

        // 2. Export XML and verify compliant tags
        let xml_str = export_invoice_xml_store(&store, &created.id).await.unwrap();
        assert!(xml_str.contains("<TipoDocumento>TD24</TipoDocumento>"));
        assert!(xml_str.contains("<DatiDDT>"));
        assert!(xml_str.contains("<NumeroDDT>DDT-45</NumeroDDT>"));
        assert!(xml_str.contains("<DataDDT>2026-03-10</DataDDT>"));
        assert!(xml_str.contains("<RiferimentoNumeroLinea>1</RiferimentoNumeroLinea>"));
        assert!(xml_str.contains("<NumeroDDT>DDT-46</NumeroDDT>"));
        assert!(xml_str.contains("<DataDDT>2026-03-20</DataDDT>"));
        assert!(xml_str.contains("<Allegati>"));
        assert!(xml_str.contains("<NomeAttachment>DDT_45_firmato.pdf</NomeAttachment>"));
        assert!(xml_str.contains("<FormatoAttachment>PDF</FormatoAttachment>"));
        assert!(xml_str.contains(
            "<DescrizioneAttachment>Scansione copia firmata DDT 45</DescrizioneAttachment>"
        ));
        assert!(xml_str.contains(&b64_attachment));

        // 3. Import this XML in a completely fresh store and verify persistence of DDT and Allegati
        let temp2 =
            std::env::temp_dir().join(format!("eattura-ddt-allegati-imp-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp2).unwrap();
        let store2 = crate::storage::FileStore::open(temp2.join("data.json")).unwrap();

        let imported = import_xml_content(&store2, &xml_str).await.unwrap();
        assert_eq!(imported.tipo_documento, "TD24");
        assert_eq!(imported.dati_ddt.len(), 2);
        assert_eq!(imported.dati_ddt[0].numero_ddt, "DDT-45");
        assert_eq!(imported.dati_ddt[1].numero_ddt, "DDT-46");
        assert_eq!(imported.allegati.len(), 1);
        assert_eq!(imported.allegati[0].nome_attachment, "DDT_45_firmato.pdf");
        assert_eq!(imported.allegati[0].size_bytes, dummy_pdf_content.len());

        let _ = std::fs::remove_dir_all(&temp);
        let _ = std::fs::remove_dir_all(&temp2);
    }

    #[tokio::test]
    async fn test_b2c_cf_only_pec_destinatario_and_multiple_natura_riepilogo() {
        let dir = std::env::temp_dir().join(format!("eattura-b2c-pec-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::storage::FileStore::open(dir.join("index.json")).unwrap();

        let (seller_id, _) = setup_test_parties(&store).await;
        let b2c_buyer = crate::commands::clients::create_client_store(
            &store,
            crate::commands::clients::CreateClientRequest {
                denominazione: None,
                nome: Some("Mario".to_string()),
                cognome: Some("Rossi".to_string()),
                id_paese: "IT".to_string(),
                id_codice: String::new(),
                codice_fiscale: Some("RSSMRA80A01H501U".to_string()),
                regime_fiscale: None,
                indirizzo: "Via Garibaldi 10".to_string(),
                numero_civico: None,
                cap: "20100".to_string(),
                comune: "Milano".to_string(),
                provincia: Some("MI".to_string()),
                nazione: "IT".to_string(),
                telefono: None,
                email: None,
                pec: Some("mario.rossi@pec.it".to_string()),
                codice_destinatario: Some("0000000".to_string()),
                is_pa: Some(false),
            },
        )
        .await
        .unwrap();

        let mut req = sample_invoice_request(&seller_id, &b2c_buyer.id, "5/2026");
        req.linee = vec![
            CreateLineRequest {
                descrizione: "Compenso regime forfettario".to_string(),
                quantita: Some(1.0),
                unita_misura: None,
                fields: Default::default(),
                data_inizio_periodo: None,
                data_fine_periodo: None,
                prezzo_unitario: 500.0,
                aliquota_iva: 0.0,
                natura: Some("N2.2".to_string()),
            },
            CreateLineRequest {
                descrizione: "Riaddebito imposta di bollo".to_string(),
                quantita: Some(1.0),
                unita_misura: None,
                fields: Default::default(),
                data_inizio_periodo: None,
                data_fine_periodo: None,
                prezzo_unitario: 2.0,
                aliquota_iva: 0.0,
                natura: Some("N1".to_string()),
            },
        ];

        let created = create_invoice_store(&store, req).await.unwrap();
        let fattura = build_fattura_from_store(&store, &created.id).await.unwrap();
        let validation = common::validation::rules::validate(&fattura);
        assert!(
            validation.is_valid(),
            "Validation errors: {:?}",
            validation.errors
        );

        let header = fattura.header.as_ref().unwrap();
        assert_eq!(
            header
                .dati_trasmissione
                .as_ref()
                .unwrap()
                .pec_destinatario
                .as_deref(),
            Some("mario.rossi@pec.it")
        );
        let cess_da = header
            .cessionario_committente
            .as_ref()
            .unwrap()
            .dati_anagrafici
            .as_ref()
            .unwrap();
        assert!(cess_da.id_fiscale_iva.is_none());
        assert_eq!(cess_da.codice_fiscale.as_deref(), Some("RSSMRA80A01H501U"));

        let riepilogo = &fattura.body[0]
            .dati_beni_servizi
            .as_ref()
            .unwrap()
            .dati_riepilogo;
        assert_eq!(riepilogo.len(), 2);

        let next_num = next_invoice_number_store(&store, &seller_id, "2026")
            .await
            .unwrap();
        assert_eq!(next_num, "6");

        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Export a specific embedded attachment from an invoice to the filesystem.
#[tauri::command]
pub async fn export_attachment_to_file(
    state: tauri::State<'_, TauriState>,
    invoice_id: String,
    attachment_index: usize,
    target_path: String,
) -> Result<String, String> {
    let invoice = fetch_invoice_detail(&state.store, &invoice_id).await?;
    let att = invoice.allegati.get(attachment_index).ok_or_else(|| {
        format!("Attachment index {attachment_index} not found for invoice {invoice_id}")
    })?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&att.attachment_base64)
        .map_err(|e| format!("Failed to decode base64 attachment: {e}"))?;
    std::fs::write(&target_path, &bytes)
        .map_err(|e| format!("Failed to write attachment to {target_path}: {e}"))?;
    Ok(target_path)
}
