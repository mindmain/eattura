use std::{path::Path, time::Duration};

use rmcp::{
    ErrorData, RoleClient, RoleServer, ServerHandler, ServiceExt,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig,
    },
    service::{Peer, RequestContext},
    transport::async_rw::AsyncRwTransport,
};
use secrecy::{ExposeSecret, SecretString};

use super::{
    ipc,
    tools::{DESCRIPTION, INSTRUCTIONS},
};

const TIMEOUT: Duration = Duration::from_secs(35);
const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024;
const TOOL_NAMES: &[&str] = &[
    "get_app_info",
    "list_invoices",
    "get_invoice",
    "list_clients",
    "get_client",
    "create_draft",
    "update_draft",
    "update_draft_lines",
    "validate_invoice",
];

struct LocalBridge {
    peer: Peer<RoleClient>,
}

impl ServerHandler for LocalBridge {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("eattura", env!("CARGO_PKG_VERSION"))
                    .with_title("Eattura — open the app to connect")
                    .with_description(DESCRIPTION),
            )
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut result = tokio::time::timeout(TIMEOUT, self.peer.list_tools(request))
            .await
            .map_err(|_| unavailable())?
            .map_err(|_| unavailable())?;
        result
            .tools
            .retain(|tool| TOOL_NAMES.contains(&tool.name.as_ref()));
        Ok(result)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if !TOOL_NAMES.contains(&request.name.as_ref()) {
            return Err(ErrorData::invalid_params(
                "This tool is not available in Eattura",
                None,
            ));
        }
        let result = tokio::time::timeout(TIMEOUT, self.peer.call_tool(request)).await;
        match result {
            Ok(Ok(result)) => Ok(result.into()),
            _ => Ok(CallToolResult::error(vec![ContentBlock::text("Connection unavailable or interrupted. Open Eattura and keep it running. Check the saved draft in the app before retrying a write.")]).into()),
        }
    }
}

fn unavailable() -> ErrorData {
    ErrorData::internal_error(
        "Open Eattura and keep it running, then reconnect the MCP client",
        None,
    )
}

fn read_token(home: &Path) -> Result<SecretString, ()> {
    let path = home.join(".eattura/mcp-token");
    let contents: SecretString = super::clients::read_config(home, &path)
        .map_err(|_| ())?
        .ok_or(())?
        .into();
    let token = contents.expose_secret().trim();
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(());
    }
    Ok(token.into())
}

async fn transport(
    home: &Path,
    token: &SecretString,
) -> Result<
    AsyncRwTransport<
        RoleClient,
        tokio::io::ReadHalf<ipc::Stream>,
        tokio::io::WriteHalf<ipc::Stream>,
    >,
    (),
> {
    let contents = super::clients::read_config(home, &home.join(".eattura/mcp-ipc.json"))
        .map_err(|_| ())?
        .ok_or(())?;
    let config: serde_json::Value = serde_json::from_str(&contents).map_err(|_| ())?;
    let endpoint = config
        .get("endpoint")
        .and_then(serde_json::Value::as_str)
        .ok_or(())?;
    let mut stream = ipc::connect(endpoint).await.map_err(|_| ())?;
    ipc::authenticate_client(&mut stream, token)
        .await
        .map_err(|_| ())?;
    let (reader, writer) = tokio::io::split(stream);
    Ok(AsyncRwTransport::new(reader, writer).with_max_line_length(MAX_MESSAGE_SIZE))
}

async fn serve_stdio(home: &Path) -> Result<(), ()> {
    let token = read_token(home)?;
    let connection = tokio::time::timeout(TIMEOUT, async {
        ().serve(transport(home, &token).await?)
            .await
            .map_err(|_| ())
    })
    .await
    .map_err(|_| ())?
    .map_err(|_| ())?;
    let bridge = LocalBridge {
        peer: connection.peer().clone(),
    };
    let (reader, writer) = rmcp::transport::stdio_pipes();
    let stdio = AsyncRwTransport::new(reader, writer).with_max_line_length(MAX_MESSAGE_SIZE);
    let service = tokio::time::timeout(TIMEOUT, bridge.serve(stdio))
        .await
        .map_err(|_| ())?
        .map_err(|_| ())?;
    service.waiting().await.map_err(|_| ())?;
    let _ = connection.cancel().await;
    Ok(())
}

