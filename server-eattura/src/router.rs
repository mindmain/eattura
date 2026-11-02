use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{health, pec, tools, vies};
use crate::state::AppState;

/// Build the Axum router for the stateless "passa-carte" server.
///
/// Route map:
/// ```text
/// GET    /api/health                       -> health check
/// GET    /api/proxy/vat-lookup             -> VIES VAT lookup
///
/// POST   /api/proxy/pec/send               -> send invoice via PEC SMTP
/// POST   /api/proxy/pec/poll               -> poll SDI notifications via PEC IMAP
///
/// POST   /api/tools/validate               -> validate invoice XML or JSON against SDI rules
/// POST   /api/tools/convert/xml-to-json     -> decode SDI XML into JSON
/// POST   /api/tools/convert/json-to-xml     -> encode JSON into SDI XML
/// POST   /api/tools/render-pdf             -> generate courtesy PDF stream
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
        .layer(axum::middleware::from_fn(
            crate::middleware::require_api_key,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
