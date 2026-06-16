/// Health check endpoint.
/// Returns 200 OK with a simple JSON status message.
/// Used by load balancers and monitoring to verify the server is running.
pub async fn health() -> &'static str {
    r#"{"status": "ok"}"#
}