fn run_inner() -> Result<(), ()> {
    let home = dirs::home_dir().ok_or(())?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| ())?;
    let result = runtime.block_on(serve_stdio(&home));
    runtime.shutdown_timeout(Duration::from_secs(1));
    result
}

pub fn run() -> std::io::Result<()> {
    run_inner().map_err(|_| std::io::Error::other("Open Eattura and reconnect the MCP client"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app_log::AppLog, storage::FileStore};
    use rmcp::model::CallToolRequestParams;
    use std::sync::Arc;

    #[tokio::test]
    async fn missing_credentials_do_not_create_an_archive_or_start_the_app() {
        let home = tempfile::tempdir().unwrap();
        assert!(serve_stdio(home.path()).await.is_err());
        assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
        std::fs::create_dir(home.path().join(".eattura")).unwrap();
        std::fs::write(home.path().join(".eattura/mcp-token"), "invalid").unwrap();
        assert!(serve_stdio(home.path()).await.is_err());
        assert_eq!(
            std::fs::read_dir(home.path().join(".eattura"))
                .unwrap()
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn stdio_uses_the_running_apps_store_over_private_os_ipc() {
        let home = tempfile::tempdir().unwrap();
        let data = home.path().join(".eattura");
        std::fs::create_dir(&data).unwrap();
        let token = Arc::new(super::super::load_token(&data.join("mcp-token")).unwrap());
        let store = FileStore::open(data.join("eattura.json")).unwrap();
        let mut listener = ipc::Listener::bind().unwrap();
        let endpoint = listener.endpoint().to_owned();
        super::super::write_json(
            &data.join("mcp-ipc.json"),
            &serde_json::json!({"endpoint": endpoint}),
        )
        .unwrap();
        let credential = token.clone();
        let app = tokio::spawn(async move {
            let stream = listener.accept().await.unwrap();
            super::super::serve_connection(
                stream,
                &credential,
                super::super::InvoiceServer::new(store, AppLog::memory()),
            )
            .await
        });
        let connection = ().serve(transport(home.path(), &token).await.unwrap()).await.unwrap();
        let bridge = LocalBridge {
            peer: connection.peer().clone(),
        };
        let (client_io, server_io) = tokio::io::duplex(16 * 1024);
        let stdio_task = tokio::spawn(async move {
            let (reader, writer) = tokio::io::split(server_io);
            bridge
                .serve(AsyncRwTransport::new(reader, writer).with_max_line_length(MAX_MESSAGE_SIZE))
                .await
                .unwrap()
        });
        let client = ().serve(client_io).await.unwrap();
        let stdio = stdio_task.await.unwrap();
        let info = serde_json::to_string(&client.peer_info()).unwrap();
        assert!(info.contains("open Eattura"));
        assert!(info.contains("personally perform any transmission"));
        let tools = client.list_tools(None).await.unwrap();
        assert_eq!(tools.tools.len(), TOOL_NAMES.len());
        assert!(
            tools
                .tools
                .iter()
                .all(|tool| TOOL_NAMES.contains(&tool.name.as_ref()))
        );
        let result = client
            .call_tool(CallToolRequestParams::new("get_app_info"))
            .await
            .unwrap();
        let encoded = serde_json::to_value(result).unwrap();
        assert_eq!(encoded["structuredContent"]["canSendInvoices"], false);
        assert!(!encoded.to_string().contains(token.expose_secret()));
        assert!(
            client
                .call_tool(CallToolRequestParams::new("send_invoice"))
                .await
                .is_err()
        );
        app.abort();
        let _ = app.await;
        let result = client
            .call_tool(CallToolRequestParams::new("get_app_info"))
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(true));
        assert!(
            serde_json::to_string(&result)
                .unwrap()
                .contains("Open Eattura")
        );
        let _ = client.cancel().await;
        let _ = stdio.cancel().await;
        let _ = connection.cancel().await;
        assert!(ipc::connect(&endpoint).await.is_err());
    }
}
