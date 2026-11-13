use rmcp::{ServiceExt, model::ClientRequest};
use secrecy::ExposeSecret;
use serde_json::{Value, json};

use super::*;
use crate::commands::{clients, invoices};

struct Fixture {
    _dir: tempfile::TempDir,
    store: FileStore,
    app: InvoiceServer,
    token: Arc<SecretString>,
    drafts: Arc<std::sync::atomic::AtomicUsize>,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::open(dir.path().join("eattura.json")).unwrap();
        let token = Arc::new(load_token(&dir.path().join("mcp-token")).unwrap());
        let drafts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = drafts.clone();
        let app = InvoiceServer::new(store.clone(), AppLog::memory()).with_draft_callback(
            Arc::new(move || {
                counter.fetch_add(1, Ordering::Relaxed);
            }),
        );
        Self {
            _dir: dir,
            store,
            app,
            token,
            drafts,
        }
    }

    async fn rpc(&self, method: &str, params: Value) -> Value {
        let (mut client_io, server_io) = tokio::io::duplex(16 * 1024);
        let app = self.app.clone();
        let token = self.token.clone();
        let task = tokio::spawn(async move { serve_connection(server_io, &token, app).await });
        ipc::authenticate_client(&mut client_io, &self.token)
            .await
            .unwrap();
        let client = ().serve(client_io).await.unwrap();
        let response = if method == "initialize" {
            json!({"result": client.peer_info()})
        } else {
            let request: ClientRequest =
                serde_json::from_value(json!({"method": method, "params": params})).unwrap();
            match client.send_request(request).await {
                Ok(result) => json!({"result": result}),
                Err(_) => json!({"error": "Request rejected"}),
            }
        };
        let _ = client.cancel().await;
        task.abort();
        let _ = task.await;
        response
    }

    async fn call(&self, name: &str, arguments: Value) -> Value {
        self.rpc("tools/call", json!({"name": name, "arguments": arguments}))
            .await
    }

    async fn parties(&self) -> (String, String) {
        let mut ids = Vec::new();
        for (name, tax) in [("Issuer", "01234567890"), ("Customer", "09876543210")] {
            let client = serde_json::from_value(json!({
                "denominazione": name, "idPaese": "IT", "idCodice": tax,
                "regimeFiscale": "RF01", "indirizzo": "Via Roma", "cap": "00100",
                "comune": "Roma", "nazione": "IT", "codiceDestinatario": "0000000"
            }))
            .unwrap();
            ids.push(
                clients::create_client_store(&self.store, client)
                    .await
                    .unwrap()
                    .id,
            );
        }
        (ids.remove(0), ids.remove(0))
    }
}

fn draft(issuer: &str, customer: &str) -> Value {
    json!({"invoice": {
        "numero": "MCP/1", "data": "2026-11-02", "tipoDocumento": "TD01", "divisa": "EUR",
        "cedenteId": issuer, "cessionarioId": customer,
        "linee": [{"descrizione": "Consulting", "quantita": 1, "prezzoUnitario": 100, "aliquotaIva": 22}],
        "pagamenti": []
    }})
}

fn failed(response: &Value) -> bool {
    response.get("error").is_some() || response["result"]["isError"] == true
}

#[tokio::test]
async fn initialize_describes_open_app_requirement_and_tool_allowlist() {
    let fixture = Fixture::new();
    let initialized = fixture
        .rpc(
            "initialize",
            json!({
                "protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0"}
            }),
        )
        .await;
    assert_eq!(
        initialized["result"]["serverInfo"]["description"],
        DESCRIPTION
    );
    assert!(
        initialized["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("open Eattura")
    );
    assert_eq!(initialized["result"]["capabilities"], json!({"tools": {}}));
    let listing = fixture.rpc("tools/list", json!({})).await;
    let tools = listing["result"]["tools"].as_array().unwrap();
    let mut names: Vec<_> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "create_draft",
            "get_app_info",
            "get_client",
            "get_invoice",
            "list_clients",
            "list_invoices",
            "validate_invoice"
        ]
    );
    for tool in tools {
        assert_eq!(
            tool["annotations"]["readOnlyHint"],
            tool["name"] != "create_draft"
        );
        assert_eq!(tool["annotations"]["openWorldHint"], false);
    }
}

