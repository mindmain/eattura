use server_eattura::{config, router, state};

/// Entry point for the Eattura stateless "passa-carte" server.
///
/// Initializes:
/// 1. Tracing subscriber for structured logging
/// 2. Server configuration from environment variables (HOST and PORT)
/// 3. In-memory ephemeral relay state and HTTP router
/// 4. Binds to configured host:port and serves requests
#[tokio::main]
async fn main() {
    // Initialize structured logging with env filter (e.g. RUST_LOG=info).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load server configuration from environment variables.
    let config = config::ServerConfig::from_env().unwrap_or_default();

    tracing::info!(
        "Starting stateless Eattura 'passa-carte' server on {}:{}",
        config.host,
        config.port
    );

    // Build application state and router without database.
    let app_state = state::AppState::new(config.clone());
    let app = router::create_router(app_state);

    // Bind and serve.
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind TCP listener");

    tracing::info!("Stateless server listening on {addr}");

    axum::serve(listener, app).await.expect("Server error");
}
