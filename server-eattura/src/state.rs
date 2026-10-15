use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use crate::config::ServerConfig;
use crate::services::relay_service::RelayStore;

/// Shared application state for the stateless "passa-carte" server.
/// Holds server configuration and an in-memory ephemeral relay store for E2EE sync.
/// Zero database connections, zero persistent files.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub relay: Arc<RwLock<RelayStore>>,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Arc::new(config),
            relay: Arc::new(RwLock::new(RelayStore::new(Duration::from_secs(600)))), // 10 min TTL
        }
    }
}
