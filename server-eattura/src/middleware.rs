//! HTTP middleware for the Eattura server.

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Header carrying the API key.
const API_KEY_HEADER: &str = "x-api-key";

/// Minimal API-key authentication.
///
/// When the `API_KEY` environment variable is set, every request (except the
/// health check) must present a matching `X-API-Key` header. When `API_KEY` is
/// unset the middleware is a no-op, which keeps local development frictionless.
pub async fn require_api_key(request: Request, next: Next) -> Response {
    let Ok(expected) = std::env::var("API_KEY") else {
        return next.run(request).await; // auth disabled
    };

    // Always allow the health probe through.
    if request.uri().path() == "/api/health" {
        return next.run(request).await;
    }

    let provided = request
        .headers()
        .get(API_KEY_HEADER)
        .and_then(|v| v.to_str().ok());

    match provided {
        Some(key) if key == expected => next.run(request).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "error": "Missing or invalid API key" })),
        )
            .into_response(),
    }
}
