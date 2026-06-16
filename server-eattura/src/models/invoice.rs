use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Database row for the invoices table.
///
/// Maps to the main invoice record. Related line items and payments
/// are in separate tables with FK to this invoice's ID.
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct InvoiceRow {
    pub id: Uuid,
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub divisa: String,
    pub importo_totale: Option<f64>,
    pub cedente_id: Uuid,
    pub cessionario_id: Uuid,
    /// Invoice lifecycle: draft -> validated -> sent -> accepted | rejected
    pub stato: String,
    /// Raw XML content if imported from file
    pub xml_content: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Database row for invoice line items (DettaglioLinee).
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct InvoiceLineRow {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub numero_linea: i32,
    pub descrizione: String,
    pub quantita: Option<f64>,
    pub unita_misura: Option<String>,
    pub prezzo_unitario: f64,
    pub prezzo_totale: f64,
    pub aliquota_iva: f64,
    pub natura: Option<String>,
}

/// Database row for invoice payment details (DettaglioPagamento).
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct InvoicePaymentRow {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub condizioni_pagamento: String,
    pub modalita_pagamento: String,
    pub importo_pagamento: f64,
    pub data_scadenza_pagamento: Option<String>,
    pub iban: Option<String>,
    pub istituto_finanziario: Option<String>,
}

/// Summary view of an invoice for list endpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct InvoiceSummary {
    pub id: Uuid,
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub importo_totale: Option<f64>,
    pub cedente_denominazione: String,
    pub cessionario_denominazione: String,
    pub stato: String,
}

/// Full invoice detail including lines and payments.
#[derive(Debug, Serialize, Deserialize)]
pub struct InvoiceDetail {
    pub id: Uuid,
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub divisa: String,
    pub importo_totale: Option<f64>,
    pub cedente_denominazione: String,
    pub cessionario_denominazione: String,
    pub stato: String,
    pub causale: Vec<String>,
    pub linee: Vec<InvoiceLineDetail>,
    pub pagamenti: Vec<InvoicePaymentDetail>,
}

/// Line item detail for API responses.
#[derive(Debug, Serialize, Deserialize)]
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

/// Payment detail for API responses.
#[derive(Debug, Serialize, Deserialize)]
pub struct InvoicePaymentDetail {
    pub modalita_pagamento: String,
    pub importo_pagamento: f64,
    pub data_scadenza_pagamento: Option<String>,
    pub iban: Option<String>,
    pub istituto_finanziario: Option<String>,
}

/// Request body for creating/updating an invoice.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceRequest {
    pub numero: String,
    pub data: String,
    pub tipo_documento: String,
    pub divisa: String,
    pub importo_totale: Option<f64>,
    pub cedente_id: Uuid,
    pub cessionario_id: Uuid,
    pub causale: Vec<String>,
    pub linee: Vec<CreateInvoiceLineRequest>,
    pub pagamenti: Vec<CreateInvoicePaymentRequest>,
}

/// Request body for a single line item.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceLineRequest {
    pub numero_linea: i32,
    pub descrizione: String,
    pub quantita: Option<f64>,
    pub unita_misura: Option<String>,
    pub prezzo_unitario: f64,
    pub aliquota_iva: f64,
    pub natura: Option<String>,
}

/// Request body for a single payment detail.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoicePaymentRequest {
    pub condizioni_pagamento: String,
    pub modalita_pagamento: String,
    pub importo_pagamento: f64,
    pub data_scadenza_pagamento: Option<String>,
    pub iban: Option<String>,
    pub istituto_finanziario: Option<String>,
}
