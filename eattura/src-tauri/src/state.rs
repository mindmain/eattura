/// Shared desktop state backed by a local file store.
pub struct TauriState {
    pub store: crate::storage::FileStore,
}
