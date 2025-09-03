use serde::Deserialize;
use std::env;

/// Application configuration loaded from environment variables
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    /// The admin DID for moderation (intentionally hardcoded for security)
    pub admin_did: String,

    /// Owner handle for the default status page
    pub owner_handle: String,

    /// Database URL (defaults to local SQLite)
    pub database_url: String,

    /// OAuth redirect base URL
    pub oauth_redirect_base: String,

    /// Server host
    pub server_host: String,

    /// Server port
    pub server_port: u16,

    /// Enable firehose ingester
    pub enable_firehose: bool,

    /// Log level
    pub log_level: String,

    /// Dev mode for testing with dummy data
    pub dev_mode: bool,
}

impl Config {
    /// Load configuration from environment variables with sensible defaults
    pub fn from_env() -> Result<Self, env::VarError> {
        // Admin DID is intentionally hardcoded as discussed
        let admin_did = "did:plc:xbtmt2zjwlrfegqvch7fboei".to_string();

        Ok(Config {
            admin_did,
            owner_handle: env::var("OWNER_HANDLE").unwrap_or_else(|_| "zzstoatzz.io".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://./statusphere.sqlite3".to_string()),
            oauth_redirect_base: env::var("OAUTH_REDIRECT_BASE")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            enable_firehose: env::var("ENABLE_FIREHOSE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            dev_mode: env::var("DEV_MODE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        })
    }
}
