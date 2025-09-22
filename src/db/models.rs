use actix_web::web::Data;
use async_sqlite::{
    Pool,
    rusqlite::{Error, Row, types::Type},
};
use atrium_api::types::string::Did;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusFromDb {
    pub uri: String,
    pub author_did: String,
    pub status: String, // Keep for backwards compat, but this is the emoji
    pub text: Option<String>,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub indexed_at: DateTime<Utc>,
    pub handle: Option<String>,
}

impl StatusFromDb {
    /// Creates a new [StatusFromDb]
    pub fn new(uri: String, author_did: String, status: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            uri,
            author_did,
            status,
            text: None,
            started_at: now,
            expires_at: None,
            indexed_at: now,
            handle: None,
        }
    }

    /// Loads a status by its ATProto URI
    pub async fn load_by_uri(
        pool: &Data<Arc<Pool>>,
        uri: &str,
    ) -> Result<Option<Self>, async_sqlite::Error> {
        let target_uri = uri.to_string();
        pool.conn(move |conn| {
            let mut stmt = conn.prepare("SELECT * FROM status WHERE uri = ?1 LIMIT 1")?;
            stmt.query_row([target_uri.as_str()], Self::map_from_row)
                .map(Some)
                .or_else(|err| {
                    if err == async_sqlite::rusqlite::Error::QueryReturnedNoRows {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                })
        })
        .await
    }

    /// Helper to map from [Row] to [StatusDb]
    fn map_from_row(row: &Row) -> Result<Self, async_sqlite::rusqlite::Error> {
        Ok(Self {
            uri: row.get(0)?,
            author_did: row.get(1)?,
            status: row.get(2)?, // emoji
            text: row.get(3)?,
            //DateTimes are stored as INTEGERS then parsed into a DateTime<UTC>
            started_at: {
                let timestamp: i64 = row.get(4)?;
                DateTime::from_timestamp(timestamp, 0).ok_or_else(|| {
                    Error::InvalidColumnType(4, "Invalid timestamp".parse().unwrap(), Type::Text)
                })?
            },
            expires_at: {
                let timestamp: Option<i64> = row.get(5)?;
                timestamp.and_then(|ts| DateTime::from_timestamp(ts, 0))
            },
            //DateTimes are stored as INTEGERS then parsed into a DateTime<UTC>
            indexed_at: {
                let timestamp: i64 = row.get(6)?;
                DateTime::from_timestamp(timestamp, 0).ok_or_else(|| {
                    Error::InvalidColumnType(6, "Invalid timestamp".parse().unwrap(), Type::Text)
                })?
            },
            handle: None,
        })
    }

    /// Check if status is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Saves the [StatusDb]
    pub async fn save(&self, pool: Data<Arc<Pool>>) -> Result<(), async_sqlite::Error> {
        let cloned_self = self.clone();
        pool.conn(move |conn| {
            conn.execute(
                "INSERT INTO status (uri, authorDid, emoji, text, startedAt, expiresAt, indexedAt) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                async_sqlite::rusqlite::params![
                    &cloned_self.uri,
                    &cloned_self.author_did,
                    &cloned_self.status,  // emoji value
                    &cloned_self.text,
                    &cloned_self.started_at.timestamp().to_string(),
                    &cloned_self.expires_at.map(|e| e.timestamp().to_string()),
                    &cloned_self.indexed_at.timestamp().to_string(),
                ],
            )
        })
            .await?;
        Ok(())
    }

    /// Saves or updates a status by its did(uri)
    pub async fn save_or_update(&self, pool: &Pool) -> Result<(), async_sqlite::Error> {
        let cloned_self = self.clone();
        pool.conn(move |conn| {
            //We check to see if the session already exists, if so we need to update not insert
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM status WHERE uri = ?1")?;
            let count: i64 = stmt.query_row([&cloned_self.uri], |row| row.get(0))?;
            match count > 0 {
                true => {
                    let mut update_stmt =
                        conn.prepare("UPDATE status SET emoji = ?2, text = ?3, startedAt = ?4, expiresAt = ?5, indexedAt = ?6 WHERE uri = ?1")?;
                    update_stmt.execute(async_sqlite::rusqlite::params![
                        &cloned_self.uri,
                        &cloned_self.status,
                        &cloned_self.text,
                        &cloned_self.started_at.timestamp().to_string(),
                        &cloned_self.expires_at.map(|e| e.timestamp().to_string()),
                        &cloned_self.indexed_at.timestamp().to_string()
                    ])?;
                    Ok(())
                }
                false => {
                    conn.execute(
                        "INSERT INTO status (uri, authorDid, emoji, text, startedAt, expiresAt, indexedAt) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        async_sqlite::rusqlite::params![
                            &cloned_self.uri,
                            &cloned_self.author_did,
                            &cloned_self.status,  // emoji value
                            &cloned_self.text,
                            &cloned_self.started_at.timestamp().to_string(),
                            &cloned_self.expires_at.map(|e| e.timestamp().to_string()),
                            &cloned_self.indexed_at.timestamp().to_string(),
                        ],
                    )?;
                    Ok(())
                }
            }
        })
        .await?;
        Ok(())
    }

    pub async fn delete_by_uri(pool: &Pool, uri: String) -> Result<(), async_sqlite::Error> {
        pool.conn(move |conn| {
            let mut stmt = conn.prepare("DELETE FROM status WHERE uri = ?1")?;
            stmt.execute([&uri])
        })
        .await?;
        Ok(())
    }

    /// Loads the last 10 statuses we have saved
    #[allow(dead_code)]
    pub async fn load_latest_statuses(
        pool: &Data<Arc<Pool>>,
    ) -> Result<Vec<Self>, async_sqlite::Error> {
        pool
            .conn(move |conn| {
                let mut stmt =
                    conn.prepare("SELECT * FROM status WHERE (hidden IS NULL OR hidden = FALSE) ORDER BY startedAt DESC LIMIT 10")?;
                let status_iter = stmt
                    .query_map([], |row| Ok(Self::map_from_row(row).unwrap()))
                    .unwrap();

                let mut statuses = Vec::new();
                for status in status_iter {
                    statuses.push(status?);
                }
                Ok(statuses)
            })
            .await
    }

    /// Loads paginated statuses for infinite scrolling
    #[allow(dead_code)]
    pub async fn load_statuses_paginated(
        pool: &Data<Arc<Pool>>,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<Self>, async_sqlite::Error> {
        pool
            .conn(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT * FROM status WHERE (hidden IS NULL OR hidden = FALSE) ORDER BY startedAt DESC LIMIT ?1 OFFSET ?2"
                )?;
                let status_iter = stmt
                    .query_map(async_sqlite::rusqlite::params![limit, offset], |row| {
                        Ok(Self::map_from_row(row).unwrap())
                    })
                    .unwrap();

                let mut statuses = Vec::new();
                for status in status_iter {
                    statuses.push(status?);
                }
                Ok(statuses)
            })
            .await
    }

    /// Loads the logged-in users current status
    pub async fn my_status(
        pool: &Data<Arc<Pool>>,
        did: &Did,
    ) -> Result<Option<Self>, async_sqlite::Error> {
        let did = did.to_string();
        pool.conn(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT * FROM status WHERE authorDid = ?1 ORDER BY startedAt DESC LIMIT 1",
            )?;
            stmt.query_row([did.as_str()], Self::map_from_row)
                .map(Some)
                .or_else(|err| {
                    if err == async_sqlite::rusqlite::Error::QueryReturnedNoRows {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                })
        })
        .await
    }

    /// Loads user's status history
    pub async fn load_user_statuses(
        pool: &Data<Arc<Pool>>,
        did: &Did,
        limit: usize,
    ) -> Result<Vec<Self>, async_sqlite::Error> {
        let did = did.to_string();
        pool.conn(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT * FROM status WHERE authorDid = ?1 ORDER BY startedAt DESC LIMIT ?2",
            )?;
            let status_iter = stmt.query_map([did.as_str(), &limit.to_string()], |row| {
                Self::map_from_row(row)
            })?;
            let mut statuses = vec![];
            for status in status_iter {
                statuses.push(status?);
            }
            Ok(statuses)
        })
        .await
    }

    /// ui helper to show a handle or did if the handle cannot be found
    pub fn author_display_name(&self) -> String {
        match self.handle.as_ref() {
            Some(handle) => handle.to_string(),
            None => self.author_did.to_string(),
        }
    }

    /// Friendly emoji label suitable for text-only contexts
    pub fn share_emoji_label(&self) -> String {
        if let Some(name) = self.status.strip_prefix("custom:") {
            format!(":{}:", name)
        } else {
            self.status.clone()
        }
    }

    fn share_caption(&self) -> Option<String> {
        self.text
            .as_ref()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_owned())
    }

    /// Short title combining emoji and handle for link previews
    pub fn share_title(&self) -> String {
        format!(
            "{} @{}",
            self.share_emoji_label(),
            self.author_display_name()
        )
    }

    /// Description prioritizing the freeform text when present
    pub fn share_description(&self) -> String {
        self.share_caption()
            .unwrap_or_else(|| format!("{} shared a status", self.author_display_name()))
    }

    /// Combined share text used for copy/share flows
    pub fn share_text(&self) -> String {
        self.share_caption()
            .map(|caption| format!("{} — {}", self.share_title(), caption))
            .unwrap_or_else(|| self.share_title())
    }

    /// Returns the record key component of the ATProto URI (rkey)
    pub fn record_key(&self) -> Option<String> {
        self.uri.rsplit_once('/').map(|(_, rkey)| rkey.to_string())
    }

    /// Generates the relative share path used by the UI (e.g. `/s/did:plc:abc/rkey`)
    pub fn share_path(&self) -> String {
        let rkey = self.record_key().unwrap_or_default();
        format!("/s/{}/{}", self.author_did, rkey)
    }
}

