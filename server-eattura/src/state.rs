use std::sync::Arc;
use crate::config::ServerConfig;

/// Shared application state passed to all Axum handlers via `State` extractor.
///
/// Contains the database pool and server configuration.
/// Cloneable because PgPool uses an internal Arc.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    #[allow(dead_code)] // available to handlers; not all read it yet
    pub config: Arc<ServerConfig>,
}
