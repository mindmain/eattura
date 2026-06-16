mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
mod router;
mod services;
mod state;

use std::sync::Arc;

/// Entry point for the Eattura server.
///
/// Initializes:
/// 1. Tracing subscriber for structured logging
/// 2. Server configuration from environment variables
/// 3. PostgreSQL connection pool
/// 4. Axum HTTP router with all routes
/// 5. Binds to configured host:port and serves requests
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
    let config = config::ServerConfig::from_env().expect("Failed to load config: DATABASE_URL must be set");

    tracing::info!("Starting server on {}:{}", config.host, config.port);

    // Create the PostgreSQL connection pool.
    let pool = sqlx::PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Run database migrations.
    db::migrations::run_migrations(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database migrations completed");

    // Build application state and router.
    let app_state = state::AppState {
        db: pool,
        config: Arc::new(config.clone()),
    };

    let app = router::create_router(app_state);

    // Bind and serve.
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind TCP listener");

    tracing::info!("Server listening on {addr}");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
