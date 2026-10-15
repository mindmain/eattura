/// Server configuration loaded from environment variables.
///
/// Expected env vars:
/// - `HOST`: Bind address (default: "0.0.0.0")
/// - `PORT`: Bind port (default: 3000)
/// - `API_KEY`: Optional secret key required in `X-Api-Key` header
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub api_key: Option<String>,
}

impl ServerConfig {
    /// Load configuration from environment variables.
    /// Defaults to 0.0.0.0:3000 if not set.
    pub fn from_env() -> Result<Self, std::env::VarError> {
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);
        let api_key = std::env::var("API_KEY").ok();

        Ok(Self {
            host,
            port,
            api_key,
        })
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            api_key: None,
        }
    }
}
