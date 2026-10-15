use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{health, pec, relay, tools, vies};
use crate::state::AppState;

/// Build the Axum router for the stateless "passa-carte" server.
///
/// Route map:
/// ```text
/// GET    /api/health                       -> health check
///
/// POST   /api/proxy/pec/send               -> send invoice via PEC SMTP
/// POST   /api/proxy/pec/poll               -> poll SDI notifications via PEC IMAP
///
/// POST   /api/tools/validate               -> validate invoice XML or JSON against SDI rules
/// POST   /api/tools/convert/xml-to-json     -> decode SDI XML into JSON
/// POST   /api/tools/convert/json-to-xml     -> encode JSON into SDI XML
/// POST   /api/tools/render-pdf             -> generate courtesy PDF stream
/// POST   /api/tools/sign-p7m               -> proxy CAdES-BES PKCS#7 signature
///
/// POST   /api/relay/push                   -> push E2EE encrypted blob to ephemeral topic
/// GET    /api/relay/pull/{topic}           -> pull and consume E2EE encrypted blobs
/// GET    /api/relay/peek/{topic}           -> peek at E2EE encrypted blobs
/// ```
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health route
        .route("/api/health", get(health::health))
        // VIES VAT lookup proxy route
        .route("/api/proxy/vat-lookup", get(vies::vat_lookup))
        // PEC proxy routes
        .route("/api/proxy/pec/send", post(pec::send_pec))
        .route("/api/proxy/pec/poll", post(pec::poll_pec))
        // Tool & transformation routes
        .route("/api/tools/validate", post(tools::validate))
        .route("/api/tools/convert/xml-to-json", post(tools::xml_to_json))
        .route("/api/tools/convert/json-to-xml", post(tools::json_to_xml))
        .route("/api/tools/render-pdf", post(tools::render_pdf))
        .route("/api/tools/sign-p7m", post(tools::sign_p7m))
        // E2EE relay routes (stateless multi-device sync)
        .route("/api/relay/push", post(relay::push))
        .route("/api/relay/pull/{topic}", get(relay::pull))
        .route("/api/relay/peek/{topic}", get(relay::peek))
        .layer(axum::middleware::from_fn(crate::middleware::require_api_key))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