#[tokio::test]
async fn tool_calls_create_drafts_but_cannot_send_or_change_status() {
    let fixture = Fixture::new();
    let (issuer, customer) = fixture.parties().await;
    let mut request = draft(&issuer, &customer);
    request["invoice"]["stato"] = json!("sent");
    let response = fixture.call("create_draft", request).await;
    assert!(!failed(&response), "{response}");
    let created = &response["result"]["structuredContent"];
    assert_eq!(created["stato"], "draft");
    assert_eq!(created["sent"], false);
    assert_eq!(fixture.drafts.load(Ordering::Relaxed), 1);
    let id = created["id"].as_str().unwrap();
    for name in [
        "send_invoice_pec",
        "send_invoice",
        "set_invoice_status",
        "update_invoice",
        "delete_invoice",
        "sync_pec",
    ] {
        let response = fixture
            .call(name, json!({"id": id, "status": "sent"}))
            .await;
        assert!(failed(&response), "Forbidden tool accepted: {name}");
    }
    fixture.call("validate_invoice", json!({"id": id})).await;
    let detail = fixture.call("get_invoice", json!({"id": id})).await;
    assert_eq!(detail["result"]["structuredContent"]["stato"], "draft");
    let duplicate = fixture
        .call("create_draft", draft(&issuer, &customer))
        .await;
    assert!(failed(&duplicate));
    assert_eq!(fixture.drafts.load(Ordering::Relaxed), 1);
    assert_eq!(
        invoices::list_invoices_store(&fixture.store)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn other_archives_cannot_read_invoices_or_use_foreign_clients() {
    let owner = Fixture::new();
    let other = Fixture::new();
    let (issuer, customer) = owner.parties().await;
    let created = owner.call("create_draft", draft(&issuer, &customer)).await;
    let id = &created["result"]["structuredContent"]["id"];
    assert!(failed(&other.call("get_invoice", json!({"id": id})).await));
    assert!(failed(
        &other.call("get_client", json!({"id": issuer})).await
    ));
    assert!(failed(
        &other.call("create_draft", draft(&issuer, &customer)).await
    ));
    assert!(
        invoices::list_invoices_store(&other.store)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn pagination_and_argument_validation_are_bounded() {
    let fixture = Fixture::new();
    let (issuer, customer) = fixture.parties().await;
    fixture
        .call("create_draft", draft(&issuer, &customer))
        .await;
    for params in [
        json!({"limit": 0}),
        json!({"limit": 101}),
        json!({"offset": 10001}),
        json!({"send": true}),
    ] {
        assert!(failed(&fixture.call("list_invoices", params).await));
    }
    let response = fixture
        .call(
            "list_invoices",
            json!({"query": "Consulting", "status": "draft", "year": 2026, "limit": 1}),
        )
        .await;
    assert_eq!(response["result"]["structuredContent"]["total"], 1);
    let clients = fixture.call("list_clients", json!({"limit": 1})).await;
    assert_eq!(clients["result"]["structuredContent"]["nextOffset"], 1);
    let clients = fixture
        .call("list_clients", json!({"limit": 1, "offset": 1}))
        .await;
    assert_eq!(
        clients["result"]["structuredContent"]["nextOffset"],
        Value::Null
    );
}

#[tokio::test]
async fn app_info_omits_pec_settings_and_local_tokens() {
    let fixture = Fixture::new();
    fixture.store.insert("settings", json!({"key": "app", "value": json!({
        "company": null, "pec": {"email": "private@pec.example", "imap_host": "private-imap", "imap_port": 993,
            "smtp_host": "private-smtp", "smtp_port": 465}
    }).to_string()})).await.unwrap();
    let response = fixture.call("get_app_info", json!({})).await;
    assert!(!failed(&response));
    assert_eq!(
        response["result"]["structuredContent"]["canSendInvoices"],
        false
    );
    let serialized = response.to_string();
    assert!(!serialized.contains("private@pec.example"));
    assert!(!serialized.contains("private-smtp"));
    assert!(!serialized.contains(fixture.token.expose_secret()));
}

#[tokio::test]
async fn invoice_details_omit_attachment_bytes_and_raw_receipt_xml() {
    let fixture = Fixture::new();
    let (issuer, customer) = fixture.parties().await;
    let mut request = draft(&issuer, &customer);
    request["invoice"]["allegati"] = json!([{
        "nomeAttachment": "details.txt", "attachmentBase64": "cHJpdmF0ZS1hdHRhY2htZW50"
    }]);
    let created = fixture.call("create_draft", request).await;
    assert!(!failed(&created), "{created}");
    let id = created["result"]["structuredContent"]["id"]
        .as_str()
        .unwrap();
    fixture.store.insert("sdi_notifications", json!({
        "id": "receipt", "tipo": "RC", "invoice_id": id, "raw_xml": "<private-receipt/>", "errori": "[]"
    })).await.unwrap();
    let response = fixture.call("get_invoice", json!({"id": id})).await;
    assert!(!failed(&response), "{response}");
    let invoice = &response["result"]["structuredContent"];
    assert_eq!(invoice["allegati"][0]["nomeAttachment"], "details.txt");
    assert_eq!(invoice["notifications"][0]["tipo"], "RC");
    assert!(invoice["allegati"][0].get("attachmentBase64").is_none());
    assert!(invoice["notifications"][0].get("rawXml").is_none());
    let text = response.to_string();
    assert!(!text.contains("cHJpdmF0ZS1hdHRhY2htZW50"));
    assert!(!text.contains("private-receipt"));
}

#[test]
fn token_and_client_configuration_are_stable_and_private() {
    let dir = tempfile::tempdir().unwrap();
    let token_path = dir.path().join("mcp-token");
    let first = load_token(&token_path).unwrap();
    let second = load_token(&token_path).unwrap();
    assert_eq!(first.expose_secret(), second.expose_secret());
    let config_path = dir.path().join("mcp.json");
    write_json(&config_path, &client_config().unwrap()).unwrap();
    write_json(&config_path, &client_config().unwrap()).unwrap();
    let config: Value = serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
    assert_eq!(config, client_config().unwrap());
    assert!(!config.to_string().contains(first.expose_secret()));
    assert!(!config.to_string().contains("Authorization"));
    assert_eq!(
        config["mcpServers"]["eattura-open-app-required"]["args"],
        json!(["--mcp-stdio"])
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [&token_path, &config_path] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    std::fs::write(&token_path, "invalid").unwrap();
    assert!(load_token(&token_path).is_err());
}

#[tokio::test]
async fn oversized_requests_are_rejected() {
    let fixture = Fixture::new();
    let (mut client_io, server_io) = tokio::io::duplex(16 * 1024);
    let app = fixture.app.clone();
    let token = fixture.token.clone();
    let task = tokio::spawn(async move { serve_connection(server_io, &token, app).await });
    ipc::authenticate_client(&mut client_io, &fixture.token)
        .await
        .unwrap();
    let client = ().serve(client_io).await.unwrap();
    let arguments = serde_json::from_value(json!({"query": "x".repeat(MAX_MESSAGE_SIZE)})).unwrap();
    let oversized =
        rmcp::model::CallToolRequestParams::new("list_invoices").with_arguments(arguments);
    assert!(
        tokio::time::timeout(Duration::from_secs(1), client.call_tool(oversized))
            .await
            .is_err()
    );
    let healthy = tokio::time::timeout(Duration::from_secs(1), client.list_tools(None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(healthy.tools.len(), 7);
    assert!(
        invoices::list_invoices_store(&fixture.store)
            .await
            .unwrap()
            .is_empty()
    );
    let _ = client.cancel().await;
    task.abort();
    let _ = task.await;
}
