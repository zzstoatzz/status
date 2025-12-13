pub mod models;
pub mod queries;
pub mod webhooks;

pub use models::{AuthSession, AuthState, StatusFromDb};
pub use queries::{get_frequent_emojis, get_user_preferences, save_user_preferences};
pub use webhooks::{
    Webhook, create_webhook, delete_webhook, get_user_webhooks, rotate_webhook_secret,
    update_webhook,
};

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

        // webhooks
        conn.execute(
            "CREATE TABLE IF NOT EXISTS webhooks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            did TEXT NOT NULL,
            url TEXT NOT NULL,
            secret TEXT NOT NULL,
            events TEXT DEFAULT '*',
            active BOOLEAN DEFAULT TRUE,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
            [],
        )
        .unwrap();

        // index for fast lookups by did
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_webhooks_did ON webhooks(did)",
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

        Ok(())
    })
    .await?;
    Ok(())
}
