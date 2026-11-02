use axum::Json;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::AppError;
use common::pdf::generate_pdf_from_xml;
use common::sdi::v1::FatturaElettronica;
use common::validation::{ValidationError, validate as run_sdi_validation};
use common::xml::{decode, encode};

#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    pub xml: Option<String>,
    pub invoice: Option<FatturaElettronica>,
}

#[derive(Debug, Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Deserialize)]
pub struct ConvertXmlToJsonRequest {
    pub xml: String,
}

#[derive(Debug, Deserialize)]
pub struct ConvertJsonToXmlRequest {
    pub invoice: FatturaElettronica,
}

#[derive(Debug, Deserialize)]
pub struct RenderPdfRequest {
    pub xml: String,
}

/// Validate an electronic invoice (XML or JSON) against SDI rules
pub async fn validate(
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, AppError> {
    let fattura = if let Some(invoice) = req.invoice {
        invoice
    } else if let Some(xml) = req.xml {
        decode::decode(&xml)?
    } else {
        return Err(AppError::BadRequest(
            "Either 'xml' or 'invoice' must be provided".to_string(),
        ));
    };

    let result = run_sdi_validation(&fattura);
    Ok(Json(ValidateResponse {
        valid: result.is_valid(),
        errors: result.errors,
    }))
}

/// Convert SDI XML into JSON structure
pub async fn xml_to_json(
    Json(req): Json<ConvertXmlToJsonRequest>,
) -> Result<Json<FatturaElettronica>, AppError> {
    let fattura = decode::decode(&req.xml)?;
    Ok(Json(fattura))
}

/// Convert JSON structure into compliant SDI XML
pub async fn json_to_xml(
    Json(req): Json<ConvertJsonToXmlRequest>,
) -> Result<Json<Value>, AppError> {
    let xml = encode::encode(&req.invoice)?;
    Ok(Json(json!({ "xml": xml })))
}

/// Render a courtesy PDF from invoice XML
pub async fn render_pdf(Json(req): Json<RenderPdfRequest>) -> Result<Response, AppError> {
    let pdf_bytes = generate_pdf_from_xml(&req.xml)?;

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/pdf".parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        "inline; filename=\"fattura_cortesia.pdf\"".parse().unwrap(),
    );

    Ok((StatusCode::OK, headers, pdf_bytes).into_response())
}
