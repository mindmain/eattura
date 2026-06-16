mod commands;
mod db;
mod error;
mod pec;
mod state;

use tauri::Manager;

use state::TauriState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Resolve the app data directory and ensure it exists.
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            // Initialize the SQLite database synchronously during setup.
            let db_path = app_data_dir.join("eattura.db");
            let pool = tauri::async_runtime::block_on(db::setup::initialize(&db_path))
                .map_err(|e| format!("Failed to initialize database: {e}"))?;

            // Register the database pool as managed Tauri state.
            app.manage(TauriState { db: pool.clone() });

            // Background PEC cron: periodically poll the inbox for SDI
            // notifications. No-op while PEC is not configured.
            spawn_pec_cron(pool);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::invoices::list_invoices,
            commands::invoices::get_invoice,
            commands::invoices::create_invoice,
            commands::invoices::update_invoice,
            commands::invoices::delete_invoice,
            commands::invoices::set_invoice_status,
            commands::invoices::next_invoice_number,
            commands::invoices::validate_invoice,
            commands::invoices::export_invoice_xml,
            commands::invoices::import_invoice_xml,
            commands::invoices::import_invoices_from_dir,
            commands::clients::list_clients,
            commands::clients::get_client,
            commands::clients::create_client,
            commands::clients::update_client,
            commands::clients::delete_client,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::pec::set_pec_password,
            commands::pec::send_invoice_pec,
            commands::pec::sync_pec,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Interval between automatic PEC inbox polls.
const PEC_CRON_INTERVAL_SECS: u64 = 300;

/// Spawn a background task that periodically syncs the PEC inbox.
///
/// Each tick calls the same routine as the `sync_pec` command. When PEC is not
/// configured (no settings or no keyring password) the run returns an error
/// that is logged and ignored, so the cron is effectively off until set up.
fn spawn_pec_cron(pool: sqlx::SqlitePool) {
    tauri::async_runtime::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(PEC_CRON_INTERVAL_SECS));
        // Skip the immediate first tick so startup is not blocked.
        interval.tick().await;
        loop {
            interval.tick().await;
            match commands::pec::run_sync(&pool).await {
                Ok(notifications) if !notifications.is_empty() => {
                    eprintln!("[pec-cron] processed {} notification(s)", notifications.len());
                }
                Ok(_) => {}
                Err(e) => eprintln!("[pec-cron] skipped: {e}"),
            }
        }
    });
}
