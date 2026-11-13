use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    sync::atomic::Ordering,
};

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::Manager;
use toml_edit::{Array, DocumentMut, Item, Table, value};

use super::{McpState, client_entry, private_options};

const LEGACY_ENDPOINT: &str = "http://127.0.0.1:18765/mcp";

const SERVER_NAME: &str = "eattura-open-app-required";
const MAX_CONFIG_SIZE: u64 = 1024 * 1024;
static CONFIG_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClientId {
    Codex,
    ClaudeCode,
    GeminiCli,
    ClaudeDesktop,
    Chatgpt,
}

impl ClientId {
    fn definition(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Codex => ("Codex", Some("codex")),
            Self::ClaudeCode => ("Claude Code", Some("claude")),
            Self::GeminiCli => ("Gemini CLI", Some("gemini")),
            Self::ClaudeDesktop => ("Claude Desktop", None),
            Self::Chatgpt => ("ChatGPT", None),
        }
    }

    fn config_path(self, home: &Path) -> Option<PathBuf> {
        match self {
            Self::Codex => Some(home.join(".codex/config.toml")),
            Self::ClaudeCode => Some(home.join(".claude.json")),
            Self::GeminiCli => Some(home.join(".gemini/settings.json")),
            Self::ClaudeDesktop if cfg!(target_os = "macos") => {
                Some(home.join("Library/Application Support/Claude/claude_desktop_config.json"))
            }
            Self::ClaudeDesktop if cfg!(windows) => std::env::var_os("APPDATA").map(|directory| {
                PathBuf::from(directory).join("Claude/claude_desktop_config.json")
            }),
            _ => None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpClient {
    id: ClientId,
    name: &'static str,
    installed: bool,
    configured: bool,
    can_install: bool,
    config_path: Option<String>,
    detail: String,
}

fn failure(message: &str) -> io::Error {
    io::Error::other(message)
}

fn check_path(home: &Path, path: &Path) -> io::Result<()> {
    if !path.starts_with(home)
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(failure(
            "Configuration must remain in the current user's home",
        ));
    }
    for ancestor in path
        .ancestors()
        .take_while(|ancestor| ancestor.starts_with(home))
    {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(failure(
                    "Symbolic links are not supported for client configuration",
                ));
            }
            Ok(_) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(_) => return Err(failure("Cannot inspect client configuration")),
        }
    }
    Ok(())
}

pub(super) fn read_config(home: &Path, path: &Path) -> io::Result<Option<String>> {
    check_path(home, path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || metadata.len() > MAX_CONFIG_SIZE => {
            return Err(failure(
                "Client configuration is not a regular file or exceeds 1 MiB",
            ));
        }
        Ok(_) => (),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(failure("Cannot inspect client configuration")),
    }
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(failure("Cannot read client configuration")),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_CONFIG_SIZE {
        return Err(failure(
            "Client configuration is not a regular file or exceeds 1 MiB",
        ));
    }
    let mut contents = String::new();
    file.take(MAX_CONFIG_SIZE + 1)
        .read_to_string(&mut contents)
        .map_err(|_| failure("Client configuration is not valid UTF-8"))?;
    if contents.len() as u64 > MAX_CONFIG_SIZE {
        return Err(failure("Client configuration exceeds 1 MiB"));
    }
    Ok(Some(contents))
}

