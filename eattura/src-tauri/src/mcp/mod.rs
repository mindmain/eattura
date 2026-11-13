pub mod bridge;
pub mod clients;
mod ipc;
mod tools;

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use rmcp::{ServiceExt, transport::async_rw::AsyncRwTransport};
use secrecy::SecretString;
use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    app_log::{AppLog, LogLevel},
    storage::FileStore,
};
use tools::{DESCRIPTION, InvoiceServer};

const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct McpState {
    running: Arc<AtomicBool>,
    token: Option<Arc<SecretString>>,
    config_path: PathBuf,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    running: bool,
    transport: &'static str,
    config_path: String,
    description: &'static str,
    error: Option<String>,
}

#[tauri::command]
pub fn get_mcp_status(state: tauri::State<'_, McpState>) -> McpStatus {
    state.status()
}

#[tauri::command]
pub fn get_mcp_client_config(state: tauri::State<'_, McpState>) -> Result<Value, String> {
    if !state.running.load(Ordering::Acquire) {
        return Err("Open Eattura and check the MCP status before connecting".into());
    }
    client_config().map_err(|_| "Cannot locate Eattura executable".into())
}

impl McpState {
    fn status(&self) -> McpStatus {
        McpStatus {
            running: self.running.load(Ordering::Acquire),
            transport: "stdio",
            config_path: self.config_path.to_string_lossy().into_owned(),
            description: DESCRIPTION,
            error: self.error.clone().or_else(|| {
                (!self.running.load(Ordering::Acquire)).then(|| {
                    "MCP non disponibile: controlla il registro attività e riavvia Eattura.".into()
                })
            }),
        }
    }
}

fn client_entry() -> io::Result<Value> {
    let executable = std::env::current_exe()?;
    let command = executable
        .to_str()
        .ok_or_else(|| io::Error::other("Executable path must be UTF-8"))?;
    Ok(json!({"command": command, "args": ["--mcp-stdio"]}))
}

fn client_config() -> io::Result<Value> {
    Ok(json!({"mcpServers": {"eattura-open-app-required": client_entry()?}}))
}

fn private_options() -> std::fs::OpenOptions {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

fn load_token(path: &Path) -> io::Result<SecretString> {
    let generated = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    match private_options().open(path) {
        Ok(mut file) => {
            file.write_all(generated.as_bytes())?;
            file.sync_all()?;
            Ok(generated.into())
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if !std::fs::symlink_metadata(path)?.file_type().is_file() {
                return Err(io::Error::other("MCP token must be a regular file"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
            }
            let token = std::fs::read_to_string(path)?;
            if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(io::Error::other("Invalid MCP token file"));
            }
            Ok(token.into())
        }
        Err(error) => Err(error),
    }
}

fn write_json(path: &Path, value: &Value) -> io::Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = private_options().open(&temporary)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

async fn serve_connection<Stream>(
    mut stream: Stream,
    token: &SecretString,
    server: InvoiceServer,
) -> io::Result<()>
where
    Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    tokio::time::timeout(
        HANDSHAKE_TIMEOUT,
        ipc::authenticate_server(&mut stream, token),
    )
    .await??;
    let (reader, writer) = tokio::io::split(stream);
    let transport = AsyncRwTransport::new(reader, writer).with_max_line_length(MAX_MESSAGE_SIZE);
    let service = tokio::time::timeout(HANDSHAKE_TIMEOUT, server.serve(transport))
        .await?
        .map_err(|_| io::Error::other("MCP initialization failed"))?;
    service
        .waiting()
        .await
        .map_err(|_| io::Error::other("MCP disconnected"))?;
    Ok(())
}

pub fn start(
    store: FileStore,
    log: AppLog,
    data_dir: &Path,
    on_draft: Arc<dyn Fn() + Send + Sync>,
) -> McpState {
    let config_path = data_dir.join("mcp.json");
    let running = Arc::new(AtomicBool::new(false));
    let prepared = (|| -> io::Result<_> {
        let lock_path = data_dir.join("mcp.lock");
        let lock = match private_options().open(&lock_path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if !std::fs::symlink_metadata(&lock_path)?.is_file() {
                    return Err(io::Error::other("Invalid MCP lock"));
                }
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(lock_path)?
            }
            Err(error) => return Err(error),
        };
        lock.try_lock()
            .map_err(|_| io::Error::other("Another Eattura MCP instance is already running"))?;
        let token = Arc::new(load_token(&data_dir.join("mcp-token"))?);
        write_json(&config_path, &client_config()?)?;
        Ok((lock, token))
    })();
    match prepared {
        Ok((lock, token)) => {
            let active = running.clone();
            let credential = token.clone();
            let discovery = data_dir.join("mcp-ipc.json");
            tauri::async_runtime::spawn(async move {
                let _lock = lock;
                let result = async {
                    let mut listener = ipc::Listener::bind()?;
                    write_json(&discovery, &json!({"endpoint": listener.endpoint()}))?;
                    active.store(true, Ordering::Release);
                    log.record(LogLevel::Info, "mcp", "Local MCP stdio ready; private OS IPC only, no HTTP listener; draft creation only");
                    let permits = Arc::new(tokio::sync::Semaphore::new(16));
                    let mut connections = tokio::task::JoinSet::new();
                    loop {
                        tokio::select! {
                            stream = listener.accept() => {
                                let stream = match stream { Ok(stream) => stream, Err(error) => break Err::<(), _>(error) };
                                let Ok(permit) = permits.clone().try_acquire_owned() else { continue; };
                                let token = credential.clone();
                                let server = InvoiceServer::new(store.clone(), log.clone()).with_draft_callback(on_draft.clone());
                                connections.spawn(async move {
                                    let _permit = permit;
                                    let _ = serve_connection(stream, &token, server).await;
                                });
                            }
                            _ = connections.join_next(), if !connections.is_empty() => {}
                        }
                    }
                }.await;
                active.store(false, Ordering::Release);
                if let Err(error) = result {
                    log.record(
                        LogLevel::Error,
                        "mcp",
                        &format!("Local MCP stopped: {error}"),
                    );
                }
            });
            McpState {
                running,
                token: Some(token),
                config_path,
                error: None,
            }
        }
        Err(error) => {
            log.record(
                LogLevel::Error,
                "mcp",
                &format!("Local MCP unavailable: {error}"),
            );
            McpState { running, token: None, config_path, error: Some("MCP non disponibile: verifica che Eattura non sia già aperta e controlla i permessi della cartella dati.".into()) }
        }
    }
}

#[cfg(test)]
mod tests;
