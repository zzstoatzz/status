use serde::Deserialize;
use std::env;

/// Application configuration loaded from environment variables
#[derive(Debug, Clone, Deserialize)]
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
}

impl Config {
    /// Load configuration from environment variables with sensible defaults
    pub fn from_env() -> Result<Self, env::VarError> {
        // Admin DID is intentionally hardcoded as discussed
        let admin_did = "did:plc:xbtmt2zjwlrfegqvch7fboei".to_string();
        
        // Handle backward compatibility with old environment variable names
        let server_host = env::var("SERVER_HOST")
            .or_else(|_| env::var("HOST"))  // Fallback to HOST for Fly.io compatibility
            .unwrap_or_else(|_| "127.0.0.1".to_string());
            
        let server_port = env::var("SERVER_PORT")
            .or_else(|_| env::var("PORT"))  // Fallback to PORT 
            .or_else(|_| env::var("LISTEN_PORT"))  // Also check LISTEN_PORT
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080);
            
        // For oauth_redirect_base, also check PUBLIC_URL for backward compatibility
        let oauth_redirect_base = env::var("OAUTH_REDIRECT_BASE")
            .or_else(|_| env::var("PUBLIC_URL"))  // Fallback to PUBLIC_URL
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        
        Ok(Config {
            admin_did,
            owner_handle: env::var("OWNER_HANDLE").unwrap_or_else(|_| "zzstoatzz.io".to_string()),
            // Support both DATABASE_URL and DB_PATH for backward compatibility
            database_url: env::var("DATABASE_URL")
                .or_else(|_| env::var("DB_PATH").map(|path| {
                    // If DB_PATH doesn't have sqlite:// prefix, add it
                    if path.starts_with("sqlite://") {
                        path
                    } else {
                        format!("sqlite://{}", path)
                    }
                }))
                .unwrap_or_else(|_| "sqlite://./statusphere.sqlite3".to_string()),
            oauth_redirect_base,
            server_host,
            server_port,
            enable_firehose: env::var("ENABLE_FIREHOSE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        })
    }
}