fn merge_config(
    id: ClientId,
    original: Option<&str>,
    token: &SecretString,
) -> io::Result<(String, bool)> {
    let authorization = format!("Bearer {}", token.expose_secret());
    let mut entry = client_entry()?;
    if matches!(id, ClientId::Codex) {
        let mut document = original
            .unwrap_or("")
            .parse::<DocumentMut>()
            .map_err(|_| failure("Invalid TOML configuration; no changes made"))?;
        let command = entry["command"]
            .as_str()
            .ok_or_else(|| failure("Invalid executable"))?;
        if let Some(existing) = document
            .get("mcp_servers")
            .and_then(|servers| servers.get(SERVER_NAME))
        {
            let table = existing
                .as_table_like()
                .ok_or_else(|| failure("Unsupported MCP entry"))?;
            let configured = table.len() == 2
                && existing.get("command").and_then(Item::as_str) == Some(command)
                && existing
                    .get("args")
                    .and_then(Item::as_array)
                    .is_some_and(|args| {
                        args.len() == 1
                            && args.get(0).and_then(toml_edit::Value::as_str) == Some("--mcp-stdio")
                    });
            if configured {
                return Ok((original.unwrap_or("").into(), true));
            }
            let legacy = table.len() == 2
                && existing.get("url").and_then(Item::as_str) == Some(LEGACY_ENDPOINT)
                && existing
                    .get("http_headers")
                    .and_then(Item::as_table_like)
                    .is_some_and(|headers| {
                        headers.len() == 1
                            && headers.get("Authorization").and_then(Item::as_str)
                                == Some(authorization.as_str())
                    });
            if !legacy {
                return Err(failure(
                    "An Eattura entry already exists with different settings",
                ));
            }
        }
        if document.get("mcp_servers").is_none() {
            document["mcp_servers"] = Item::Table(Table::new());
        }
        let servers = document["mcp_servers"]
            .as_table_mut()
            .ok_or_else(|| failure("Unsupported MCP table; no changes made"))?;
        let mut table = Table::new();
        table["command"] = value(command);
        let mut args = Array::new();
        args.push("--mcp-stdio");
        table["args"] = value(args);
        servers.insert(SERVER_NAME, Item::Table(table));
        return Ok((document.to_string(), false));
    }
    let legacy = match id {
        ClientId::ClaudeCode => {
            entry["type"] = json!("stdio");
            json!({"type":"http", "url":LEGACY_ENDPOINT, "headers":{"Authorization":authorization}})
        }
        ClientId::GeminiCli => {
            json!({"httpUrl":LEGACY_ENDPOINT, "headers":{"Authorization":authorization}})
        }
        ClientId::ClaudeDesktop => Value::Null,
        _ => {
            return Err(failure(
                "Automatic installation is not supported for this client",
            ));
        }
    };
    let mut document: Value = match original {
        Some(contents) => serde_json::from_str(contents)
            .map_err(|_| failure("Invalid JSON configuration; no changes made"))?,
        None => json!({}),
    };
    let root = document
        .as_object_mut()
        .ok_or_else(|| failure("Client configuration must be an object"))?;
    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| failure("MCP configuration must be an object"))?;
    if let Some(existing) = servers.get(SERVER_NAME) {
        if existing == &entry {
            return Ok((original.unwrap_or("").into(), true));
        }
        if legacy.is_null() || existing != &legacy {
            return Err(failure(
                "An Eattura entry already exists with different settings",
            ));
        }
    }
    servers.insert(SERVER_NAME.into(), entry);
    let contents = serde_json::to_string_pretty(&document)
        .map_err(|_| failure("Cannot encode client configuration"))?;
    Ok((format!("{contents}\n"), false))
}

fn executable_exists(directory: &Path, name: &str) -> bool {
    let candidates = if cfg!(windows) {
        vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            format!("{name}.bat"),
        ]
    } else {
        vec![name.into()]
    };
    candidates.iter().any(|candidate| {
        directory.join(candidate).metadata().is_ok_and(|metadata| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            }
            #[cfg(not(unix))]
            {
                metadata.is_file()
            }
        })
    })
}

fn binary_directories(home: &Path) -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .filter(|path| path.is_absolute())
                .collect()
        })
        .unwrap_or_default();
    directories.extend([
        home.join(".local/bin"),
        home.join(".cargo/bin"),
        home.join(".bun/bin"),
        home.join(".npm-global/bin"),
    ]);
    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            directories.push(PathBuf::from(appdata).join("npm"));
        }
    } else {
        directories.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ]);
    }
    directories
}

