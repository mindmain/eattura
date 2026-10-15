mod commands;
mod error;
mod pec;
mod state;
mod storage;

use std::path::{Path, PathBuf};
use tauri::Manager;

use state::TauriState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Resolve ~/.eattura in the user's home directory so invoice data
            // persists across application uninstalls and updates.
            let data_dir = resolve_data_directory(app)?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("Emesse"))?;
            std::fs::create_dir_all(data_dir.join("Ricevute"))?;

            let store = storage::FileStore::open(data_dir.join("eattura.json"))?;

            // Spawn real-time filesystem debounced watcher
            let watcher = commands::vault::spawn_vault_watcher(app.handle().clone(), data_dir.clone()).ok();
            let watcher_state = std::sync::Arc::new(std::sync::Mutex::new(watcher));

            app.manage(TauriState {
                store: store.clone(),
                watcher: watcher_state,
            });

            // Background PEC cron: periodically poll the inbox for SDI
            // notifications. No-op while PEC is not configured.
            spawn_pec_cron(store);

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
            commands::invoices::render_invoice_pdf,
            commands::invoices::sign_invoice_p7m,
            commands::invoices::verify_p7m_signature,
            commands::invoices::import_invoice_xml,
            commands::invoices::import_invoices_from_dir,
            commands::invoices::rectify_rejected_invoice,
            commands::invoices::generate_autofattura_draft,
            commands::invoices::export_attachment_to_file,
            commands::pec::import_sdi_receipt,
            commands::clients::list_clients,
            commands::clients::get_client,
            commands::clients::create_client,
            commands::clients::update_client,
            commands::clients::delete_client,
            commands::clients::lookup_vat_vies,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_data_directory,
            commands::settings::open_data_directory,
            commands::pec::set_pec_password,
            commands::pec::send_invoice_pec,
            commands::pec::sync_pec,
            commands::pec::pec_sync_status,
            commands::vault::get_active_vault,
            commands::vault::switch_vault,
            commands::vault::rebuild_vault_index,
            commands::vault::verify_vault_integrity,
            commands::vault::list_known_vaults,
            commands::vault::add_known_vault,
            commands::vault::remove_known_vault,
            commands::export::export_accountant_package,
            commands::export::export_accountant_csv,
            commands::export::export_agid_preservation_package,
            commands::export::verify_agid_preservation_package,
            commands::sync::generate_sync_pairing_code,
            commands::sync::sync_push_changes,
            commands::sync::sync_pull_changes,
            commands::sync::sync_check_relay,
            commands::payments::get_payment_schedule,
            commands::payments::register_invoice_payment,
            commands::payments::get_invoice_payment_history,
            commands::payments::calculate_iva_liquidation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Interval between automatic PEC inbox polls.
const PEC_CRON_INTERVAL_SECS: i64 = 300;

/// Spawn a background task that periodically syncs the PEC inbox.
///
/// Each tick calls the same routine as the `sync_pec` command. When PEC is not
/// configured (no settings or no keyring password) the run returns an error
/// that is logged and ignored, so the cron is effectively off until set up.
fn spawn_pec_cron(store: storage::FileStore) {
    tauri::async_runtime::spawn(async move {
        let mut last_failed_at = 0_i64;
        loop {
            let now = chrono::Utc::now().timestamp();
            let due = commands::pec::sync_status_store(&store).await
                .map(|status| status.configured && status.next_sync_at <= now * 1000)
                .unwrap_or(false);
            if due && now - last_failed_at >= PEC_CRON_INTERVAL_SECS {
                match commands::pec::run_sync(&store).await {
                Ok(notifications) if !notifications.is_empty() => {
                    eprintln!(
                        "[pec-cron] processed {} notification(s)",
                        notifications.len()
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    last_failed_at = chrono::Utc::now().timestamp();
                    eprintln!("[pec-cron] skipped: {e}");
                }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });
}

/// Resolves `~/.eattura` in the user's home directory.
///
/// If existing data is found in the legacy `app_data_dir` (e.g. `Application Support`)
/// and `~/.eattura` is not yet populated, migrates it transparently.
fn resolve_data_directory<R: tauri::Runtime>(
    app: &tauri::App<R>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = app.path().home_dir()?;
    let target_dir = home.join(".eattura");

    if let Ok(old_dir) = app.path().app_data_dir() {
        if (old_dir.join("index.json").exists() || old_dir.join("eattura.json").exists())
            && !target_dir.join("index.json").exists()
            && !target_dir.join("eattura.json").exists()
        {
            std::fs::create_dir_all(&target_dir)?;
            let _ = copy_directory_contents(&old_dir, &target_dir);
        }
    }

    Ok(target_dir)
}

/// Recursively copy all files and subdirectories from `from` into `to`.
fn copy_directory_contents(from: &Path, to: &Path) -> std::io::Result<()> {
    if !from.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_directory_contents(&entry.path(), &target)?;
        } else {
            let _ = std::fs::copy(entry.path(), &target);
        }
    }
    Ok(())
}
