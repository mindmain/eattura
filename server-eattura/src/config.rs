/// Server configuration loaded from environment variables.
///
/// Expected env vars:
/// - `DATABASE_URL`: PostgreSQL connection string (required)
/// - `HOST`: Bind address (default: "0.0.0.0")
/// - `PORT`: Bind port (default: 3000)
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
}

impl ServerConfig {
    /// Load configuration from environment variables.
    /// Falls back to defaults for HOST and PORT if not set.
    /// Returns an error if DATABASE_URL is not set.
    pub fn from_env() -> Result<Self, std::env::VarError> {
        let database_url = std::env::var("DATABASE_URL")?;
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        Ok(Self {
            database_url,
            host,
            port,
        })
    }
}
