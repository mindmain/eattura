use crate::config::ServerConfig;
use std::sync::Arc;

/// Shared application state for the stateless "passa-carte" server.
/// Holds server configuration. Zero database connections, zero persistent files.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}
