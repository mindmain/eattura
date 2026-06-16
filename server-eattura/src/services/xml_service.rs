use common::sdi::v1::FatturaElettronica;
use common::validation::ValidationResult;
use common::xml::enum_map::formato_trasmissione_to_sdi;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::invoice::InvoiceDetail;
use crate::services::invoice_service::InvoiceService;

/// Orchestrates XML import/export operations.
///
/// Coordinates between the common crate's XML codec, the invoice service,
/// and the validation engine.
pub struct XmlService {
    pub invoice_service: InvoiceService,
}

impl XmlService {
    /// Export a stored invoice as SDI XML.
    ///
    /// Steps:
    /// 1. Load invoice from DB via InvoiceService
    /// 2. Convert to FatturaElettronica proto
    /// 3. Encode to XML via common::xml::encode
    pub async fn export(&self, id: Uuid) -> Result<String, AppError> {
        let fattura = self.invoice_service.to_fattura(id).await?;
        let xml = common::xml::encode::encode(&fattura)?;

        // Persist the generated XML as an audit trail.
        sqlx::query("UPDATE invoices SET xml_content = $2, updated_at = NOW() WHERE id = $1")
            .bind(id.to_string())
            .bind(&xml)
            .execute(&self.invoice_service.db)
            .await?;

        Ok(xml)
    }

    /// Import an invoice from SDI XML.
    ///
    /// Steps:
    /// 1. Decode XML to FatturaElettronica via common::xml::decode
    /// 2. Persist via InvoiceService::from_fattura
    pub async fn import(&self, xml: &str) -> Result<InvoiceDetail, AppError> {
        let fattura = common::xml::decode::decode(xml)?;
        let detail = self.invoice_service.from_fattura(fattura).await?;
        Ok(detail)
    }

    /// Validate an invoice XML without importing.
    ///
    /// Steps:
    /// 1. Decode XML to FatturaElettronica
    /// 2. Run common::validation::rules::validate
    /// 3. Return validation result
    pub fn validate_xml(xml: &str) -> Result<ValidationResult, AppError> {
        let fattura = common::xml::decode::decode(xml)?;
        let result = common::validation::rules::validate(&fattura);
        Ok(result)
    }

    /// Validate a stored invoice against SDI rules.
    ///
    /// Loads the invoice, converts to proto, and runs validation.
    pub async fn validate_invoice(&self, id: Uuid) -> Result<ValidationResult, AppError> {
        let fattura = self.invoice_service.to_fattura(id).await?;
        let result = common::validation::rules::validate(&fattura);
        Ok(result)
    }

    /// Generate SDI-compliant filename for an invoice XML.
    ///
    /// Format: `{IdPaese}{IdCodice}_{Progressivo}.xml`
    /// Example: `IT01234567890_00001.xml`
    pub fn generate_filename(fattura: &FatturaElettronica) -> String {
        let (id_paese, id_codice, progressivo) = fattura
            .header
            .as_ref()
            .and_then(|h| h.dati_trasmissione.as_ref())
            .map(|dt| {
                let id = dt.id_trasmittente.as_ref();
                let paese = id.map(|i| i.id_paese.as_str()).unwrap_or("IT");
                let codice = id.map(|i| i.id_codice.as_str()).unwrap_or("00000000000");
                let prog = &dt.progressivo_invio;
                (paese, codice, prog.as_str())
            })
            .unwrap_or(("IT", "00000000000", "00001"));

        let _formato = fattura
            .header
            .as_ref()
            .and_then(|h| h.dati_trasmissione.as_ref())
            .map(|dt| dt.formato_trasmissione)
            .and_then(formato_trasmissione_to_sdi)
            .unwrap_or("FPA12");

        format!("{id_paese}{id_codice}_{progressivo}.xml")
    }
}
