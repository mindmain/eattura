use std::sync::{Arc, Mutex};

/// Shared desktop state backed by a local JSON snapshot and real-time filesystem watcher.
pub struct TauriState {
    pub store: crate::storage::FileStore,
    pub watcher: Arc<Mutex<Option<crate::commands::vault::VaultWatcher>>>,
}
