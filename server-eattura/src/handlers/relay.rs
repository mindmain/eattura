use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::AppError;
use crate::services::relay_service::RelayMessage;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PushRelayRequest {
    pub topic: String,
    pub payload: String, // Encrypted ciphertext (e.g. XChaCha20-Poly1305 / AES-256-GCM)
    pub sender_id: String,
}

/// Push an end-to-end encrypted blob to an ephemeral topic queue
pub async fn push(
    State(state): State<AppState>,
    Json(req): Json<PushRelayRequest>,
) -> Result<Json<Value>, AppError> {
    let msg_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    let message = RelayMessage {
        id: msg_id.clone(),
        topic: req.topic.clone(),
        payload: req.payload,
        sender_id: req.sender_id,
        created_at: now,
    };

    let mut relay = state.relay.write().await;
    relay.push(req.topic, message);

    Ok(Json(json!({
        "status": "queued",
        "message_id": msg_id
    })))
}

/// Pull and consume encrypted blobs for a given topic
pub async fn pull(
    State(state): State<AppState>,
    Path(topic): Path<String>,
) -> Result<Json<Value>, AppError> {
    let mut relay = state.relay.write().await;
    let messages = relay.pull(&topic);

    Ok(Json(json!({
        "topic": topic,
        "count": messages.len(),
        "messages": messages
    })))
}

/// Peek at messages without consuming them
pub async fn peek(
    State(state): State<AppState>,
    Path(topic): Path<String>,
) -> Result<Json<Value>, AppError> {
    let mut relay = state.relay.write().await;
    let messages = relay.peek(&topic);

    Ok(Json(json!({
        "topic": topic,
        "count": messages.len(),
        "messages": messages
    })))
}