fn application_directories(home: &Path) -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        return vec![PathBuf::from("/Applications"), home.join("Applications")];
    }
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA")
            .map(|directory| {
                let directory = PathBuf::from(directory);
                vec![directory.clone(), directory.join("Programs")]
            })
            .unwrap_or_default();
    }
    vec![
        home.join(".local/share/applications"),
        PathBuf::from("/usr/share/applications"),
    ]
}

fn desktop_present(directories: &[PathBuf], app: &str) -> bool {
    directories.iter().any(|directory| {
        if cfg!(target_os = "macos") {
            directory.join(format!("{app}.app")).is_dir()
        } else if cfg!(windows) {
            directory.join(app).is_dir()
        } else {
            directory
                .join(format!("{}.desktop", app.to_lowercase()))
                .is_file()
        }
    })
}

fn describe(
    id: ClientId,
    home: &Path,
    directories: &[PathBuf],
    applications: &[PathBuf],
    token: Option<&SecretString>,
) -> McpClient {
    let (name, binary) = id.definition();
    let path = id.config_path(home);
    let installed = binary.is_some_and(|binary| {
        directories
            .iter()
            .any(|directory| executable_exists(directory, binary))
    }) || matches!(id, ClientId::Codex) && desktop_present(applications, "Codex")
        || matches!(id, ClientId::ClaudeDesktop) && desktop_present(applications, "Claude")
        || matches!(id, ClientId::Chatgpt) && desktop_present(applications, "ChatGPT");
    let mut result = McpClient {
        id,
        name,
        installed,
        configured: false,
        can_install: false,
        config_path: path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        detail: if matches!(id, ClientId::Chatgpt) {
            "ChatGPT richiede un collegamento MCP remoto, escluso da questo flusso solo locale: nessun tunnel o server pubblico viene creato.".into()
        } else {
            "Installazione MCP locale non disponibile per questa applicazione su questa piattaforma.".into()
        },
    };
    if !installed {
        return result;
    }
    if let Some(path) = path {
        let custom_home = match id {
            ClientId::Codex => std::env::var_os("CODEX_HOME")
                .is_some_and(|directory| Path::new(&directory) != home.join(".codex")),
            ClientId::ClaudeCode => std::env::var_os("CLAUDE_CONFIG_DIR").is_some(),
            ClientId::GeminiCli => std::env::var_os("GEMINI_CLI_HOME").is_some(),
            _ => false,
        };
        if custom_home {
            result.detail = "È configurata una directory client personalizzata, non supportata dall'installazione automatica. Il file standard indicato non verrà modificato.".into();
            return result;
        }
        result.detail = "Non rilevato nei percorsi standard.".into();
        if let Some(token) = token {
            match read_config(home, &path).and_then(|original| merge_config(id, original.as_deref(), token)) {
                Ok((_, configured)) => {
                    result.configured = configured;
                    result.can_install = installed && !configured;
                    if configured { result.detail = "Configurazione presente. Riavvia il client, verifica MCP e mantieni Eattura aperta. La scansione non verifica la connessione.".into(); }
                    else if installed { result.detail = "Pronto per stdio. Chiudi il client prima di continuare: verrà configurata soltanto la voce Eattura, migrando il precedente HTTP riconosciuto e creando un backup del file esistente.".into(); }
                }
                Err(_) => result.detail = "Configurazione non modificabile, non valida o voce Eattura differente. Controlla il file indicato; nessuna modifica automatica.".into(),
            }
        } else {
            result.detail =
                "Apri Eattura e risolvi lo stato del server MCP prima di collegare il client."
                    .into();
        }
    }
    result
}

fn scan_clients(
    home: &Path,
    directories: &[PathBuf],
    applications: &[PathBuf],
    token: Option<&SecretString>,
) -> Vec<McpClient> {
    [
        ClientId::Codex,
        ClientId::ClaudeCode,
        ClientId::GeminiCli,
        ClientId::ClaudeDesktop,
        ClientId::Chatgpt,
    ]
    .into_iter()
    .map(|id| describe(id, home, directories, applications, token))
    .filter(|client| client.installed)
    .collect()
}

