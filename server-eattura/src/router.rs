use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{clients, health, invoices, pec};
use crate::state::AppState;

/// Build the Axum router with all API routes.
///
/// Route map:
/// ```text
/// GET    /api/health                   -> health check
///
/// GET    /api/invoices                 -> list invoices (paginated)
/// GET    /api/invoices/:id             -> get invoice by ID
/// POST   /api/invoices                 -> create invoice
/// PUT    /api/invoices/:id             -> update invoice
/// DELETE /api/invoices/:id             -> delete invoice
/// POST   /api/invoices/:id/validate    -> validate against SDI rules
/// GET    /api/invoices/:id/xml         -> export as SDI XML
/// POST   /api/invoices/xml/import      -> import from SDI XML
///
/// GET    /api/clients                  -> list clients
/// GET    /api/clients/:id              -> get client by ID
/// POST   /api/clients                  -> create client
/// PUT    /api/clients/:id              -> update client
/// DELETE /api/clients/:id              -> delete client
/// ```
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        // Invoice routes
        .route("/api/invoices", get(invoices::list_invoices).post(invoices::create_invoice))
        .route(
            "/api/invoices/{id}",
            get(invoices::get_invoice)
                .put(invoices::update_invoice)
                .delete(invoices::delete_invoice),
        )
        .route("/api/invoices/{id}/status", post(invoices::set_status))
        .route("/api/invoices/{id}/validate", post(invoices::validate_invoice))
        .route("/api/invoices/{id}/xml", get(invoices::export_xml))
        .route("/api/invoices/{id}/send", post(pec::send_invoice))
        .route("/api/invoices/xml/import", post(invoices::import_xml))
        .route("/api/pec/sync", post(pec::sync))
        // Client routes
        .route("/api/clients", get(clients::list_clients).post(clients::create_client))
        .route(
            "/api/clients/{id}",
            get(clients::get_client)
                .put(clients::update_client)
                .delete(clients::delete_client),
        )
        .layer(axum::middleware::from_fn(crate::middleware::require_api_key))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