/// AuthSession table data type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthSession {
    pub key: String,
    pub session: String,
}

impl AuthSession {
    /// Creates a new [AuthSession]
    pub fn new<V>(key: String, session: V) -> Self
    where
        V: Serialize,
    {
        let session = serde_json::to_string(&session).unwrap();
        Self {
            key: key.to_string(),
            session,
        }
    }

    /// Helper to map from [Row] to [AuthSession]
    fn map_from_row(row: &Row) -> Result<Self, Error> {
        let key: String = row.get(0)?;
        let session: String = row.get(1)?;
        Ok(Self { key, session })
    }

    /// Gets a session by the users did(key)
    pub async fn get_by_did(pool: &Pool, did: String) -> Result<Option<Self>, async_sqlite::Error> {
        let did = Did::new(did).unwrap();
        pool.conn(move |conn| {
            let mut stmt = conn.prepare("SELECT * FROM auth_session WHERE key = ?1")?;
            stmt.query_row([did.as_str()], Self::map_from_row)
                .map(Some)
                .or_else(|err| {
                    if err == Error::QueryReturnedNoRows {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                })
        })
        .await
    }

    /// Saves or updates the session by its did(key)
    pub async fn save_or_update(&self, pool: &Pool) -> Result<(), async_sqlite::Error> {
        let cloned_self = self.clone();
        pool.conn(move |conn| {
            //We check to see if the session already exists, if so we need to update not insert
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM auth_session WHERE key = ?1")?;
            let count: i64 = stmt.query_row([&cloned_self.key], |row| row.get(0))?;
            match count > 0 {
                true => {
                    let mut update_stmt =
                        conn.prepare("UPDATE auth_session SET session = ?2 WHERE key = ?1")?;
                    update_stmt.execute([&cloned_self.key, &cloned_self.session])?;
                    Ok(())
                }
                false => {
                    conn.execute(
                        "INSERT INTO auth_session (key, session) VALUES (?1, ?2)",
                        [&cloned_self.key, &cloned_self.session],
                    )?;
                    Ok(())
                }
            }
        })
        .await?;
        Ok(())
    }

    /// Deletes the session by did
    pub async fn delete_by_did(pool: &Pool, did: String) -> Result<(), async_sqlite::Error> {
        pool.conn(move |conn| {
            let mut stmt = conn.prepare("DELETE FROM auth_session WHERE key = ?1")?;
            stmt.execute([&did])
        })
        .await?;
        Ok(())
    }

    /// Deletes all the sessions
    pub async fn delete_all(pool: &Pool) -> Result<(), async_sqlite::Error> {
        pool.conn(move |conn| {
            let mut stmt = conn.prepare("DELETE FROM auth_session")?;
            stmt.execute([])
        })
        .await?;
        Ok(())
    }
}

/// AuthState table datatype
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthState {
    pub key: String,
    pub state: String,
}

impl AuthState {
    /// Creates a new [AuthState]
    pub fn new<V>(key: String, state: V) -> Self
    where
        V: Serialize,
    {
        let state = serde_json::to_string(&state).unwrap();
        Self {
            key: key.to_string(),
            state,
        }
    }

