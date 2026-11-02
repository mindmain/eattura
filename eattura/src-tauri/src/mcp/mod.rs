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

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{StatusCode, header::AUTHORIZATION},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use serde_json::{Value, json};
use subtle::ConstantTimeEq;
use tower_http::timeout::TimeoutLayer;

use crate::{
    app_log::{AppLog, LogLevel},
    storage::FileStore,
};
use tools::{DESCRIPTION, InvoiceServer};

const PORT: u16 = 18765;
const ENDPOINT: &str = "http://127.0.0.1:18765/mcp";

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
    endpoint: &'static str,
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
    state
        .token
        .as_ref()
        .map(|token| client_config(token))
        .ok_or_else(|| "MCP is unavailable; open Eattura and check diagnostics".into())
}

impl McpState {
    fn status(&self) -> McpStatus {
        McpStatus {
            running: self.running.load(Ordering::Acquire),
            endpoint: ENDPOINT,
            config_path: self.config_path.to_string_lossy().into_owned(),
            description: DESCRIPTION,
            error: self.error.clone(),
        }
    }
}

fn client_config(token: &SecretString) -> Value {
    json!({"mcpServers": {"eattura-open-app-required": {
        "type": "http", "url": ENDPOINT,
        "headers": {"Authorization": format!("Bearer {}", token.expose_secret())}
    }}})
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

fn write_config(path: &Path, token: &SecretString) -> io::Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = private_options().open(&temporary)?;
        serde_json::to_writer_pretty(&mut file, &client_config(token))?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

async fn authenticate(
    State(token): State<Arc<SecretString>>,
    request: Request,
    next: Next,
) -> Response {
    let authorized = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .is_some_and(|provided| {
            bool::from(provided.as_bytes().ct_eq(token.expose_secret().as_bytes()))
        });
    if !authorized {
        return (
            StatusCode::UNAUTHORIZED,
            [(
                axum::http::header::WWW_AUTHENTICATE,
                "Bearer realm=\"eattura\"",
            )],
            "MCP authentication required. Open Eattura and use its local MCP configuration.",
        )
            .into_response();
    }
    next.run(request).await
}

async fn limit_body(request: Request, next: Next) -> Response {
    let (parts, body) = request.into_parts();
    match to_bytes(body, 2 * 1024 * 1024).await {
        Ok(bytes) => {
            next.run(Request::from_parts(parts, Body::from(bytes)))
                .await
        }
        Err(_) => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "MCP requests must not exceed 2 MiB",
        )
            .into_response(),
    }
}

fn router(
    store: FileStore,
    log: AppLog,
    token: Arc<SecretString>,
    on_draft: Arc<dyn Fn() + Send + Sync>,
) -> Router {
    let mut config = StreamableHttpServerConfig::default().enforce_origin_validation();
    config.legacy_session_mode = false;
    config.json_response = true;
    config.allowed_hosts = vec![format!("127.0.0.1:{PORT}")];
    let service =
        StreamableHttpService::new(
            move || {
                Ok(InvoiceServer::new(store.clone(), log.clone())
                    .with_draft_callback(on_draft.clone()))
            },
            Arc::new(LocalSessionManager::default()),
            config,
        );
    Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn(limit_body))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(middleware::from_fn_with_state(token, authenticate))
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
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, PORT))?;
        listener.set_nonblocking(true)?;
        let token = Arc::new(load_token(&data_dir.join("mcp-token"))?);
        write_config(&config_path, &token)?;
        Ok((listener, token))
    })();
    match prepared {
        Ok((listener, token)) => {
            let app = router(store, log.clone(), token.clone(), on_draft);
            let active = running.clone();
            tauri::async_runtime::spawn(async move {
                let result = async {
                    let listener = tokio::net::TcpListener::from_std(listener)?;
                    active.store(true, Ordering::Release);
                    log.record(LogLevel::Info, "mcp", "Local MCP started on 127.0.0.1:18765; draft creation only, no invoice transmission");
                    axum::serve(listener, app).await
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
            McpState { running, token: None, config_path, error: Some("MCP non disponibile: controlla la porta 18765 e i permessi della cartella dati, poi riavvia Eattura.".into()) }
        }
    }
}

#[cfg(test)]
mod tests;
