use axum::extract::Query;
use axum::Json;
use common::vies::{normalize_vat, RawViesResponse, VatLookupResponse};
use serde::Deserialize;

use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct VatLookupQuery {
    pub country_code: String,
    pub vat_number: String,
}

/// Stateless VIES VAT lookup handler.
/// Queries the official European Commission VIES REST API and extracts
/// normalized company name and legal seat address without saving any data.
pub async fn vat_lookup(
    Query(query): Query<VatLookupQuery>,
) -> Result<Json<VatLookupResponse>, AppError> {
    if query.country_code.trim().is_empty() || query.vat_number.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Both country_code and vat_number query parameters are required".into(),
        ));
    }

    let (country, vat) = normalize_vat(&query.country_code, &query.vat_number);
    let url = format!(
        "https://ec.europa.eu/taxation_customs/vies/rest-api/ms/{country}/vat/{vat}"
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {e}")))?;

    let res = client
        .get(&url)
        .header("User-Agent", "eattura/0.1.0")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::BadGateway(format!("Failed to connect to VIES API: {e}")))?;

    let status = res.status();
    if status.is_success() {
        let raw: RawViesResponse = res
            .json()
            .await
            .map_err(|e| AppError::BadGateway(format!("Failed to parse VIES JSON response: {e}")))?;
        let lookup_resp = raw.into_vat_lookup_response(&query.country_code, &vat);
        Ok(Json(lookup_resp))
    } else {
        let body = res.text().await.unwrap_or_default();
        if let Ok(raw) = serde_json::from_str::<RawViesResponse>(&body) {
            let lookup_resp = raw.into_vat_lookup_response(&query.country_code, &vat);
            Ok(Json(lookup_resp))
        } else {
            Ok(Json(VatLookupResponse {
                valid: false,
                country_code: query.country_code.to_uppercase(),
                vat_number: vat,
                name: None,
                address: None,
                street: None,
                postal_code: None,
                city: None,
                province: None,
                user_error: Some(format!("VIES error HTTP {status}: {body}")),
            }))
        }
    }
}
