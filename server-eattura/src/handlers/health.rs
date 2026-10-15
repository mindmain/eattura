use axum::Json;
use serde_json::{json, Value};

/// Health check endpoint confirming the server is operational in stateless passacarte mode.
pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "mode": "stateless_passacarte",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
