use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Application-wide error type that converts to HTTP responses.
///
/// Each variant maps to an appropriate HTTP status code:
/// - NotFound -> 404
/// - Validation -> 422
/// - Database -> 500
/// - Xml -> 422
/// - Internal -> 500
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation failed")]
    #[allow(dead_code)] // reserved for endpoints that surface SDI validation errors as 422
    Validation(common::validation::ValidationResult),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("XML error: {0}")]
    Xml(#[from] common::xml::XmlError),

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
            AppError::Database(err) => {
                tracing::error!("Database error: {err}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({ "error": "Internal server error" }),
                )
            }
            AppError::Xml(err) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                serde_json::json!({ "error": err.to_string() }),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({ "error": msg }),
            ),
        };

        (status, axum::Json(body)).into_response()
    }
}
