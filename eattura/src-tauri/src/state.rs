/// Application state managed by Tauri, accessible in all commands.
///
/// Contains the SQLite connection pool for local database operations.
/// Injected via `tauri::State<'_, TauriState>` in command parameters.
pub struct TauriState {
    pub db: sqlx::SqlitePool,
}
