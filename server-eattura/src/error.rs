use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Application-wide error type that converts to HTTP responses for the stateless server.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation failed")]
    Validation(common::validation::ValidationResult),

    #[error("XML error: {0}")]
    Xml(#[from] common::xml::XmlError),

    #[error("PDF error: {0}")]
    Pdf(#[from] common::pdf::PdfError),

    #[error("PEC error: {0}")]
    Pec(String),

    #[error("Bad gateway: {0}")]
    BadGateway(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                serde_json::json!({ "error": msg }),
            ),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                serde_json::json!({ "error": msg }),
            ),
            AppError::Conflict(msg) => (
                StatusCode::CONFLICT,
                serde_json::json!({ "error": msg }),
            ),
            AppError::Validation(result) => {
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
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    serde_json::json!({ "error": "Validation failed", "errors": errors }),
                )
            }
            AppError::Xml(err) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                serde_json::json!({ "error": err.to_string() }),
            ),
            AppError::Pdf(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({ "error": err.to_string() }),
            ),
            AppError::Pec(msg) => (
                StatusCode::BAD_GATEWAY,
                serde_json::json!({ "error": msg }),
            ),
            AppError::BadGateway(msg) => (
                StatusCode::BAD_GATEWAY,
                serde_json::json!({ "error": msg }),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({ "error": msg }),
            ),
        };

        (status, axum::Json(body)).into_response()
    }
}
