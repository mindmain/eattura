use serde::{Deserialize, Serialize};

use crate::state::TauriState;

/// Application settings stored locally.
///
/// Contains the user's company information (used as default cedente in invoices)
/// and PEC credentials for SDI communication.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    /// Company info (default invoice issuer).
    pub company: Option<CompanySettings>,
    /// PEC credentials for SDI.
    pub pec: Option<PecSettings>,
}

/// Company information used as default cedente (invoice issuer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanySettings {
    pub denominazione: Option<String>,
    pub nome: Option<String>,
    pub cognome: Option<String>,
    pub id_paese: String,
    pub id_codice: String,
    pub codice_fiscale: Option<String>,
    pub regime_fiscale: String,
    pub indirizzo: String,
    pub numero_civico: Option<String>,
    pub cap: String,
    pub comune: String,
    pub provincia: Option<String>,
    pub nazione: String,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub pec: Option<String>,
}

/// PEC (Certified Email) configuration for SDI communication.
#[derive(Debug, Serialize, Deserialize)]
pub struct PecSettings {
    /// PEC email address.
    pub email: String,
    /// IMAP server hostname.
    pub imap_host: String,
    /// IMAP server port (usually 993 for IMAPS).
    pub imap_port: u16,
    /// SMTP server hostname.
    pub smtp_host: String,
    /// SMTP server port (usually 465 for SMTPS).
    pub smtp_port: u16,
    /// SDI PEC address assigned to this sender after the first reply.
    #[serde(default)]
    pub sdi_address: Option<String>,
}

impl PecSettings {
    pub fn sdi_recipient(&self) -> &str {
        self.sdi_address
            .as_deref()
            .map(str::trim)
            .filter(|address| !address.is_empty())
            .unwrap_or(common::pec::SDI_PEC_ADDRESS)
    }
}

/// Get current application settings.
///
/// Reads from the settings record in the local file store.
/// Returns default empty settings if no entry exists yet.
#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, TauriState>) -> Result<AppSettings, String> {
    get_settings_store(&state.store).await
}

pub async fn get_settings_store(store: &crate::storage::FileStore) -> Result<AppSettings, String> {
    let row = store
        .read("settings", "settings", vec![], "")
        .await
        .fetch_optional()
        .await
        .map_err(|e| e.to_string())?;

    match row {
        Some(row) => {
            let json: String = row.get("value");
            serde_json::from_str(&json).map_err(|e| e.to_string())
        }
        None => Ok(AppSettings {
            company: None,
            pec: None,
        }),
    }
}

/// Update application settings.
///
/// Serializes the settings to JSON and persists them using a file snapshot.
#[tauri::command]
pub async fn update_settings(
    state: tauri::State<'_, TauriState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    update_settings_store(&state.store, settings).await
}

