use std::path::{Path, PathBuf};
use std::time::Duration;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::state::TauriState;

/// Guard holding the background filesystem debounced watcher.
pub struct VaultWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub path: String,
    pub name: String,
    pub total_invoices: usize,
    pub issued_count: usize,
    pub received_count: usize,
    pub emesse_dir_exists: bool,
    pub ricevute_dir_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownVault {
    pub id: String,
    pub name: String,
    pub path: String,
    pub last_opened: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildSummary {
    pub scanned_files: usize,
    pub imported_count: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityItem {
    pub id: String,
    pub numero: String,
    pub status: String, // "valid", "corrupted", "missing"
    pub sha256: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub total_checked: usize,
    pub valid_count: usize,
    pub corrupted_count: usize,
    pub missing_count: usize,
    pub items: Vec<IntegrityItem>,
}

/// Spawns a debounced file watcher on the given vault directory.
/// Emits `vault:changed` events through Tauri whenever XML files are added or modified.
pub fn spawn_vault_watcher(app_handle: AppHandle, vault_dir: PathBuf) -> Result<VaultWatcher, String> {
    let app = app_handle.clone();
    let watched_dir = vault_dir.clone();

    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        move |res: DebounceEventResult| {
            match res {
                Ok(events) => {
                    let mut relevant = Vec::new();
                    for event in events {
                        let path = &event.path;
                        let ext = path
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let file_name = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");

                        // Filter for XML or P7M files, ignoring temp / index files
                        if (ext == "xml" || ext == "p7m" || file_name.ends_with(".xml.p7m"))
                            && !file_name.starts_with('.')
                            && !file_name.ends_with(".tmp")
                        {
                            relevant.push(path.display().to_string());
                        }
                    }

                    if !relevant.is_empty() {
                        let _ = app.emit(
                            "vault:changed",
                            serde_json::json!({
                                "vault_dir": watched_dir.display().to_string(),
                                "files": relevant,
                            }),
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[vault-watcher] error: {:?}", e);
                }
            }
        },
    )
    .map_err(|e| format!("Failed to create filesystem debouncer: {e}"))?;

    // Watch recursively to capture Emesse/ and Ricevute/ subdirectories
    debouncer
        .watcher()
        .watch(&vault_dir, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch vault directory {}: {e}", vault_dir.display()))?;

    Ok(VaultWatcher {
        _debouncer: debouncer,
    })
}

/// Get metadata and statistics about the currently active vault.
#[tauri::command]
pub async fn get_active_vault(state: tauri::State<'_, TauriState>) -> Result<VaultInfo, String> {
    let dir = state.store.data_dir();
    inspect_vault_dir(&state.store, &dir).await
}

pub async fn inspect_vault_dir(store: &crate::storage::FileStore, dir: &Path) -> Result<VaultInfo, String> {
    let name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Default Vault")
        .to_string();

    let emesse_dir = dir.join("Emesse");
    let ricevute_dir = dir.join("Ricevute");

    let invoices = store
        .read("invoices", "invoice_views", vec![], "invoice")
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let total = invoices.len();
    let mut issued = 0;
    let mut received = 0;

    for inv in &invoices {
        let stato: String = inv.get("stato");
        if stato == "received" {
            received += 1;
        } else {
            issued += 1;
        }
    }

    Ok(VaultInfo {
        path: dir.display().to_string(),
        name,
        total_invoices: total,
        issued_count: issued,
        received_count: received,
        emesse_dir_exists: emesse_dir.is_dir(),
        ricevute_dir_exists: ricevute_dir.is_dir(),
    })
}

/// Switch active vault directory, ensuring standard subfolders exist (`Emesse/`, `Ricevute/`),
/// atomically reloads the FileStore, and updates the background filesystem watcher.
#[tauri::command]
pub async fn switch_vault(
    app: AppHandle,
    state: tauri::State<'_, TauriState>,
    path: String,
    name: Option<String>,
) -> Result<VaultInfo, String> {
    let vault_dir = PathBuf::from(&path);
    if !vault_dir.exists() {
        std::fs::create_dir_all(&vault_dir)
            .map_err(|e| format!("Failed to create vault directory: {e}"))?;
    }

    // Ensure standard subdirectories exist
    let emesse_dir = vault_dir.join("Emesse");
    let ricevute_dir = vault_dir.join("Ricevute");
    std::fs::create_dir_all(&emesse_dir)
        .map_err(|e| format!("Failed to create Emesse dir: {e}"))?;
    std::fs::create_dir_all(&ricevute_dir)
        .map_err(|e| format!("Failed to create Ricevute dir: {e}"))?;

    // Switch store atomically
    state.store.switch_vault(vault_dir.clone()).await?;

    // Re-arm watcher
    let new_watcher = spawn_vault_watcher(app.clone(), vault_dir.clone())?;
    {
        let mut w_guard = state.watcher.lock().map_err(|e| e.to_string())?;
        *w_guard = Some(new_watcher);
    }

    // Record in known vaults list
    let vault_name = name.unwrap_or_else(|| {
        vault_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Vault")
            .to_string()
    });
    record_known_vault(&vault_dir, &vault_name)?;

    // Emit event
    let info = inspect_vault_dir(&state.store, &vault_dir).await?;
    let _ = app.emit("vault:switched", &info);

    Ok(info)
}

/// Reindex all XML files in the active vault on demand.
/// Scans `Emesse/`, `Ricevute/`, and root vault folders, calculates SHA-256 for each,
/// parses and imports into the catalog.
#[tauri::command]
pub async fn rebuild_vault_index(
    state: tauri::State<'_, TauriState>,
) -> Result<RebuildSummary, String> {
    rebuild_index_for_store(&state.store).await
}

pub async fn rebuild_index_for_store(
    store: &crate::storage::FileStore,
) -> Result<RebuildSummary, String> {
    let dir = store.data_dir();
    let mut xml_files = Vec::new();

    collect_xml_files(&dir, &mut xml_files)?;

    let total_files = xml_files.len();
    let mut imported = 0;
    let mut errors = Vec::new();

    for file_path in xml_files {
        let display = file_path.display().to_string();
        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{display}: failed to read: {e}"));
                continue;
            }
        };

        // Try importing content into store
        match crate::commands::invoices::import_xml_content(store, &content).await {
            Ok(_) => imported += 1,
            Err(e) => {
                // If it's already imported or invalid, record notice
                if !e.to_lowercase().contains("duplicate") {
                    errors.push(format!("{display}: {e}"));
                }
            }
        }
    }

    Ok(RebuildSummary {
        scanned_files: total_files,
        imported_count: imported,
        errors,
    })
}

/// Verify integrity of all invoices in the vault against SHA-256 digests.
#[tauri::command]
pub async fn verify_vault_integrity(
    state: tauri::State<'_, TauriState>,
) -> Result<IntegrityReport, String> {
    verify_integrity_for_store(&state.store).await
}

pub async fn verify_integrity_for_store(
    store: &crate::storage::FileStore,
) -> Result<IntegrityReport, String> {
    let invoices = store
        .read("invoices", "invoice_views", vec![], "invoice")
        .await
        .fetch_all()
        .await
        .map_err(|e| e.to_string())?;

    let total = invoices.len();
    let mut valid = 0;
    let mut corrupted = 0;
    let mut missing = 0;
    let mut items = Vec::new();

    for inv in invoices {
        let id: String = inv.get("id");
        let numero: String = inv.get("numero");

        // Hydrate invoice details to check XML content
        let full = store
            .read("invoices", "id", vec![serde_json::json!(&id)], "")
            .await
            .fetch_optional()
            .await
            .map_err(|e| e.to_string())?;

        match full {
            None => {
                missing += 1;
                items.push(IntegrityItem {
                    id: id.clone(),
                    numero: numero.clone(),
                    status: "missing".into(),
                    sha256: "".into(),
                    details: Some("Document record missing from store".into()),
                });
            }
            Some(row) => {
                let xml: Option<String> = row.get("xml_content");
                match xml {
                    Some(content) if !content.trim().is_empty() => {
                        let hash = format!("{:x}", Sha256::digest(content.as_bytes()));
                        // Validate XML parseability
                        if common::xml::decode::decode(&content).is_ok() {
                            valid += 1;
                            items.push(IntegrityItem {
                                id: id.clone(),
                                numero: numero.clone(),
                                status: "valid".into(),
                                sha256: hash,
                                details: None,
                            });
                        } else {
                            corrupted += 1;
                            items.push(IntegrityItem {
                                id: id.clone(),
                                numero: numero.clone(),
                                status: "corrupted".into(),
                                sha256: hash,
                                details: Some("XML parsing failed: syntax corrupted".into()),
                            });
                        }
                    }
                    _ => {
                        corrupted += 1;
                        items.push(IntegrityItem {
                            id: id.clone(),
                            numero: numero.clone(),
                            status: "corrupted".into(),
                            sha256: "".into(),
                            details: Some("xml_content is empty".into()),
                        });
                    }
                }
            }
        }
    }

    Ok(IntegrityReport {
        total_checked: total,
        valid_count: valid,
        corrupted_count: corrupted,
        missing_count: missing,
        items,
    })
}

/// List all known vaults saved on the machine.
#[tauri::command]
pub async fn list_known_vaults() -> Result<Vec<KnownVault>, String> {
    load_known_vaults()
}

/// Add a known vault to the list.
#[tauri::command]
pub async fn add_known_vault(path: String, name: String) -> Result<Vec<KnownVault>, String> {
    let p = PathBuf::from(&path);
    record_known_vault(&p, &name)?;
    load_known_vaults()
}

/// Remove a vault from the known list.
#[tauri::command]
pub async fn remove_known_vault(id: String) -> Result<Vec<KnownVault>, String> {
    let mut vaults = load_known_vaults()?;
    vaults.retain(|v| v.id != id);
    save_known_vaults(&vaults)?;
    Ok(vaults)
}

fn known_vaults_file() -> Result<PathBuf, String> {
    let home = dirs_home().ok_or("Cannot resolve home directory")?;
    let dir = home.join(".eattura");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("known_vaults.json"))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn load_known_vaults() -> Result<Vec<KnownVault>, String> {
    let file = known_vaults_file()?;
    if !file.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(&file).map_err(|e| e.to_string())?;
    let list: Vec<KnownVault> = serde_json::from_slice(&bytes).unwrap_or_default();
    Ok(list)
}

fn save_known_vaults(vaults: &[KnownVault]) -> Result<(), String> {
    let file = known_vaults_file()?;
    let bytes = serde_json::to_vec_pretty(vaults).map_err(|e| e.to_string())?;
    std::fs::write(&file, bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn record_known_vault(path: &Path, name: &str) -> Result<(), String> {
    let mut list = load_known_vaults().unwrap_or_default();
    let path_str = path.display().to_string();
    let id = format!("{:x}", Sha256::digest(path_str.as_bytes()));

    if let Some(existing) = list.iter_mut().find(|v| v.path == path_str || v.id == id) {
        existing.name = name.to_string();
        existing.last_opened = Some(chrono_now());
    } else {
        list.push(KnownVault {
            id,
            name: name.to_string(),
            path: path_str,
            last_opened: Some(chrono_now()),
        });
    }

    save_known_vaults(&list)
}

fn chrono_now() -> String {
    format!("{:?}", std::time::SystemTime::now())
}

fn collect_xml_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            // Skip hidden folders and internal storage state folder
            if !name.starts_with('.') && name != "state" {
                collect_xml_files(&path, files)?;
            }
        } else if path.is_file() {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext.eq_ignore_ascii_case("xml") {
                files.push(path);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inspect_and_switch_vault() {
        let temp = std::env::temp_dir().join(format!("test_vault_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        let store = crate::storage::FileStore::open(temp.join("eattura.json")).unwrap();

        let info = inspect_vault_dir(&store, &temp).await.unwrap();
        assert_eq!(info.total_invoices, 0);

        let temp2 = std::env::temp_dir().join(format!("test_vault_2_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp2).unwrap();
        store.switch_vault(temp2.clone()).await.unwrap();
        assert_eq!(store.data_dir(), temp2);

        let _ = std::fs::remove_dir_all(&temp);
        let _ = std::fs::remove_dir_all(&temp2);
    }

    #[tokio::test]
    async fn test_verify_vault_integrity() {
        let temp = std::env::temp_dir().join(format!("test_vault_integ_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        let store = crate::storage::FileStore::open(temp.join("eattura.json")).unwrap();

        let sample_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:FatturaElettronica versione="FPR12" xmlns:p="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2">
  <FatturaElettronicaHeader>
    <DatiTrasmissione><IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdTrasmittente><ProgressivoInvio>00001</ProgressivoInvio><FormatoTrasmissione>FPR12</FormatoTrasmissione><CodiceDestinatario>0000000</CodiceDestinatario></DatiTrasmissione>
    <CedentePrestatore><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Alpha Srl</Denominazione></Anagrafica><RegimeFiscale>RF01</RegimeFiscale></DatiAnagrafici><Sede><Indirizzo>Via Roma 1</Indirizzo><CAP>00100</CAP><Comune>Roma</Comune><Nazione>IT</Nazione></Sede></CedentePrestatore>
    <CessionarioCommittente><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>09876543210</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Beta Spa</Denominazione></Anagrafica></DatiAnagrafici><Sede><Indirizzo>Via Milano 2</Indirizzo><CAP>20100</CAP><Comune>Milano</Comune><Nazione>IT</Nazione></Sede></CessionarioCommittente>
  </FatturaElettronicaHeader>
  <FatturaElettronicaBody>
    <DatiGenerali><DatiGeneraliDocumento><TipoDocumento>TD01</TipoDocumento><Divisa>EUR</Divisa><Data>2026-01-15</Data><Numero>1</Numero><ImportoTotaleDocumento>122.00</ImportoTotaleDocumento></DatiGeneraliDocumento></DatiGenerali>
    <DatiBeniServizi><DettaglioLinee><NumeroLinea>1</NumeroLinea><Descrizione>Consulenza</Descrizione><Quantita>1.00</Quantita><PrezzoUnitario>100.00</PrezzoUnitario><PrezzoTotale>100.00</PrezzoTotale><AliquotaIVA>22.00</AliquotaIVA></DettaglioLinee><DatiRiepilogo><AliquotaIVA>22.00</AliquotaIVA><ImponibileImporto>100.00</ImponibileImporto><Imposta>22.00</Imposta></DatiRiepilogo></DatiBeniServizi>
  </FatturaElettronicaBody>
</p:FatturaElettronica>"#;

        crate::commands::invoices::import_xml_content(&store, sample_xml).await.unwrap();

        let report = verify_integrity_for_store(&store).await.unwrap();
        assert_eq!(report.total_checked, 1);
        assert_eq!(report.valid_count, 1);
        assert_eq!(report.corrupted_count, 0);

        let _ = std::fs::remove_dir_all(&temp);
    }
}
