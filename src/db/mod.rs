pub mod models;
pub mod queries;

pub use models::{AuthSession, AuthState, StatusFromDb, WebhookConfig, WebhookDelivery};
pub use queries::{get_frequent_emojis, get_user_preferences, save_user_preferences};

use async_sqlite::Pool;

/// Creates the tables in the db.
pub async fn create_tables_in_database(pool: &Pool) -> Result<(), async_sqlite::Error> {
    pool.conn(move |conn| {
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();

        // status
        conn.execute(
            "CREATE TABLE IF NOT EXISTS status (
            uri TEXT PRIMARY KEY,
            authorDid TEXT NOT NULL,
            emoji TEXT NOT NULL,
            text TEXT,
            startedAt INTEGER NOT NULL,
            expiresAt INTEGER,
            indexedAt INTEGER NOT NULL
        )",
            [],
        )
        .unwrap();

        // auth_session
        conn.execute(
            "CREATE TABLE IF NOT EXISTS auth_session (
            key TEXT PRIMARY KEY,
            session TEXT NOT NULL
        )",
            [],
        )
        .unwrap();

        // auth_state
        conn.execute(
            "CREATE TABLE IF NOT EXISTS auth_state (
            key TEXT PRIMARY KEY,
            state TEXT NOT NULL
        )",
            [],
        )
        .unwrap();

        // user_preferences
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_preferences (
            did TEXT PRIMARY KEY,
            font_family TEXT DEFAULT 'mono',
            accent_color TEXT DEFAULT '#1DA1F2',
            updated_at INTEGER NOT NULL
        )",
            [],
        )
        .unwrap();

        // Note: custom_emojis table removed - we serve emojis directly from static/emojis/ directory

        // Add indexes for performance optimization
        // Index on startedAt for feed queries (ORDER BY startedAt DESC)
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_status_startedAt ON status(startedAt DESC)",
            [],
        )
        .unwrap();

        // Composite index for user status queries (WHERE authorDid = ? ORDER BY startedAt DESC)
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_status_authorDid_startedAt ON status(authorDid, startedAt DESC)",
            [],
        )
        .unwrap();

        // Add hidden column for moderation (won't error if already exists)
        let _ = conn.execute(
            "ALTER TABLE status ADD COLUMN hidden BOOLEAN DEFAULT FALSE",
            [],
        );

        // webhook_configs table for storing webhook configurations
        conn.execute(
            "CREATE TABLE IF NOT EXISTS webhook_configs (
            id INTEGER PRIMARY KEY,
            user_did TEXT NOT NULL,
            webhook_url TEXT NOT NULL,
            webhook_secret TEXT NOT NULL,
            enabled BOOLEAN DEFAULT TRUE,
            last_delivery_at INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(user_did)
        )",
            [],
        )
        .unwrap();

        // webhook_deliveries table for tracking delivery history
        conn.execute(
            "CREATE TABLE IF NOT EXISTS webhook_deliveries (
            id INTEGER PRIMARY KEY,
            config_id INTEGER NOT NULL,
            event_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL,
            delivered_at INTEGER NOT NULL,
            response_status INTEGER,
            response_body TEXT,
            retry_count INTEGER DEFAULT 0,
            next_retry_at INTEGER,
            success BOOLEAN DEFAULT FALSE,
            FOREIGN KEY(config_id) REFERENCES webhook_configs(id) ON DELETE CASCADE
        )",
            [],
        )
        .unwrap();

        // Add indexes for webhook tables
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_config_id ON webhook_deliveries(config_id)",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_delivered_at ON webhook_deliveries(delivered_at DESC)",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_event_id ON webhook_deliveries(event_id)",
            [],
        )
        .unwrap();

        Ok(())
    })
    .await?;
    Ok(())
}