fn save_config(home: &Path, path: &Path, original: Option<&str>, contents: &str) -> io::Result<()> {
    if contents.len() as u64 > MAX_CONFIG_SIZE {
        return Err(failure("Client configuration exceeds 1 MiB"));
    }
    check_path(home, path)?;
    let parent = path
        .parent()
        .ok_or_else(|| failure("Missing configuration directory"))?;
    fs::create_dir_all(parent)?;
    check_path(home, path)?;
    let temporary = path.with_extension(format!("eattura-{}.tmp", uuid::Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = private_options().open(&temporary)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        if read_config(home, path)?.as_deref() != original {
            return Err(failure(
                "Configuration changed during installation; close the client and retry",
            ));
        }
        if let Some(original) = original {
            let backup =
                path.with_extension(format!("eattura-{}.bak", uuid::Uuid::new_v4().simple()));
            let mut file = private_options().open(backup)?;
            file.write_all(original.as_bytes())?;
            file.sync_all()?;
        }
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[tauri::command]
pub async fn scan_mcp_clients(
    app: tauri::AppHandle,
    state: tauri::State<'_, McpState>,
) -> Result<Vec<McpClient>, String> {
    let home = app.path().home_dir().map_err(|_| "User home unavailable")?;
    let token = state.token.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = CONFIG_LOCK.lock().map_err(|_| "MCP configuration busy")?;
        let directories = binary_directories(&home);
        let applications = application_directories(&home);
        Ok(scan_clients(
            &home,
            &directories,
            &applications,
            token.as_deref(),
        ))
    })
    .await
    .map_err(|_| "Cannot scan MCP clients")?
}

#[tauri::command]
pub async fn install_mcp_client(
    app: tauri::AppHandle,
    state: tauri::State<'_, McpState>,
    client_id: ClientId,
) -> Result<McpClient, String> {
    if !state.running.load(Ordering::Acquire) {
        return Err("MCP unavailable; open Eattura".into());
    }
    let token = state.token.clone().ok_or("MCP unavailable")?;
    let home = app.path().home_dir().map_err(|_| "User home unavailable")?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = CONFIG_LOCK.lock().map_err(|_| "MCP configuration busy")?;
        let directories = binary_directories(&home);
        let applications = application_directories(&home);
        let status = describe(client_id, &home, &directories, &applications, Some(&token));
        if status.configured {
            return Ok(status);
        }
        if !status.can_install {
            return Err("Client not detected or configuration requires manual review".into());
        }
        let path = client_id
            .config_path(&home)
            .ok_or("Unsupported MCP client")?;
        let install = || -> io::Result<()> {
            let original = read_config(&home, &path)?;
            let (contents, _) = merge_config(client_id, original.as_deref(), &token)?;
            save_config(&home, &path, original.as_deref(), &contents)
        };
        install().map_err(|_| {
            "Cannot configure MCP client; close the client and check configuration permissions"
                .to_string()
        })?;
        Ok(describe(
            client_id,
            &home,
            &directories,
            &applications,
            Some(&token),
        ))
    })
    .await
    .map_err(|_| "Cannot configure MCP client")?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token() -> SecretString {
        "fixture-token-not-a-real-credential".into()
    }

    #[test]
    fn recognized_http_entries_migrate_to_stdio_without_copying_the_secret() {
        let home = tempfile::tempdir().unwrap();
        for id in [ClientId::Codex, ClientId::ClaudeCode, ClientId::GeminiCli] {
            let original = match id {
                ClientId::Codex => format!("# Keep this comment\nmodel = \"example\"\n[mcp_servers.{SERVER_NAME}]\nurl = \"{LEGACY_ENDPOINT}\"\n[mcp_servers.{SERVER_NAME}.http_headers]\nAuthorization = \"Bearer {}\"\n", token().expose_secret()),
                ClientId::ClaudeCode => json!({"theme":"dark", "mcpServers":{SERVER_NAME:{"type":"http", "url":LEGACY_ENDPOINT,"headers":{"Authorization":format!("Bearer {}",token().expose_secret())}}}}).to_string(),
                _ => json!({"theme":"dark", "mcpServers":{SERVER_NAME:{"httpUrl":LEGACY_ENDPOINT,"headers":{"Authorization":format!("Bearer {}",token().expose_secret())}}}}).to_string(),
            };
            let (contents, configured) = merge_config(id, Some(&original), &token()).unwrap();
            assert!(!configured);
            assert!(contents.contains("--mcp-stdio"));
            assert!(!contents.contains(LEGACY_ENDPOINT));
            assert!(!contents.contains(token().expose_secret()));
            assert!(!contents.contains("Authorization"));
            if matches!(id, ClientId::Codex) {
                assert!(contents.contains("# Keep this comment"));
            }
            assert!(merge_config(id, Some(&contents), &token()).unwrap().1);
            let wrong: SecretString = "other-users-token".into();
            assert!(merge_config(id, Some(&original), &wrong).is_err());
            let path = home
                .path()
                .join(format!("{}.config", uuid::Uuid::new_v4().simple()));
            fs::write(&path, &original).unwrap();
            save_config(home.path(), &path, Some(&original), &contents).unwrap();
            assert_eq!(fs::read_to_string(&path).unwrap(), contents);
            assert!(
                fs::read_dir(home.path())
                    .unwrap()
                    .filter_map(Result::ok)
                    .any(
                        |entry| entry.path().extension().is_some_and(|ext| ext == "bak")
                            && fs::read_to_string(entry.path()).unwrap() == original
                    )
            );
        }
    }

    #[test]
    fn modified_http_entries_are_not_silently_migrated() {
        let original = json!({"mcpServers":{SERVER_NAME:{"type":"http", "url":LEGACY_ENDPOINT,
            "headers":{"Authorization":format!("Bearer {}",token().expose_secret())}, "env":{"CUSTOM":"preserve"}}}}).to_string();
        assert!(merge_config(ClientId::ClaudeCode, Some(&original), &token()).is_err());
        let original = format!(
            "[mcp_servers.{SERVER_NAME}]\nurl = \"{LEGACY_ENDPOINT}\"\nenabled = false\n[mcp_servers.{SERVER_NAME}.http_headers]\nAuthorization = \"Bearer {}\"\n",
            token().expose_secret()
        );
        assert!(merge_config(ClientId::Codex, Some(&original), &token()).is_err());
    }

    #[test]
    fn json_clients_preserve_unrelated_settings_and_use_native_transports() {
        for id in [ClientId::ClaudeCode, ClientId::GeminiCli] {
            let original = r#"{"theme":"dark","mcpServers":{"other":{"command":"existing"}},"projects":{"private":{}}}"#;
            let (contents, configured) = merge_config(id, Some(original), &token()).unwrap();
            assert!(!configured);
            let parsed: Value = serde_json::from_str(&contents).unwrap();
            assert_eq!(parsed["theme"], "dark");
            assert_eq!(parsed["mcpServers"]["other"]["command"], "existing");
            assert!(parsed["projects"].get("private").is_some());
            let entry = &parsed["mcpServers"][SERVER_NAME];
            assert_eq!(entry["args"], json!(["--mcp-stdio"]));
            assert_eq!(
                entry["command"],
                std::env::current_exe().unwrap().to_str().unwrap()
            );
            assert!(entry.get("url").is_none());
            assert!(entry.get("httpUrl").is_none());
            assert!(!contents.contains("fixture-token"));
            let (second, configured) = merge_config(id, Some(&contents), &token()).unwrap();
            assert!(configured);
            assert_eq!(second, contents);
        }
    }

    #[test]
    fn codex_keeps_comments_and_other_servers_and_is_idempotent() {
        let original = "# Personal settings\nmodel = \"example\"\n[mcp_servers.other]\nurl = \"https://example.com\"\n";
        let (contents, configured) =
            merge_config(ClientId::Codex, Some(original), &token()).unwrap();
        assert!(!configured);
        assert!(contents.contains("# Personal settings"));
        let parsed = contents.parse::<DocumentMut>().unwrap();
        assert_eq!(parsed["model"].as_str(), Some("example"));
        assert_eq!(
            parsed["mcp_servers"]["other"]["url"].as_str(),
            Some("https://example.com")
        );
        assert_eq!(
            parsed["mcp_servers"][SERVER_NAME]["command"].as_str(),
            std::env::current_exe().unwrap().to_str()
        );
        let (second, configured) =
            merge_config(ClientId::Codex, Some(&contents), &token()).unwrap();
        assert!(configured);
        assert_eq!(second, contents);
    }

    #[test]
    fn invalid_configs_conflicts_and_cloud_clients_are_rejected_without_secret_errors() {
        for (id, original) in [
            (ClientId::Codex, "broken = ["),
            (ClientId::Codex, "mcp_servers = false"),
            (
                ClientId::Codex,
                "[mcp_servers.eattura-open-app-required]\nurl = \"https://other\"",
            ),
            (ClientId::ClaudeCode, "{broken"),
            (ClientId::GeminiCli, "[]"),
            (ClientId::ClaudeCode, r#"{"mcpServers":false}"#),
            (
                ClientId::GeminiCli,
                r#"{"mcpServers":{"eattura-open-app-required":{"httpUrl":"https://other"}}}"#,
            ),
            (ClientId::ClaudeDesktop, "{broken"),
            (ClientId::Chatgpt, "{}"),
        ] {
            let error = merge_config(id, Some(original), &token())
                .unwrap_err()
                .to_string();
            assert!(!error.contains("fixture-token"));
            assert!(!error.contains(original));
        }
        assert!(serde_json::from_str::<ClientId>(r#""../../arbitrary-path""#).is_err());
        assert!(serde_json::from_str::<ClientId>(r#""gemini""#).is_err());
    }

    #[test]
    fn installation_is_scoped_private_atomic_and_backed_up() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(".gemini/settings.json");
        save_config(home.path(), &path, None, "{}").unwrap();
        let (contents, _) = merge_config(ClientId::GeminiCli, Some("{}"), &token()).unwrap();
        save_config(home.path(), &path, Some("{}"), &contents).unwrap();
        assert_eq!(
            read_config(home.path(), &path).unwrap().as_deref(),
            Some(contents.as_str())
        );
        let files: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        let backup = files
            .iter()
            .find(|path| path.extension().is_some_and(|extension| extension == "bak"))
            .unwrap();
        assert_eq!(fs::read_to_string(backup).unwrap(), "{}");
        assert_eq!(files.len(), 2);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(backup).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let other_user = tempfile::tempdir().unwrap();
        assert!(
            save_config(
                home.path(),
                &other_user.path().join("settings.json"),
                None,
                "{}"
            )
            .is_err()
        );
        assert!(read_config(home.path(), &other_user.path().join("settings.json")).is_err());
        assert_eq!(fs::read_dir(other_user.path()).unwrap().count(), 0);
    }

    #[test]
    fn concurrent_changes_and_oversized_files_are_not_overwritten() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("settings.json");
        fs::write(&path, "new data").unwrap();
        assert!(save_config(home.path(), &path, Some("old data"), "replacement").is_err());
        assert!(save_config(home.path(), &path, None, "replacement").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "new data");
        assert_eq!(fs::read_dir(home.path()).unwrap().count(), 1);
        fs::File::create(&path)
            .unwrap()
            .set_len(MAX_CONFIG_SIZE + 1)
            .unwrap();
        assert!(read_config(home.path(), &path).is_err());
        assert!(read_config(home.path(), home.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_files_and_directories_are_never_modified() {
        use std::os::unix::fs::symlink;
        let home = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let original = other.path().join("original.json");
        fs::write(&original, "{}").unwrap();
        let link = home.path().join("linked.json");
        symlink(&original, &link).unwrap();
        assert!(read_config(home.path(), &link).is_err());
        assert!(save_config(home.path(), &link, Some("{}"), "replacement").is_err());
        let directory = home.path().join("linked-directory");
        symlink(other.path(), &directory).unwrap();
        assert!(save_config(home.path(), &directory.join("new.json"), None, "{}").is_err());
        assert_eq!(fs::read_to_string(original).unwrap(), "{}");
        assert_eq!(fs::read_dir(other.path()).unwrap().count(), 1);
    }

    #[test]
    fn scan_is_read_only_and_returns_no_credentials() {
        let home = tempfile::tempdir().unwrap();
        assert!(scan_clients(home.path(), &[], &[], Some(&token())).is_empty());
        for id in [
            ClientId::Codex,
            ClientId::ClaudeCode,
            ClientId::GeminiCli,
            ClientId::ClaudeDesktop,
            ClientId::Chatgpt,
        ] {
            let status = describe(id, home.path(), &[], &[], Some(&token()));
            assert!(!status.installed);
            assert!(!status.configured);
            assert!(!status.can_install);
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.contains("fixture-token"));
            assert!(!json.contains("Authorization"));
        }
        assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
    }

    #[test]
    fn binary_detection_uses_only_known_names_without_execution() {
        let directory = tempfile::tempdir().unwrap();
        let binary = directory
            .path()
            .join(if cfg!(windows) { "codex.cmd" } else { "codex" });
        fs::write(&binary, "this must not be executed").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        }
        assert!(executable_exists(directory.path(), "codex"));
        assert!(!executable_exists(directory.path(), "gemini"));
        let home = tempfile::tempdir().unwrap();
        let clients = scan_clients(home.path(), &[directory.path().into()], &[], Some(&token()));
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].name, "Codex");
        assert!(clients[0].installed);
        assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
    }

    #[test]
    fn detected_chat_apps_do_not_offer_automatic_registration() {
        let home = tempfile::tempdir().unwrap();
        let applications = tempfile::tempdir().unwrap();
        if cfg!(target_os = "macos") {
            fs::create_dir(applications.path().join("ChatGPT.app")).unwrap();
        } else if cfg!(windows) {
            fs::create_dir(applications.path().join("ChatGPT")).unwrap();
        } else {
            fs::write(applications.path().join("chatgpt.desktop"), "fixture").unwrap();
        }
        let client = describe(
            ClientId::Chatgpt,
            home.path(),
            &[],
            &[applications.path().into()],
            Some(&token()),
        );
        assert!(client.installed);
        assert!(!client.can_install);
        assert!(!client.configured);
        assert!(client.config_path.is_none());
        assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
    }

    #[test]
    fn claude_desktop_registers_the_bundled_bridge_without_token_or_external_programs() {
        let original =
            r#"{"preferences":{"theme":"dark"},"mcpServers":{"other":{"command":"existing"}}}"#;
        let (contents, configured) =
            merge_config(ClientId::ClaudeDesktop, Some(original), &token()).unwrap();
        assert!(!configured);
        let parsed: Value = serde_json::from_str(&contents).unwrap();
        let entry = &parsed["mcpServers"][SERVER_NAME];
        assert_eq!(
            entry["command"],
            std::env::current_exe().unwrap().to_str().unwrap()
        );
        assert_eq!(entry["args"], json!(["--mcp-stdio"]));
        assert!(entry.get("env").is_none());
        assert!(!contents.contains("fixture-token"));
        assert!(!contents.contains("Bearer"));
        assert_eq!(parsed["preferences"]["theme"], "dark");
        assert_eq!(parsed["mcpServers"]["other"]["command"], "existing");
        let (second, configured) =
            merge_config(ClientId::ClaudeDesktop, Some(&contents), &token()).unwrap();
        assert!(configured);
        assert_eq!(second, contents);
    }
}
