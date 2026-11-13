use axum::body::Body;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use server_eattura::config::ServerConfig;
use server_eattura::error::AppError;
use server_eattura::handlers::health::health;
use server_eattura::router::create_router;
use server_eattura::state::AppState;
use tower::ServiceExt;

#[tokio::test]
async fn test_health_handler() {
    let res = health().await;
    assert_eq!(res.0["status"], "ok");
    assert_eq!(res.0["mode"], "stateless_passacarte");
}

#[test]
fn test_error_status_mapping() {
    // NotFound -> 404
    let err = AppError::NotFound("item-123".into());
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // BadRequest -> 400
    let err = AppError::BadRequest("invalid date".into());
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Conflict -> 409
    let err = AppError::Conflict("conflict".into());
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // Internal -> 500
    let err = AppError::Internal("unexpected failure".into());
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // Xml -> 422
    let err = AppError::Xml(common::xml::XmlError::MissingElement(
        "FatturaElettronica".into(),
    ));
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Validation -> 422
    let val_result = common::validation::ValidationResult {
        errors: vec![common::validation::ValidationError {
            code: "00400".into(),
            message: "Natura required when Aliquota is 0".into(),
            element_path: Some("DatiBeniServizi.DettaglioLinee".into()),
        }],
    };
    let err = AppError::Validation(val_result);
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_config_env_loading() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 1. Valid custom HOST and PORT
    unsafe {
        std::env::set_var("HOST", "127.0.0.1");
        std::env::set_var("PORT", "9090");
    }
    let config = server_eattura::config::ServerConfig::from_env().unwrap();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 9090);

    // 2. Defaults for HOST and PORT
    unsafe {
        std::env::remove_var("HOST");
        std::env::remove_var("PORT");
    }
    let config = server_eattura::config::ServerConfig::from_env().unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
}

#[tokio::test]
async fn test_stateless_routes_e2e() {
    let state = AppState::new(ServerConfig::default());
    let router = create_router(state);

    // 1. Health check via router
    let req = axum::http::Request::builder()
        .uri("/api/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["mode"], "stateless_passacarte");

    // 2. Tools: Render PDF
    let sample_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:FatturaElettronica xmlns:p="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2" versione="FPR12">
  <FatturaElettronicaHeader>
    <DatiTrasmissione>
      <IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>01234567897</IdCodice></IdTrasmittente>
      <ProgressivoInvio>00001</ProgressivoInvio>
      <FormatoTrasmissione>FPR12</FormatoTrasmissione>
      <CodiceDestinatario>0000000</CodiceDestinatario>
    </DatiTrasmissione>
    <CedentePrestatore>
      <DatiAnagrafici>
        <IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>12345678903</IdCodice></IdFiscaleIVA>
        <Anagrafica><Denominazione>ACME SRL</Denominazione></Anagrafica>
        <RegimeFiscale>RF01</RegimeFiscale>
      </DatiAnagrafici>
      <Sede>
        <Indirizzo>Via Roma 1</Indirizzo>
        <CAP>00100</CAP>
        <Comune>Roma</Comune>
        <Nazione>IT</Nazione>
      </Sede>
    </CedentePrestatore>
    <CessionarioCommittente>
      <DatiAnagrafici>
        <IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>98765432103</IdCodice></IdFiscaleIVA>
        <Anagrafica><Denominazione>CLIENTE SPA</Denominazione></Anagrafica>
      </DatiAnagrafici>
      <Sede>
        <Indirizzo>Via Milano 2</Indirizzo>
        <CAP>20100</CAP>
        <Comune>Milano</Comune>
        <Nazione>IT</Nazione>
      </Sede>
    </CessionarioCommittente>
  </FatturaElettronicaHeader>
  <FatturaElettronicaBody>
    <DatiGenerali>
      <DatiGeneraliDocumento>
        <TipoDocumento>TD01</TipoDocumento>
        <Divisa>EUR</Divisa>
        <Data>2026-03-30</Data>
        <Numero>1</Numero>
        <ImportoTotaleDocumento>122.00</ImportoTotaleDocumento>
      </DatiGeneraliDocumento>
    </DatiGenerali>
    <DatiBeniServizi>
      <DettaglioLinee>
        <NumeroLinea>1</NumeroLinea>
        <Descrizione>Consulenza Software</Descrizione>
        <PrezzoUnitario>100.00</PrezzoUnitario>
        <PrezzoTotale>100.00</PrezzoTotale>
        <AliquotaIVA>22.00</AliquotaIVA>
      </DettaglioLinee>
      <DatiRiepilogo>
        <AliquotaIVA>22.00</AliquotaIVA>
        <ImponibileImporto>100.00</ImponibileImporto>
        <Imposta>22.00</Imposta>
      </DatiRiepilogo>
    </DatiBeniServizi>
  </FatturaElettronicaBody>
</p:FatturaElettronica>"#;

    let pdf_payload = serde_json::json!({ "xml": sample_xml });
    let req = axum::http::Request::builder()
        .uri("/api/tools/render-pdf")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&pdf_payload).unwrap()))
        .unwrap();

    let res = router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("Content-Type").unwrap(),
        "application/pdf"
    );
    let pdf_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    // 3. Tools: Validate
    let validate_payload = serde_json::json!({ "xml": sample_xml });
    let req = axum::http::Request::builder()
        .uri("/api/tools/validate")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&validate_payload).unwrap()))
        .unwrap();

    let res = router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let val_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let val_json: serde_json::Value = serde_json::from_slice(&val_bytes).unwrap();
    assert_eq!(val_json["valid"], true, "{val_json}");

    // 4. Proxy: VIES VAT Lookup (validates parameter requirements and route registration)
    let req = axum::http::Request::builder()
        .uri("/api/proxy/vat-lookup?country_code=&vat_number=")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
