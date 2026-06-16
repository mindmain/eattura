use common::validation::ValidationResult;
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

    /// Validate a stored invoice against SDI rules.
    ///
    /// Loads the invoice, converts to proto, and runs validation.
    pub async fn validate_invoice(&self, id: Uuid) -> Result<ValidationResult, AppError> {
        let fattura = self.invoice_service.to_fattura(id).await?;
        let result = common::validation::rules::validate(&fattura);
        Ok(result)
    }
}
