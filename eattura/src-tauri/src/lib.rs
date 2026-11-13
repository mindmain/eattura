#![forbid(unsafe_code)]

mod app_log;
mod commands;
mod error;
mod mcp;
mod pec;
mod state;
mod storage;

use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};

use app_log::{AppLog, LogLevel};
use state::TauriState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--mcp-stdio")) {
        if mcp::bridge::run().is_err() {
            eprintln!("MCP unavailable. Open Eattura, keep it running and reconnect the MCP client.");
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Resolve ~/.eattura in the user's home directory so invoice data
            // persists across application uninstalls and updates.
            let data_dir = resolve_data_directory(app)?;
            std::fs::create_dir_all(&data_dir)?;
            let log = AppLog::open(data_dir.join("logs")).unwrap_or_else(|error| {
                let log = AppLog::memory();
                log.record(
                    LogLevel::Warn,
                    "app",
                    &format!("Persistent logging unavailable: {error}"),
                );
                log
            });
            log.record(
                LogLevel::Info,
                "app",
                &format!("Eattura {} started", app.package_info().version),
            );
            app.manage(log.clone());

            let store =
                storage::FileStore::open(data_dir.join("eattura.json")).inspect_err(|error| {
                    log.record(LogLevel::Error, "storage", error);
                })?;

            app.manage(TauriState {
                store: store.clone(),
            });

            let handle = app.handle().clone();
            let event_log = log.clone();
            app.manage(mcp::start(
                store.clone(),
                log.clone(),
                &data_dir,
                std::sync::Arc::new(move || {
                    if let Err(error) = handle.emit("mcp-draft-created", ()) {
                        event_log.record(
                            LogLevel::Warn,
                            "mcp",
                            &format!("Unable to notify the app about a new draft: {error}"),
                        );
                    }
                }),
            ));

            // Background PEC cron: periodically poll the inbox for SDI
            // notifications. No-op while PEC is not configured.
            spawn_pec_cron(store, log);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            mcp::get_mcp_status,
            mcp::get_mcp_client_config,
            mcp::clients::scan_mcp_clients,
            mcp::clients::install_mcp_client,
            app_log::get_app_logs,
            app_log::write_app_log,
            app_log::open_app_log_directory,
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
            commands::export::export_accountant_package,
            commands::export::export_accountant_csv,
            commands::export::export_agid_preservation_package,
            commands::export::verify_agid_preservation_package,
            commands::payments::get_payment_schedule,
            commands::payments::register_invoice_payment,
            commands::payments::get_invoice_payment_history,
            commands::payments::calculate_iva_liquidation,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("Error while running Eattura: {error}");
            std::process::exit(1);
        });
}

/// Interval between automatic PEC inbox polls.
const PEC_CRON_INTERVAL_SECS: i64 = 300;

/// Spawn a background task that periodically syncs the PEC inbox.
///
/// Each tick calls the same routine as the `sync_pec` command. When PEC is not
/// configured (no settings or no keyring password) the run returns an error
/// that is logged and ignored, so the cron is effectively off until set up.
fn spawn_pec_cron(store: storage::FileStore, log: AppLog) {
    tauri::async_runtime::spawn(async move {
        log.record(
            LogLevel::Info,
            "pec",
            "PEC scheduler started; checking configuration",
        );
        let mut last_failed_at = 0_i64;
        loop {
            let now = chrono::Utc::now().timestamp();
            let due = match commands::pec::sync_status_store(&store).await {
                Ok(status) => status.configured && status.next_sync_at <= now * 1000,
                Err(error) => {
                    if now - last_failed_at >= PEC_CRON_INTERVAL_SECS {
                        log.record(
                            LogLevel::Error,
                            "pec",
                            &format!("Unable to read PEC schedule: {error}"),
                        );
                        last_failed_at = now;
                    }
                    false
                }
            };
            if due
                && now - last_failed_at >= PEC_CRON_INTERVAL_SECS
                && commands::pec::run_sync(&store, &log).await.is_err()
            {
                last_failed_at = chrono::Utc::now().timestamp();
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

    if let Ok(old_dir) = app.path().app_data_dir()
        && (old_dir.join("index.json").exists() || old_dir.join("eattura.json").exists())
        && !target_dir.join("index.json").exists()
        && !target_dir.join("eattura.json").exists()
    {
        std::fs::create_dir_all(&target_dir)?;
        let _ = copy_directory_contents(&old_dir, &target_dir);
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