pub async fn update_settings_store(
    store: &crate::storage::FileStore,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let mut tx = store.begin().await?;
    if let Some(company) = &settings.company
        && company.id_paese.trim().eq_ignore_ascii_case("IT")
    {
        // A PEC-only update can carry the same legacy issuer data.
        let previous = tx
            .read("settings", "settings", vec![], "")
            .fetch_optional()
            .await?
            .map(|row| serde_json::from_str::<AppSettings>(&row.get::<String>("value")))
            .transpose()
            .map_err(|error| error.to_string())?
            .and_then(|settings| settings.company);
        if !previous.as_ref().is_some_and(|previous| {
            previous.id_paese == company.id_paese
                && previous.id_codice == company.id_codice
                && previous.codice_fiscale == company.codice_fiscale
        }) {
            super::clients::validate_italian_tax_fields(
                Some(&company.id_codice),
                company.codice_fiscale.as_deref().filter(|value| !value.is_empty()),
            )?;
        }
    }
    let json = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
    let existing = if let Some(company) = &settings.company
        && !company.id_codice.trim().is_empty()
    {
        tx.read(
            "clients",
            "tax_identity",
            vec![
                serde_json::json!(&company.id_paese),
                serde_json::json!(&company.id_codice),
            ],
            "",
        )
        .fetch_optional()
        .await?
    } else {
        None
    };
    tx.insert(
        "settings",
        serde_json::json!({"key": "app_settings", "value": &json}),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Maintain the company profile in the clients directory so it is directly
    // selectable and referentiable as a cedente in invoices.
    if let Some(company) = &settings.company
        && !company.id_codice.trim().is_empty()
    {
        let client_id = crate::storage::digest::client_id(&company.id_paese, &company.id_codice)?;
        if let Some(row) = existing {
            let id: String = row.get("id");
            let preserved_cd = crate::commands::clients::normalize_recipient_code(
                row.get::<Option<String>>("codice_destinatario").as_deref(),
            )
            .unwrap_or_else(|| "0000000".to_string());
            tx.update(
                "clients",
                &id,
                serde_json::json!({
                    "denominazione": &company.denominazione,
                    "nome": &company.nome,
                    "cognome": &company.cognome,
                    "codice_fiscale": &company.codice_fiscale,
                    "regime_fiscale": &company.regime_fiscale,
                    "indirizzo": &company.indirizzo,
                    "numero_civico": &company.numero_civico,
                    "cap": &company.cap,
                    "comune": &company.comune,
                    "provincia": &company.provincia,
                    "nazione": &company.nazione,
                    "telefono": &company.telefono,
                    "email": &company.email,
                    "pec": &company.pec,
                    "codice_destinatario": preserved_cd,
                }),
            )
            .await
            .map_err(|e| e.to_string())?;
        } else {
            tx.insert(
                "clients",
                serde_json::json!({
                    "id": &client_id,
                    "id_paese": &company.id_paese,
                    "id_codice": &company.id_codice,
                    "denominazione": &company.denominazione,
                    "nome": &company.nome,
                    "cognome": &company.cognome,
                    "codice_fiscale": &company.codice_fiscale,
                    "regime_fiscale": &company.regime_fiscale,
                    "indirizzo": &company.indirizzo,
                    "numero_civico": &company.numero_civico,
                    "cap": &company.cap,
                    "comune": &company.comune,
                    "provincia": &company.provincia,
                    "nazione": &company.nazione,
                    "telefono": &company.telefono,
                    "email": &company.email,
                    "pec": &company.pec,
                    "codice_destinatario": "0000000",
                }),
            )
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(settings)
}

/// Get the absolute path to the local data directory (~/.eattura).
#[tauri::command]
pub async fn get_data_directory(app: tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let data_dir = home.join(".eattura");
    Ok(data_dir.to_string_lossy().to_string())
}

/// Open the local data directory (~/.eattura) in the system file manager (Finder / Explorer).
#[tauri::command]
pub async fn open_data_directory(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_opener::OpenerExt;
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let data_dir = home.join(".eattura");
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let path_str = data_dir.to_string_lossy().to_string();
    app.opener()
        .open_path(path_str, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store(name: &str) -> (crate::storage::FileStore, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "eattura-test-settings-{}-{}",
            name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.json");
        let store = crate::storage::FileStore::open(path).unwrap();
        (store, dir)
    }

    #[tokio::test]
    async fn test_settings_default_and_roundtrip() {
        let (store, _dir) = test_store("roundtrip");

        // 1. Initial settings are empty
        let initial = get_settings_store(&store).await.unwrap();
        assert!(initial.company.is_none());
        assert!(initial.pec.is_none());

        // 2. Save settings
        let settings = AppSettings {
            company: Some(CompanySettings {
                denominazione: Some("My Company SRL".to_string()),
                nome: None,
                cognome: None,
                id_paese: "IT".to_string(),
                id_codice: "99988877769".to_string(),
                codice_fiscale: Some("99988877769".to_string()),
                regime_fiscale: "RF01".to_string(),
                indirizzo: "Corso Italia 10".to_string(),
                numero_civico: Some("10".to_string()),
                cap: "20100".to_string(),
                comune: "Milano".to_string(),
                provincia: Some("MI".to_string()),
                nazione: "IT".to_string(),
                telefono: Some("021234567".to_string()),
                email: Some("fatture@mycompany.it".to_string()),
                pec: Some("mycompany@pec.it".to_string()),
            }),
            pec: Some(PecSettings {
                email: "mycompany@pec.it".to_string(),
                imap_host: "imaps.pec.it".to_string(),
                imap_port: 993,
                smtp_host: "smtps.pec.it".to_string(),
                smtp_port: 465,
                sdi_address: None,
            }),
        };

        let saved = update_settings_store(&store, settings).await.unwrap();
        assert_eq!(saved.company.as_ref().unwrap().comune, "Milano");

        // 3. Reload settings from disk
        let reloaded = get_settings_store(&store).await.unwrap();
        assert_eq!(reloaded.company.as_ref().unwrap().id_codice, "99988877769");
        assert_eq!(reloaded.pec.as_ref().unwrap().imap_host, "imaps.pec.it");
        assert_eq!(reloaded.pec.as_ref().unwrap().smtp_port, 465);
        assert_eq!(
            reloaded.pec.as_ref().unwrap().sdi_recipient(),
            common::pec::SDI_PEC_ADDRESS
        );

        // 4. Verify company was automatically synced into clients directory
        let client_rows = store
            .read(
                "clients",
                "tax_identity",
                vec![serde_json::json!("IT"), serde_json::json!("99988877769")],
                "",
            )
            .await
            .fetch_optional()
            .await
            .unwrap();
        assert!(client_rows.is_some());
        let client = client_rows.unwrap();
        assert_eq!(client.get::<String>("denominazione"), "My Company SRL");
    }

    #[tokio::test]
    async fn test_personal_issuer_is_persisted_as_name_and_surname() {
        let (store, _dir) = test_store("personal-issuer");
        let settings = AppSettings {
            company: Some(CompanySettings {
                denominazione: None,
                nome: Some("Mario".into()),
                cognome: Some("Rossi".into()),
                id_paese: "IT".into(),
                id_codice: "12345678903".into(),
                codice_fiscale: Some("RSSMRA80A01H501U".into()),
                regime_fiscale: "RF19".into(),
                indirizzo: "Via Roma".into(),
                numero_civico: None,
                cap: "00100".into(),
                comune: "Roma".into(),
                provincia: Some("RM".into()),
                nazione: "IT".into(),
                telefono: None,
                email: None,
                pec: None,
            }),
            pec: None,
        };

        update_settings_store(&store, settings).await.unwrap();
        let reloaded = get_settings_store(&store).await.unwrap();
        let company = reloaded.company.unwrap();
        assert_eq!(company.nome.as_deref(), Some("Mario"));
        assert_eq!(company.cognome.as_deref(), Some("Rossi"));
        assert!(company.denominazione.is_none());

        let client = store
            .read(
                "clients",
                "tax_identity",
                vec![serde_json::json!("IT"), serde_json::json!("12345678903")],
                "",
            )
            .await
            .fetch_one()
            .await
            .unwrap();
        assert_eq!(
            client.get::<Option<String>>("nome").as_deref(),
            Some("Mario")
        );
        assert_eq!(
            client.get::<Option<String>>("cognome").as_deref(),
            Some("Rossi")
        );
        assert!(client.get::<Option<String>>("denominazione").is_none());
    }

    #[test]
    fn old_pec_settings_use_first_send_address_and_assigned_address_is_preserved() {
        let mut pec: PecSettings = serde_json::from_str(
            r#"{"email":"user@pec.example.it","imap_host":"imap.example.it","imap_port":993,"smtp_host":"smtp.example.it","smtp_port":465}"#,
        )
        .unwrap();
        assert_eq!(pec.sdi_recipient(), common::pec::SDI_PEC_ADDRESS);

        pec.sdi_address = Some(" sdi12@pec.fatturapa.it ".into());
        let reloaded: PecSettings =
            serde_json::from_str(&serde_json::to_string(&pec).unwrap()).unwrap();
        assert_eq!(reloaded.sdi_recipient(), "sdi12@pec.fatturapa.it");
    }
}