    /// Helper to map from [Row] to [AuthState]
    fn map_from_row(row: &Row) -> Result<Self, Error> {
        let key: String = row.get(0)?;
        let state: String = row.get(1)?;
        Ok(Self { key, state })
    }

    /// Gets a state by the users key
    pub async fn get_by_key(pool: &Pool, key: String) -> Result<Option<Self>, async_sqlite::Error> {
        pool.conn(move |conn| {
            let mut stmt = conn.prepare("SELECT * FROM auth_state WHERE key = ?1")?;
            stmt.query_row([key.as_str()], Self::map_from_row)
                .map(Some)
                .or_else(|err| {
                    if err == Error::QueryReturnedNoRows {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                })
        })
        .await
    }

    /// Saves or updates the state by its key
    pub async fn save_or_update(&self, pool: &Pool) -> Result<(), async_sqlite::Error> {
        let cloned_self = self.clone();
        pool.conn(move |conn| {
            //We check to see if the state already exists, if so we need to update
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM auth_state WHERE key = ?1")?;
            let count: i64 = stmt.query_row([&cloned_self.key], |row| row.get(0))?;
            match count > 0 {
                true => {
                    let mut update_stmt =
                        conn.prepare("UPDATE auth_state SET state = ?2 WHERE key = ?1")?;
                    update_stmt.execute([&cloned_self.key, &cloned_self.state])?;
                    Ok(())
                }
                false => {
                    conn.execute(
                        "INSERT INTO auth_state (key, state) VALUES (?1, ?2)",
                        [&cloned_self.key, &cloned_self.state],
                    )?;
                    Ok(())
                }
            }
        })
        .await?;
        Ok(())
    }

    pub async fn delete_by_key(pool: &Pool, key: String) -> Result<(), async_sqlite::Error> {
        pool.conn(move |conn| {
            let mut stmt = conn.prepare("DELETE FROM auth_state WHERE key = ?1")?;
            stmt.execute([&key])
        })
        .await?;
        Ok(())
    }

    pub async fn delete_all(pool: &Pool) -> Result<(), async_sqlite::Error> {
        pool.conn(move |conn| {
            let mut stmt = conn.prepare("DELETE FROM auth_state")?;
            stmt.execute([])
        })
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub did: String,
    pub font_family: String,
    pub accent_color: String,
    pub updated_at: i64,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            did: String::new(),
            font_family: "mono".to_string(),
            accent_color: "#1DA1F2".to_string(),
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }
}
